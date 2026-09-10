-- Restore legitimate relationship-scoped reads under the production NOBYPASSRLS
-- application role without broadening tenant visibility.
--
-- The browser harness now runs the application with the same FORCE-RLS boundary
-- as production. That exposed two historical assumptions which were previously
-- hidden by administrator credentials:
--   * Teacher/Parent/Student repository joins need selected related users, while
--     users_select_policy historically exposed only self (or manager school-wide).
--   * the governed source AFTER INSERT trigger advances an asset revision and
--     therefore needs a tightly bounded trusted write even though managers must
--     never receive general knowledge_assets UPDATE rights.

DO $prerequisites$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM public.edutalent_migration_files
        WHERE path = 'migrations/20260820010000_fix_enrollment_student_rls_recursion.sql'
    ) OR NOT EXISTS (
        SELECT 1
        FROM public.edutalent_migration_files
        WHERE path = 'migrations/20260901090000_bind_knowledge_ocr_to_reviewed_source.sql'
    ) THEN
        RAISE EXCEPTION
            'runtime relationship RLS fix requires finalized enrollment and governed-knowledge RLS';
    END IF;
END
$prerequisites$;

-- This helper is deliberately boolean-only and SECURITY DEFINER.  Its owner is
-- the controlled migration identity, so policy evaluation does not recurse back
-- through users/students/enrollments.  Every branch independently proves the
-- authenticated actor, current school, and concrete relationship before a
-- related user row is disclosed.
CREATE OR REPLACE FUNCTION public.user_relationship_visible_to_actor(
    p_target_user_id UUID
)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $function$
    SELECT
        p_target_user_id IS NOT NULL
        AND public.get_user_id() IS NOT NULL
        AND public.get_school_id() IS NOT NULL
        AND EXISTS (
            SELECT 1
            FROM public.users AS actor
            JOIN public.roles AS actor_role
              ON actor_role.id = actor.role_id
            WHERE actor.id = public.get_user_id()
              AND actor.school_id = public.get_school_id()
              AND actor.is_active = TRUE
              AND actor_role.name::text = public.get_role()
        )
        AND EXISTS (
            SELECT 1
            FROM public.users AS target
            WHERE target.id = p_target_user_id
              AND target.school_id = public.get_school_id()
              AND target.is_active = TRUE
              AND (
                  -- Teachers may read active students enrolled in a class they
                  -- are actually assigned to teach.
                  (
                      public.get_role() = 'Teacher'
                      AND EXISTS (
                          SELECT 1
                          FROM public.students AS student
                          JOIN public.enrollments AS enrollment
                            ON enrollment.student_id = student.id
                          JOIN public.teaching_assignments AS teaching
                            ON teaching.class_section_id = enrollment.class_section_id
                          JOIN public.teachers AS teacher_actor
                            ON teacher_actor.id = teaching.teacher_id
                          JOIN public.class_sections AS class_section
                            ON class_section.id = enrollment.class_section_id
                          WHERE student.user_id = target.id
                            AND student.school_id = public.get_school_id()
                            AND teacher_actor.user_id = public.get_user_id()
                            AND teacher_actor.school_id = public.get_school_id()
                            AND class_section.school_id = public.get_school_id()
                      )
                  )

                  -- Parents may read their linked child and the active teachers
                  -- assigned to that child's enrolled classes.  The latter is
                  -- needed for persisted grading/assignment presentation.
                  OR (
                      public.get_role() = 'Parent'
                      AND (
                          EXISTS (
                              SELECT 1
                              FROM public.students AS child
                              WHERE child.user_id = target.id
                                AND child.parent_id = public.get_user_id()
                                AND child.school_id = public.get_school_id()
                          )
                          OR EXISTS (
                              SELECT 1
                              FROM public.students AS child
                              JOIN public.enrollments AS enrollment
                                ON enrollment.student_id = child.id
                              JOIN public.teaching_assignments AS teaching
                                ON teaching.class_section_id = enrollment.class_section_id
                              JOIN public.teachers AS teacher_target
                                ON teacher_target.id = teaching.teacher_id
                              JOIN public.class_sections AS class_section
                                ON class_section.id = enrollment.class_section_id
                              WHERE child.parent_id = public.get_user_id()
                                AND child.school_id = public.get_school_id()
                                AND teacher_target.user_id = target.id
                                AND teacher_target.school_id = public.get_school_id()
                                AND class_section.school_id = public.get_school_id()
                          )
                      )
                  )

                  -- Students may read active teachers assigned to their own
                  -- enrolled classes, e.g. for grader/assignment attribution.
                  OR (
                      public.get_role() = 'Student'
                      AND EXISTS (
                          SELECT 1
                          FROM public.students AS student_actor
                          JOIN public.enrollments AS enrollment
                            ON enrollment.student_id = student_actor.id
                          JOIN public.teaching_assignments AS teaching
                            ON teaching.class_section_id = enrollment.class_section_id
                          JOIN public.teachers AS teacher_target
                            ON teacher_target.id = teaching.teacher_id
                          JOIN public.class_sections AS class_section
                            ON class_section.id = enrollment.class_section_id
                          WHERE student_actor.user_id = public.get_user_id()
                            AND student_actor.school_id = public.get_school_id()
                            AND teacher_target.user_id = target.id
                            AND teacher_target.school_id = public.get_school_id()
                            AND class_section.school_id = public.get_school_id()
                      )
                  )
              )
        );
