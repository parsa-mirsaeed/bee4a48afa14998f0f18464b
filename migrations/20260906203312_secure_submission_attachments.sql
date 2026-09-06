-- Private originals: reserve -> upload/verify -> atomic submission finalization.
-- An intent must commit before any external write. Removed intents are durable
-- cleanup tombstones; no database cascade can strand an untracked storage object.
CREATE TABLE IF NOT EXISTS public.submission_attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    custom_assignment_id UUID NOT NULL REFERENCES public.custom_assignments(id) ON DELETE RESTRICT,
    student_id UUID NOT NULL REFERENCES public.students(id) ON DELETE RESTRICT,
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE RESTRICT,
    submission_id UUID REFERENCES public.submissions(id) ON DELETE RESTRICT,
    request_id UUID NOT NULL,
    filename TEXT NOT NULL CHECK (octet_length(filename) BETWEEN 1 AND 255),
    media_type TEXT NOT NULL CHECK (media_type IN ('application/pdf','image/jpeg','image/png')),
    byte_size BIGINT NOT NULL CHECK (byte_size BETWEEN 1 AND 10485760),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','ready','submitted','removed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified_at TIMESTAMPTZ,
    finalized_at TIMESTAMPTZ,
    cleanup_after TIMESTAMPTZ NOT NULL DEFAULT NOW()+INTERVAL '24 hours',
    cleaned_at TIMESTAMPTZ,
    UNIQUE (custom_assignment_id, request_id),
    CHECK ((status='submitted') = (submission_id IS NOT NULL)),
    CHECK (status NOT IN ('ready','submitted') OR verified_at IS NOT NULL),
    CHECK (status<>'submitted' OR finalized_at IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS submission_attachments_assignment ON public.submission_attachments(custom_assignment_id);
CREATE INDEX IF NOT EXISTS submission_attachments_cleanup ON public.submission_attachments(cleanup_after) WHERE status<>'submitted';
ALTER TABLE public.submission_attachments ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.submission_attachments FORCE ROW LEVEL SECURITY;

-- No Data API grant. Only the constrained gateway role receives table rights
-- from configure_database_role.sh, and it still cannot bypass these policies.
REVOKE ALL ON public.submission_attachments FROM PUBLIC;
DO $$ BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname='anon') THEN REVOKE ALL ON public.submission_attachments FROM anon; END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname='authenticated') THEN REVOKE ALL ON public.submission_attachments FROM authenticated; END IF;
END $$;

