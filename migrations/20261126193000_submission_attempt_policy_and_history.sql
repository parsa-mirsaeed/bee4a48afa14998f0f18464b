-- ET-UX-04: additive foundation for bounded, immutable accepted attempts.
-- This migration is part of an atomic feature PR; the application must not
-- deploy it before all mutation paths and policy controls use the new contract.
-- Legacy rows are retained; only the latest demonstrably accepted revision per
-- custom assignment is recorded. Unrecoverable overwritten work is not invented.
ALTER TABLE public.assignments
    ADD COLUMN max_submission_attempts SMALLINT NOT NULL DEFAULT 1
        CHECK (max_submission_attempts BETWEEN 1 AND 10),
    ADD COLUMN submission_policy_version BIGINT NOT NULL DEFAULT 1
        CHECK (submission_policy_version > 0);

-- Advance the version only on an actual policy change. The final teacher
-- command will compare its expected version while locking this assignment.
CREATE FUNCTION public.advance_submission_policy_version()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    IF NEW.max_submission_attempts IS DISTINCT FROM OLD.max_submission_attempts THEN
        NEW.submission_policy_version := OLD.submission_policy_version + 1;
    ELSE
        NEW.submission_policy_version := OLD.submission_policy_version;
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION public.advance_submission_policy_version() FROM PUBLIC;
CREATE TRIGGER assignment_submission_policy_version
BEFORE UPDATE OF max_submission_attempts, submission_policy_version ON public.assignments
FOR EACH ROW EXECUTE FUNCTION public.advance_submission_policy_version();

-- Composite references prevent an attempt from being attached to another
-- student or assignment even if an application bug supplies unrelated IDs.
ALTER TABLE public.custom_assignments
    ADD CONSTRAINT custom_assignment_attempt_identity UNIQUE (id, student_id);
ALTER TABLE public.submissions
    ADD CONSTRAINT submission_attempt_identity UNIQUE (id, custom_assignment_id, student_id);
ALTER TABLE public.submission_attachments
    ADD CONSTRAINT submission_original_attempt_identity UNIQUE (id, submission_id);

CREATE TABLE public.submission_attempts (
    custom_assignment_id UUID NOT NULL,
    student_id UUID NOT NULL,
    school_id UUID NOT NULL REFERENCES public.schools(id),
    submission_id UUID NOT NULL,
    attempt_number SMALLINT NOT NULL CHECK (attempt_number BETWEEN 1 AND 10),
    submission_revision UUID NOT NULL,
    request_id UUID,
    request_fingerprint TEXT,
    accepted_content JSONB NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (custom_assignment_id, attempt_number),
    UNIQUE (custom_assignment_id, attempt_number, submission_id),
    UNIQUE (custom_assignment_id, request_id),
    UNIQUE (submission_id, submission_revision),
    CHECK (request_id IS NULL OR request_fingerprint IS NOT NULL),
    FOREIGN KEY (custom_assignment_id, student_id)
        REFERENCES public.custom_assignments(id, student_id) ON DELETE CASCADE,
    FOREIGN KEY (submission_id, custom_assignment_id, student_id)
        REFERENCES public.submissions(id, custom_assignment_id, student_id) ON DELETE CASCADE
);
CREATE INDEX submission_attempts_submission_idx ON public.submission_attempts(submission_id);

-- Each accepted version references exactly its committed originals; the
-- original object is never copied or mutated for a subsequent attempt.
CREATE TABLE public.submission_attempt_originals (
    custom_assignment_id UUID NOT NULL,
    attempt_number SMALLINT NOT NULL,
    submission_id UUID NOT NULL,
    attachment_id UUID NOT NULL,
    PRIMARY KEY (custom_assignment_id, attempt_number, attachment_id),
    FOREIGN KEY (custom_assignment_id, attempt_number, submission_id)
        REFERENCES public.submission_attempts(custom_assignment_id, attempt_number, submission_id)
        ON DELETE CASCADE,
    FOREIGN KEY (attachment_id, submission_id)
        REFERENCES public.submission_attachments(id, submission_id) ON DELETE RESTRICT
);
CREATE INDEX submission_attempt_originals_attachment_idx
    ON public.submission_attempt_originals(attachment_id, submission_id);

-- BEGIN LEGACY ATTEMPT BACKFILL
-- Preserve the latest accepted submission only; historical duplicate rows
-- are retained untouched. A submission timestamp alone is not sufficient
-- because it is also defaulted on the historical submissions table.
INSERT INTO public.submission_attempts (
    custom_assignment_id, student_id, school_id, submission_id, attempt_number,
    submission_revision, request_id, request_fingerprint, accepted_content, accepted_at
)
SELECT DISTINCT ON (ca.id)
    ca.id, ca.student_id, cs.school_id, s.id, 1,
    s.revision, s.last_submit_request, s.last_submit_fingerprint, s.content, s.submitted_at
