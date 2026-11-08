-- Guarded, evidence-aware worker completion. This neither reviews suggestions
-- nor changes official student grades. Global job actors receive only the
-- assigned active teacher UUID, never submission content or model evidence.

-- FORCE RLS remains intact. The extra SELECT policy is accessible only when a
-- SECURITY DEFINER owned by the suggestion table owner runs with an elevated,
-- no-school system-job context. It does not grant any runtime/Data API rights.
CREATE POLICY ai_grading_suggestion_owner_completion_select
ON public.ai_grading_suggestions FOR SELECT
USING (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid = 'public.ai_grading_suggestions'::regclass)
    )
    AND public.get_role() = 'system_job'
    AND public.get_elevated_operation()
    AND public.get_school_id() IS NULL
);

CREATE OR REPLACE FUNCTION public.resolve_ai_grading_job_teacher(
    p_worker_id UUID,
    p_job_id UUID
)
RETURNS UUID
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    teacher_user_id UUID;
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);
    -- This discovery step uses metadata only and does not read the student's
    -- submission or teacher-only AI suggestions. The following Teacher-scoped
    -- AuthorizedTx must check every actual content/eligibility condition.
    SELECT u.id INTO teacher_user_id
    FROM public.ai_grading_jobs j
    JOIN public.ai_grading_rubric_approvals a
      ON a.id=j.rubric_approval_id AND a.school_id=j.school_id
     AND a.revoked_at IS NULL
    JOIN public.teachers t
      ON t.id=a.teacher_id AND t.school_id=j.school_id
    JOIN public.users u
      ON u.id=t.user_id AND u.school_id=j.school_id AND u.is_active
    JOIN public.roles r ON r.id=u.role_id
    WHERE j.id=p_job_id
      AND j.status='running'
      AND j.lease_owner=p_worker_id
      AND j.lease_until > NOW()
      AND r.name::text = 'Teacher';
    RETURN teacher_user_id;
END;
$$;
REVOKE ALL ON FUNCTION public.resolve_ai_grading_job_teacher(UUID,UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.complete_ai_grading_job(
    p_worker_id UUID,
    p_job_id UUID,
    p_suggestion_id UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    affected INTEGER;
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);
    IF p_suggestion_id IS NULL
       OR p_suggestion_id = '00000000-0000-0000-0000-000000000000'::UUID THEN
        RETURN FALSE;
    END IF;

    -- The teacher's previous scoped transaction has already committed
    -- immutable evidence. Verify its exact job identity and current state
    -- again; never trust an arbitrary caller-provided "success" flag.
    UPDATE public.ai_grading_jobs j
    SET status='succeeded',
        completed_at=NOW(),
        lease_owner=NULL,
        lease_until=NULL,
        last_error_code=NULL
    WHERE j.id=p_job_id
      AND j.status='running'
      AND j.lease_owner=p_worker_id
      AND j.lease_until > NOW()
      AND EXISTS (
          SELECT 1 FROM public.ai_grading_suggestions g
          JOIN public.submissions s ON s.id=g.submission_id
          JOIN public.ai_grading_rubric_approvals a
            ON a.id=g.rubric_approval_id
          WHERE g.id=p_suggestion_id
            AND g.school_id=j.school_id
            AND g.submission_id=j.submission_id
            AND g.submission_revision=j.submission_revision
            AND g.rubric_approval_id=j.rubric_approval_id
            AND g.status IN ('pending_review','accepted')
            AND s.revision=j.submission_revision
            AND s.grade IS NULL
            AND s.graded_by IS NULL
            AND a.revoked_at IS NULL
            AND a.school_id=j.school_id
      );
    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected = 1;
END;
$$;
REVOKE ALL ON FUNCTION public.complete_ai_grading_job(UUID,UUID,UUID) FROM PUBLIC;

COMMENT ON FUNCTION public.complete_ai_grading_job(UUID,UUID,UUID)
IS 'System-job-only exact-lease completion backed by revision and teacher-scoped suggestion evidence; never publishes a grade.';
