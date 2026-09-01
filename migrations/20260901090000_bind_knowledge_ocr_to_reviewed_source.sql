-- Bind verified OCR to the exact private source revision that a Platform Admin
-- successfully reviewed. This closes the gap where OCR could be saved against
-- syntactically valid metadata without durable proof of the reviewed bytes.
--
-- Legacy OCR is backfilled only when a single hashed source exists. Ambiguous
-- or unhashed legacy records remain unbound and therefore cannot advance into
-- embedding until they are explicitly reviewed and re-verified.

ALTER TABLE public.knowledge_ocr_texts
    ADD COLUMN IF NOT EXISTS source_file_id UUID
        REFERENCES public.knowledge_source_files(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS source_sha256 TEXT
        CHECK (source_sha256 IS NULL OR source_sha256 ~ '^[0-9a-f]{64}$');

CREATE INDEX IF NOT EXISTS idx_knowledge_ocr_source_file
    ON public.knowledge_ocr_texts (source_file_id);

WITH single_hashed_source AS (
    SELECT
        asset_id,
        (array_agg(id ORDER BY created_at DESC, id DESC))[1] AS source_file_id,
        lower((array_agg(sha256 ORDER BY created_at DESC, id DESC))[1]) AS source_sha256
    FROM public.knowledge_source_files
    WHERE sha256 IS NOT NULL
    GROUP BY asset_id
    HAVING count(*) = 1
)
UPDATE public.knowledge_ocr_texts AS ocr
SET source_file_id = source.source_file_id,
    source_sha256 = source.source_sha256
FROM single_hashed_source AS source
WHERE ocr.asset_id = source.asset_id
  AND ocr.source_file_id IS NULL
  AND ocr.source_sha256 IS NULL;

CREATE OR REPLACE FUNCTION public.enforce_reviewed_source_for_verified_ocr()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_source_id UUID;
    current_source_sha256 TEXT;
BEGIN
    SELECT source.id, lower(source.sha256)
    INTO current_source_id, current_source_sha256
    FROM public.knowledge_source_files AS source
    WHERE source.asset_id = NEW.asset_id
    ORDER BY source.created_at DESC, source.id DESC
    LIMIT 1
    FOR SHARE;

    IF current_source_id IS NULL OR current_source_sha256 IS NULL THEN
        RAISE EXCEPTION 'Verified OCR requires a hashed governed source document'
            USING ERRCODE = '23514';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.knowledge_audit_logs AS audit
        WHERE audit.action = 'knowledge_asset.source_reviewed'
          AND audit.target_type = 'knowledge_asset'
          AND audit.target_id = NEW.asset_id
          AND audit.actor_id = NEW.ocr_verified_by
          AND audit.details_json ->> 'source_file_id' = current_source_id::text
          AND lower(audit.details_json ->> 'source_sha256') = current_source_sha256
    ) THEN
        RAISE EXCEPTION 'The current governed source must be reviewed before verified OCR can be saved'
            USING ERRCODE = '23514';
    END IF;

    NEW.source_file_id := current_source_id;
    NEW.source_sha256 := current_source_sha256;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_require_reviewed_source_for_verified_ocr
    ON public.knowledge_ocr_texts;
CREATE TRIGGER trg_require_reviewed_source_for_verified_ocr
BEFORE INSERT OR UPDATE OF raw_text, clean_text, ocr_provider, ocr_verified_by,
    ocr_verified_at, text_sha256, revision
ON public.knowledge_ocr_texts
FOR EACH ROW
EXECUTE FUNCTION public.enforce_reviewed_source_for_verified_ocr();

CREATE OR REPLACE FUNCTION public.validate_knowledge_ocr_source_provenance()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_source_id UUID;
    current_source_sha256 TEXT;
    verified_source_id UUID;
    verified_source_sha256 TEXT;
BEGIN
    IF NEW.status NOT IN ('embedding_pending', 'embedded', 'published') THEN
        RETURN NEW;
    END IF;

    SELECT source.id, lower(source.sha256)
    INTO current_source_id, current_source_sha256
    FROM public.knowledge_source_files AS source
    WHERE source.asset_id = NEW.id
    ORDER BY source.created_at DESC, source.id DESC
    LIMIT 1;

    SELECT ocr.source_file_id, lower(ocr.source_sha256)
    INTO verified_source_id, verified_source_sha256
    FROM public.knowledge_ocr_texts AS ocr
    WHERE ocr.asset_id = NEW.id;

    IF current_source_id IS NULL
       OR current_source_sha256 IS NULL
       OR verified_source_id IS DISTINCT FROM current_source_id
       OR verified_source_sha256 IS DISTINCT FROM current_source_sha256 THEN
        RAISE EXCEPTION 'Verified OCR does not match the current governed source revision'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_validate_knowledge_ocr_source_provenance
    ON public.knowledge_assets;
CREATE TRIGGER trg_validate_knowledge_ocr_source_provenance
BEFORE UPDATE OF status ON public.knowledge_assets
FOR EACH ROW
EXECUTE FUNCTION public.validate_knowledge_ocr_source_provenance();

COMMENT ON COLUMN public.knowledge_ocr_texts.source_file_id IS
    'Immutable governed source row successfully reviewed for this verified OCR revision.';
COMMENT ON COLUMN public.knowledge_ocr_texts.source_sha256 IS
    'SHA-256 of the exact governed source bytes successfully reviewed for this verified OCR revision.';
