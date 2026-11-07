-- Controlled teacher-only persistence for non-authoritative AI suggestions.
-- Every stored artifact is bound to a separately approved teacher rubric.
-- No UPDATE or DELETE policy is granted: publishing/review is a distinct gate.
ALTER TABLE public.ai_grading_suggestions
ADD COLUMN rubric_approval_id UUID
    REFERENCES public.ai_grading_rubric_approvals(id) ON DELETE RESTRICT;
CREATE INDEX ai_grading_suggestions_approval
    ON public.ai_grading_suggestions (rubric_approval_id);

-- This policy only admits the current, active, assigned teacher of an enrolled
-- student's published assignment. This is defense-in-depth alongside the
-- privileged-write integrity trigger below.
CREATE POLICY ai_grading_suggestion_teacher_select
ON public.ai_grading_suggestions FOR SELECT
USING (
    public.get_role() = 'Teacher'
    AND NOT public.get_elevated_operation()
    AND public.get_user_id() IS NOT NULL
    AND school_id = public.get_school_id()
    AND EXISTS (
        SELECT 1
        FROM public.submissions s
        JOIN public.custom_assignments ca
          ON ca.id = s.custom_assignment_id AND ca.student_id = s.student_id
        JOIN public.assignments a ON a.id = ca.assignment_id
        JOIN public.class_sections cs ON cs.id = a.class_section_id
        JOIN public.teachers t ON t.id = a.teacher_id AND t.school_id = cs.school_id
        JOIN public.users u ON u.id = t.user_id AND u.school_id = cs.school_id
        JOIN public.roles role ON role.id = u.role_id
        JOIN public.students student ON student.id = ca.student_id
            AND student.school_id = cs.school_id
        JOIN public.enrollments enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = a.class_section_id
        JOIN public.teaching_assignments ta
          ON ta.teacher_id = t.id AND ta.class_section_id = a.class_section_id
        WHERE s.id = ai_grading_suggestions.submission_id
          AND u.id = public.get_user_id() AND u.is_active
          AND role.name::TEXT = 'Teacher'
          AND cs.school_id = ai_grading_suggestions.school_id
          AND a.status = 'Published'::public.assignment_status
    )
);

CREATE POLICY ai_grading_suggestion_teacher_insert
ON public.ai_grading_suggestions FOR INSERT
WITH CHECK (
    public.get_role() = 'Teacher'
    AND NOT public.get_elevated_operation()
    AND public.get_user_id() IS NOT NULL
    AND school_id = public.get_school_id()
    AND status = 'pending_review'
    AND reviewed_by IS NULL
    AND reviewed_at IS NULL
    AND EXISTS (
        SELECT 1
        FROM public.ai_grading_rubric_approvals approval
        JOIN public.submissions s
          ON s.custom_assignment_id = approval.custom_assignment_id
        JOIN public.custom_assignments ca
          ON ca.id = approval.custom_assignment_id AND ca.student_id = s.student_id
        JOIN public.assignments a ON a.id = ca.assignment_id
        JOIN public.class_sections cs ON cs.id = a.class_section_id
        JOIN public.teachers t ON t.id = a.teacher_id
          AND t.school_id = cs.school_id
        JOIN public.users u ON u.id = t.user_id AND u.school_id = cs.school_id
        JOIN public.roles role ON role.id = u.role_id
        JOIN public.teaching_assignments ta
          ON ta.teacher_id = t.id AND ta.class_section_id = a.class_section_id
        JOIN public.students student
          ON student.id = ca.student_id AND student.school_id = cs.school_id
        JOIN public.users student_user
          ON student_user.id = student.user_id AND student_user.school_id = cs.school_id
        JOIN public.enrollments enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = a.class_section_id
        WHERE approval.id = ai_grading_suggestions.rubric_approval_id
          AND s.id = ai_grading_suggestions.submission_id
          AND approval.revoked_at IS NULL
          AND approval.school_id = ai_grading_suggestions.school_id
          AND approval.teacher_id = t.id
          AND approval.rubric_fingerprint = ai_grading_suggestions.rubric_fingerprint
          AND approval.grade_scale = ai_grading_suggestions.grade_scale
          AND s.revision = ai_grading_suggestions.submission_revision
          AND s.grade IS NULL AND s.graded_by IS NULL
          AND ca.graded_at IS NULL
          AND ca.status = 'Submitted'::public.custom_status
          AND ca.submitted_at IS NOT NULL
          AND s.submitted_at IS NOT NULL
          AND a.status = 'Published'::public.assignment_status
          AND u.id = public.get_user_id() AND u.is_active
          AND student_user.is_active
          AND role.name::TEXT = 'Teacher'
          AND cs.school_id = ai_grading_suggestions.school_id
    )
);

-- Even privileged direct SQL must not insert suggestions for the wrong
-- approval, wrong version, wrong teacher, wrong school or incomplete submission.
-- The independently installed revision/tenant/terminal-review guards stay
-- active. The approval row lock makes revocation serialize with insertion.
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
    IF TG_OP = 'UPDATE'
        AND NEW.rubric_approval_id IS DISTINCT FROM OLD.rubric_approval_id THEN
        RAISE EXCEPTION 'AI suggestion rubric approval provenance is immutable'
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
      AND NOT EXISTS (
          SELECT 1 FROM public.submission_attachments attachment
          WHERE attachment.submission_id = s.id
            AND attachment.custom_assignment_id = ca.id
            AND attachment.student_id = ca.student_id
            AND attachment.school_id = cs.school_id
            AND attachment.status = 'submitted'
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
           OR NOT criterion ?& ARRAY['criterion_id', 'awarded_points', 'possible_points', 'evidence']
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
CREATE TRIGGER ai_grading_suggestion_authority_guard
BEFORE INSERT OR UPDATE OR DELETE ON public.ai_grading_suggestions
FOR EACH ROW EXECUTE FUNCTION edutalent_internal.guard_ai_grading_suggestion_authority();
