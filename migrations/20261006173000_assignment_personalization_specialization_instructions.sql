-- Governed school-level specialization instructions for assignment personalization.
--
-- Editable guidance remains lower priority than the fixed application safety/system
-- boundary. Guidance is snapshotted into durable jobs and is not exposed in job telemetry.

ALTER TABLE public.assignment_personalization_school_policies
    ADD COLUMN IF NOT EXISTS specialization_instructions TEXT NOT NULL
        DEFAULT 'Adapt difficulty, scope, format, and scaffolding to the learner profile while preserving the original learning objective, required knowledge, and grading intent.';

ALTER TABLE public.assignment_personalization_school_policies
    DROP CONSTRAINT IF EXISTS assignment_personalization_school_specialization_instructions;
ALTER TABLE public.assignment_personalization_school_policies
    ADD CONSTRAINT assignment_personalization_school_specialization_instructions CHECK (
        BTRIM(specialization_instructions) <> ''
        AND char_length(specialization_instructions) <= 4000
    );

ALTER TABLE public.assignment_personalization_jobs
    ADD COLUMN IF NOT EXISTS specialization_instructions TEXT NOT NULL
        DEFAULT 'Adapt difficulty, scope, format, and scaffolding to the learner profile while preserving the original learning objective, required knowledge, and grading intent.';

ALTER TABLE public.assignment_personalization_jobs
    DROP CONSTRAINT IF EXISTS assignment_personalization_job_specialization_instructions;
ALTER TABLE public.assignment_personalization_jobs
    ADD CONSTRAINT assignment_personalization_job_specialization_instructions CHECK (
        BTRIM(specialization_instructions) <> ''
        AND char_length(specialization_instructions) <= 4000
    );

