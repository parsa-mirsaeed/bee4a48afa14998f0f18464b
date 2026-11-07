-- Durable, non-authoritative AI grading work queue.
--
-- Queue creation is part of the student finalize transaction. The actual
-- inference worker and teacher publication workflow are deliberately separate
-- changes. Do not enqueue file-bearing work until a scoped OCR pipeline exists.
-- No Student, Parent or Data API actor may read or mutate this private queue.

CREATE TABLE public.ai_grading_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE RESTRICT,
    submission_id UUID NOT NULL REFERENCES public.submissions(id) ON DELETE RESTRICT,
    submission_revision UUID NOT NULL,
    rubric_approval_id UUID NOT NULL
        REFERENCES public.ai_grading_rubric_approvals(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lease_owner UUID,
    lease_until TIMESTAMPTZ,
    last_error_code TEXT
        CHECK (last_error_code IS NULL OR last_error_code ~ '^[a-z0-9_]{1,64}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    CONSTRAINT ai_grading_jobs_revision_approval_unique
        UNIQUE (submission_id, submission_revision, rubric_approval_id),
    CHECK (submission_revision <> '00000000-0000-0000-0000-000000000000'::UUID),
    CHECK (status <> 'running' OR lease_owner IS NOT NULL)
);

CREATE INDEX ai_grading_jobs_ready_idx
    ON public.ai_grading_jobs (available_at, created_at)
    WHERE status = 'queued';
CREATE INDEX ai_grading_jobs_school_status_idx
    ON public.ai_grading_jobs (school_id, status, created_at DESC);
CREATE INDEX ai_grading_jobs_lease_idx
    ON public.ai_grading_jobs (lease_until)
    WHERE status = 'running';

ALTER TABLE public.ai_grading_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ai_grading_jobs FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.ai_grading_jobs FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        REVOKE ALL ON public.ai_grading_jobs FROM anon;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        REVOKE ALL ON public.ai_grading_jobs FROM authenticated;
    END IF;
END $$;
-- FORCE RLS applies even to a non-BYPASSRLS migration owner. Grant
-- precisely SELECT/INSERT/UPDATE to the *effective table owner* so its
-- SECURITY DEFINER triggers may create/cancel work. No ordinary runtime
-- role can inherit this identity; runtime table grants are revoked separately.
-- No owner DELETE policy: queue audit history is append-or-status-only.
CREATE POLICY ai_grading_jobs_owner_select ON public.ai_grading_jobs
FOR SELECT USING (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid = 'public.ai_grading_jobs'::regclass)
    )
);
CREATE POLICY ai_grading_jobs_owner_insert ON public.ai_grading_jobs
FOR INSERT WITH CHECK (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid = 'public.ai_grading_jobs'::regclass)
    )
);
CREATE POLICY ai_grading_jobs_owner_update ON public.ai_grading_jobs
FOR UPDATE USING (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid = 'public.ai_grading_jobs'::regclass)
    )
)
WITH CHECK (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid = 'public.ai_grading_jobs'::regclass)
    )
);
-- Future worker claim/complete functions require a separate audit.

