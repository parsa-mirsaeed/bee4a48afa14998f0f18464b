-- P0 native PDF grading: prior text-only SQL gates rejected all
-- submitted attachments at both immutable-suggestion INSERT and Teacher
-- accept/reject. Preserve the original assigned-Teacher, live revision,
-- authorized rubric, lifecycle, grade atomicity and audit checks; widen
-- only the source predicate for one finalized PDF at an opted-in school.
-- Original PDF bytes are never OCR'd or stored in grading evidence.
-- This migration does NOT enable a school, a worker or a model.

-- This source predicate is only used inside the existing authoritative
-- Teacher, tenant, rubric and revision guards; it is not a user-facing API.
CREATE OR REPLACE FUNCTION edutalent_internal.ai_grading_review_source_allowed(
    p_submission_id UUID,
    p_school_id UUID,
    p_custom_assignment_id UUID,
    p_student_id UUID
)
RETURNS BOOLEAN
LANGUAGE SQL STABLE SECURITY DEFINER
SET search_path = pg_catalog, public
AS $source$
    SELECT
        -- Preserve the existing text-only path without extending its privileges.
        NOT EXISTS (
            SELECT 1 FROM public.submission_attachments a
            WHERE a.submission_id=p_submission_id
              AND a.status='submitted'
        )
        OR (
            EXISTS (
                SELECT 1 FROM public.ai_grading_pdf_school_optins optin
                WHERE optin.school_id=p_school_id AND optin.enabled IS TRUE
            )
            AND (
                SELECT COUNT(*) FROM public.submission_attachments a
                WHERE a.custom_assignment_id=p_custom_assignment_id
                  AND a.student_id=p_student_id
                  AND a.school_id=p_school_id
                  AND a.status <> 'removed'
            ) = 1
            AND EXISTS (
                SELECT 1 FROM public.submission_attachments pdf
                WHERE pdf.submission_id=p_submission_id
                  AND pdf.custom_assignment_id=p_custom_assignment_id
                  AND pdf.student_id=p_student_id
                  AND pdf.school_id=p_school_id
                  AND pdf.status='submitted'
                  AND pdf.media_type='application/pdf'
                  AND pdf.byte_size BETWEEN 1 AND 1000000
                  AND pdf.sha256 ~ '^[0-9a-f]{64}$'
                  AND pdf.verified_at IS NOT NULL
                  AND pdf.finalized_at IS NOT NULL
            )
        );
$source$;
REVOKE ALL ON FUNCTION edutalent_internal.ai_grading_review_source_allowed(
    UUID, UUID, UUID, UUID
) FROM PUBLIC;

