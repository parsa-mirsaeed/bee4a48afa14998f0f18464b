-- Harden #34 source replacement so no active OCR row can survive a canonical
-- source change. Historical verified OCR remains preserved in
-- knowledge_ocr_revision_provenance; knowledge_ocr_texts represents only OCR
-- for the currently canonical source.

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
        -- A freshly inserted source cannot already have governed current OCR:
        -- source review and OCR verification occur only after this trigger has
        -- made the revision canonical. Therefore every active OCR row for the
        -- asset belongs to the superseded source and must be retired. Immutable
        -- revision provenance was recorded when that OCR was verified.
        DELETE FROM public.knowledge_ocr_texts AS ocr
        WHERE ocr.asset_id = NEW.asset_id;

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
