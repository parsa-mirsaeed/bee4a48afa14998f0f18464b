-- Bounded durable AI grading queue lifecycle. Pure scheduling; the inference
-- executor, evidence-aware completion, model egress and teacher approval remain
-- separate changes. This layer never reads the teacher-RLS suggestion table
-- or claims to authorize student content from a global system-job context.
-- Never expose job metadata to Student, Parent or a direct application query.
-- All publicly callable entry points require a transaction-scoped system_job
-- context, elevated=true, no school and an exact non-nil worker identity.

CREATE OR REPLACE FUNCTION edutalent_internal.require_ai_grading_worker(
    p_worker_id UUID
)
RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF p_worker_id IS NULL
       OR p_worker_id = '00000000-0000-0000-0000-000000000000'::UUID
       OR public.get_role() IS DISTINCT FROM 'system_job'
       OR public.get_elevated_operation() IS DISTINCT FROM TRUE
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS DISTINCT FROM p_worker_id THEN
        RAISE EXCEPTION 'bounded AI grading worker context required'
            USING ERRCODE = '42501';
    END IF;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.require_ai_grading_worker(UUID)
    FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.claim_next_ai_grading_job(
    p_worker_id UUID
)
RETURNS TABLE (
    job_id UUID,
    school_id UUID,
    submission_id UUID,
    submission_revision UUID,
    rubric_approval_id UUID,
    attempt_count INTEGER,
    lease_owner UUID,
    lease_until TIMESTAMPTZ
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);

    -- This global queue scheduler sees only opaque job metadata. The actual
    -- worker MUST validate teacher, school, revision, rubric, enrollment,
    -- attachments and grading state under an authorized teacher transaction
    -- before model egress; invalid work is then explicitly cancelled.

    UPDATE public.ai_grading_jobs job
    SET status='failed', completed_at=NOW(), lease_owner=NULL,
        lease_until=NULL, last_error_code='retry_limit'
    WHERE job.status='queued' AND job.attempt_count >= 5;

    RETURN QUERY
    WITH candidate AS (
        SELECT job.id
        FROM public.ai_grading_jobs job
        WHERE job.status='queued'
          AND job.available_at <= NOW()
          AND job.attempt_count < 5
        ORDER BY job.available_at, job.created_at, job.id
        FOR UPDATE OF job SKIP LOCKED
        LIMIT 1
    )
    UPDATE public.ai_grading_jobs job
    SET status='running',
        attempt_count=job.attempt_count + 1,
        started_at=COALESCE(job.started_at, NOW()),
        lease_owner=p_worker_id,
        lease_until=NOW() + INTERVAL '180 seconds',
        completed_at=NULL,
        last_error_code=NULL
    FROM candidate
    WHERE job.id=candidate.id
    RETURNING job.id, job.school_id, job.submission_id,
              job.submission_revision, job.rubric_approval_id,
              job.attempt_count, job.lease_owner, job.lease_until;
END;
$$;
REVOKE ALL ON FUNCTION public.claim_next_ai_grading_job(UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.heartbeat_ai_grading_job(
    p_worker_id UUID,
    p_job_id UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    affected INTEGER;
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);
    UPDATE public.ai_grading_jobs
    SET lease_until=NOW() + INTERVAL '180 seconds'
    WHERE id=p_job_id
      AND status='running'
      AND lease_owner=p_worker_id
      AND lease_until > NOW();
    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected = 1;
END;
$$;
REVOKE ALL ON FUNCTION public.heartbeat_ai_grading_job(UUID,UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.record_ai_grading_job_failure(
    p_worker_id UUID,
    p_job_id UUID,
    p_error_code TEXT,
    p_retryable BOOLEAN,
    p_retry_after_seconds INTEGER
)
RETURNS TEXT
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    result_status TEXT;
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);
    IF p_error_code IS NULL
       OR p_error_code !~ '^[a-z0-9_]{1,64}$'
       OR p_retryable IS NULL
       OR p_retry_after_seconds IS NULL THEN
        RAISE EXCEPTION 'invalid AI grading failure classification'
            USING ERRCODE = '23514';
    END IF;

    UPDATE public.ai_grading_jobs job
    SET status = CASE
            WHEN p_retryable AND job.attempt_count < 5
                THEN 'queued'
            ELSE 'failed'
        END,
        available_at = NOW() + make_interval(
            secs=>GREATEST(5,LEAST(p_retry_after_seconds,3600))::DOUBLE PRECISION
        ),
        lease_owner=NULL,
        lease_until=NULL,
        completed_at=CASE
            WHEN p_retryable AND job.attempt_count < 5
                THEN NULL
            ELSE NOW()
        END,
        last_error_code=p_error_code
    WHERE job.id=p_job_id
      AND job.status='running'
      AND job.lease_owner=p_worker_id
      AND job.lease_until > NOW()
    RETURNING job.status INTO result_status;
    RETURN result_status;
END;
$$;
REVOKE ALL ON FUNCTION
    public.record_ai_grading_job_failure(UUID,UUID,TEXT,BOOLEAN,INTEGER)
    FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.recover_expired_ai_grading_jobs(
    p_worker_id UUID
)
RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    affected BIGINT;
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);
    WITH recovered AS (
        UPDATE public.ai_grading_jobs job
        SET status=CASE
                WHEN job.attempt_count < 5
                THEN 'queued'
                ELSE 'failed'
            END,
            available_at=NOW(),
            lease_owner=NULL, lease_until=NULL,
            completed_at=CASE
                WHEN job.attempt_count < 5
                THEN NULL
                ELSE NOW()
            END,
            last_error_code='lease_expired'
        WHERE job.status='running'
          AND (job.lease_until IS NULL OR job.lease_until <= NOW())
        RETURNING 1
    )
    SELECT COUNT(*) INTO affected FROM recovered;
    RETURN affected;
END;
$$;
REVOKE ALL ON FUNCTION public.recover_expired_ai_grading_jobs(UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.cancel_ai_grading_job(
    p_worker_id UUID,
    p_job_id UUID,
    p_reason TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $
DECLARE
    affected INTEGER;
BEGIN
    PERFORM edutalent_internal.require_ai_grading_worker(p_worker_id);
    IF p_reason NOT IN (
        'authorization_revoked', 'stale_submission',
        'rubric_revoked', 'attachment_unsupported', 'already_graded'
    ) THEN
        RAISE EXCEPTION 'invalid AI grading cancellation reason'
            USING ERRCODE = '23514';
    END IF;
    UPDATE public.ai_grading_jobs
    SET status='cancelled', completed_at=NOW(), lease_owner=NULL,
        lease_until=NULL, last_error_code=p_reason
    WHERE id=p_job_id
      AND status='running'
      AND lease_owner=p_worker_id
      AND lease_until > NOW();
    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected = 1;
END;
$;
REVOKE ALL ON FUNCTION public.cancel_ai_grading_job(UUID,UUID,TEXT) FROM PUBLIC;

COMMENT ON FUNCTION public.claim_next_ai_grading_job(UUID) IS
    'Bounded system_job queue claim. Never grade or publish during claim.';