FROM public.custom_assignments ca
JOIN public.assignments a ON a.id = ca.assignment_id
JOIN public.class_sections cs ON cs.id = a.class_section_id
JOIN public.students st ON st.id = ca.student_id AND st.school_id = cs.school_id
JOIN public.submissions s ON s.custom_assignment_id = ca.id AND s.student_id = ca.student_id
WHERE ca.status IN ('Submitted'::public.custom_status, 'Graded'::public.custom_status)
   OR ca.submitted_at IS NOT NULL OR ca.graded_at IS NOT NULL
ORDER BY ca.id, s.submitted_at DESC, s.id DESC
ON CONFLICT (custom_assignment_id, attempt_number) DO NOTHING;

INSERT INTO public.submission_attempt_originals (
    custom_assignment_id, attempt_number, submission_id, attachment_id
)
SELECT attempt.custom_assignment_id, attempt.attempt_number, attempt.submission_id, original.id
FROM public.submission_attempts attempt
JOIN public.submission_attachments original
  ON original.submission_id = attempt.submission_id
 AND original.custom_assignment_id = attempt.custom_assignment_id
 AND original.student_id = attempt.student_id
 AND original.school_id = attempt.school_id
 AND original.status = 'submitted'
WHERE attempt.attempt_number = 1
ON CONFLICT (custom_assignment_id, attempt_number, attachment_id) DO NOTHING;
-- END LEGACY ATTEMPT BACKFILL

-- No direct Data API privileges. Production's constrained app role still
-- requires the exact relationship policies below. No app insert/update/delete
-- policy at the legacy backfill boundary; the exact append policies below
-- apply only to runtime finalization. No update/delete policies.
ALTER TABLE public.submission_attempts ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.submission_attempts FORCE ROW LEVEL SECURITY;
ALTER TABLE public.submission_attempt_originals ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.submission_attempt_originals FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.submission_attempts, public.submission_attempt_originals FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        REVOKE ALL ON public.submission_attempts, public.submission_attempt_originals FROM anon;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        REVOKE ALL ON public.submission_attempts, public.submission_attempt_originals FROM authenticated;
    END IF;
END;
$$;

CREATE POLICY submission_attempt_student_read ON public.submission_attempts FOR SELECT
USING (
    NOT public.get_elevated_operation()
    AND public.owns_submission_assignment(custom_assignment_id, student_id, school_id)
);
CREATE POLICY submission_attempt_teacher_read ON public.submission_attempts FOR SELECT
USING (
    NOT public.get_elevated_operation()
    AND public.can_read_submission_original(custom_assignment_id, student_id, school_id, submission_id)
);
CREATE POLICY submission_attempt_original_read ON public.submission_attempt_originals FOR SELECT
USING (
    EXISTS (
        SELECT 1 FROM public.submission_attempts attempt
        WHERE attempt.custom_assignment_id = submission_attempt_originals.custom_assignment_id
          AND attempt.attempt_number = submission_attempt_originals.attempt_number
          AND attempt.submission_id = submission_attempt_originals.submission_id
    )
);

CREATE FUNCTION public.reject_submission_attempt_update()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    RAISE EXCEPTION 'Accepted submission history is immutable' USING ERRCODE = '55000';
END;
$$;
REVOKE ALL ON FUNCTION public.reject_submission_attempt_update() FROM PUBLIC;
CREATE TRIGGER submission_attempt_immutable
BEFORE UPDATE ON public.submission_attempts
FOR EACH ROW EXECUTE FUNCTION public.reject_submission_attempt_update();
CREATE TRIGGER submission_attempt_original_immutable
BEFORE UPDATE ON public.submission_attempt_originals
FOR EACH ROW EXECUTE FUNCTION public.reject_submission_attempt_update();

COMMENT ON TABLE public.submission_attempts IS
    'Accepted work snapshots, one durable row per attempt; parent submission owns retention. Legacy backfill records only latest accepted work.';
COMMENT ON TABLE public.submission_attempt_originals IS
    'Immutable references to original bytes accepted for a specific submission revision.';