CREATE OR REPLACE FUNCTION public.owns_submission_assignment(p_assignment UUID, p_student UUID, p_school UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY INVOKER SET search_path=pg_catalog,public AS $$
    SELECT public.get_role()='Student' AND p_school=public.get_school_id() AND EXISTS (
        SELECT 1 FROM public.custom_assignments ca
        JOIN public.assignments a ON a.id=ca.assignment_id
        JOIN public.class_sections cs ON cs.id=a.class_section_id
        JOIN public.students s ON s.id=ca.student_id
        JOIN public.users u ON u.id=s.user_id
        JOIN public.roles r ON r.id=u.role_id
        JOIN public.enrollments e ON e.student_id=s.id AND e.class_section_id=cs.id
        WHERE ca.id=p_assignment AND s.id=p_student AND s.user_id=public.get_user_id()
          AND u.is_active AND r.name::text='Student' AND s.school_id=p_school
          AND u.school_id=p_school AND cs.school_id=p_school AND a.status='Published'::public.assignment_status
    )
$$;
DROP POLICY IF EXISTS submission_attachment_student_read ON public.submission_attachments;
CREATE POLICY submission_attachment_student_read ON public.submission_attachments FOR SELECT
USING (public.owns_submission_assignment(custom_assignment_id,student_id,school_id));
DROP POLICY IF EXISTS submission_attachment_student_insert ON public.submission_attachments;
CREATE POLICY submission_attachment_student_insert ON public.submission_attachments FOR INSERT
WITH CHECK (status='pending' AND public.owns_submission_assignment(custom_assignment_id,student_id,school_id));
DROP POLICY IF EXISTS submission_attachment_student_update ON public.submission_attachments;
CREATE POLICY submission_attachment_student_update ON public.submission_attachments FOR UPDATE
USING (public.owns_submission_assignment(custom_assignment_id,student_id,school_id))
WITH CHECK (public.owns_submission_assignment(custom_assignment_id,student_id,school_id));
DROP POLICY IF EXISTS submission_attachment_teacher_read ON public.submission_attachments;
CREATE POLICY submission_attachment_teacher_read ON public.submission_attachments FOR SELECT USING (
    status='submitted' AND school_id=public.get_school_id() AND public.get_role()='Teacher' AND EXISTS (
        SELECT 1 FROM public.custom_assignments ca
        JOIN public.assignments a ON a.id=ca.assignment_id
        JOIN public.class_sections cs ON cs.id=a.class_section_id
        JOIN public.teachers t ON t.id=a.teacher_id
        JOIN public.users u ON u.id=t.user_id
        JOIN public.roles r ON r.id=u.role_id
        JOIN public.students s ON s.id=ca.student_id
        JOIN public.enrollments e ON e.student_id=s.id AND e.class_section_id=cs.id
        JOIN public.submissions sub ON sub.id=submission_attachments.submission_id
            AND sub.custom_assignment_id=ca.id AND sub.student_id=s.id
        WHERE ca.id=submission_attachments.custom_assignment_id AND s.id=submission_attachments.student_id
          AND u.id=public.get_user_id() AND u.is_active AND r.name::text='Teacher'
          AND u.school_id=submission_attachments.school_id AND cs.school_id=u.school_id
          AND s.school_id=u.school_id AND a.status='Published'::public.assignment_status
    )
);

-- Reject provenance reassignment and mutation/deletion of submitted originals,
-- including direct SQL through a role with broader privileges.
CREATE OR REPLACE FUNCTION public.guard_submission_attachment_original()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
BEGIN
    IF TG_OP='DELETE' THEN
        RAISE EXCEPTION 'Attachment intents require durable cleanup tombstones' USING ERRCODE='55000';
    END IF;
    IF (NEW.id,NEW.custom_assignment_id,NEW.student_id,NEW.school_id,NEW.request_id,NEW.filename,NEW.media_type,NEW.byte_size,NEW.sha256,NEW.created_at)
        IS DISTINCT FROM (OLD.id,OLD.custom_assignment_id,OLD.student_id,OLD.school_id,OLD.request_id,OLD.filename,OLD.media_type,OLD.byte_size,OLD.sha256,OLD.created_at)
       OR OLD.status='submitted' AND NEW IS DISTINCT FROM OLD
       OR OLD.status='removed' AND NEW.status<>'removed'
       OR OLD.status='ready' AND NEW.status='pending' THEN
        RAISE EXCEPTION 'Attachment original is immutable' USING ERRCODE='55000';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS submission_attachment_original_guard ON public.submission_attachments;
CREATE TRIGGER submission_attachment_original_guard BEFORE UPDATE OR DELETE ON public.submission_attachments
FOR EACH ROW EXECUTE FUNCTION public.guard_submission_attachment_original();

-- The scheduler can only claim an expired/unsubmitted object for deletion. Its
-- SECURITY DEFINER implementation is in an unexposed schema, with explicit
-- system-context checks and no caller-controlled object/tenant argument.
CREATE SCHEMA IF NOT EXISTS edutalent_internal;
REVOKE ALL ON SCHEMA edutalent_internal FROM PUBLIC;
CREATE OR REPLACE FUNCTION edutalent_internal.claim_submission_attachment_cleanup()
RETURNS TABLE(attachment_id UUID, school_id UUID)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF public.get_role() IS DISTINCT FROM 'system_job' OR NOT public.get_elevated_operation()
       OR public.get_school_id() IS NOT NULL OR public.get_user_id() IS NULL THEN
        RAISE EXCEPTION 'bounded system queue context required' USING ERRCODE='42501';
    END IF;
    RETURN QUERY WITH candidate AS (
        SELECT a.id FROM public.submission_attachments a
        WHERE a.status<>'submitted' AND a.cleanup_after<=NOW()
        ORDER BY a.cleanup_after,a.id FOR UPDATE SKIP LOCKED LIMIT 1
    ) UPDATE public.submission_attachments a SET status='removed',cleanup_after=NOW()+INTERVAL '5 minutes'
    FROM candidate c WHERE a.id=c.id RETURNING a.id,a.school_id;
END $$;
REVOKE ALL ON FUNCTION edutalent_internal.claim_submission_attachment_cleanup() FROM PUBLIC;
DROP POLICY IF EXISTS submission_attachment_cleanup_read ON public.submission_attachments;
CREATE POLICY submission_attachment_cleanup_read ON public.submission_attachments FOR SELECT USING (
    public.get_role()='system_job' AND public.get_elevated_operation() AND school_id=public.get_school_id() AND status='removed'
);
DROP POLICY IF EXISTS submission_attachment_cleanup_update ON public.submission_attachments;
CREATE POLICY submission_attachment_cleanup_update ON public.submission_attachments FOR UPDATE USING (
    public.get_role()='system_job' AND public.get_elevated_operation() AND school_id=public.get_school_id() AND status='removed'
) WITH CHECK (status='removed' AND school_id=public.get_school_id());

ALTER TABLE public.submissions ADD COLUMN IF NOT EXISTS revision UUID NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE public.submissions ADD COLUMN IF NOT EXISTS last_submit_request UUID;
ALTER TABLE public.submissions ADD COLUMN IF NOT EXISTS last_submit_fingerprint TEXT;