CREATE OR REPLACE FUNCTION edutalent_internal.guard_ai_grading_suggestion_authority()
RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    approval RECORD;
    criterion JSONB;
    identifier TEXT;
    expected_points NUMERIC;
    awarded NUMERIC;
    possible NUMERIC;
    awarded_sum NUMERIC := 0;
    seen_ids TEXT[] := ARRAY[]::TEXT[];
    matching_count INTEGER;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'AI grading suggestions cannot be deleted'
            USING ERRCODE = '23514';
    END IF;
    -- Review and publication have not yet been exposed through an audited
    -- teacher decision API. Deny any UPDATE, including privileged DB writes,
    -- until that separate authority path is explicitly implemented.
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'AI grading review must use the future audited decision API'
            USING ERRCODE = '23514';
    END IF;
    IF public.get_role() IS DISTINCT FROM 'Teacher'
        OR public.get_elevated_operation()
        OR public.get_user_id() IS NULL
        OR public.get_school_id() IS NULL THEN
        RAISE EXCEPTION 'AI suggestion requires assigned teacher session'
            USING ERRCODE = '42501';
    END IF;

    SELECT approval_row.id, approval_row.school_id, approval_row.teacher_id,
           approval_row.grade_scale, approval_row.rubric_fingerprint,
           approval_row.criteria, ca.graded_at, ca.status, ca.submitted_at,
           s.grade, s.graded_by, s.grade_scale AS submission_scale,
           s.submitted_at AS submission_submitted_at, s.revision
    INTO approval
    FROM public.ai_grading_rubric_approvals approval_row
    JOIN public.submissions s
      ON s.custom_assignment_id = approval_row.custom_assignment_id
    JOIN public.custom_assignments ca
      ON ca.id = s.custom_assignment_id AND ca.student_id = s.student_id
    JOIN public.assignments a ON a.id = ca.assignment_id
    JOIN public.class_sections cs ON cs.id = a.class_section_id
    JOIN public.teachers t
      ON t.id = a.teacher_id AND t.school_id = cs.school_id
    JOIN public.users u ON u.id = t.user_id AND u.school_id = cs.school_id
    JOIN public.roles role ON role.id = u.role_id
    JOIN public.teaching_assignments ta
      ON ta.teacher_id = t.id AND ta.class_section_id = a.class_section_id
    JOIN public.students student
      ON student.id = ca.student_id AND student.school_id = cs.school_id
    JOIN public.users student_user
      ON student_user.id = student.user_id
     AND student_user.school_id = cs.school_id
    JOIN public.enrollments enrollment
      ON enrollment.student_id = student.id
     AND enrollment.class_section_id = a.class_section_id
    WHERE approval_row.id = NEW.rubric_approval_id
      AND approval_row.revoked_at IS NULL
      AND s.id = NEW.submission_id
      AND cs.school_id = NEW.school_id
      AND approval_row.school_id = cs.school_id
      AND approval_row.teacher_id = t.id
      AND u.id = public.get_user_id()
      AND u.is_active AND student_user.is_active
      AND role.name::TEXT = 'Teacher'
      AND a.status = 'Published'::public.assignment_status
      AND cs.school_id = public.get_school_id()
      AND edutalent_internal.ai_grading_review_source_allowed(
          s.id, cs.school_id, ca.id, ca.student_id
      )
    FOR SHARE OF approval_row, s;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Suggestion lacks current assigned-teacher rubric authority'
            USING ERRCODE = '23514';
    END IF;
    IF approval.grade_scale IS DISTINCT FROM NEW.grade_scale
       OR approval.submission_scale IS DISTINCT FROM NEW.grade_scale
       OR approval.rubric_fingerprint IS DISTINCT FROM NEW.rubric_fingerprint
       OR approval.revision IS DISTINCT FROM NEW.submission_revision
       OR approval.grade IS NOT NULL OR approval.graded_by IS NOT NULL
       OR approval.graded_at IS NOT NULL
       OR approval.status IS DISTINCT FROM 'Submitted'::public.custom_status
       OR approval.submitted_at IS NULL OR approval.submission_submitted_at IS NULL THEN
        RAISE EXCEPTION 'AI grading suggestion has invalid rubric or submission provenance'
            USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT'
       AND (NEW.status <> 'pending_review'
            OR NEW.reviewed_by IS NOT NULL OR NEW.reviewed_at IS NOT NULL) THEN
        RAISE EXCEPTION 'AI-generated grades must start pending teacher review'
            USING ERRCODE = '23514';
    END IF;

    IF jsonb_typeof(NEW.criteria) IS DISTINCT FROM 'array'
       OR jsonb_array_length(NEW.criteria) <> jsonb_array_length(approval.criteria) THEN
        RAISE EXCEPTION 'AI criterion count differs from teacher rubric'
            USING ERRCODE = '23514';
    END IF;
    FOR criterion IN SELECT value FROM jsonb_array_elements(NEW.criteria) AS element(value) LOOP
        IF jsonb_typeof(criterion) IS DISTINCT FROM 'object'
           OR (criterion ?& ARRAY['criterion_id', 'awarded_points', 'possible_points', 'evidence']) IS NOT TRUE
           OR (SELECT COUNT(*) FROM jsonb_object_keys(criterion)) <> 4
           OR jsonb_typeof(criterion->'criterion_id') IS DISTINCT FROM 'string'
           OR jsonb_typeof(criterion->'awarded_points') IS DISTINCT FROM 'number'
           OR jsonb_typeof(criterion->'possible_points') IS DISTINCT FROM 'number'
           OR jsonb_typeof(criterion->'evidence') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'Malformed AI criterion evidence' USING ERRCODE = '23514';
        END IF;
        identifier := criterion->>'criterion_id';
        possible := (criterion->>'possible_points')::NUMERIC;
        awarded := (criterion->>'awarded_points')::NUMERIC;
        IF identifier = ANY(seen_ids)
           OR awarded < 0 OR awarded > possible
           OR length(criterion->>'evidence') > 1000
           OR btrim(criterion->>'evidence') = '' THEN
            RAISE EXCEPTION 'Invalid duplicate or out-of-bounds AI criterion'
                USING ERRCODE = '23514';
        END IF;
        SELECT COUNT(*) INTO matching_count
        FROM jsonb_array_elements(approval.criteria) AS approved(value)
        WHERE approved.value->>'criterion_id' = identifier
          AND abs((approved.value->>'possible_points')::NUMERIC - possible) <= 0.000001;
        IF matching_count <> 1 THEN
            RAISE EXCEPTION 'AI criterion differs from teacher-approved points'
                USING ERRCODE = '23514';
        END IF;
        seen_ids := array_append(seen_ids, identifier);
        awarded_sum := awarded_sum + awarded;
    END LOOP;
    IF abs(awarded_sum - NEW.suggested_grade::NUMERIC) > 0.000001 THEN
        RAISE EXCEPTION 'AI grade total differs from rubric criterion evidence'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.guard_ai_grading_suggestion_authority() FROM PUBLIC;