-- Serialize student writes with teacher policy changes using the existing
-- exact-actor lock. No additional privileges or public entry point.
CREATE OR REPLACE FUNCTION edutalent_internal.lock_student_submission_assignment(p_assignment UUID)
RETURNS TABLE(student_id UUID, school_id UUID, graded_at TIMESTAMPTZ)
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
    SELECT ca.student_id, cs.school_id, ca.graded_at
    FROM public.custom_assignments ca
    JOIN public.assignments a ON a.id=ca.assignment_id
    JOIN public.class_sections cs ON cs.id=a.class_section_id
    JOIN public.students s ON s.id=ca.student_id
    JOIN public.users u ON u.id=s.user_id
    JOIN public.roles r ON r.id=u.role_id
    JOIN public.enrollments e ON e.student_id=s.id AND e.class_section_id=cs.id
    WHERE ca.id=p_assignment
      AND public.get_role()='Student'
      AND NOT public.get_elevated_operation()
      AND public.get_user_id() IS NOT NULL
      AND public.get_school_id() IS NOT NULL
      AND s.user_id=public.get_user_id()
      AND u.is_active AND r.name::text='Student'
      AND s.school_id=public.get_school_id()
      AND u.school_id=public.get_school_id()
      AND cs.school_id=public.get_school_id()
      AND a.status='Published'::public.assignment_status
    FOR UPDATE OF ca
    FOR SHARE OF a
$$;
REVOKE ALL ON FUNCTION edutalent_internal.lock_student_submission_assignment(UUID) FROM PUBLIC;

CREATE POLICY submission_attempt_student_append ON public.submission_attempts FOR INSERT
WITH CHECK (
    NOT public.get_elevated_operation()
    AND public.owns_submission_assignment(custom_assignment_id, student_id, school_id)
    AND EXISTS (
        SELECT 1 FROM public.submissions s
        JOIN public.custom_assignments ca ON ca.id = s.custom_assignment_id
        WHERE s.id = submission_attempts.submission_id
          AND ca.id = submission_attempts.custom_assignment_id
          AND ca.graded_at IS NULL AND s.grade IS NULL
          AND s.student_id = submission_attempts.student_id
          AND s.revision = submission_attempts.submission_revision
          AND s.content = submission_attempts.accepted_content
          AND s.submitted_at = submission_attempts.accepted_at
          AND s.last_submit_request IS NOT DISTINCT FROM submission_attempts.request_id
          AND s.last_submit_fingerprint IS NOT DISTINCT FROM submission_attempts.request_fingerprint
    )
);
CREATE POLICY submission_attempt_original_student_append
ON public.submission_attempt_originals FOR INSERT
WITH CHECK (
    public.get_role() = 'Student' AND NOT public.get_elevated_operation()
    AND EXISTS (
        SELECT 1 FROM public.submission_attempts attempt
        JOIN public.submissions s ON s.id = attempt.submission_id
        JOIN public.submission_attachments original
          ON original.id = submission_attempt_originals.attachment_id
         AND original.submission_id = s.id
         AND original.custom_assignment_id = attempt.custom_assignment_id
         AND original.student_id = attempt.student_id
         AND original.school_id = attempt.school_id
         AND original.status = 'submitted'
        WHERE attempt.custom_assignment_id = submission_attempt_originals.custom_assignment_id
          AND attempt.attempt_number = submission_attempt_originals.attempt_number
          AND attempt.submission_id = submission_attempt_originals.submission_id
          AND attempt.submission_revision = s.revision
          AND public.owns_submission_assignment(attempt.custom_assignment_id, attempt.student_id, attempt.school_id)
    )
);
-- Reject forged sequences or exhausted caps using the same lock as finalize.
CREATE FUNCTION public.guard_submission_attempt_append()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE
    cap SMALLINT;
    used BIGINT;
BEGIN
    IF public.get_role() IS DISTINCT FROM 'Student'
       OR public.get_elevated_operation()
       OR NOT public.owns_submission_assignment(NEW.custom_assignment_id, NEW.student_id, NEW.school_id) THEN
        RAISE EXCEPTION 'Submission attempt actor denied' USING ERRCODE = '42501';
    END IF;
    PERFORM 1 FROM edutalent_internal.lock_student_submission_assignment(NEW.custom_assignment_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Submission attempt actor denied' USING ERRCODE = '42501';
    END IF;
    SELECT a.max_submission_attempts INTO cap
    FROM public.custom_assignments ca
    JOIN public.assignments a ON a.id = ca.assignment_id
    WHERE ca.id = NEW.custom_assignment_id AND ca.graded_at IS NULL;
    SELECT count(*) INTO used FROM public.submission_attempts
    WHERE custom_assignment_id = NEW.custom_assignment_id;
    IF cap IS NULL OR used >= cap OR NEW.attempt_number <> used + 1 THEN
        RAISE EXCEPTION 'Submission attempt limit or sequence conflict' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION public.guard_submission_attempt_append() FROM PUBLIC;
CREATE TRIGGER submission_attempt_append_guard
BEFORE INSERT ON public.submission_attempts
FOR EACH ROW EXECUTE FUNCTION public.guard_submission_attempt_append();
