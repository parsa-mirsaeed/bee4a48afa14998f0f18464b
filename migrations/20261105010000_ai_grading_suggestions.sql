-- Private, non-authoritative AI grading suggestions. No role can publish a
-- grade by inserting into this table. Application integration is separate.
CREATE TABLE public.ai_grading_suggestions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id UUID NOT NULL REFERENCES public.submissions(id) ON DELETE RESTRICT,
    submission_revision UUID NOT NULL,
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE RESTRICT,
    rubric_fingerprint TEXT NOT NULL CHECK (rubric_fingerprint ~ '^[0-9a-f]{64}$'),
    contract_version SMALLINT NOT NULL CHECK (contract_version = 1),
    grade_scale SMALLINT NOT NULL CHECK (grade_scale IN (20, 100)),
    suggested_grade DOUBLE PRECISION NOT NULL,
    confidence DOUBLE PRECISION NOT NULL,
    feedback TEXT NOT NULL CHECK (char_length(feedback) BETWEEN 1 AND 4000),
    criteria JSONB NOT NULL CHECK (
        jsonb_typeof(criteria) = 'array'
        AND jsonb_array_length(criteria) BETWEEN 1 AND 20
    ),
    status TEXT NOT NULL DEFAULT 'pending_review'
        CHECK (status IN ('pending_review', 'rejected', 'accepted')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ,
    reviewed_by UUID REFERENCES public.teachers(id) ON DELETE RESTRICT,
    CHECK (submission_revision <> '00000000-0000-0000-0000-000000000000'::uuid),
    CHECK (suggested_grade >= 0 AND suggested_grade <= grade_scale),
    CHECK (confidence >= 0 AND confidence <= 1),
    CHECK (
        (status = 'pending_review' AND reviewed_at IS NULL AND reviewed_by IS NULL)
        OR (status <> 'pending_review' AND reviewed_at IS NOT NULL AND reviewed_by IS NOT NULL)
    ),
    UNIQUE (submission_id, submission_revision)
);
CREATE INDEX ai_grading_suggestions_school_pending
    ON public.ai_grading_suggestions (school_id, created_at)
    WHERE status = 'pending_review';

-- Defense in depth: deny Data API and direct runtime access until dedicated
-- tenant-authorized read/write functions are implemented and reviewed.
ALTER TABLE public.ai_grading_suggestions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ai_grading_suggestions FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.ai_grading_suggestions FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        REVOKE ALL ON public.ai_grading_suggestions FROM anon;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        REVOKE ALL ON public.ai_grading_suggestions FROM authenticated;
    END IF;
END $$;
