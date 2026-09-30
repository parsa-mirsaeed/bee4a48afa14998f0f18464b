-- Runtime-safe Platform Admin read/retry boundary for assignment personalization.
--
-- Core users/teachers/assignments RLS intentionally does not grant PlatformAdmin
-- broad catalog visibility. The personalization control center therefore uses
-- narrowly scoped SECURITY DEFINER functions that:
--   * re-authorize the active PlatformAdmin actor;
--   * return only personalization-operational fields;
--   * never return prompts, provider payloads, credentials, or student names;
--   * keep retry mutations bounded to eligible failed/cancelled jobs.

CREATE OR REPLACE FUNCTION public.assignment_personalization_assert_platform_admin()
RETURNS UUID
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    actor_id UUID := public.get_user_id();
    actor_role TEXT := public.get_role();
BEGIN
    IF actor_id IS NULL
       OR actor_role <> 'PlatformAdmin'
       OR COALESCE(public.get_elevated_operation(), FALSE) THEN
        RAISE EXCEPTION 'PlatformAdmin personalization context required'
            USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.users AS actor
        JOIN public.roles AS role_row ON role_row.id = actor.role_id
        WHERE actor.id = actor_id
          AND actor.is_active = TRUE
          AND role_row.name::text = 'PlatformAdmin'
    ) THEN
        RAISE EXCEPTION 'Active PlatformAdmin actor required'
            USING ERRCODE = '42501';
    END IF;

    RETURN actor_id;
END
$$;

REVOKE ALL
ON FUNCTION public.assignment_personalization_assert_platform_admin()
FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.list_assignment_personalization_teacher_policies_for_admin(
    p_school_id UUID
)
RETURNS TABLE (
    teacher_id UUID,
    teacher_user_id UUID,
    teacher_name TEXT,
    school_id UUID,
    mode TEXT,
    enabled_override BOOLEAN,
    paused_override BOOLEAN,
    llm_profile_id_override TEXT,
    delivery_policy_override TEXT,
    override_version INTEGER,
    effective_enabled BOOLEAN,
    effective_paused BOOLEAN,
    effective_llm_profile_id TEXT,
    effective_delivery_policy TEXT,
    effective_scope TEXT,
    effective_version INTEGER
)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    PERFORM public.assignment_personalization_assert_platform_admin();

    IF p_school_id IS NULL OR NOT EXISTS (
        SELECT 1 FROM public.schools AS school WHERE school.id = p_school_id
    ) THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT
        teacher.id,
        teacher.user_id,
        teacher_user.name,
        teacher.school_id,
        COALESCE(override_policy.mode, 'inherit'),
        override_policy.enabled_override,
        override_policy.paused_override,
        override_policy.llm_profile_id_override,
        override_policy.delivery_policy_override,
        COALESCE(override_policy.override_version, 1),
        effective.enabled,
        effective.paused,
        effective.llm_profile_id,
        effective.delivery_policy,
        effective.policy_scope,
        effective.policy_version
    FROM public.teachers AS teacher
    JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
    JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
    CROSS JOIN LATERAL public.resolve_assignment_personalization_policy(
        teacher.school_id,
        teacher.user_id
    ) AS effective
    LEFT JOIN public.assignment_personalization_teacher_overrides AS override_policy
      ON override_policy.teacher_id = teacher.id
     AND override_policy.school_id = teacher.school_id
    WHERE teacher.school_id = p_school_id
      AND teacher_user.is_active = TRUE
      AND teacher_role.name::text = 'Teacher'
    ORDER BY teacher_user.name, teacher.id;
END
$$;

REVOKE ALL
ON FUNCTION public.list_assignment_personalization_teacher_policies_for_admin(UUID)
FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.assignment_personalization_teacher_school_for_admin(
    p_teacher_id UUID
)
RETURNS UUID
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    resolved_school UUID;
BEGIN
    PERFORM public.assignment_personalization_assert_platform_admin();

    SELECT teacher.school_id
    INTO resolved_school
    FROM public.teachers AS teacher
    JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
    JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
    WHERE teacher.id = p_teacher_id
      AND teacher_user.is_active = TRUE
      AND teacher_role.name::text = 'Teacher';

    RETURN resolved_school;
END
$$;

REVOKE ALL
ON FUNCTION public.assignment_personalization_teacher_school_for_admin(UUID)
FROM PUBLIC;

