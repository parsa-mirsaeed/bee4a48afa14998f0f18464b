-- A constrained NOBYPASSRLS application database login is not the
-- migration owner. The existing owner-only read policy intentionally denies
-- operator policy inspection, but an authorized Teacher must be able to read
-- just the consent bit for their own school before original-PDF model egress.
-- No INSERT/UPDATE/DELETE policies or Data API grants are added.

CREATE POLICY ai_grading_pdf_optin_active_teacher_select
ON public.ai_grading_pdf_school_optins
FOR SELECT
USING (
    public.get_role() = 'Teacher'
    AND public.get_elevated_operation() IS NOT TRUE
    AND public.get_user_id() IS NOT NULL
    AND public.get_school_id() = ai_grading_pdf_school_optins.school_id
    AND EXISTS (
        SELECT 1
        FROM public.users teacher_user
        JOIN public.roles teacher_role ON teacher_role.id = teacher_user.role_id
        WHERE teacher_user.id = public.get_user_id()
          AND teacher_user.is_active IS TRUE
          AND teacher_user.school_id = ai_grading_pdf_school_optins.school_id
          AND teacher_role.name::text = 'Teacher'
    )
);

-- Preserve deny-all Data API posture. The separately configured application
-- role receives normal SELECT on all public tables, but FORCE RLS restricts
-- its visibility to only the actor's canonical school. No school can be
-- opted in without an owner-executed DDL migration (plus write trigger).
REVOKE ALL ON public.ai_grading_pdf_school_optins FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        REVOKE ALL ON public.ai_grading_pdf_school_optins FROM anon;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        REVOKE ALL ON public.ai_grading_pdf_school_optins FROM authenticated;
    END IF;
END $$;

COMMENT ON POLICY ai_grading_pdf_optin_active_teacher_select
ON public.ai_grading_pdf_school_optins IS
    'Read-only own-school PDF grading consent for active Teacher under NOBYPASSRLS application role. No request actor may mutate opt-ins.';