CREATE OR REPLACE FUNCTION public.resolve_assignment_personalization_policy_v2(
    p_school_id UUID,
    p_teacher_user_id UUID
)
RETURNS TABLE (
    enabled BOOLEAN,
    paused BOOLEAN,
    llm_profile_id TEXT,
    delivery_policy TEXT,
    specialization_instructions TEXT,
    policy_scope TEXT,
    policy_version INTEGER
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(teacher_override.enabled_override, school_policy.enabled, TRUE)
            ELSE COALESCE(school_policy.enabled, TRUE)
        END,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(teacher_override.paused_override, school_policy.paused, FALSE)
            ELSE COALESCE(school_policy.paused, FALSE)
        END,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(teacher_override.llm_profile_id_override, school_policy.llm_profile_id, 'deepseek-chat-v1')
            ELSE COALESCE(school_policy.llm_profile_id, 'deepseek-chat-v1')
        END,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(teacher_override.delivery_policy_override, school_policy.delivery_policy, 'require_personalized')
            ELSE COALESCE(school_policy.delivery_policy, 'require_personalized')
        END,
        COALESCE(NULLIF(BTRIM(school_policy.specialization_instructions), ''), 'Adapt difficulty, scope, format, and scaffolding to the learner profile while preserving the original learning objective, required knowledge, and grading intent.'),
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override' THEN 'teacher'
            ELSE 'school'
        END,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override' THEN teacher_override.override_version
            ELSE COALESCE(school_policy.policy_version, 1)
        END
    FROM (SELECT 1) AS singleton
    LEFT JOIN public.assignment_personalization_school_policies AS school_policy
      ON school_policy.school_id = p_school_id
    LEFT JOIN public.teachers AS teacher
      ON teacher.user_id = p_teacher_user_id
     AND teacher.school_id = p_school_id
    LEFT JOIN public.assignment_personalization_teacher_overrides AS teacher_override
      ON teacher_override.teacher_id = teacher.id
     AND teacher_override.school_id = p_school_id
    LIMIT 1
$$;
REVOKE ALL ON FUNCTION public.resolve_assignment_personalization_policy_v2(UUID, UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.prevent_assignment_personalization_contract_rewrite()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF NEW.llm_profile_id IS DISTINCT FROM OLD.llm_profile_id
       OR NEW.llm_provider IS DISTINCT FROM OLD.llm_provider
       OR NEW.model_name IS DISTINCT FROM OLD.model_name
       OR NEW.profile_name IS DISTINCT FROM OLD.profile_name
       OR NEW.profile_version IS DISTINCT FROM OLD.profile_version
       OR NEW.policy_scope IS DISTINCT FROM OLD.policy_scope
       OR NEW.policy_version IS DISTINCT FROM OLD.policy_version
       OR NEW.delivery_policy IS DISTINCT FROM OLD.delivery_policy
       OR NEW.specialization_instructions IS DISTINCT FROM OLD.specialization_instructions THEN
        RAISE EXCEPTION 'Assignment personalization execution contract is immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION public.enqueue_assignment_personalization_job()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    INSERT INTO public.assignment_personalization_jobs (
        school_id, assignment_id, student_id, class_section_id, requested_by,
        status, attempt_count, available_at, lease_owner, heartbeat_at,
        last_error_code, last_error_summary, completed_at, idempotency_key,
        model_name, profile_name, profile_version, llm_profile_id, llm_provider,
        policy_scope, policy_version, delivery_policy, specialization_instructions,
        processing_stage
    )
    SELECT
        target.school_id,
        target.assignment_id,
        NEW.student_id,
        target.class_section_id,
        target.requested_by,
        CASE WHEN effective.enabled THEN 'queued' ELSE 'cancelled' END,
        0,
        NOW(),
        NULL,
        NULL,
        CASE WHEN effective.enabled THEN NULL ELSE 'policy_disabled' END,
        CASE WHEN effective.enabled THEN NULL ELSE 'Assignment personalization is disabled by policy' END,
        CASE WHEN effective.enabled THEN NULL ELSE NOW() END,
        concat(target.assignment_id::text, ':', NEW.student_id::text, ':assignment_personalization_v1:1'),
        'deepseek-chat',
        'assignment_personalization_v1',
        1,
        effective.llm_profile_id,
        'deepseek',
        effective.policy_scope,
        effective.policy_version,
        effective.delivery_policy,
        effective.specialization_instructions,
        CASE WHEN NOT effective.enabled THEN 'cancelled'
             WHEN effective.paused THEN 'paused'
             ELSE 'queued' END
    FROM (
        SELECT
            teacher.school_id,
            assignment.id AS assignment_id,
            assignment.class_section_id,
            teacher.user_id AS requested_by
        FROM public.assignments AS assignment
        JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
        JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
        JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
        JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
        JOIN public.teaching_assignments AS teaching_assignment
          ON teaching_assignment.teacher_id = teacher.id
         AND teaching_assignment.class_section_id = assignment.class_section_id
        JOIN public.students AS student ON student.id = NEW.student_id
        JOIN public.users AS student_user ON student_user.id = student.user_id
        JOIN public.enrollments AS enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = assignment.class_section_id
        WHERE assignment.id = NEW.assignment_id
          AND assignment.status = 'Published'::assignment_status
          AND teacher_user.is_active = TRUE
          AND teacher_role.name::text = 'Teacher'
          AND teacher.school_id = class_section.school_id
          AND student.school_id = teacher.school_id
          AND student_user.school_id = teacher.school_id
          AND student_user.is_active = TRUE
          AND NEW.prompt_ctx IS NULL
    ) AS target
    CROSS JOIN LATERAL public.resolve_assignment_personalization_policy_v2(
        target.school_id,
        target.requested_by
    ) AS effective
    ON CONFLICT (assignment_id, student_id, profile_name, profile_version) DO NOTHING;

    RETURN NEW;
END
$$;
REVOKE ALL ON FUNCTION public.enqueue_assignment_personalization_job() FROM PUBLIC;

DROP FUNCTION IF EXISTS public.claim_next_assignment_personalization_job(UUID);

CREATE OR REPLACE FUNCTION public.claim_next_assignment_personalization_job(
    p_worker_id UUID
)
RETURNS TABLE (
    job_id UUID,
    school_id UUID,
    assignment_id UUID,
    student_id UUID,
    requested_by UUID,
    attempt_count INTEGER,
    model_name TEXT,
    profile_name TEXT,
    profile_version INTEGER,
    llm_profile_id TEXT,
    llm_provider TEXT,
    policy_scope TEXT,
    policy_version INTEGER,
    delivery_policy TEXT,
    specialization_instructions TEXT,
    lease_owner UUID
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF public.get_role() <> 'system_job'
       OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS NULL
       OR public.get_user_id() <> p_worker_id THEN
        RAISE EXCEPTION 'bounded assignment personalization queue context required'
            USING ERRCODE = '42501';
    END IF;

    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'succeeded',
        completed_at = COALESCE(job.completed_at, NOW()),
        lease_owner = NULL,
        heartbeat_at = NULL,
        last_error_code = NULL,
        last_error_summary = NULL,
        processing_stage = 'ready'
    FROM public.custom_assignments AS custom_assignment
    WHERE job.status IN ('queued', 'running')
      AND custom_assignment.assignment_id = job.assignment_id
      AND custom_assignment.student_id = job.student_id
      AND custom_assignment.prompt_ctx IS NOT NULL;

    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'cancelled',
        completed_at = NOW(),
        lease_owner = NULL,
        heartbeat_at = NULL,
        last_error_code = 'authorization_revoked',
        last_error_summary = 'Assignment personalization authorization is no longer valid',
        processing_stage = 'cancelled'
    WHERE job.status IN ('queued', 'running')
      AND NOT EXISTS (
          SELECT 1
          FROM public.assignments AS assignment
          JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
          JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
          JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
          JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
          JOIN public.teaching_assignments AS teaching_assignment
            ON teaching_assignment.teacher_id = teacher.id
           AND teaching_assignment.class_section_id = assignment.class_section_id
          JOIN public.students AS student ON student.id = job.student_id
          JOIN public.users AS student_user ON student_user.id = student.user_id
          JOIN public.enrollments AS enrollment
            ON enrollment.student_id = student.id
           AND enrollment.class_section_id = assignment.class_section_id
          JOIN public.custom_assignments AS custom_assignment
            ON custom_assignment.assignment_id = assignment.id
           AND custom_assignment.student_id = student.id
          WHERE assignment.id = job.assignment_id
            AND assignment.status = 'Published'::assignment_status
            AND teacher.user_id = job.requested_by
            AND teacher_user.is_active = TRUE
            AND teacher_role.name::text = 'Teacher'
            AND teacher.school_id = job.school_id
            AND class_section.school_id = job.school_id
            AND student.school_id = job.school_id
            AND student_user.school_id = job.school_id
            AND student_user.is_active = TRUE
            AND custom_assignment.prompt_ctx IS NULL
      );

    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'cancelled',
        completed_at = NOW(),
        lease_owner = NULL,
        heartbeat_at = NULL,
        last_error_code = 'policy_disabled',
        last_error_summary = 'Assignment personalization is disabled by policy',
        processing_stage = 'cancelled'
    WHERE job.status IN ('queued', 'running')
      AND NOT (
          SELECT effective.enabled
          FROM public.resolve_assignment_personalization_policy(
              job.school_id,
              job.requested_by
          ) AS effective
      );

    RETURN QUERY
    WITH candidate AS (
        SELECT job.id
        FROM public.assignment_personalization_jobs AS job
        JOIN public.assignments AS assignment ON assignment.id = job.assignment_id
        JOIN public.teachers AS teacher ON teacher.id = assignment.teacher_id
        JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
        JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
        JOIN public.class_sections AS class_section ON class_section.id = assignment.class_section_id
        JOIN public.teaching_assignments AS teaching_assignment
          ON teaching_assignment.teacher_id = teacher.id
         AND teaching_assignment.class_section_id = assignment.class_section_id
        JOIN public.students AS student ON student.id = job.student_id
        JOIN public.users AS student_user ON student_user.id = student.user_id
        JOIN public.enrollments AS enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = assignment.class_section_id
        JOIN public.custom_assignments AS custom_assignment
          ON custom_assignment.assignment_id = assignment.id
         AND custom_assignment.student_id = student.id
        CROSS JOIN LATERAL public.resolve_assignment_personalization_policy(
            job.school_id,
            job.requested_by
        ) AS effective
        WHERE job.status = 'queued'
          AND job.available_at <= NOW()
          AND effective.enabled
          AND NOT effective.paused
          AND assignment.status = 'Published'::assignment_status
          AND teacher.user_id = job.requested_by
          AND teacher_user.is_active = TRUE
          AND teacher_role.name::text = 'Teacher'
          AND teacher.school_id = job.school_id
          AND class_section.school_id = job.school_id
          AND student.school_id = job.school_id
          AND student_user.school_id = job.school_id
          AND student_user.is_active = TRUE
          AND custom_assignment.prompt_ctx IS NULL
        ORDER BY job.available_at, job.created_at
        FOR UPDATE OF job SKIP LOCKED
        LIMIT 1
    )
    UPDATE public.assignment_personalization_jobs AS job
    SET status = 'running',
        attempt_count = job.attempt_count + 1,
        started_at = COALESCE(job.started_at, NOW()),
        lease_owner = p_worker_id,
        heartbeat_at = NOW(),
        last_error_code = NULL,
        last_error_summary = NULL,
        processing_stage = 'authorizing'
    FROM candidate
    WHERE job.id = candidate.id
    RETURNING
        job.id,
        job.school_id,
        job.assignment_id,
        job.student_id,
        job.requested_by,
        job.attempt_count,
        job.model_name,
        job.profile_name,
        job.profile_version,
        job.llm_profile_id,
        job.llm_provider,
        job.policy_scope,
        job.policy_version,
        job.delivery_policy,
        job.specialization_instructions,
        job.lease_owner;
END
$$;
REVOKE EXECUTE ON FUNCTION public.claim_next_assignment_personalization_job(UUID) FROM PUBLIC;