$function$;

REVOKE ALL
ON FUNCTION public.user_relationship_visible_to_actor(UUID)
FROM PUBLIC;

DROP POLICY IF EXISTS users_select_policy ON public.users;
CREATE POLICY users_select_policy
ON public.users
FOR SELECT
USING (
    id = public.get_user_id()
    OR (public.is_school_manager() AND school_id = public.get_school_id())
    OR public.user_relationship_visible_to_actor(id)
);

-- Source-revision insertion is already limited by knowledge_source_files RLS to
-- PlatformAdmin or a SchoolManager operating in their own school.  The trigger
-- needs a trusted write because SELECT ... FOR UPDATE is also checked against
-- the knowledge_assets UPDATE policy, which intentionally remains admin-only.
-- SECURITY DEFINER is therefore bounded again inside the function by actor role,
-- actor identity, school and the exact NEW.asset_id from the inserted source row.
CREATE OR REPLACE FUNCTION public.advance_knowledge_current_source()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $function$
DECLARE
    asset_status public.knowledge_asset_status;
    existing_source UUID;
    actor_role TEXT := public.get_role();
    actor_id UUID := public.get_user_id();
    actor_school UUID := public.get_school_id();
BEGIN
    IF actor_id IS NULL
       OR actor_role NOT IN ('SchoolManager', 'PlatformAdmin')
       OR public.get_elevated_operation() THEN
        RAISE EXCEPTION 'Governed source revision requires a non-elevated manager/admin actor'
            USING ERRCODE = '42501';
    END IF;

    IF actor_role = 'SchoolManager' AND actor_school IS NULL THEN
        RAISE EXCEPTION 'SchoolManager source revision requires school scope'
            USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.users AS actor
        JOIN public.roles AS role_row ON role_row.id = actor.role_id
        WHERE actor.id = actor_id
          AND actor.is_active = TRUE
          AND role_row.name::text = actor_role
          AND (
              actor_role = 'PlatformAdmin'
              OR actor.school_id = actor_school
          )
    ) THEN
        RAISE EXCEPTION 'Governed source revision actor is not active in the claimed scope'
            USING ERRCODE = '42501';
    END IF;

    SELECT asset.status, asset.current_source_file_id
    INTO asset_status, existing_source
    FROM public.knowledge_assets AS asset
    WHERE asset.id = NEW.asset_id
      AND (
          actor_role = 'PlatformAdmin'
          OR asset.school_id = actor_school
      )
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Knowledge asset does not exist in the actor scope for source revision'
            USING ERRCODE = '23503';
    END IF;

    IF asset_status = 'archived' AND existing_source IS NOT NULL THEN
        RAISE EXCEPTION 'Archived knowledge assets cannot receive a new source revision'
            USING ERRCODE = '23514';
    END IF;

    UPDATE public.knowledge_assets AS asset
    SET current_source_file_id = NEW.id,
        status = CASE
            WHEN existing_source IS NULL THEN asset.status
            WHEN asset.status IN ('submitted', 'ocr_pending') THEN asset.status
            ELSE 'ocr_pending'::public.knowledge_asset_status
        END,
        reviewed_by = CASE WHEN existing_source IS NULL THEN asset.reviewed_by ELSE NULL END,
        published_at = CASE WHEN existing_source IS NULL THEN asset.published_at ELSE NULL END,
        failure_reason = CASE WHEN existing_source IS NULL THEN asset.failure_reason ELSE NULL END
    WHERE asset.id = NEW.asset_id
      AND (
          actor_role = 'PlatformAdmin'
          OR asset.school_id = actor_school
      );

    IF existing_source IS NOT NULL THEN
        UPDATE public.teacher_asset_selections
        SET enabled = FALSE,
            updated_at = NOW()
        WHERE asset_id = NEW.asset_id
          AND enabled = TRUE;

        UPDATE public.ingestion_jobs
        SET status = 'cancelled',
            finished_at = NOW(),
            error_message = 'Superseded by a new governed source revision',
            updated_at = NOW()
        WHERE asset_id = NEW.asset_id
          AND status IN ('queued', 'running');
    END IF;

    RETURN NEW;
END;
$function$;

REVOKE ALL
ON FUNCTION public.advance_knowledge_current_source()
FROM PUBLIC;

COMMENT ON FUNCTION public.user_relationship_visible_to_actor(UUID) IS
    'Boolean-only SECURITY DEFINER relationship check used by users_select_policy; actor and target remain school/relationship scoped.';
COMMENT ON FUNCTION public.advance_knowledge_current_source() IS
    'Trusted source-revision trigger boundary; manager writes are limited to the exact inserted same-school source revision and do not grant general asset UPDATE access.';
