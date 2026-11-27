-- ET-UX-04: parent-first serialization for AI review versus a new attempt.
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
    -- Lock the assignment first, as student finalization and manual grading do.
    -- Then read the complete authorization and revision predicates again after
    -- waiting; do not carry stale suggestion evidence across the lock.
    PERFORM ca.id
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
    FOR UPDATE OF ca;
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;

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
    FOR UPDATE OF s,g,approval;

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