-- The teacher-override table itself has a narrow PlatformAdmin write policy, but
-- its canonical-school validation trigger must inspect the teacher catalog that
-- intentionally remains hidden from PlatformAdmin RLS. Recreate only that
-- validator as SECURITY DEFINER and re-authorize the actor inside the boundary.
CREATE OR REPLACE FUNCTION public.validate_assignment_personalization_teacher_override()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $
DECLARE
    canonical_school UUID;
BEGIN
    PERFORM public.assignment_personalization_assert_platform_admin();

    SELECT teacher.school_id
    INTO canonical_school
    FROM public.teachers AS teacher
    JOIN public.users AS teacher_user ON teacher_user.id = teacher.user_id
    JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
    WHERE teacher.id = NEW.teacher_id
      AND teacher_user.is_active = TRUE
      AND teacher_role.name::text = 'Teacher';

    IF canonical_school IS NULL OR canonical_school <> NEW.school_id THEN
        RAISE EXCEPTION 'Teacher override school does not match canonical teacher school'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END
$;

REVOKE ALL
ON FUNCTION public.validate_assignment_personalization_teacher_override()
FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.list_assignment_personalization_jobs_for_admin(
    p_school_id UUID DEFAULT NULL,
    p_teacher_user_id UUID DEFAULT NULL,
    p_limit INTEGER DEFAULT 200
)
RETURNS TABLE (
    job_id UUID,
    school_id UUID,
    school_name TEXT,
    teacher_user_id UUID,
    teacher_name TEXT,
    assignment_id UUID,
    assignment_title TEXT,
    student_reference TEXT,
    status TEXT,
    processing_stage TEXT,
    attempt_count INTEGER,
    llm_profile_id TEXT,
    llm_provider TEXT,
    model_name TEXT,
    policy_scope TEXT,
    policy_version INTEGER,
    delivery_policy TEXT,
    last_error_code TEXT,
    last_error_summary TEXT,
    created_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    heartbeat_at TIMESTAMPTZ,
    talent_profile_present BOOLEAN,
    teacher_report_count INTEGER,
    performance_context_present BOOLEAN,
    class_material_chunk_count INTEGER,
    governed_knowledge_chunk_count INTEGER,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    generated_content_changed BOOLEAN
)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    bounded_limit INTEGER := GREATEST(1, LEAST(COALESCE(p_limit, 200), 500));
BEGIN
    PERFORM public.assignment_personalization_assert_platform_admin();

    RETURN QUERY
    SELECT
        job.id,
        job.school_id,
        school.name,
        job.requested_by,
        teacher_user.name,
        job.assignment_id,
        assignment.title,
        LEFT(job.student_id::text, 8),
        job.status,
        job.processing_stage,
        job.attempt_count,
        job.llm_profile_id,
        job.llm_provider,
        job.model_name,
        job.policy_scope,
        job.policy_version,
        job.delivery_policy,
        job.last_error_code,
        job.last_error_summary,
        job.created_at,
        job.started_at,
        job.completed_at,
        job.heartbeat_at,
        job.talent_profile_present,
        job.teacher_report_count,
        job.performance_context_present,
        job.class_material_chunk_count,
        job.governed_knowledge_chunk_count,
        job.prompt_tokens,
        job.completion_tokens,
        job.total_tokens,
        job.generated_content_changed
    FROM public.assignment_personalization_jobs AS job
    JOIN public.schools AS school ON school.id = job.school_id
    JOIN public.users AS teacher_user ON teacher_user.id = job.requested_by
    JOIN public.roles AS teacher_role ON teacher_role.id = teacher_user.role_id
    JOIN public.assignments AS assignment ON assignment.id = job.assignment_id
    WHERE (p_school_id IS NULL OR job.school_id = p_school_id)
      AND (p_teacher_user_id IS NULL OR job.requested_by = p_teacher_user_id)
      AND teacher_user.is_active = TRUE
      AND teacher_role.name::text = 'Teacher'
    ORDER BY job.created_at DESC, job.id
    LIMIT bounded_limit;
END
$$;

REVOKE ALL
ON FUNCTION public.list_assignment_personalization_jobs_for_admin(UUID, UUID, INTEGER)
FROM PUBLIC;

COMMENT ON FUNCTION public.list_assignment_personalization_teacher_policies_for_admin(UUID) IS
    'Bounded PlatformAdmin personalization-teacher policy projection; bypasses unrelated teacher/user RLS without granting general catalog visibility.';
COMMENT ON FUNCTION public.list_assignment_personalization_jobs_for_admin(UUID, UUID, INTEGER) IS
    'Bounded PlatformAdmin personalization job projection; exposes only safe operational metadata and short student references.';
