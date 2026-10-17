-- Immutable new execution generations for Platform Admin regenerated
-- personalization using CURRENT persisted school/teacher policy. Old jobs and
-- their profile/model/policy snapshots remain untouched, auditable evidence.
ALTER TABLE public.assignment_personalization_jobs
    ADD COLUMN IF NOT EXISTS execution_generation INTEGER NOT NULL DEFAULT 1
        CHECK (execution_generation >= 1),
    ADD COLUMN IF NOT EXISTS supersedes_job_id UUID
        REFERENCES public.assignment_personalization_jobs(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS regenerated_by UUID
        REFERENCES public.users(id) ON DELETE RESTRICT;

DO $constraints$
DECLARE old_unique text;
BEGIN
    SELECT con.conname INTO old_unique
    FROM pg_constraint con
    WHERE con.conrelid = 'public.assignment_personalization_jobs'::regclass
      AND con.contype = 'u'
      AND pg_get_constraintdef(con.oid) =
          'UNIQUE (assignment_id, student_id, profile_name, profile_version)'
    LIMIT 1;

    IF old_unique IS NOT NULL THEN
        EXECUTE format(
            'ALTER TABLE public.assignment_personalization_jobs DROP CONSTRAINT %I',
            old_unique
        );
    END IF;
END
$constraints$;

ALTER TABLE public.assignment_personalization_jobs
    ADD CONSTRAINT assignment_personalization_generation_unique
    UNIQUE (assignment_id, student_id, profile_name, profile_version, execution_generation);

CREATE UNIQUE INDEX assignment_personalization_one_active_generation
    ON public.assignment_personalization_jobs
       (assignment_id, student_id, profile_name, profile_version)
    WHERE status IN ('queued', 'running');

CREATE INDEX assignment_personalization_lineage
    ON public.assignment_personalization_jobs (supersedes_job_id)
    WHERE supersedes_job_id IS NOT NULL;

-- Preserve v1 format identity and initial idempotency while rebinding its
-- conflict target to generation 1 (SQL column DEFAULT). Do not edit history.
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
        CASE effective.llm_profile_id
            WHEN 'deepseek-chat-v1' THEN 'deepseek-chat'
            WHEN 'deepseek-v41-flash-v1' THEN 'deepseek-v4.1-flash'
            ELSE substring(effective.llm_profile_id from 11)
        END,
        'assignment_personalization_v1',
        1,
        effective.llm_profile_id,
        CASE effective.llm_profile_id
            WHEN 'deepseek-chat-v1' THEN 'deepseek'
            WHEN 'deepseek-v41-flash-v1' THEN 'gapgpt'
            ELSE 'custom'
        END,
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
    ON CONFLICT (assignment_id, student_id, profile_name, profile_version, execution_generation) DO NOTHING;

    RETURN NEW;
END
$$;


-- Prevent original-retry action from resurrecting generation 1 once a newer
-- immutable execution was created for this same assignment/student.
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
          AND NOT EXISTS (
              SELECT 1 FROM public.assignment_personalization_jobs AS newer
              WHERE newer.assignment_id = job.assignment_id
                AND newer.student_id = job.student_id
                AND newer.profile_name = job.profile_name
                AND newer.profile_version = job.profile_version
                AND newer.execution_generation > job.execution_generation
          )
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



-- Protect generation/lineage from later DML changes, just like model/policy.
CREATE OR REPLACE FUNCTION public.prevent_assignment_personalization_contract_rewrite()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF NEW.execution_generation IS DISTINCT FROM OLD.execution_generation
       OR NEW.supersedes_job_id IS DISTINCT FROM OLD.supersedes_job_id
       OR NEW.regenerated_by IS DISTINCT FROM OLD.regenerated_by
       OR NEW.llm_profile_id IS DISTINCT FROM OLD.llm_profile_id
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

CREATE OR REPLACE FUNCTION public.regenerate_assignment_personalization_jobs_admin(
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
AS $function$
DECLARE
    inserted BIGINT;
    bounded_limit INTEGER := LEAST(GREATEST(COALESCE(p_limit, 100), 1), 100);
BEGIN
    IF public.get_role() <> 'PlatformAdmin'
       OR public.get_user_id() IS NULL
       OR COALESCE(public.get_elevated_operation(), FALSE) THEN
        RAISE EXCEPTION 'Active non-elevated PlatformAdmin required'
            USING ERRCODE = '42501';
    END IF;

    IF p_job_id IS NULL AND p_school_id IS NULL THEN
        RAISE EXCEPTION 'Explicit job or school scope required'
            USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM public.users actor
        JOIN public.roles role ON role.id = actor.role_id
        WHERE actor.id = public.get_user_id()
          AND actor.is_active
          AND role.name::text = 'PlatformAdmin'
    ) THEN
        RAISE EXCEPTION 'Active PlatformAdmin actor required'
            USING ERRCODE = '42501';
    END IF;

    WITH eligible AS MATERIALIZED (
        SELECT job.*,
               effective.llm_profile_id AS new_llm_profile,
               effective.delivery_policy AS new_delivery_policy,
               effective.specialization_instructions AS new_instructions,
               effective.policy_scope AS new_scope,
               effective.policy_version AS new_version
        FROM public.assignment_personalization_jobs job
        JOIN public.assignments assignment ON assignment.id = job.assignment_id
        JOIN public.teachers teacher ON teacher.user_id = job.requested_by
            AND teacher.id = assignment.teacher_id
            AND teacher.school_id = job.school_id
        JOIN public.users teacher_user ON teacher_user.id = teacher.user_id
        JOIN public.roles teacher_role ON teacher_role.id = teacher_user.role_id
        JOIN public.class_sections class_section ON class_section.id = job.class_section_id
            AND class_section.id = assignment.class_section_id
            AND class_section.school_id = job.school_id
        JOIN public.teaching_assignments teaching_assignment
          ON teaching_assignment.teacher_id = teacher.id
         AND teaching_assignment.class_section_id = assignment.class_section_id
        JOIN public.students student ON student.id = job.student_id
            AND student.school_id = job.school_id
        JOIN public.users student_user ON student_user.id = student.user_id
            AND student_user.school_id = job.school_id
        JOIN public.enrollments enrollment
          ON enrollment.student_id = student.id
         AND enrollment.class_section_id = assignment.class_section_id
        JOIN public.custom_assignments personalized
          ON personalized.assignment_id = assignment.id
         AND personalized.student_id = student.id
        CROSS JOIN LATERAL public.resolve_assignment_personalization_policy_v2(
            job.school_id, job.requested_by
        ) effective
        WHERE job.status IN ('failed', 'cancelled')
          AND assignment.status = 'Published'::assignment_status
          AND teacher_user.is_active AND student_user.is_active
          AND teacher_role.name::text = 'Teacher'
          AND personalized.prompt_ctx IS NULL
          AND effective.enabled AND NOT effective.paused
          AND (p_job_id IS NULL OR job.id = p_job_id)
          AND (p_school_id IS NULL OR job.school_id = p_school_id)
          AND (p_teacher_user_id IS NULL OR job.requested_by = p_teacher_user_id)
          AND (p_assignment_id IS NULL OR job.assignment_id = p_assignment_id)
          AND NOT EXISTS (
              SELECT 1 FROM public.assignment_personalization_jobs later
              WHERE later.assignment_id = job.assignment_id
                AND later.student_id = job.student_id
                AND later.profile_name = job.profile_name
                AND later.profile_version = job.profile_version
                AND later.execution_generation > job.execution_generation
          )
        ORDER BY job.created_at, job.id
        FOR UPDATE OF job SKIP LOCKED
        LIMIT bounded_limit
    ), inserted_jobs AS (
        INSERT INTO public.assignment_personalization_jobs (
            school_id, assignment_id, student_id, class_section_id, requested_by,
            target_scope, status, attempt_count, available_at, idempotency_key,
            model_name, profile_name, profile_version,
            llm_profile_id, llm_provider, policy_scope, policy_version,
            delivery_policy, specialization_instructions, processing_stage,
            execution_generation, supersedes_job_id, regenerated_by
        )
        SELECT e.school_id, e.assignment_id, e.student_id, e.class_section_id,
               e.requested_by, 'student', 'queued', 0, NOW(),
               concat(e.assignment_id::text, ':', e.student_id::text,
                      ':assignment_personalization_v1:gen:', (e.execution_generation + 1)::text),
               CASE e.new_llm_profile
                   WHEN 'deepseek-chat-v1' THEN 'deepseek-chat'
                   WHEN 'deepseek-v41-flash-v1' THEN 'deepseek-v4.1-flash'
                   ELSE substring(e.new_llm_profile FROM 11)
               END,
               e.profile_name, e.profile_version, e.new_llm_profile,
               CASE e.new_llm_profile
                   WHEN 'deepseek-chat-v1' THEN 'deepseek'
                   WHEN 'deepseek-v41-flash-v1' THEN 'gapgpt'
                   ELSE 'custom'
               END,
               e.new_scope, e.new_version, e.new_delivery_policy,
               e.new_instructions, 'queued',
               e.execution_generation + 1, e.id, public.get_user_id()
        FROM eligible e
        ON CONFLICT DO NOTHING
        RETURNING id
    )
    SELECT COUNT(*) INTO inserted FROM inserted_jobs;

    RETURN inserted;
END
$function$;

REVOKE ALL ON FUNCTION public.regenerate_assignment_personalization_jobs_admin(
    UUID, UUID, UUID, UUID, INTEGER
) FROM PUBLIC;

COMMENT ON FUNCTION public.regenerate_assignment_personalization_jobs_admin(
    UUID, UUID, UUID, UUID, INTEGER
) IS 'Create new immutable current-policy personalization execution for one or bounded school scope; original failed jobs remain unchanged.';

-- Student delivery must select the current immutable retry generation, never
-- the superseded failed job, including timestamp ties on rapid retries.
CREATE OR REPLACE FUNCTION public.get_student_assignment_personalization_delivery(
    p_custom_assignment_id UUID
)
RETURNS TABLE (
    delivery_state TEXT,
    delivery_policy TEXT,
    job_status TEXT,
    processing_stage TEXT
)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    current_user_id UUID := NULLIF(current_setting('app.user_id', true), '')::UUID;
    current_school_id UUID := NULLIF(current_setting('app.school_id', true), '')::UUID;
BEGIN
    IF current_user_id IS NULL
       OR current_school_id IS NULL THEN
        RAISE EXCEPTION 'student assignment delivery context required'
            USING ERRCODE = '42501';
    END IF;

    RETURN QUERY
    SELECT
        CASE
            WHEN custom_assignment.prompt_ctx ? 'personalized_assignment'
                THEN 'ready'
            WHEN job.id IS NULL
                THEN 'preparing'
            WHEN job.status IN ('queued', 'running')
                THEN 'preparing'
            WHEN job.status IN ('failed', 'cancelled')
                 AND job.delivery_policy = 'allow_original_fallback'
                THEN 'fallback_original'
            WHEN job.status IN ('failed', 'cancelled')
                THEN 'unavailable'
            WHEN job.status = 'succeeded'
                THEN 'unavailable'
            ELSE 'preparing'
        END AS delivery_state,
        COALESCE(job.delivery_policy, 'require_personalized') AS delivery_policy,
        job.status,
        job.processing_stage
    FROM public.custom_assignments AS custom_assignment
    JOIN public.assignments AS assignment
      ON assignment.id = custom_assignment.assignment_id
    JOIN public.class_sections AS class_section
      ON class_section.id = assignment.class_section_id
    JOIN public.students AS student
      ON student.id = custom_assignment.student_id
    JOIN public.users AS student_user
      ON student_user.id = student.user_id
    JOIN public.roles AS student_role
      ON student_role.id = student_user.role_id
    JOIN public.enrollments AS enrollment
      ON enrollment.student_id = student.id
     AND enrollment.class_section_id = assignment.class_section_id
    LEFT JOIN LATERAL (
        SELECT
            personalization_job.id,
            personalization_job.status,
            personalization_job.processing_stage,
            personalization_job.delivery_policy
        FROM public.assignment_personalization_jobs AS personalization_job
        WHERE personalization_job.assignment_id = custom_assignment.assignment_id
          AND personalization_job.student_id = custom_assignment.student_id
        ORDER BY personalization_job.execution_generation DESC,
                 personalization_job.created_at DESC,
                 personalization_job.id DESC
        LIMIT 1
    ) AS job ON TRUE
    WHERE custom_assignment.id = p_custom_assignment_id
      AND student.user_id = current_user_id
      AND student.school_id = current_school_id
      AND student_user.school_id = current_school_id
      AND student_user.is_active = TRUE
      AND student_role.name::text = 'Student'
      AND class_section.school_id = current_school_id
      AND assignment.status = 'Published'::assignment_status;
END
$$;

REVOKE ALL ON FUNCTION public.get_student_assignment_personalization_delivery(UUID) FROM PUBLIC;

COMMENT ON FUNCTION public.get_student_assignment_personalization_delivery(UUID) IS
    'Returns only the safe student delivery state after validating current Student identity, school, enrollment, and published assignment. Operational queue diagnostics remain hidden.';
