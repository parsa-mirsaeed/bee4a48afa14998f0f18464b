-- Teacher-controlled AI grading decisions. Keep original model evidence
-- immutable: decisions are append-only and are NOT grade publications
-- themselves. The audited function below publishes an accepted grade
-- transactionally, preserving the submission's authoritative grading scale.

CREATE TABLE public.ai_grading_decisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    suggestion_id UUID NOT NULL UNIQUE
        REFERENCES public.ai_grading_suggestions(id) ON DELETE RESTRICT,
    submission_id UUID NOT NULL REFERENCES public.submissions(id) ON DELETE RESTRICT,
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE RESTRICT,
    teacher_id UUID NOT NULL REFERENCES public.teachers(id) ON DELETE RESTRICT,
    decision TEXT NOT NULL CHECK (decision IN ('accept','reject')),
    final_grade DOUBLE PRECISION,
    feedback TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        (decision='accept' AND final_grade IS NOT NULL
         AND feedback IS NOT NULL)
        OR (decision='reject' AND final_grade IS NULL
            AND feedback IS NULL)
    )
);
CREATE INDEX ai_grading_decisions_school_time
ON public.ai_grading_decisions(school_id, created_at DESC);
ALTER TABLE public.ai_grading_decisions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ai_grading_decisions FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.ai_grading_decisions FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname='anon') THEN
        REVOKE ALL ON public.ai_grading_decisions FROM anon;
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname='authenticated') THEN
        REVOKE ALL ON public.ai_grading_decisions FROM authenticated;
    END IF;
END $$;

-- Direct Teacher writes are unavailable even if the runtime role has table
-- grants: only the audited SECURITY DEFINER can insert a decision.
CREATE POLICY ai_grading_decisions_owner_select ON public.ai_grading_decisions
FOR SELECT USING (
    current_user=pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid='public.ai_grading_decisions'::regclass))
);
CREATE POLICY ai_grading_decisions_owner_insert ON public.ai_grading_decisions
FOR INSERT WITH CHECK (
    current_user=pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid='public.ai_grading_decisions'::regclass))
);
CREATE POLICY ai_grading_decisions_teacher_select ON public.ai_grading_decisions
FOR SELECT USING (
    public.get_role()='Teacher'
    AND NOT public.get_elevated_operation()
    AND school_id=public.get_school_id()
    AND EXISTS (
        SELECT 1 FROM public.teachers t
        JOIN public.users u ON u.id=t.user_id AND u.is_active
        JOIN public.ai_grading_suggestions suggestion
          ON suggestion.id=ai_grading_decisions.suggestion_id
        WHERE t.id=ai_grading_decisions.teacher_id
          AND t.school_id=ai_grading_decisions.school_id
          AND u.id=public.get_user_id()
          AND suggestion.school_id=ai_grading_decisions.school_id
          AND suggestion.submission_id=ai_grading_decisions.submission_id
    )
);

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
      AND NOT EXISTS (
          SELECT 1 FROM public.submission_attachments f
          WHERE f.submission_id=s.id AND f.status='submitted'
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
CREATE TRIGGER ai_grading_decisions_guard
BEFORE INSERT OR UPDATE OR DELETE ON public.ai_grading_decisions
FOR EACH ROW EXECUTE FUNCTION edutalent_internal.guard_ai_grading_decision();

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
      AND NOT EXISTS (
          SELECT 1 FROM public.submission_attachments f
          WHERE f.submission_id=s.id AND f.status='submitted'
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

COMMENT ON FUNCTION public.decide_ai_grading_suggestion(UUID,TEXT)
IS 'Assigned Teacher accept/reject only, append-only decision; accepted grade is published atomically on original scale.';
