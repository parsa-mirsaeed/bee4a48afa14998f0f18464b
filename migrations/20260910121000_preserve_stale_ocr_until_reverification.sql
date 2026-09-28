-- Finalize #34 OCR invalidation semantics for source replacement.
--
-- knowledge_ocr_texts is the single active/candidate OCR record for an asset;
-- source identity on that row determines whether it is current. Replacing the
-- canonical source invalidates prior OCR by source mismatch, while immutable
-- knowledge_ocr_revision_provenance preserves every verified revision. A later
-- PlatformAdmin review may re-verify the replacement source by updating the
-- singleton with a new OCR revision. Do not weaken FORCE RLS or grant a
-- SchoolManager a delete path to governed OCR.

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
          AND (actor_role = 'PlatformAdmin' OR actor.school_id = actor_school)
    ) THEN
        RAISE EXCEPTION 'Governed source revision actor is not active in the claimed scope'
            USING ERRCODE = '42501';
    END IF;

    SELECT asset.status, asset.current_source_file_id
    INTO asset_status, existing_source
    FROM public.knowledge_assets AS asset
    WHERE asset.id = NEW.asset_id
      AND (actor_role = 'PlatformAdmin' OR asset.school_id = actor_school)
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Knowledge asset does not exist in the actor scope for source revision'
            USING ERRCODE = '23503';
    END IF;

    IF asset_status = 'archived' AND existing_source IS NOT NULL THEN
        RAISE EXCEPTION 'Archived knowledge assets cannot receive a new source revision'
            USING ERRCODE = '23514';
    END IF;

    IF existing_source IS NOT NULL THEN
        UPDATE public.teacher_asset_selections
        SET enabled = FALSE, updated_at = NOW()
        WHERE asset_id = NEW.asset_id AND enabled = TRUE;

        UPDATE public.ingestion_jobs
        SET status = 'cancelled',
            finished_at = NOW(),
            error_message = 'Superseded by a new governed source revision',
            updated_at = NOW()
        WHERE asset_id = NEW.asset_id
          AND status IN ('queued', 'running');

        DELETE FROM public.knowledge_chunks
        WHERE asset_id = NEW.asset_id;
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
      AND (actor_role = 'PlatformAdmin' OR asset.school_id = actor_school);

    RETURN NEW;
END;
$function$;

REVOKE ALL ON FUNCTION public.advance_knowledge_current_source() FROM PUBLIC;

-- A PlatformAdmin OCR update that will be rebound from a superseded source to
-- the canonical source must advance the immutable OCR revision. This check uses
-- OLD versus database-authoritative canonical provenance, not only caller-set
-- NEW source columns, because the trigger itself assigns NEW.source_* below.
CREATE OR REPLACE FUNCTION public.enforce_reviewed_source_for_verified_ocr()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $function$
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

    IF public.get_role() = 'PlatformAdmin'
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
        IF OLD.source_file_id IS DISTINCT FROM current_source_id
           OR lower(OLD.source_sha256) IS DISTINCT FROM current_source_sha256
           OR NEW.source_file_id IS DISTINCT FROM OLD.source_file_id
           OR lower(NEW.source_sha256) IS DISTINCT FROM lower(OLD.source_sha256) THEN
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

    NEW.source_file_id := current_source_id;
    NEW.source_sha256 := current_source_sha256;
    RETURN NEW;
END;
$function$;

COMMENT ON FUNCTION public.enforce_reviewed_source_for_verified_ocr() IS
    'Binds verified OCR to the canonical reviewed source, requires explicit PlatformAdmin source identity, and requires a new OCR revision when replacing stale source provenance.';
