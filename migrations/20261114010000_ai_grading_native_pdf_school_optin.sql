-- Strict, initially disabled native-PDF grading enqueue.
-- This does not enable any worker or provider. Only the migration owner may
-- enable a single school via a separately approved operator action.
-- Original PDF bytes never enter the job queue; it contains IDs only.

CREATE TABLE public.ai_grading_pdf_school_optins (
    school_id UUID PRIMARY KEY REFERENCES public.schools(id) ON DELETE RESTRICT,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
ALTER TABLE public.ai_grading_pdf_school_optins ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ai_grading_pdf_school_optins FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.ai_grading_pdf_school_optins FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='anon') THEN
        REVOKE ALL ON public.ai_grading_pdf_school_optins FROM anon;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='authenticated') THEN
        REVOKE ALL ON public.ai_grading_pdf_school_optins FROM authenticated;
    END IF;
END $$;
CREATE POLICY ai_grading_pdf_optin_owner_select
ON public.ai_grading_pdf_school_optins FOR SELECT
USING (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid='public.ai_grading_pdf_school_optins'::regclass)
    )
);
CREATE POLICY ai_grading_pdf_optin_owner_insert
ON public.ai_grading_pdf_school_optins FOR INSERT
WITH CHECK (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid='public.ai_grading_pdf_school_optins'::regclass)
    )
);
CREATE POLICY ai_grading_pdf_optin_owner_update
ON public.ai_grading_pdf_school_optins FOR UPDATE
USING (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid='public.ai_grading_pdf_school_optins'::regclass)
    )
)
WITH CHECK (
    current_user = pg_get_userbyid(
        (SELECT relowner FROM pg_class
         WHERE oid='public.ai_grading_pdf_school_optins'::regclass)
    )
);

-- Reinstate all of the queue's existing revision/rubric/Teacher/enrollment
-- rules, modifying only the alternative source eligibility predicate.
CREATE OR REPLACE FUNCTION edutalent_internal.enqueue_ai_grading_if_eligible(
    p_submission_id UUID,
    p_approval_id UUID DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
BEGIN
    -- Take the lock in its own SQL statement. This avoids ambiguity between
    -- SELECT row-locking syntax and INSERT ... ON CONFLICT, and serializes
    -- enqueue with revocation across separate transactions.
    PERFORM 1
    FROM public.ai_grading_rubric_approvals approval
    JOIN public.submissions s
      ON s.custom_assignment_id = approval.custom_assignment_id
    WHERE s.id = p_submission_id
      AND (p_approval_id IS NULL OR approval.id = p_approval_id)
      AND approval.revoked_at IS NULL
    FOR SHARE OF approval;

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
      AND btrim(COALESCE(
          NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
          a.body
      )) <> ''
      AND octet_length(COALESCE(
          NULLIF(ca.prompt_ctx #>> '{personalized_assignment,body}', ''),
          a.body
      )) <= 16000
      AND (
          -- Preserve the existing exact text-only eligibility and deny
          -- unfinished/pending originals before model inference.
          (
              jsonb_typeof(s.content->'text') = 'string'
              AND btrim(s.content->>'text') <> ''
              AND octet_length(s.content->>'text') <= 32000
              AND NOT EXISTS (
                  SELECT 1
                  FROM public.submission_attachments attachment
                  WHERE attachment.custom_assignment_id = ca.id
                    AND attachment.student_id = ca.student_id
                    AND attachment.school_id = cs.school_id
                    AND attachment.status <> 'removed'
              )
          )
          OR
          (
              -- Native documents require explicit owner-controlled,
              -- per-school enrollment. No school is enabled by migration.
              EXISTS (
                  SELECT 1 FROM public.ai_grading_pdf_school_optins optin
                  WHERE optin.school_id = cs.school_id
                    AND optin.enabled IS TRUE
              )
              AND octet_length(COALESCE(s.content->>'text', '')) <= 32000
              -- Exactly one finalized original; pending/ready and images
              -- cannot be silently omitted from the graded answer.
              AND (
                  SELECT count(*)
                  FROM public.submission_attachments attachment
                  WHERE attachment.custom_assignment_id = ca.id
                    AND attachment.student_id = ca.student_id
                    AND attachment.school_id = cs.school_id
                    AND attachment.submission_id = s.id
                    AND attachment.status <> 'removed'
              ) = 1
              AND EXISTS (
                  SELECT 1 FROM public.submission_attachments attachment
                  WHERE attachment.custom_assignment_id = ca.id
                    AND attachment.student_id = ca.student_id
                    AND attachment.school_id = cs.school_id
                    AND attachment.submission_id = s.id
                    AND attachment.status = 'submitted'
                    AND attachment.media_type = 'application/pdf'
                    AND attachment.byte_size BETWEEN 1 AND 1000000
                    AND attachment.sha256 ~ '^[0-9a-f]{64}
      AND NOT EXISTS (
          SELECT 1 FROM public.ai_grading_suggestions suggestion
          WHERE suggestion.submission_id = s.id
            AND suggestion.submission_revision = s.revision
            AND suggestion.rubric_approval_id = approval.id
      )
    ON CONFLICT (submission_id, submission_revision, rubric_approval_id)
       DO NOTHING;
END;
$$;

              )
          )
      )
      AND NOT EXISTS (
          SELECT 1 FROM public.ai_grading_suggestions suggestion
          WHERE suggestion.submission_id = s.id
            AND suggestion.submission_revision = s.revision
            AND suggestion.rubric_approval_id = approval.id
      )
    ON CONFLICT (submission_id, submission_revision, rubric_approval_id)
       DO NOTHING;
END;
$$;

REVOKE ALL ON FUNCTION
    edutalent_internal.enqueue_ai_grading_if_eligible(UUID, UUID) FROM PUBLIC;

COMMENT ON TABLE public.ai_grading_pdf_school_optins IS
    'Owner-only, initially empty school enablement for native PDF grading queue. External gateway and worker separately remain OFF.';
