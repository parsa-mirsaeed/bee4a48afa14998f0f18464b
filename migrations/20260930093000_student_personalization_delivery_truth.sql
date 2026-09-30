-- Student-visible delivery truth for assignment personalization.
--
-- Students must never need direct SELECT access to the operational queue. This
-- narrow SECURITY DEFINER function validates the canonical current student and
-- returns only the minimum delivery state needed by the product UI.

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
    current_user_id UUID := public.get_user_id();
    current_school_id UUID := public.get_school_id();
    current_role TEXT := public.get_role();
BEGIN
    IF current_role <> 'Student'
       OR current_user_id IS NULL
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
        ORDER BY personalization_job.created_at DESC
        LIMIT 1
    ) AS job ON TRUE
    WHERE custom_assignment.id = p_custom_assignment_id
      AND student.user_id = current_user_id
      AND student.school_id = current_school_id
      AND student_user.school_id = current_school_id
      AND student_user.is_active = TRUE
      AND class_section.school_id = current_school_id
      AND assignment.status = 'Published'::assignment_status;
END
$$;

REVOKE ALL ON FUNCTION public.get_student_assignment_personalization_delivery(UUID) FROM PUBLIC;

COMMENT ON FUNCTION public.get_student_assignment_personalization_delivery(UUID) IS
    'Returns only the safe student delivery state after validating current Student identity, school, enrollment, and published assignment. Operational queue diagnostics remain hidden.';
