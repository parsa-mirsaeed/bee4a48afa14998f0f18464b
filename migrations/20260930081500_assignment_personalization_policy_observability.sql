-- Governed school/teacher policy and durable observability for assignment personalization.
--
-- Provider credentials and raw prompts/responses are intentionally absent. The
-- queue persists only the approved execution contract plus bounded diagnostics.

CREATE TABLE IF NOT EXISTS public.assignment_personalization_school_policies (
    school_id UUID PRIMARY KEY REFERENCES public.schools(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    paused BOOLEAN NOT NULL DEFAULT FALSE,
    llm_profile_id TEXT NOT NULL DEFAULT 'deepseek-chat-v1'
        CHECK (llm_profile_id = 'deepseek-chat-v1'),
    delivery_policy TEXT NOT NULL DEFAULT 'require_personalized'
        CHECK (delivery_policy IN ('require_personalized', 'allow_original_fallback')),
    policy_version INTEGER NOT NULL DEFAULT 1 CHECK (policy_version > 0),
    updated_by UUID REFERENCES public.users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS public.assignment_personalization_teacher_overrides (
    teacher_id UUID PRIMARY KEY REFERENCES public.teachers(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE CASCADE,
    mode TEXT NOT NULL DEFAULT 'inherit' CHECK (mode IN ('inherit', 'override')),
    enabled_override BOOLEAN,
    paused_override BOOLEAN,
    llm_profile_id_override TEXT
        CHECK (llm_profile_id_override IS NULL OR llm_profile_id_override = 'deepseek-chat-v1'),
    delivery_policy_override TEXT
        CHECK (
            delivery_policy_override IS NULL
            OR delivery_policy_override IN ('require_personalized', 'allow_original_fallback')
        ),
    override_version INTEGER NOT NULL DEFAULT 1 CHECK (override_version > 0),
    updated_by UUID REFERENCES public.users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT assignment_personalization_teacher_override_shape CHECK (
        mode = 'override'
        OR (
            enabled_override IS NULL
            AND paused_override IS NULL
            AND llm_profile_id_override IS NULL
            AND delivery_policy_override IS NULL
        )
    )
);

CREATE INDEX IF NOT EXISTS assignment_personalization_teacher_overrides_school_idx
    ON public.assignment_personalization_teacher_overrides (school_id, teacher_id);

INSERT INTO public.assignment_personalization_school_policies (school_id)
SELECT id FROM public.schools
ON CONFLICT (school_id) DO NOTHING;

CREATE OR REPLACE FUNCTION public.set_assignment_personalization_policy_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog, public
AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS assignment_personalization_school_policy_updated_at
    ON public.assignment_personalization_school_policies;
CREATE TRIGGER assignment_personalization_school_policy_updated_at
BEFORE UPDATE ON public.assignment_personalization_school_policies
FOR EACH ROW
EXECUTE FUNCTION public.set_assignment_personalization_policy_updated_at();

DROP TRIGGER IF EXISTS assignment_personalization_teacher_override_updated_at
    ON public.assignment_personalization_teacher_overrides;
CREATE TRIGGER assignment_personalization_teacher_override_updated_at
BEFORE UPDATE ON public.assignment_personalization_teacher_overrides
FOR EACH ROW
EXECUTE FUNCTION public.set_assignment_personalization_policy_updated_at();

CREATE OR REPLACE FUNCTION public.ensure_assignment_personalization_school_policy()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    INSERT INTO public.assignment_personalization_school_policies (school_id)
    VALUES (NEW.id)
    ON CONFLICT (school_id) DO NOTHING;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS schools_assignment_personalization_policy
    ON public.schools;
CREATE TRIGGER schools_assignment_personalization_policy
AFTER INSERT ON public.schools
FOR EACH ROW
EXECUTE FUNCTION public.ensure_assignment_personalization_school_policy();

CREATE OR REPLACE FUNCTION public.validate_assignment_personalization_teacher_override()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog, public
AS $$
DECLARE
    canonical_school UUID;
BEGIN
    SELECT school_id INTO canonical_school
    FROM public.teachers
    WHERE id = NEW.teacher_id;

    IF canonical_school IS NULL OR canonical_school <> NEW.school_id THEN
        RAISE EXCEPTION 'Teacher override school does not match canonical teacher school'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS validate_assignment_personalization_teacher_override
    ON public.assignment_personalization_teacher_overrides;
CREATE TRIGGER validate_assignment_personalization_teacher_override
BEFORE INSERT OR UPDATE OF teacher_id, school_id
ON public.assignment_personalization_teacher_overrides
FOR EACH ROW
EXECUTE FUNCTION public.validate_assignment_personalization_teacher_override();

ALTER TABLE public.assignment_personalization_school_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_personalization_school_policies FORCE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_personalization_teacher_overrides ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_personalization_teacher_overrides FORCE ROW LEVEL SECURITY;

CREATE POLICY assignment_personalization_school_policy_select
ON public.assignment_personalization_school_policies
FOR SELECT
USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'Teacher'
        AND school_id = public.get_school_id()
    )
);

CREATE POLICY assignment_personalization_school_policy_admin_insert
ON public.assignment_personalization_school_policies
FOR INSERT
WITH CHECK (public.get_role() = 'PlatformAdmin');

CREATE POLICY assignment_personalization_school_policy_admin_update
ON public.assignment_personalization_school_policies
FOR UPDATE
USING (public.get_role() = 'PlatformAdmin')
WITH CHECK (public.get_role() = 'PlatformAdmin');

CREATE POLICY assignment_personalization_teacher_override_select
ON public.assignment_personalization_teacher_overrides
FOR SELECT
USING (
    public.get_role() = 'PlatformAdmin'
    OR (
        public.get_role() = 'Teacher'
        AND school_id = public.get_school_id()
        AND teacher_id IN (
            SELECT t.id
            FROM public.teachers t
            WHERE t.user_id = public.get_user_id()
        )
    )
);

CREATE POLICY assignment_personalization_teacher_override_admin_insert
ON public.assignment_personalization_teacher_overrides
FOR INSERT
WITH CHECK (public.get_role() = 'PlatformAdmin');

CREATE POLICY assignment_personalization_teacher_override_admin_update
ON public.assignment_personalization_teacher_overrides
FOR UPDATE
USING (public.get_role() = 'PlatformAdmin')
WITH CHECK (public.get_role() = 'PlatformAdmin');

CREATE POLICY assignment_personalization_teacher_override_admin_delete
ON public.assignment_personalization_teacher_overrides
FOR DELETE
USING (public.get_role() = 'PlatformAdmin');

CREATE OR REPLACE FUNCTION public.resolve_assignment_personalization_policy(
    p_school_id UUID,
    p_teacher_user_id UUID
)
RETURNS TABLE (
    enabled BOOLEAN,
    paused BOOLEAN,
    llm_profile_id TEXT,
    delivery_policy TEXT,
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
        END AS enabled,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(teacher_override.paused_override, school_policy.paused, FALSE)
            ELSE COALESCE(school_policy.paused, FALSE)
        END AS paused,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(
                    teacher_override.llm_profile_id_override,
                    school_policy.llm_profile_id,
                    'deepseek-chat-v1'
                )
            ELSE COALESCE(school_policy.llm_profile_id, 'deepseek-chat-v1')
        END AS llm_profile_id,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN COALESCE(
                    teacher_override.delivery_policy_override,
                    school_policy.delivery_policy,
                    'require_personalized'
                )
            ELSE COALESCE(school_policy.delivery_policy, 'require_personalized')
        END AS delivery_policy,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN 'teacher'
            ELSE 'school'
        END AS policy_scope,
        CASE
            WHEN COALESCE(teacher_override.mode, 'inherit') = 'override'
                THEN teacher_override.override_version
            ELSE COALESCE(school_policy.policy_version, 1)
        END AS policy_version
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

REVOKE ALL ON FUNCTION public.resolve_assignment_personalization_policy(UUID, UUID) FROM PUBLIC;

ALTER TABLE public.assignment_personalization_jobs
    ADD COLUMN IF NOT EXISTS llm_profile_id TEXT NOT NULL DEFAULT 'deepseek-chat-v1',
    ADD COLUMN IF NOT EXISTS llm_provider TEXT NOT NULL DEFAULT 'deepseek',
    ADD COLUMN IF NOT EXISTS policy_scope TEXT NOT NULL DEFAULT 'school',
    ADD COLUMN IF NOT EXISTS policy_version INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS delivery_policy TEXT NOT NULL DEFAULT 'require_personalized',
    ADD COLUMN IF NOT EXISTS processing_stage TEXT NOT NULL DEFAULT 'queued',
    ADD COLUMN IF NOT EXISTS talent_profile_present BOOLEAN,
    ADD COLUMN IF NOT EXISTS teacher_report_count INTEGER,
    ADD COLUMN IF NOT EXISTS performance_context_present BOOLEAN,
    ADD COLUMN IF NOT EXISTS class_material_chunk_count INTEGER,
    ADD COLUMN IF NOT EXISTS governed_knowledge_chunk_count INTEGER,
    ADD COLUMN IF NOT EXISTS prompt_tokens INTEGER,
    ADD COLUMN IF NOT EXISTS completion_tokens INTEGER,
    ADD COLUMN IF NOT EXISTS total_tokens INTEGER,
    ADD COLUMN IF NOT EXISTS generated_content_changed BOOLEAN;

UPDATE public.assignment_personalization_jobs
SET llm_profile_id = 'deepseek-chat-v1',
    llm_provider = 'deepseek',
    policy_scope = COALESCE(NULLIF(policy_scope, ''), 'school'),
    policy_version = GREATEST(policy_version, 1),
    delivery_policy = COALESCE(NULLIF(delivery_policy, ''), 'require_personalized'),
    processing_stage = CASE status
        WHEN 'queued' THEN 'queued'
        WHEN 'running' THEN 'authorizing'
        WHEN 'succeeded' THEN 'ready'
        WHEN 'failed' THEN 'failed'
        WHEN 'cancelled' THEN 'cancelled'
        ELSE 'queued'
    END;

ALTER TABLE public.assignment_personalization_jobs
    DROP CONSTRAINT IF EXISTS assignment_personalization_job_llm_contract,
    DROP CONSTRAINT IF EXISTS assignment_personalization_job_policy_contract,
    DROP CONSTRAINT IF EXISTS assignment_personalization_job_stage_contract,
    DROP CONSTRAINT IF EXISTS assignment_personalization_job_diagnostics;

ALTER TABLE public.assignment_personalization_jobs
    ADD CONSTRAINT assignment_personalization_job_llm_contract CHECK (
        llm_profile_id = 'deepseek-chat-v1'
        AND llm_provider = 'deepseek'
        AND model_name = 'deepseek-chat'
    ),
    ADD CONSTRAINT assignment_personalization_job_policy_contract CHECK (
        policy_scope IN ('school', 'teacher', 'legacy_default')
        AND policy_version > 0
        AND delivery_policy IN ('require_personalized', 'allow_original_fallback')
    ),
    ADD CONSTRAINT assignment_personalization_job_stage_contract CHECK (
        processing_stage IN (
            'queued',
            'paused',
            'authorizing',
            'building_student_context',
            'retrieving_context',
            'ai_gateway',
            'provider',
            'validating_response',
            'saving',
            'ready',
            'failed',
            'cancelled'
        )
    ),
    ADD CONSTRAINT assignment_personalization_job_diagnostics CHECK (
        (teacher_report_count IS NULL OR teacher_report_count >= 0)
        AND (class_material_chunk_count IS NULL OR class_material_chunk_count >= 0)
        AND (governed_knowledge_chunk_count IS NULL OR governed_knowledge_chunk_count >= 0)
        AND (prompt_tokens IS NULL OR prompt_tokens >= 0)
        AND (completion_tokens IS NULL OR completion_tokens >= 0)
        AND (total_tokens IS NULL OR total_tokens >= 0)
    );

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
       OR NEW.delivery_policy IS DISTINCT FROM OLD.delivery_policy THEN
        RAISE EXCEPTION 'Assignment personalization execution contract is immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS assignment_personalization_contract_immutable
    ON public.assignment_personalization_jobs;
CREATE TRIGGER assignment_personalization_contract_immutable
BEFORE UPDATE ON public.assignment_personalization_jobs
FOR EACH ROW
EXECUTE FUNCTION public.prevent_assignment_personalization_contract_rewrite();

CREATE OR REPLACE FUNCTION public.enqueue_assignment_personalization_job()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    INSERT INTO public.assignment_personalization_jobs (
        school_id,
        assignment_id,
        student_id,
        class_section_id,
        requested_by,
        status,
        attempt_count,
        available_at,
        lease_owner,
        heartbeat_at,
        last_error_code,
        last_error_summary,
        completed_at,
        idempotency_key,
        model_name,
        profile_name,
        profile_version,
        llm_profile_id,
        llm_provider,
        policy_scope,
        policy_version,
        delivery_policy,
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
        CASE
            WHEN effective.enabled THEN NULL
            ELSE 'Assignment personalization is disabled by policy'
        END,
        CASE WHEN effective.enabled THEN NULL ELSE NOW() END,
        concat(
            target.assignment_id::text,
            ':',
            NEW.student_id::text,
            ':assignment_personalization_v1:1'
        ),
        'deepseek-chat',
        'assignment_personalization_v1',
        1,
        effective.llm_profile_id,
        'deepseek',
        effective.policy_scope,
        effective.policy_version,
        effective.delivery_policy,
        CASE
            WHEN NOT effective.enabled THEN 'cancelled'
            WHEN effective.paused THEN 'paused'
            ELSE 'queued'
        END
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
    CROSS JOIN LATERAL public.resolve_assignment_personalization_policy(
        target.school_id,
        target.requested_by
    ) AS effective
    ON CONFLICT (assignment_id, student_id, profile_name, profile_version) DO NOTHING;

    RETURN NEW;
END
$$;

REVOKE ALL ON FUNCTION public.enqueue_assignment_personalization_job() FROM PUBLIC;

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
        job.lease_owner;
END
$$;

REVOKE EXECUTE ON FUNCTION public.claim_next_assignment_personalization_job(UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.recover_stale_assignment_personalization_jobs(
    p_stale_after_seconds BIGINT,
    p_max_attempts INTEGER DEFAULT 5
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    reconciled BIGINT;
    max_attempts INTEGER := GREATEST(1, LEAST(COALESCE(p_max_attempts, 5), 10));
BEGIN
    IF public.get_role() <> 'system_job'
       OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL
       OR public.get_user_id() IS NULL THEN
        RAISE EXCEPTION 'bounded assignment personalization queue context required'
            USING ERRCODE = '42501';
    END IF;

    WITH updated AS (
        UPDATE public.assignment_personalization_jobs AS job
        SET status = CASE
                WHEN job.attempt_count >= max_attempts THEN 'failed'
                ELSE 'queued'
            END,
            available_at = CASE
                WHEN job.attempt_count >= max_attempts THEN job.available_at
                ELSE NOW()
            END,
            lease_owner = NULL,
            heartbeat_at = NULL,
            completed_at = CASE
                WHEN job.attempt_count >= max_attempts THEN NOW()
                ELSE NULL
            END,
            last_error_code = CASE
                WHEN job.attempt_count >= max_attempts THEN 'worker_restart_limit'
                ELSE 'stale_lease_recovered'
            END,
            last_error_summary = CASE
                WHEN job.attempt_count >= max_attempts
                    THEN 'Personalization stopped after repeated worker interruptions'
                ELSE 'Recovered after stale personalization worker lease'
            END,
            processing_stage = CASE
                WHEN job.attempt_count >= max_attempts THEN 'failed'
                ELSE 'queued'
            END
        WHERE job.status = 'running'
          AND COALESCE(job.heartbeat_at, job.started_at, job.created_at)
                < NOW() - make_interval(
                    secs => GREATEST(p_stale_after_seconds, 60)::DOUBLE PRECISION
                )
        RETURNING 1
    )
    SELECT COUNT(*) INTO reconciled FROM updated;

    RETURN reconciled;
END
$$;

REVOKE EXECUTE ON FUNCTION public.recover_stale_assignment_personalization_jobs(BIGINT, INTEGER)
FROM PUBLIC;

COMMENT ON TABLE public.assignment_personalization_school_policies IS
    'Platform-admin governed default assignment-personalization policy for one school.';
COMMENT ON TABLE public.assignment_personalization_teacher_overrides IS
    'Optional Platform-admin teacher override; inherit mode keeps teachers out of provider operations.';
COMMENT ON FUNCTION public.resolve_assignment_personalization_policy(UUID, UUID) IS
    'Internal resolver for the effective school/teacher personalization policy.';
COMMENT ON COLUMN public.assignment_personalization_jobs.processing_stage IS
    'Safe operator-visible progress stage; never contains prompt/provider payload content.';
