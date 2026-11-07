-- Bounded durable AI grading queue lifecycle. Pure scheduling; the inference
-- executor, model egress and teacher approval remain separate changes.
-- Never expose job metadata to Student, Parent or a direct application query.
-- All publicly callable entry points require a transaction-scoped system_job
-- context, elevated=true, no school and an exact non-nil worker identity.

CREATE OR REPLACE FUNCTION edutalent_internal.ai_grading_job_current(
    p_job_id UUID
)
RETURNS BOOLEAN
LANGUAGE SQL SECURITY DEFINER
SET search_path = pg_catalog, public
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM public.ai_grading_jobs job
        JOIN public.submissions s
          ON s.id = job.submission_id
         AND s.revision = job.submission_revision
        JOIN public.custom_assignments ca
          ON ca.id = s.custom_assignment_id AND ca.student_id = s.student_id
        JOIN public.assignments a ON a.id = ca.assignment_id
        JOIN public.class_sections cs ON cs.id = a.class_section_id
        JOIN public.teachers t
          ON t.id = a.teacher_id AND t.school_id = cs.school_id
        JOIN public.users teacher_user
          ON teacher_user.id = t.user_id
         AND teacher_user.school_id = cs.school_id
        JOIN public.roles teacher_role ON teacher_role.id = teacher_user.role_id
        JOIN public.teaching_assignments teaching
          ON teaching.teacher_id = t.id
         AND teaching.class_section_id = a.class_section_id
        JOIN public.students student
          ON student.id = ca.student_id AND student.school_id = cs.school_id
        JOIN public.users student_user
          ON student_user.id = student.user_id
         AND student_user.school_id = cs.school_id
        JOIN public.enrollments enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = a.class_section_id
        JOIN public.ai_grading_rubric_approvals approval
          ON approval.id = job.rubric_approval_id
         AND approval.custom_assignment_id = ca.id
         AND approval.teacher_id = t.id
         AND approval.school_id = cs.school_id
         AND approval.grade_scale = s.grade_scale
        WHERE job.id = p_job_id
          AND cs.school_id = job.school_id
          AND approval.revoked_at IS NULL
          AND a.status = 'Published'::public.assignment_status
          AND ca.status = 'Submitted'::public.custom_status
          AND ca.submitted_at IS NOT NULL AND ca.graded_at IS NULL
          AND s.submitted_at IS NOT NULL
          AND s.grade IS NULL AND s.graded_by IS NULL
          AND teacher_user.is_active
          AND teacher_role.name::TEXT = 'Teacher'
          AND student_user.is_active
          AND jsonb_typeof(s.content->'text') = 'string'
          AND btrim(s.content->>'text') <> ''
          AND octet_length(s.content->>'text') <= 32000
          AND btrim(COALESCE(
              NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
              a.body
          )) <> ''
          AND octet_length(COALESCE(
              NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
              a.body
          )) <= 16000
          AND NOT EXISTS (
              SELECT 1
              FROM public.submission_attachments attachment
              WHERE attachment.custom_assignment_id = ca.id
                AND attachment.student_id = ca.student_id
                AND attachment.school_id = cs.school_id
                AND attachment.status <> 'removed'
          )
          AND NOT EXISTS (
              SELECT 1
              FROM public.ai_grading_suggestions suggestion
              WHERE suggestion.submission_id = job.submission_id
                AND suggestion.submission_revision = job.submission_revision
                AND suggestion.rubric_approval_id = job.rubric_approval_id
          )
    );
$$;
REVOKE ALL ON FUNCTION edutalent_internal.ai_grading_job_current(UUID)
    FROM PUBLIC;

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

    -- A crash after saving immutable evidence but before marking job success
    -- is safe: acknowledge the already-stored result instead of reusing tokens.
    UPDATE public.ai_grading_jobs job
    SET status='succeeded', completed_at=NOW(), lease_owner=NULL,
        lease_until=NULL, last_error_code=NULL
    WHERE job.status IN ('queued', 'running')
      AND EXISTS (
          SELECT 1 FROM public.ai_grading_suggestions suggestion
          WHERE suggestion.submission_id = job.submission_id
            AND suggestion.submission_revision = job.submission_revision
            AND suggestion.rubric_approval_id = job.rubric_approval_id
      );

    -- Enqueue is not authority. Cancel any now-invalid queued request before
    -- claiming, including changed enrollment, teacher, grade or attachments.
    UPDATE public.ai_grading_jobs job
    SET status='cancelled', completed_at=NOW(), lease_owner=NULL,
        lease_until=NULL, last_error_code='authorization_revoked'
    WHERE job.status='queued'
      AND NOT edutalent_internal.ai_grading_job_current(job.id);

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
          AND edutalent_internal.ai_grading_job_current(job.id)
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
                 AND edutalent_internal.ai_grading_job_current(job.id)
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
                 AND edutalent_internal.ai_grading_job_current(job.id)
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
                WHEN EXISTS (
                    SELECT 1 FROM public.ai_grading_suggestions suggestion
                    WHERE suggestion.submission_id=job.submission_id
                      AND suggestion.submission_revision=job.submission_revision
                      AND suggestion.rubric_approval_id=job.rubric_approval_id
                ) THEN 'succeeded'
                WHEN job.attempt_count < 5
                     AND edutalent_internal.ai_grading_job_current(job.id)
                THEN 'queued'
                ELSE 'failed'
            END,
            available_at=NOW(),
            lease_owner=NULL, lease_until=NULL,
            completed_at=CASE
                WHEN job.attempt_count < 5
                     AND edutalent_internal.ai_grading_job_current(job.id)
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

CREATE OR REPLACE FUNCTION public.complete_ai_grading_job(
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
    -- The immutable suggestion must already have been inserted by an
    -- authorized teacher-scoped operation and is not itself a final grade.
    UPDATE public.ai_grading_jobs job
    SET status='succeeded', completed_at=NOW(), lease_owner=NULL,
        lease_until=NULL, last_error_code=NULL
    WHERE job.id=p_job_id
      AND job.status='running'
      AND job.lease_owner=p_worker_id
      AND job.lease_until > NOW()
      AND EXISTS (
          SELECT 1
          FROM public.ai_grading_suggestions suggestion
          WHERE suggestion.submission_id=job.submission_id
            AND suggestion.submission_revision=job.submission_revision
            AND suggestion.rubric_approval_id=job.rubric_approval_id
      );
    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected = 1;
END;
$$;
REVOKE ALL ON FUNCTION public.complete_ai_grading_job(UUID,UUID) FROM PUBLIC;

COMMENT ON FUNCTION public.claim_next_ai_grading_job(UUID) IS
    'Bounded system_job queue claim. Never grade or publish during claim.';
COMMENT ON FUNCTION public.complete_ai_grading_job(UUID,UUID) IS
    'Acknowledge immutable pending-review evidence for an owned valid lease.';