CREATE OR REPLACE FUNCTION edutalent_internal.guard_ai_grading_decision()
RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER
SET search_path=pg_catalog,public
AS $$
DECLARE
    valid_relation BOOLEAN;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'AI grading decisions are immutable'
            USING ERRCODE='23514';
    END IF;
    IF current_user IS DISTINCT FROM pg_get_userbyid(
         (SELECT relowner FROM pg_class
          WHERE oid='public.ai_grading_decisions'::regclass))
       OR public.get_role() IS DISTINCT FROM 'Teacher'
       OR public.get_elevated_operation()
       OR public.get_user_id() IS NULL
       OR public.get_school_id() IS DISTINCT FROM NEW.school_id THEN
        RAISE EXCEPTION 'AI grading decisions require the audited teacher boundary'
            USING ERRCODE='42501';
    END IF;

    SELECT TRUE INTO valid_relation
    FROM public.ai_grading_suggestions g
    JOIN public.submissions s ON s.id=g.submission_id
    JOIN public.custom_assignments ca ON ca.id=s.custom_assignment_id
    JOIN public.ai_grading_rubric_approvals approval
      ON approval.id=g.rubric_approval_id
    JOIN public.assignments a ON a.id=ca.assignment_id
    JOIN public.class_sections cs ON cs.id=a.class_section_id
    JOIN public.teachers t ON t.id=a.teacher_id
    JOIN public.users u ON u.id=t.user_id AND u.is_active
    JOIN public.roles role ON role.id=u.role_id
    JOIN public.teaching_assignments ta
      ON ta.teacher_id=t.id AND ta.class_section_id=a.class_section_id
    JOIN public.students student ON student.id=ca.student_id
    JOIN public.users student_user
      ON student_user.id=student.user_id AND student_user.is_active
    JOIN public.enrollments enrollment
      ON enrollment.student_id=student.id
     AND enrollment.class_section_id=a.class_section_id
    WHERE g.id=NEW.suggestion_id
      AND g.submission_id=NEW.submission_id
      AND g.school_id=NEW.school_id
      AND g.status='pending_review'
      AND s.revision=g.submission_revision
      AND s.grade IS NULL AND s.graded_by IS NULL
      AND ca.graded_at IS NULL AND ca.status='Submitted'::public.custom_status
      AND a.status='Published'::public.assignment_status
      AND approval.id=g.rubric_approval_id
      AND approval.revoked_at IS NULL
      AND approval.rubric_fingerprint=g.rubric_fingerprint
      AND approval.teacher_id=t.id
      AND approval.school_id=NEW.school_id
      AND cs.school_id=NEW.school_id
      AND t.school_id=NEW.school_id
      AND student.school_id=NEW.school_id
      AND student_user.school_id=NEW.school_id
      AND u.school_id=NEW.school_id
      AND u.id=public.get_user_id()
      AND role.name::text='Teacher'
      AND t.id=NEW.teacher_id
      AND s.grade_scale=g.grade_scale
      AND edutalent_internal.ai_grading_review_source_allowed(
          s.id, cs.school_id, ca.id, ca.student_id
      )
      AND (
          (NEW.decision='accept'
           AND NEW.final_grade IS NOT DISTINCT FROM g.suggested_grade
           AND NEW.feedback IS NOT DISTINCT FROM g.feedback)
          OR (NEW.decision='reject'
              AND NEW.final_grade IS NULL AND NEW.feedback IS NULL)
      );
    IF valid_relation IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'AI grading decision has stale or unauthorized evidence'
            USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.guard_ai_grading_decision() FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.decide_ai_grading_suggestion(
    p_suggestion_id UUID,
    p_action TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER
SET search_path=pg_catalog,public
AS $$
DECLARE
    evidence RECORD;
    updated_rows INTEGER;
BEGIN
    IF p_suggestion_id IS NULL OR p_action NOT IN ('accept','reject')
       OR public.get_role() IS DISTINCT FROM 'Teacher'
       OR public.get_elevated_operation()
       OR public.get_user_id() IS NULL
       OR public.get_school_id() IS NULL THEN
        RAISE EXCEPTION 'teacher decision context required'
            USING ERRCODE='42501';
    END IF;

    -- Lock the canonical source and model evidence before deciding.
    -- The request transaction rolls back all writes on any error.
    SELECT g.id AS suggestion_id, g.submission_id, g.school_id,
           g.suggested_grade, g.grade_scale, g.feedback,
           s.id AS submission_id_locked, s.custom_assignment_id,
           t.id AS teacher_id
    INTO evidence
    FROM public.ai_grading_suggestions g
    JOIN public.submissions s ON s.id=g.submission_id
    JOIN public.custom_assignments ca ON ca.id=s.custom_assignment_id
    JOIN public.assignments a ON a.id=ca.assignment_id
    JOIN public.class_sections cs ON cs.id=a.class_section_id
    JOIN public.teachers t
      ON t.id=a.teacher_id AND t.school_id=cs.school_id
    JOIN public.users u
      ON u.id=t.user_id AND u.school_id=cs.school_id AND u.is_active
    JOIN public.roles role ON role.id=u.role_id
    JOIN public.teaching_assignments ta
      ON ta.teacher_id=t.id AND ta.class_section_id=a.class_section_id
    JOIN public.students student
      ON student.id=ca.student_id AND student.school_id=cs.school_id
    JOIN public.users student_user
      ON student_user.id=student.user_id
     AND student_user.school_id=cs.school_id AND student_user.is_active
    JOIN public.enrollments enrollment
      ON enrollment.student_id=student.id
     AND enrollment.class_section_id=a.class_section_id
    JOIN public.ai_grading_rubric_approvals approval
      ON approval.id=g.rubric_approval_id
     AND approval.teacher_id=t.id AND approval.school_id=cs.school_id
    WHERE g.id=p_suggestion_id
      AND g.status='pending_review'
      AND NOT EXISTS (
          SELECT 1 FROM public.ai_grading_decisions decision
          WHERE decision.suggestion_id=g.id
      )
      AND g.school_id=public.get_school_id()
      AND u.id=public.get_user_id()
      AND role.name::text='Teacher'
      AND s.revision=g.submission_revision
      AND s.grade_scale=g.grade_scale
      AND s.grade IS NULL AND s.graded_by IS NULL
      AND ca.status='Submitted'::public.custom_status
      AND ca.graded_at IS NULL AND ca.submitted_at IS NOT NULL
      AND s.submitted_at IS NOT NULL
      AND a.status='Published'::public.assignment_status
      AND approval.revoked_at IS NULL
      AND approval.rubric_fingerprint=g.rubric_fingerprint
      AND edutalent_internal.ai_grading_review_source_allowed(
          s.id, cs.school_id, ca.id, ca.student_id
      )
    FOR UPDATE OF g,s,ca,approval;

    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    INSERT INTO public.ai_grading_decisions(
        suggestion_id,submission_id,school_id,teacher_id,
        decision,final_grade,feedback
    ) VALUES (
        evidence.suggestion_id,evidence.submission_id,evidence.school_id,
        evidence.teacher_id,p_action,
        CASE WHEN p_action='accept' THEN evidence.suggested_grade ELSE NULL END,
        CASE WHEN p_action='accept' THEN evidence.feedback ELSE NULL END
    );

    IF p_action='accept' THEN
        UPDATE public.submissions
        SET grade=evidence.suggested_grade, feedback=evidence.feedback,
            graded_by=evidence.teacher_id
        WHERE id=evidence.submission_id
          AND grade IS NULL AND graded_by IS NULL;
        GET DIAGNOSTICS updated_rows=ROW_COUNT;
        IF updated_rows <> 1 THEN
            RAISE EXCEPTION 'AI grade publication lost concurrency race'
                USING ERRCODE='40001';
        END IF;

        UPDATE public.custom_assignments
        SET status='Graded'::public.custom_status,graded_at=NOW()
        WHERE id=evidence.custom_assignment_id
          AND status='Submitted'::public.custom_status
          AND graded_at IS NULL;
        GET DIAGNOSTICS updated_rows=ROW_COUNT;
        IF updated_rows <> 1 THEN
            RAISE EXCEPTION 'AI grading status publication lost concurrency race'
                USING ERRCODE='40001';
        END IF;
    END IF;
    RETURN TRUE;
END;
$$;
REVOKE ALL ON FUNCTION public.decide_ai_grading_suggestion(UUID,TEXT) FROM PUBLIC;
