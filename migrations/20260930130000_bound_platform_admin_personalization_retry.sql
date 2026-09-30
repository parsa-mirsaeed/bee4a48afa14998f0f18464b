-- Bound Platform Admin retry to the exact mutable queue fields required for
-- recovery. The runtime role remains unable to UPDATE personalization jobs
-- directly under RLS; this function validates the transaction-local actor and
-- performs only an eligible failed/cancelled -> queued transition.
CREATE OR REPLACE FUNCTION public.retry_assignment_personalization_jobs_admin(
    p_job_id UUID DEFAULT NULL,
    p_school_id UUID DEFAULT NULL,
    p_teacher_user_id UUID DEFAULT NULL,
    p_assignment_id UUID DEFAULT NULL,
    p_limit INTEGER DEFAULT 100
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    retried BIGINT;
    bounded_limit INTEGER := LEAST(GREATEST(COALESCE(p_limit, 100), 1), 100);
BEGIN
    IF public.get_role() <> 'PlatformAdmin'
       OR public.get_user_id() IS NULL
       OR COALESCE(public.get_elevated_operation(), FALSE) THEN
        RAISE EXCEPTION 'PlatformAdmin personalization retry context required'
            USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.users AS actor
        JOIN public.roles AS actor_role ON actor_role.id = actor.role_id
        WHERE actor.id = public.get_user_id()
          AND actor.is_active = TRUE
          AND actor_role.name::text = 'PlatformAdmin'
    ) THEN
        RAISE EXCEPTION 'Active PlatformAdmin actor required'
            USING ERRCODE = '42501';
    END IF;

    WITH candidates AS (
        SELECT job.id
        FROM public.assignment_personalization_jobs AS job
        JOIN public.assignments AS assignment
          ON assignment.id = job.assignment_id
        JOIN public.custom_assignments AS custom_assignment
          ON custom_assignment.assignment_id = job.assignment_id
         AND custom_assignment.student_id = job.student_id
        CROSS JOIN LATERAL public.resolve_assignment_personalization_policy(
            job.school_id,
            job.requested_by
        ) AS effective
        WHERE job.status IN ('failed', 'cancelled')
          AND assignment.status = 'Published'::assignment_status
          AND custom_assignment.prompt_ctx IS NULL
          AND effective.enabled
          AND NOT effective.paused
          AND (p_job_id IS NULL OR job.id = p_job_id)
          AND (p_school_id IS NULL OR job.school_id = p_school_id)
          AND (p_teacher_user_id IS NULL OR job.requested_by = p_teacher_user_id)
          AND (p_assignment_id IS NULL OR job.assignment_id = p_assignment_id)
        ORDER BY job.created_at, job.id
        FOR UPDATE OF job SKIP LOCKED
        LIMIT bounded_limit
    )
    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'queued',
        attempt_count = 0,
        available_at = NOW(),
        lease_owner = NULL,
        heartbeat_at = NULL,
        completed_at = NULL,
        last_error_code = NULL,
        last_error_summary = NULL,
        processing_stage = 'queued'
    FROM candidates
    WHERE job.id = candidates.id;

    GET DIAGNOSTICS retried = ROW_COUNT;
    RETURN retried;
END
$$;

REVOKE ALL
ON FUNCTION public.retry_assignment_personalization_jobs_admin(
    UUID, UUID, UUID, UUID, INTEGER
)
FROM PUBLIC;

COMMENT ON FUNCTION public.retry_assignment_personalization_jobs_admin(
    UUID, UUID, UUID, UUID, INTEGER
) IS
    'Retries at most 100 eligible failed/cancelled personalization jobs for a validated PlatformAdmin scope without granting direct table UPDATE access.';
