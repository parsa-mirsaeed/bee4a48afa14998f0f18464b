-- Require an explicit canonical source revision precondition whenever a
-- Platform Admin OCR write can be ambiguous: every OCR update and every asset
-- that has more than one immutable source revision. A first OCR insert against
-- the only source remains race-safe because source replacement serializes on the
-- asset row and immediately invalidates any OCR bound to the superseded source.

CREATE OR REPLACE FUNCTION public.enforce_reviewed_source_for_verified_ocr()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    current_source_id UUID;
    current_source_sha256 TEXT;
    expected_source_id TEXT;
    expected_source_sha256 TEXT;
    source_revision_count BIGINT;
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

    SELECT COUNT(*)
    INTO source_revision_count
    FROM public.knowledge_source_files
    WHERE asset_id = NEW.asset_id;

    -- Application PlatformAdmin writes must prove which source revision the
    -- editor was opened against whenever OCR already exists or the asset has a
    -- replacement-source history. Bootstrap/migration writes without app role
    -- context remain deliberate owner operations.
    IF get_role() = 'PlatformAdmin'
       AND (TG_OP = 'UPDATE' OR source_revision_count > 1) THEN
        expected_source_id := NULLIF(
            current_setting('app.knowledge_expected_source_file_id', true),
            ''
        );
        expected_source_sha256 := lower(NULLIF(
            current_setting('app.knowledge_expected_source_sha256', true),
            ''
        ));

        IF expected_source_id IS NULL
           OR expected_source_sha256 IS NULL
           OR expected_source_id <> current_source_id::text
           OR expected_source_sha256 <> current_source_sha256 THEN
            RAISE EXCEPTION 'Verified OCR source revision is missing or stale; refresh and review the current source'
                USING ERRCODE = '23514';
        END IF;
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

    -- Caller-provided source columns never establish provenance. They are
    -- overwritten from the locked canonical source after the source precondition
    -- (when required) and trusted review evidence have both passed.
    NEW.source_file_id := current_source_id;
    NEW.source_sha256 := current_source_sha256;
    RETURN NEW;
END;
$$;

COMMENT ON FUNCTION public.enforce_reviewed_source_for_verified_ocr() IS
    'Binds verified OCR to the canonical reviewed source and requires explicit PlatformAdmin source identity for updates or replacement-source histories.';
