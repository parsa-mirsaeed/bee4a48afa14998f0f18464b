-- Follow-up hardening for #34 source replacement.
--
-- `knowledge_ocr_revision_provenance` already records every governed OCR
-- revision when it is verified. When a new immutable source becomes current,
-- retire the stale active OCR row so PlatformAdmin can verify OCR for the new
-- source without losing the append-only revision provenance.

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

        -- The active OCR table represents only the canonical source. Historical
        -- governed OCR identity is retained by knowledge_ocr_revision_provenance,
        -- which was populated by the existing verified-OCR trigger.
        DELETE FROM public.knowledge_ocr_texts AS ocr
        WHERE ocr.asset_id = NEW.asset_id
          AND ocr.source_file_id IS DISTINCT FROM NEW.id;
    END IF;

    RETURN NEW;
END;
$function$;

REVOKE ALL ON FUNCTION public.advance_knowledge_current_source() FROM PUBLIC;

-- Re-declare the manager source mutation with an unambiguous storage-reference
-- validator. Source bytes are always placed in the fixed private bucket under
-- `<school UUID>/<opaque UUID>.pdf`; the client filename never forms the key.
CREATE OR REPLACE FUNCTION public.manager_replace_knowledge_source_revision(
    p_asset_id UUID,
    p_expected_revision BIGINT,
    p_original_file_url TEXT,
    p_original_filename TEXT,
    p_mime_type TEXT,
    p_file_size_bytes BIGINT,
    p_sha256 TEXT,
    p_page_count INTEGER,
    p_is_scanned_pdf BOOLEAN
)
RETURNS TABLE (
    asset_revision BIGINT,
    status TEXT,
    source_file_id UUID,
    vectors_invalidated BOOLEAN
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $function$
DECLARE
    actor_id UUID := public.get_user_id();
    actor_school UUID := public.get_school_id();
    actor_role TEXT := public.get_role();
    old_revision BIGINT;
    old_status public.knowledge_asset_status;
    old_source UUID;
    old_source_sha256 TEXT;
    new_source UUID;
    new_revision BIGINT;
    new_status public.knowledge_asset_status;
    invalidate_vectors BOOLEAN;
BEGIN
    IF actor_role <> 'SchoolManager'
       OR actor_id IS NULL
       OR actor_school IS NULL
       OR public.get_elevated_operation() THEN
        RAISE EXCEPTION 'knowledge_asset_forbidden' USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.users AS actor
        JOIN public.roles AS role_row ON role_row.id = actor.role_id
        WHERE actor.id = actor_id
          AND actor.school_id = actor_school
          AND actor.is_active = TRUE
          AND role_row.name::text = 'SchoolManager'
    ) THEN
        RAISE EXCEPTION 'knowledge_asset_forbidden' USING ERRCODE = '42501';
    END IF;

    IF p_original_file_url IS NULL
       OR p_original_file_url !~ '^storage://edutalent-knowledge-sources/[0-9a-f-]{36}/[0-9a-f-]{36}[.]pdf$'
       OR char_length(btrim(COALESCE(p_original_filename, ''))) NOT BETWEEN 1 AND 255
       OR p_mime_type <> 'application/pdf'
       OR p_file_size_bytes IS NULL
       OR p_file_size_bytes <= 0
       OR p_file_size_bytes > 20971520
       OR lower(COALESCE(p_sha256, '')) !~ '^[0-9a-f]{64}$'
       OR (p_page_count IS NOT NULL AND p_page_count < 0) THEN
        RAISE EXCEPTION 'knowledge_source_revision_invalid' USING ERRCODE = '22023';
    END IF;

    SELECT asset.asset_revision,
           asset.status,
           asset.current_source_file_id,
           lower(source.sha256)
    INTO old_revision, old_status, old_source, old_source_sha256
    FROM public.knowledge_assets AS asset
    LEFT JOIN public.knowledge_source_files AS source
      ON source.id = asset.current_source_file_id
     AND source.asset_id = asset.id
    WHERE asset.id = p_asset_id
      AND asset.school_id = actor_school
    FOR UPDATE OF asset;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'knowledge_asset_not_found' USING ERRCODE = 'P0002';
    END IF;
    IF p_expected_revision IS NULL OR p_expected_revision <> old_revision THEN
        RAISE EXCEPTION 'knowledge_asset_revision_conflict' USING ERRCODE = '40001';
    END IF;
    IF old_status = 'archived' THEN
        RAISE EXCEPTION 'archived_knowledge_asset_source_is_terminal' USING ERRCODE = '23514';
    END IF;

    invalidate_vectors := old_status IN ('embedding_pending', 'embedded', 'published')
        OR EXISTS (
            SELECT 1
            FROM public.knowledge_chunks AS chunk
            WHERE chunk.asset_id = p_asset_id
        );

    INSERT INTO public.knowledge_source_files (
        asset_id,
        original_file_url,
        original_filename,
        mime_type,
        file_size_bytes,
        sha256,
        page_count,
        is_scanned_pdf
    ) VALUES (
        p_asset_id,
        p_original_file_url,
        btrim(p_original_filename),
        p_mime_type,
        p_file_size_bytes,
        lower(p_sha256),
        p_page_count,
        COALESCE(p_is_scanned_pdf, FALSE)
    )
    RETURNING id INTO new_source;

    -- AFTER INSERT advance_knowledge_current_source has already moved the
    -- canonical pointer, retired stale active OCR, invalidated downstream state,
    -- and bumped the asset revision before this statement resumes.
    SELECT asset.asset_revision, asset.status
    INTO new_revision, new_status
    FROM public.knowledge_assets AS asset
    WHERE asset.id = p_asset_id;

    INSERT INTO public.knowledge_audit_logs (
        actor_id, actor_role, action, target_type, target_id, school_id, details_json
    ) VALUES (
        actor_id,
        'SchoolManager',
        'knowledge_asset.source_replaced',
        'knowledge_asset',
        p_asset_id,
        actor_school,
        jsonb_build_object(
            'previous_source_file_id', old_source,
            'previous_source_sha256', old_source_sha256,
            'source_file_id', new_source,
            'source_sha256', lower(p_sha256),
            'original_filename', btrim(p_original_filename),
            'mime_type', p_mime_type,
            'file_size_bytes', p_file_size_bytes,
            'previous_status', old_status::text,
            'status', new_status::text,
            'previous_revision', old_revision,
            'asset_revision', new_revision,
            'vectors_invalidated', invalidate_vectors
        )
    );

    RETURN QUERY SELECT new_revision, new_status::text, new_source, invalidate_vectors;
END;
$function$;

REVOKE ALL ON FUNCTION public.manager_replace_knowledge_source_revision(
    UUID, BIGINT, TEXT, TEXT, TEXT, BIGINT, TEXT, INTEGER, BOOLEAN
) FROM PUBLIC;
