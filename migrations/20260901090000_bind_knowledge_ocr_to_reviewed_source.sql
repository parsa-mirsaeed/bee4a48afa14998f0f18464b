-- Governed knowledge provenance for source review, OCR, embedding, and publication.
--
-- `knowledge_source_files` is the immutable source-revision history. Each asset
-- points at one canonical current source row. Review evidence is created only
-- after the protected source-review path proves the actual bytes. OCR and
-- embedding persist that already-established source binding; neither operation
-- is allowed to invent or repair source provenance.
--
-- Legacy OCR is intentionally NOT backfilled. A pre-existing OCR row with no
-- source binding remains stale/unbound until a Platform Admin reviews the
-- canonical source and explicitly re-verifies OCR.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE public.knowledge_assets
    ADD COLUMN IF NOT EXISTS current_source_file_id UUID;

ALTER TABLE public.knowledge_ocr_texts
    ADD COLUMN IF NOT EXISTS source_file_id UUID
        REFERENCES public.knowledge_source_files(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS source_sha256 TEXT
        CHECK (source_sha256 IS NULL OR source_sha256 ~ '^[0-9a-f]{64}$');

ALTER TABLE public.knowledge_chunks
    ADD COLUMN IF NOT EXISTS source_file_id UUID
        REFERENCES public.knowledge_source_files(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS source_sha256 TEXT
        CHECK (source_sha256 IS NULL OR source_sha256 ~ '^[0-9a-f]{64}$'),
    ADD COLUMN IF NOT EXISTS ocr_revision UUID;

-- Establish only the canonical source pointer for existing assets. This does
-- not manufacture historical review/OCR/embedding evidence.
WITH canonical_source AS (
    SELECT DISTINCT ON (asset_id) asset_id, id
    FROM public.knowledge_source_files
    ORDER BY asset_id, created_at DESC, id DESC
)
UPDATE public.knowledge_assets AS asset
SET current_source_file_id = source.id
FROM canonical_source AS source
WHERE asset.id = source.asset_id
  AND asset.current_source_file_id IS NULL;

DO $$
BEGIN
    ALTER TABLE public.knowledge_assets
        ADD CONSTRAINT knowledge_assets_current_source_file_fk
        FOREIGN KEY (current_source_file_id)
        REFERENCES public.knowledge_source_files(id)
        ON DELETE RESTRICT;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_knowledge_assets_current_source
    ON public.knowledge_assets (current_source_file_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_ocr_source_file
    ON public.knowledge_ocr_texts (source_file_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_chunks_source_file
    ON public.knowledge_chunks (source_file_id, ocr_revision);

-- Historical source rows are append-only, including for roles that bypass RLS.
CREATE OR REPLACE FUNCTION public.prevent_knowledge_source_revision_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Governed source revisions are append-only; create a new revision instead'
        USING ERRCODE = '55000';
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_source_revision_immutable
    ON public.knowledge_source_files;
CREATE TRIGGER trg_knowledge_source_revision_immutable
BEFORE UPDATE OR DELETE ON public.knowledge_source_files
FOR EACH ROW
EXECUTE FUNCTION public.prevent_knowledge_source_revision_mutation();

-- Only the source-revision insertion boundary may advance the canonical pointer.
-- pg_trigger_depth() makes a direct asset-pointer UPDATE fail even for a caller
-- that can bypass RLS; the nested UPDATE below runs from this source trigger.
CREATE OR REPLACE FUNCTION public.guard_knowledge_current_source_pointer()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.current_source_file_id IS DISTINCT FROM OLD.current_source_file_id
       AND pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION 'Current governed source can change only by appending a source revision'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_guard_knowledge_current_source_pointer
    ON public.knowledge_assets;
CREATE TRIGGER trg_guard_knowledge_current_source_pointer
BEFORE UPDATE OF current_source_file_id ON public.knowledge_assets
FOR EACH ROW
EXECUTE FUNCTION public.guard_knowledge_current_source_pointer();

-- Extend the lifecycle state machine only for the nested source-replacement
-- reset. Direct published -> ocr_pending updates remain forbidden.
CREATE OR REPLACE FUNCTION public.validate_knowledge_asset_status_transition()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;

    IF pg_trigger_depth() > 1
       AND NEW.current_source_file_id IS DISTINCT FROM OLD.current_source_file_id
       AND NEW.status = 'ocr_pending' THEN
        NEW.published_at = NULL;
        NEW.archived_at = NULL;
        NEW.failure_reason = NULL;
        RETURN NEW;
    END IF;

    IF NOT (
        (OLD.status = 'submitted' AND NEW.status IN ('ocr_pending', 'ocr_ready', 'archived', 'failed')) OR
        (OLD.status = 'ocr_pending' AND NEW.status IN ('ocr_ready', 'archived', 'failed')) OR
        (OLD.status = 'ocr_ready' AND NEW.status IN ('embedding_pending', 'archived', 'failed')) OR
        (OLD.status = 'embedding_pending' AND NEW.status IN ('embedded', 'ocr_ready', 'archived', 'failed')) OR
        (OLD.status = 'embedded' AND NEW.status IN ('embedding_pending', 'published', 'archived', 'failed')) OR
        (OLD.status = 'published' AND NEW.status IN ('embedded', 'archived')) OR
        (OLD.status = 'failed' AND NEW.status IN ('ocr_ready', 'embedding_pending', 'archived'))
    ) THEN
        RAISE EXCEPTION 'Invalid knowledge asset status transition: % -> %', OLD.status, NEW.status
            USING ERRCODE = '23514';
    END IF;

    IF NEW.status = 'published' THEN
        NEW.published_at = COALESCE(NEW.published_at, NOW());
        NEW.archived_at = NULL;
        NEW.failure_reason = NULL;
    ELSIF NEW.status = 'archived' THEN
        NEW.archived_at = COALESCE(NEW.archived_at, NOW());
    ELSIF NEW.status <> 'published' THEN
        NEW.published_at = NULL;
    END IF;

    RETURN NEW;
END;
$$;

-- Existing downstream state predates durable source provenance. Do not pretend
-- those OCR/chunk rows were reviewed against the canonical source. Walk only
-- legal historical transitions back to the safe OCR-review stage. The predicate
-- keeps migration replay idempotent once an asset has a real source binding.
UPDATE public.teacher_asset_selections AS selection
SET enabled = FALSE, updated_at = NOW()
WHERE selection.enabled = TRUE
  AND EXISTS (
      SELECT 1
      FROM public.knowledge_assets AS asset
      WHERE asset.id = selection.asset_id
        AND asset.status IN ('embedding_pending', 'embedded', 'published')
        AND NOT EXISTS (
            SELECT 1
            FROM public.knowledge_ocr_texts AS ocr
            WHERE ocr.asset_id = asset.id
              AND ocr.source_file_id = asset.current_source_file_id
              AND ocr.source_sha256 IS NOT NULL
        )
  );

UPDATE public.ingestion_jobs AS job
SET status = 'cancelled',
    finished_at = NOW(),
    error_message = 'Legacy knowledge provenance requires governed re-verification',
    updated_at = NOW()
WHERE job.status IN ('queued', 'running')
  AND EXISTS (
      SELECT 1
      FROM public.knowledge_assets AS asset
      WHERE asset.id = job.asset_id
        AND asset.status IN ('embedding_pending', 'embedded', 'published')
        AND NOT EXISTS (
            SELECT 1
            FROM public.knowledge_ocr_texts AS ocr
            WHERE ocr.asset_id = asset.id
              AND ocr.source_file_id = asset.current_source_file_id
              AND ocr.source_sha256 IS NOT NULL
        )
  );

UPDATE public.knowledge_assets AS asset
SET status = 'embedded'
WHERE asset.status = 'published'
  AND NOT EXISTS (
      SELECT 1 FROM public.knowledge_ocr_texts AS ocr
      WHERE ocr.asset_id = asset.id
        AND ocr.source_file_id = asset.current_source_file_id
        AND ocr.source_sha256 IS NOT NULL
  );

UPDATE public.knowledge_assets AS asset
SET status = 'embedding_pending'
WHERE asset.status = 'embedded'
  AND NOT EXISTS (
      SELECT 1 FROM public.knowledge_ocr_texts AS ocr
      WHERE ocr.asset_id = asset.id
        AND ocr.source_file_id = asset.current_source_file_id
        AND ocr.source_sha256 IS NOT NULL
  );

UPDATE public.knowledge_assets AS asset
SET status = 'ocr_ready', reviewed_by = NULL, failure_reason = NULL
WHERE asset.status = 'embedding_pending'
  AND NOT EXISTS (
      SELECT 1 FROM public.knowledge_ocr_texts AS ocr
      WHERE ocr.asset_id = asset.id
        AND ocr.source_file_id = asset.current_source_file_id
        AND ocr.source_sha256 IS NOT NULL
  );

CREATE OR REPLACE FUNCTION public.advance_knowledge_current_source()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    asset_status public.knowledge_asset_status;
    existing_source UUID;
BEGIN
    SELECT status, current_source_file_id
    INTO asset_status, existing_source
    FROM public.knowledge_assets
    WHERE id = NEW.asset_id
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Knowledge asset does not exist for source revision'
            USING ERRCODE = '23503';
    END IF;

    IF asset_status = 'archived' AND existing_source IS NOT NULL THEN
        RAISE EXCEPTION 'Archived knowledge assets cannot receive a new source revision'
            USING ERRCODE = '23514';
    END IF;

    UPDATE public.knowledge_assets
    SET current_source_file_id = NEW.id,
        status = CASE
            WHEN existing_source IS NULL THEN status
            WHEN status IN ('submitted', 'ocr_pending') THEN status
            ELSE 'ocr_pending'::public.knowledge_asset_status
        END,
        reviewed_by = CASE WHEN existing_source IS NULL THEN reviewed_by ELSE NULL END,
        published_at = CASE WHEN existing_source IS NULL THEN published_at ELSE NULL END,
        failure_reason = CASE WHEN existing_source IS NULL THEN failure_reason ELSE NULL END
    WHERE id = NEW.asset_id;

    IF existing_source IS NOT NULL THEN
        UPDATE public.teacher_asset_selections
        SET enabled = FALSE, updated_at = NOW()
        WHERE asset_id = NEW.asset_id AND enabled = TRUE;

        UPDATE public.ingestion_jobs
        SET status = 'cancelled', finished_at = NOW(),
            error_message = 'Superseded by a new governed source revision',
            updated_at = NOW()
        WHERE asset_id = NEW.asset_id
          AND status IN ('queued', 'running');
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_advance_knowledge_current_source
    ON public.knowledge_source_files;
CREATE TRIGGER trg_advance_knowledge_current_source
AFTER INSERT ON public.knowledge_source_files
FOR EACH ROW
EXECUTE FUNCTION public.advance_knowledge_current_source();

-- Trusted source-review evidence is separate from the generic audit stream.
CREATE TABLE IF NOT EXISTS public.knowledge_source_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL REFERENCES public.knowledge_assets(id) ON DELETE CASCADE,
    source_file_id UUID NOT NULL REFERENCES public.knowledge_source_files(id) ON DELETE RESTRICT,
    source_sha256 TEXT NOT NULL CHECK (source_sha256 ~ '^[0-9a-f]{64}$'),
    reviewed_by UUID NOT NULL REFERENCES public.users(id),
    verified_size BIGINT NOT NULL CHECK (verified_size >= 0 AND verified_size <= 20971520),
    reviewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_knowledge_source_reviews_binding
    ON public.knowledge_source_reviews (asset_id, source_file_id, source_sha256, reviewed_at DESC);

ALTER TABLE public.knowledge_source_reviews ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.knowledge_source_reviews FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS knowledge_source_reviews_admin_select
    ON public.knowledge_source_reviews;
CREATE POLICY knowledge_source_reviews_admin_select ON public.knowledge_source_reviews
FOR SELECT USING (get_role() = 'PlatformAdmin');

CREATE OR REPLACE FUNCTION public.prevent_knowledge_source_review_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Governed source-review evidence is append-only'
        USING ERRCODE = '55000';
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_source_review_immutable
    ON public.knowledge_source_reviews;
CREATE TRIGGER trg_knowledge_source_review_immutable
BEFORE UPDATE OR DELETE ON public.knowledge_source_reviews
FOR EACH ROW
EXECUTE FUNCTION public.prevent_knowledge_source_review_mutation();

-- The protected server path passes the actual downloaded bytes. The database
-- independently verifies canonical revision, MIME, size, PDF magic, and digest
-- before it can mint review evidence. A matching audit row alone is never proof.
CREATE OR REPLACE FUNCTION public.record_knowledge_source_review(
    p_asset_id UUID,
    p_source_file_id UUID,
    p_source_bytes BYTEA
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id UUID;
    school UUID;
    expected_sha256 TEXT;
    expected_size BIGINT;
    expected_mime TEXT;
    review_id UUID;
    actual_sha256 TEXT;
    actual_size BIGINT;
BEGIN
    IF get_role() <> 'PlatformAdmin' THEN
        RAISE EXCEPTION 'Only PlatformAdmin may record governed source review'
            USING ERRCODE = '42501';
    END IF;

    actor_id := get_user_id();
    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'A governed source review requires an authenticated actor'
            USING ERRCODE = '42501';
    END IF;

    SELECT asset.school_id, lower(source.sha256), source.file_size_bytes, source.mime_type
    INTO school, expected_sha256, expected_size, expected_mime
    FROM public.knowledge_assets AS asset
    JOIN public.knowledge_source_files AS source
      ON source.id = asset.current_source_file_id
     AND source.asset_id = asset.id
    WHERE asset.id = p_asset_id
      AND source.id = p_source_file_id
    FOR SHARE OF asset, source;

    IF NOT FOUND OR expected_sha256 IS NULL THEN
        RAISE EXCEPTION 'Source review requires the canonical hashed source revision'
            USING ERRCODE = '23514';
    END IF;
    IF expected_mime <> 'application/pdf' THEN
        RAISE EXCEPTION 'Only governed PDF sources can be reviewed'
            USING ERRCODE = '23514';
    END IF;

    actual_size := octet_length(p_source_bytes);
    IF actual_size > 20971520
       OR (expected_size IS NOT NULL AND actual_size <> expected_size) THEN
        RAISE EXCEPTION 'Source bytes do not match governed size metadata'
            USING ERRCODE = '23514';
    END IF;
    IF substring(p_source_bytes FROM 1 FOR 5) <> decode('255044462d', 'hex') THEN
        RAISE EXCEPTION 'Source bytes are not a PDF document'
            USING ERRCODE = '23514';
    END IF;

    actual_sha256 := lower(encode(digest(p_source_bytes, 'sha256'), 'hex'));
    IF actual_sha256 <> expected_sha256 THEN
        RAISE EXCEPTION 'Source bytes do not match governed SHA-256 metadata'
            USING ERRCODE = '23514';
    END IF;

    INSERT INTO public.knowledge_source_reviews (
        asset_id, source_file_id, source_sha256, reviewed_by, verified_size
    ) VALUES (
        p_asset_id, p_source_file_id, actual_sha256, actor_id, actual_size
    )
    RETURNING id INTO review_id;

    INSERT INTO public.knowledge_audit_logs (
        actor_id, actor_role, action, target_type, target_id, school_id, details_json
    ) VALUES (
        actor_id, 'PlatformAdmin', 'knowledge_asset.source_reviewed',
        'knowledge_asset', p_asset_id, school,
        jsonb_build_object(
            'source_review_id', review_id,
            'source_file_id', p_source_file_id,
            'source_sha256', actual_sha256,
            'byte_count', actual_size,
            'delivery', 'inline_pdf'
        )
    );

    RETURN review_id;
END;
$$;

-- Do not leave the privileged evidence minting function executable merely by
-- virtue of PostgreSQL's default PUBLIC function privilege. Production grants
-- EXECUTE explicitly to the dedicated NOBYPASSRLS application identity.
REVOKE ALL ON FUNCTION public.record_knowledge_source_review(UUID, UUID, BYTEA) FROM PUBLIC;

-- A revision UUID is historical identity for one OCR verification event. The
-- append-only provenance table prevents an existing OCR revision from ever being
-- repointed to another source, even though knowledge_ocr_texts stores the latest
-- OCR text for an asset.
CREATE TABLE IF NOT EXISTS public.knowledge_ocr_revision_provenance (
    revision UUID PRIMARY KEY,
    asset_id UUID NOT NULL REFERENCES public.knowledge_assets(id) ON DELETE CASCADE,
    source_file_id UUID NOT NULL REFERENCES public.knowledge_source_files(id) ON DELETE RESTRICT,
    source_sha256 TEXT NOT NULL CHECK (source_sha256 ~ '^[0-9a-f]{64}$'),
    text_sha256 TEXT,
    verified_by UUID NOT NULL REFERENCES public.users(id),
    verified_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE public.knowledge_ocr_revision_provenance ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.knowledge_ocr_revision_provenance FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS knowledge_ocr_revision_provenance_admin_select
    ON public.knowledge_ocr_revision_provenance;
CREATE POLICY knowledge_ocr_revision_provenance_admin_select
ON public.knowledge_ocr_revision_provenance
FOR SELECT USING (get_role() = 'PlatformAdmin');

-- Provenance history can be inserted only from the verified-OCR trigger path.
DROP POLICY IF EXISTS knowledge_ocr_revision_provenance_trigger_insert
    ON public.knowledge_ocr_revision_provenance;
CREATE POLICY knowledge_ocr_revision_provenance_trigger_insert
ON public.knowledge_ocr_revision_provenance
FOR INSERT WITH CHECK (get_role() = 'PlatformAdmin' AND pg_trigger_depth() > 0);

CREATE OR REPLACE FUNCTION public.prevent_knowledge_ocr_provenance_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'OCR revision provenance is append-only'
        USING ERRCODE = '55000';
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_ocr_revision_provenance_immutable
    ON public.knowledge_ocr_revision_provenance;
CREATE TRIGGER trg_knowledge_ocr_revision_provenance_immutable
BEFORE UPDATE OR DELETE ON public.knowledge_ocr_revision_provenance
FOR EACH ROW
EXECUTE FUNCTION public.prevent_knowledge_ocr_provenance_mutation();

CREATE OR REPLACE FUNCTION public.enforce_reviewed_source_for_verified_ocr()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_source_id UUID;
    current_source_sha256 TEXT;
BEGIN
    SELECT asset.current_source_file_id, lower(source.sha256)
    INTO current_source_id, current_source_sha256
    FROM public.knowledge_assets AS asset
    JOIN public.knowledge_source_files AS source
      ON source.id = asset.current_source_file_id
     AND source.asset_id = asset.id
    WHERE asset.id = NEW.asset_id
    FOR SHARE OF asset, source;

    IF current_source_id IS NULL OR current_source_sha256 IS NULL THEN
        RAISE EXCEPTION 'Verified OCR requires a canonical hashed source revision'
            USING ERRCODE = '23514';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.knowledge_source_reviews AS review
        WHERE review.asset_id = NEW.asset_id
          AND review.source_file_id = current_source_id
          AND review.source_sha256 = current_source_sha256
          AND review.reviewed_by = NEW.ocr_verified_by
    ) THEN
        RAISE EXCEPTION 'The canonical governed source must be reviewed before verified OCR can be saved'
            USING ERRCODE = '23514';
    END IF;

    IF TG_OP = 'UPDATE' THEN
        IF NEW.source_file_id IS DISTINCT FROM OLD.source_file_id
           OR NEW.source_sha256 IS DISTINCT FROM OLD.source_sha256 THEN
            IF NEW.revision IS NOT DISTINCT FROM OLD.revision THEN
                RAISE EXCEPTION 'OCR provenance cannot be repointed in place; create a new OCR revision'
                    USING ERRCODE = '55000';
            END IF;
        END IF;
        IF NEW.revision IS NOT DISTINCT FROM OLD.revision
           AND (NEW.raw_text IS DISTINCT FROM OLD.raw_text
                OR NEW.clean_text IS DISTINCT FROM OLD.clean_text
                OR NEW.text_sha256 IS DISTINCT FROM OLD.text_sha256
                OR NEW.ocr_provider IS DISTINCT FROM OLD.ocr_provider
                OR NEW.ocr_verified_by IS DISTINCT FROM OLD.ocr_verified_by) THEN
            RAISE EXCEPTION 'Verified OCR changes require a new OCR revision'
                USING ERRCODE = '55000';
        END IF;
    END IF;

    -- Persist the provenance already established by canonical source + protected
    -- review evidence. Caller-provided provenance never overrides database truth.
    NEW.source_file_id := current_source_id;
    NEW.source_sha256 := current_source_sha256;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_require_reviewed_source_for_verified_ocr
    ON public.knowledge_ocr_texts;
CREATE TRIGGER trg_require_reviewed_source_for_verified_ocr
BEFORE INSERT OR UPDATE ON public.knowledge_ocr_texts
FOR EACH ROW
EXECUTE FUNCTION public.enforce_reviewed_source_for_verified_ocr();

CREATE OR REPLACE FUNCTION public.record_knowledge_ocr_revision_provenance()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO public.knowledge_ocr_revision_provenance (
        revision, asset_id, source_file_id, source_sha256, text_sha256,
        verified_by, verified_at
    ) VALUES (
        NEW.revision, NEW.asset_id, NEW.source_file_id, NEW.source_sha256,
        NEW.text_sha256, NEW.ocr_verified_by, NEW.ocr_verified_at
    );
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_record_knowledge_ocr_revision_provenance
    ON public.knowledge_ocr_texts;
CREATE TRIGGER trg_record_knowledge_ocr_revision_provenance
AFTER INSERT OR UPDATE OF revision ON public.knowledge_ocr_texts
FOR EACH ROW
WHEN (NEW.source_file_id IS NOT NULL AND NEW.source_sha256 IS NOT NULL)
EXECUTE FUNCTION public.record_knowledge_ocr_revision_provenance();

-- Embedding rows are immutable snapshots. Re-embedding already replaces them by
-- DELETE + INSERT; an UPDATE must never relabel an old vector as a new source.
CREATE OR REPLACE FUNCTION public.prevent_knowledge_chunk_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Knowledge embedding chunks are immutable; re-embed the asset instead'
        USING ERRCODE = '55000';
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_chunk_immutable ON public.knowledge_chunks;
CREATE TRIGGER trg_knowledge_chunk_immutable
BEFORE UPDATE ON public.knowledge_chunks
FOR EACH ROW
EXECUTE FUNCTION public.prevent_knowledge_chunk_mutation();

-- Embedding rows are bound to the exact current OCR/source revision at insertion.
CREATE OR REPLACE FUNCTION public.bind_knowledge_chunk_provenance()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_source_id UUID;
    current_source_sha256 TEXT;
    current_ocr_revision UUID;
BEGIN
    SELECT asset.current_source_file_id, lower(source.sha256), ocr.revision
    INTO current_source_id, current_source_sha256, current_ocr_revision
    FROM public.knowledge_assets AS asset
    JOIN public.knowledge_source_files AS source
      ON source.id = asset.current_source_file_id
     AND source.asset_id = asset.id
    JOIN public.knowledge_ocr_texts AS ocr
      ON ocr.asset_id = asset.id
     AND ocr.source_file_id = asset.current_source_file_id
     AND lower(ocr.source_sha256) = lower(source.sha256)
    WHERE asset.id = NEW.asset_id;

    IF current_source_id IS NULL OR current_ocr_revision IS NULL THEN
        RAISE EXCEPTION 'Embedding requires verified OCR bound to the canonical source revision'
            USING ERRCODE = '23514';
    END IF;

    NEW.source_file_id := current_source_id;
    NEW.source_sha256 := current_source_sha256;
    NEW.ocr_revision := current_ocr_revision;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_bind_knowledge_chunk_provenance
    ON public.knowledge_chunks;
CREATE TRIGGER trg_bind_knowledge_chunk_provenance
BEFORE INSERT ON public.knowledge_chunks
FOR EACH ROW
EXECUTE FUNCTION public.bind_knowledge_chunk_provenance();

-- One reusable fail-closed predicate for teacher visibility and service/repository
-- defense-in-depth. SECURITY DEFINER is read-only and exposes only a boolean.
CREATE OR REPLACE FUNCTION public.knowledge_asset_has_current_provenance(p_asset_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM public.knowledge_assets AS asset
        JOIN public.knowledge_source_files AS source
          ON source.id = asset.current_source_file_id
         AND source.asset_id = asset.id
        JOIN public.knowledge_ocr_texts AS ocr
          ON ocr.asset_id = asset.id
         AND ocr.source_file_id = source.id
         AND lower(ocr.source_sha256) = lower(source.sha256)
        WHERE asset.id = p_asset_id
          AND EXISTS (
              SELECT 1
              FROM public.knowledge_source_reviews AS review
              WHERE review.asset_id = asset.id
                AND review.source_file_id = source.id
                AND review.source_sha256 = lower(source.sha256)
          )
          AND EXISTS (
              SELECT 1
              FROM public.knowledge_chunks AS chunk
              WHERE chunk.asset_id = asset.id
                AND chunk.source_file_id = source.id
                AND chunk.source_sha256 = lower(source.sha256)
                AND chunk.ocr_revision = ocr.revision
          )
    );
$$;

REVOKE ALL ON FUNCTION public.knowledge_asset_has_current_provenance(UUID) FROM PUBLIC;

-- Replace teacher visibility with the exact provenance contract. Platform Admin
-- and School Manager visibility stays unchanged; teacher reads fail closed even
-- if a stale row is accidentally left marked published.
DROP POLICY IF EXISTS knowledge_assets_scoped_select ON public.knowledge_assets;
CREATE POLICY knowledge_assets_scoped_select ON public.knowledge_assets
FOR SELECT USING (
    get_role() = 'PlatformAdmin'
    OR (get_role() = 'SchoolManager' AND school_id = get_school_id())
    OR (
        get_role() = 'Teacher'
        AND school_id = get_school_id()
        AND status = 'published'
        AND public.knowledge_asset_has_current_provenance(id)
    )
);

-- Advancing into downstream lifecycle states requires one exact provenance chain:
-- asset current source == trusted review == verified OCR == embedding chunks.
CREATE OR REPLACE FUNCTION public.validate_knowledge_provenance_for_lifecycle()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_source_id UUID;
    current_source_sha256 TEXT;
    current_ocr_revision UUID;
BEGIN
    IF NEW.status NOT IN ('embedding_pending', 'embedded', 'published') THEN
        RETURN NEW;
    END IF;

    SELECT asset.current_source_file_id, lower(source.sha256), ocr.revision
    INTO current_source_id, current_source_sha256, current_ocr_revision
    FROM public.knowledge_assets AS asset
    JOIN public.knowledge_source_files AS source
      ON source.id = asset.current_source_file_id
     AND source.asset_id = asset.id
    JOIN public.knowledge_ocr_texts AS ocr
      ON ocr.asset_id = asset.id
     AND ocr.source_file_id = asset.current_source_file_id
     AND lower(ocr.source_sha256) = lower(source.sha256)
    WHERE asset.id = NEW.id;

    IF current_source_id IS NULL OR current_ocr_revision IS NULL THEN
        RAISE EXCEPTION 'Verified OCR does not match the canonical governed source revision'
            USING ERRCODE = '23514';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM public.knowledge_source_reviews AS review
        WHERE review.asset_id = NEW.id
          AND review.source_file_id = current_source_id
          AND review.source_sha256 = current_source_sha256
    ) THEN
        RAISE EXCEPTION 'The canonical governed source has no trusted review evidence'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.status IN ('embedded', 'published') AND NOT EXISTS (
        SELECT 1 FROM public.knowledge_chunks AS chunk
        WHERE chunk.asset_id = NEW.id
          AND chunk.source_file_id = current_source_id
          AND chunk.source_sha256 = current_source_sha256
          AND chunk.ocr_revision = current_ocr_revision
    ) THEN
        RAISE EXCEPTION 'Embedding does not match the canonical source and OCR revision'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_validate_knowledge_ocr_source_provenance
    ON public.knowledge_assets;
DROP TRIGGER IF EXISTS trg_validate_knowledge_provenance_for_lifecycle
    ON public.knowledge_assets;
CREATE TRIGGER trg_validate_knowledge_provenance_for_lifecycle
BEFORE UPDATE OF status ON public.knowledge_assets
FOR EACH ROW
EXECUTE FUNCTION public.validate_knowledge_provenance_for_lifecycle();

COMMENT ON COLUMN public.knowledge_assets.current_source_file_id IS
    'Canonical immutable governed source revision for the asset.';
COMMENT ON COLUMN public.knowledge_ocr_texts.source_file_id IS
    'Canonical source revision against which this OCR revision was verified.';
COMMENT ON COLUMN public.knowledge_ocr_texts.source_sha256 IS
    'SHA-256 of the immutable source revision against which this OCR revision was verified.';
COMMENT ON TABLE public.knowledge_source_reviews IS
    'Append-only trusted evidence created only after canonical private source bytes pass integrity verification.';
COMMENT ON TABLE public.knowledge_ocr_revision_provenance IS
    'Append-only provenance identity for each verified OCR revision.';