CREATE OR REPLACE FUNCTION edutalent_internal.enqueue_ai_grading_if_eligible(
    p_submission_id UUID,
    p_approval_id UUID DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    INSERT INTO public.ai_grading_jobs (
        school_id, submission_id, submission_revision, rubric_approval_id,
        status, available_at
    )
    SELECT cs.school_id, s.id, s.revision, approval.id, 'queued', NOW()
    FROM public.submissions s
    JOIN public.custom_assignments ca
      ON ca.id = s.custom_assignment_id AND ca.student_id = s.student_id
    JOIN public.assignments a ON a.id = ca.assignment_id
    JOIN public.class_sections cs ON cs.id = a.class_section_id
    JOIN public.teachers t
      ON t.id = a.teacher_id AND t.school_id = cs.school_id
    JOIN public.users teacher_user
      ON teacher_user.id = t.user_id AND teacher_user.school_id = cs.school_id
    JOIN public.roles teacher_role ON teacher_role.id = teacher_user.role_id
    JOIN public.teaching_assignments ta
      ON ta.teacher_id = t.id AND ta.class_section_id = a.class_section_id
    JOIN public.students student
      ON student.id = ca.student_id AND student.school_id = cs.school_id
    JOIN public.users student_user
      ON student_user.id = student.user_id
     AND student_user.school_id = cs.school_id
    JOIN public.enrollments enrollment
      ON enrollment.student_id = student.id
     AND enrollment.class_section_id = a.class_section_id
    JOIN public.ai_grading_rubric_approvals approval
      ON approval.custom_assignment_id = ca.id
     AND approval.teacher_id = t.id
     AND approval.school_id = cs.school_id
     AND approval.grade_scale = s.grade_scale
    WHERE s.id = p_submission_id
      AND (p_approval_id IS NULL OR approval.id = p_approval_id)
      AND approval.revoked_at IS NULL
      AND a.status = 'Published'::public.assignment_status
      AND ca.status = 'Submitted'::public.custom_status
      AND ca.submitted_at IS NOT NULL
      AND ca.graded_at IS NULL
      AND s.submitted_at IS NOT NULL
      AND s.grade IS NULL AND s.graded_by IS NULL
      AND s.revision <> '00000000-0000-0000-0000-000000000000'::UUID
      AND teacher_user.is_active
      AND teacher_role.name::TEXT = 'Teacher'
      AND student_user.is_active
      AND jsonb_typeof(s.content->'text') = 'string'
      AND btrim(s.content->>'text') <> ''
      AND octet_length(s.content->>'text') <= 32000
      AND btrim(COALESCE(
          NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
          a.body
      )) <> ''
      AND octet_length(COALESCE(
          NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
          a.body
      )) <= 16000
      AND NOT EXISTS (
          SELECT 1
          FROM public.submission_attachments attachment
          WHERE attachment.custom_assignment_id = ca.id
            AND attachment.student_id = ca.student_id
            AND attachment.school_id = cs.school_id
            AND attachment.status <> 'removed'
      )
      AND NOT EXISTS (
          SELECT 1 FROM public.ai_grading_suggestions suggestion
          WHERE suggestion.submission_id = s.id
            AND suggestion.submission_revision = s.revision
            AND suggestion.rubric_approval_id = approval.id
      )
    -- Hold an approval row-share lock through the surrounding submission
    -- transaction. Concurrent revocation waits: either it commits first
    -- (and enqueue sees revoked_at), or it waits for enqueue to commit and
    -- then cancels the newly visible job. FK key-share is insufficient.
    FOR SHARE OF approval
    ON CONFLICT (submission_id, submission_revision, rubric_approval_id)
       DO NOTHING;
END;
$;
REVOKE ALL ON FUNCTION
    edutalent_internal.enqueue_ai_grading_if_eligible(UUID, UUID) FROM PUBLIC;

CREATE OR REPLACE FUNCTION edutalent_internal.sync_ai_grading_jobs_on_submission()
RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF NEW.revision IS DISTINCT FROM OLD.revision THEN
        -- Cancellation and new enqueue commit atomically with the finalization
        -- revision update; retries with an unchanged revision do nothing.
        UPDATE public.ai_grading_jobs
        SET status = 'cancelled',
            completed_at = NOW(),
            lease_owner = NULL,
            lease_until = NULL,
            last_error_code = 'submission_revised'
        WHERE submission_id = NEW.id
          AND submission_revision <> NEW.revision
          AND status IN ('queued', 'running');

        PERFORM edutalent_internal.enqueue_ai_grading_if_eligible(NEW.id, NULL);
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.sync_ai_grading_jobs_on_submission()
    FROM PUBLIC;

CREATE TRIGGER submissions_enqueue_ai_grading
AFTER UPDATE OF revision ON public.submissions
FOR EACH ROW
WHEN (NEW.revision IS DISTINCT FROM OLD.revision)
EXECUTE FUNCTION edutalent_internal.sync_ai_grading_jobs_on_submission();

CREATE OR REPLACE FUNCTION edutalent_internal.sync_ai_grading_jobs_on_approval()
RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    related_submission UUID;
BEGIN
    IF TG_OP = 'INSERT' THEN
        -- Teacher may approve a rubric *after* a student finalized work.
        FOR related_submission IN
            SELECT s.id FROM public.submissions s
            WHERE s.custom_assignment_id = NEW.custom_assignment_id
        LOOP
            PERFORM edutalent_internal.enqueue_ai_grading_if_eligible(
                related_submission, NEW.id
            );
        END LOOP;
    ELSIF OLD.revoked_at IS NULL AND NEW.revoked_at IS NOT NULL THEN
        -- A revoked rubric must not continue to consume external AI tokens.
        UPDATE public.ai_grading_jobs
        SET status = 'cancelled',
            completed_at = NOW(),
            lease_owner = NULL,
            lease_until = NULL,
            last_error_code = 'rubric_revoked'
        WHERE rubric_approval_id = NEW.id
          AND status IN ('queued', 'running');
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.sync_ai_grading_jobs_on_approval()
    FROM PUBLIC;

CREATE TRIGGER ai_grading_approval_enqueue
AFTER INSERT OR UPDATE OF revoked_at
ON public.ai_grading_rubric_approvals
FOR EACH ROW
EXECUTE FUNCTION edutalent_internal.sync_ai_grading_jobs_on_approval();

-- Make older eligible finalized submissions discoverable after deploying this
-- migration. This is idempotent and does not call an LLM or publish a grade.
DO $$
DECLARE
    candidate RECORD;
BEGIN
    FOR candidate IN
        SELECT s.id AS submission_id, approval.id AS approval_id
        FROM public.ai_grading_rubric_approvals approval
        JOIN public.submissions s
          ON s.custom_assignment_id = approval.custom_assignment_id
        WHERE approval.revoked_at IS NULL
    LOOP
        PERFORM edutalent_internal.enqueue_ai_grading_if_eligible(
            candidate.submission_id, candidate.approval_id
        );
    END LOOP;
END;
$$;
