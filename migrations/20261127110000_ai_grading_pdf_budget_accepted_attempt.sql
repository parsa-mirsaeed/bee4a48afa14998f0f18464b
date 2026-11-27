-- ET-UX-04: retain all existing durable request/byte/cost limits and the
-- operator-only school opt-in. Reserve budget only for the accepted original
-- belonging to the exact revision, not another attempt or unfinished draft.
CREATE OR REPLACE FUNCTION public.reserve_ai_grading_pdf_budget(
    p_school_id UUID,
    p_submission_id UUID,
    p_revision UUID,
    p_rubric_approval_id UUID,
    p_pdf_bytes BIGINT
)
RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $budget$
DECLARE
    policy_row edutalent_internal.ai_grading_pdf_budget_policy%ROWTYPE;
    reserved BOOLEAN := FALSE;
BEGIN
    IF p_school_id IS NULL OR p_submission_id IS NULL
       OR p_revision IS NULL OR p_rubric_approval_id IS NULL
       OR p_pdf_bytes IS NULL OR p_pdf_bytes NOT BETWEEN 1 AND 1000000
       OR public.get_role() IS DISTINCT FROM 'Teacher'
       OR public.get_elevated_operation() IS TRUE
       OR public.get_user_id() IS NULL
       OR public.get_school_id() IS DISTINCT FROM p_school_id THEN
        RETURN FALSE;
    END IF;

    -- Never let a direct caller reserve another teacher/school's submission
    -- or an unapproved, stale, graded or incomplete document.
    IF NOT EXISTS (
        SELECT 1
        FROM public.submissions s
        JOIN public.custom_assignments ca ON ca.id = s.custom_assignment_id
          AND ca.student_id = s.student_id
        LEFT JOIN LATERAL (
            SELECT h.attempt_number, h.submission_id, h.submission_revision
            FROM public.submission_attempts h
            WHERE h.custom_assignment_id=ca.id
            ORDER BY h.attempt_number DESC LIMIT 1
        ) accepted ON TRUE
        JOIN public.assignments a ON a.id = ca.assignment_id
        JOIN public.class_sections cs ON cs.id = a.class_section_id
        JOIN public.teachers t ON t.id = a.teacher_id AND t.school_id = cs.school_id
        JOIN public.users tu ON tu.id = t.user_id AND tu.school_id = cs.school_id AND tu.is_active
        JOIN public.roles role ON role.id = tu.role_id AND role.name::text = 'Teacher'
        JOIN public.teaching_assignments ta ON ta.teacher_id = t.id
          AND ta.class_section_id = a.class_section_id
        JOIN public.students st ON st.id = ca.student_id AND st.school_id = cs.school_id
        JOIN public.users su ON su.id = st.user_id AND su.school_id = cs.school_id AND su.is_active
        JOIN public.enrollments e ON e.student_id = st.id AND e.class_section_id = a.class_section_id
        JOIN public.ai_grading_rubric_approvals approval
          ON approval.custom_assignment_id = ca.id
         AND approval.teacher_id = t.id AND approval.school_id = cs.school_id
         AND approval.revoked_at IS NULL AND approval.grade_scale = s.grade_scale
        JOIN public.submission_attachments attachment
          ON attachment.submission_id = s.id
         AND attachment.custom_assignment_id = ca.id
         AND attachment.student_id = ca.student_id
         AND attachment.school_id = cs.school_id
         AND attachment.status = 'submitted'
         AND attachment.media_type = 'application/pdf'
        WHERE cs.school_id = p_school_id
          AND s.id = p_submission_id
          AND s.revision = p_revision
          AND (accepted.attempt_number IS NULL OR (
              accepted.submission_id=s.id AND accepted.submission_revision=s.revision
          ))
          AND (accepted.attempt_number IS NULL OR EXISTS (
              SELECT 1 FROM public.submission_attempt_originals original
              WHERE original.custom_assignment_id=ca.id
                AND original.attempt_number=accepted.attempt_number
                AND original.submission_id=s.id
                AND original.attachment_id=attachment.id
          ))
          AND approval.id = p_rubric_approval_id
          AND tu.id = public.get_user_id()
          AND a.status = 'Published'::public.assignment_status
          AND ca.status = 'Submitted'::public.custom_status
          AND ca.submitted_at IS NOT NULL AND s.submitted_at IS NOT NULL
          AND ca.graded_at IS NULL AND s.grade IS NULL AND s.graded_by IS NULL
          AND attachment.byte_size = p_pdf_bytes
          AND (SELECT COUNT(*) FROM public.submission_attachments sibling
               WHERE sibling.custom_assignment_id = ca.id
                 AND sibling.student_id = ca.student_id
                 AND sibling.school_id = cs.school_id
                 AND sibling.status <> 'removed'
                 AND (accepted.attempt_number IS NULL OR EXISTS (
                     SELECT 1 FROM public.submission_attempt_originals original
                     WHERE original.custom_assignment_id=ca.id
                       AND original.attempt_number=accepted.attempt_number
                       AND original.submission_id=s.id
                       AND original.attachment_id=sibling.id
                 ))) = 1
    ) THEN
        RETURN FALSE;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM public.ai_grading_pdf_school_optins optin
        WHERE optin.school_id = p_school_id AND optin.enabled
    ) THEN
        RETURN FALSE;
    END IF;

    SELECT * INTO policy_row
    FROM edutalent_internal.ai_grading_pdf_budget_policy
    WHERE school_id = p_school_id;
    IF NOT FOUND
       OR p_pdf_bytes > policy_row.max_input_bytes_per_utc_day THEN
        RETURN FALSE;
    END IF;

    -- Conflict update locks the row; two workers or replicas cannot both
    -- reserve the last available unit. This reservation commits in an
    -- independent RLS transaction BEFORE provider egress and is not refunded
    -- on network errors, preventing ambiguous retries from bypassing the cap.
    INSERT INTO edutalent_internal.ai_grading_pdf_budget_usage AS usage (
        school_id, utc_day, request_count, input_bytes, reserved_microusd
    ) VALUES (
        p_school_id, (NOW() AT TIME ZONE 'UTC')::DATE,
        1, p_pdf_bytes, policy_row.reserved_microusd_per_request
    )
    ON CONFLICT (school_id, utc_day) DO UPDATE
        SET request_count = usage.request_count + 1,
            input_bytes = usage.input_bytes + EXCLUDED.input_bytes,
            reserved_microusd = usage.reserved_microusd + EXCLUDED.reserved_microusd
        WHERE usage.request_count < policy_row.max_requests_per_utc_day
          AND usage.input_bytes + EXCLUDED.input_bytes <= policy_row.max_input_bytes_per_utc_day
          AND usage.reserved_microusd + EXCLUDED.reserved_microusd
              <= policy_row.max_reserved_microusd_per_utc_day
    RETURNING TRUE INTO reserved;
    RETURN COALESCE(reserved, FALSE);
END;
$budget$;
REVOKE ALL ON FUNCTION public.reserve_ai_grading_pdf_budget(
    UUID, UUID, UUID, UUID, BIGINT
) FROM PUBLIC;

COMMENT ON FUNCTION public.reserve_ai_grading_pdf_budget(UUID,UUID,UUID,UUID,BIGINT) IS
    'Actor-authorized, default-deny atomic PDF request/byte/precharged-budget admission. Must commit before LLM provider egress. Reserved cost is NOT independently verified provider invoice cost.';
