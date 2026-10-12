-- Operator-selected, OpenAI-compatible HTTPS LLM with one explicit model.
-- Migration is additive to existing profiles: no historic migration is edited,
-- no provider URL/secret is persisted, and the privileged fan-out authorization
-- remains the vetted definition from 20261012190000.
--
-- Profile identifier encodes the exact custom model; both PostgreSQL enqueue
-- paths derive their audit model/provider from this immutable policy value.
-- Gateway still rejects all arbitrary request-supplied models and origins.

ALTER TABLE public.assignment_personalization_school_policies
    DROP CONSTRAINT assignment_personalization_school_llm_profile_allowlist;
ALTER TABLE public.assignment_personalization_school_policies
    ADD CONSTRAINT assignment_personalization_school_llm_profile_allowlist
    CHECK (llm_profile_id IN ('deepseek-chat-v1', 'deepseek-v41-flash-v1')
           OR llm_profile_id ~ '^custom-v1:[A-Za-z0-9][A-Za-z0-9._:/-]{0,95}$');

ALTER TABLE public.assignment_personalization_teacher_overrides
    DROP CONSTRAINT assignment_personalization_teacher_llm_profile_allowlist;
ALTER TABLE public.assignment_personalization_teacher_overrides
    ADD CONSTRAINT assignment_personalization_teacher_llm_profile_allowlist
    CHECK (
        llm_profile_id_override IS NULL
        OR llm_profile_id_override IN ('deepseek-chat-v1', 'deepseek-v41-flash-v1')
        OR llm_profile_id_override ~ '^custom-v1:[A-Za-z0-9][A-Za-z0-9._:/-]{0,95}$'
    );

ALTER TABLE public.assignment_personalization_jobs
    DROP CONSTRAINT assignment_personalization_job_llm_contract;
ALTER TABLE public.assignment_personalization_jobs
    ADD CONSTRAINT assignment_personalization_job_llm_contract
    CHECK (
        (llm_profile_id = 'deepseek-chat-v1'
         AND llm_provider = 'deepseek'
         AND model_name = 'deepseek-chat')
        OR
        (llm_profile_id = 'deepseek-v41-flash-v1'
         AND llm_provider = 'gapgpt'
         AND model_name = 'deepseek-v4.1-flash')
        OR
        (llm_profile_id ~ '^custom-v1:[A-Za-z0-9][A-Za-z0-9._:/-]{0,95}$'
         AND llm_provider = 'custom'
         AND model_name = substring(llm_profile_id from 11))
    );

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
    ON CONFLICT (assignment_id, student_id, profile_name, profile_version) DO NOTHING;

    RETURN NEW;
END
$$;
REVOKE ALL ON FUNCTION public.enqueue_assignment_personalization_job() FROM PUBLIC;
