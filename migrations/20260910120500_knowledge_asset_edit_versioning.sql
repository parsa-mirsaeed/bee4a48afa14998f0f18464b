-- Lifecycle-aware editing/versioning for governed knowledge assets (#34).
--
-- The logical asset keeps one stable UUID while every mutation advances a
-- server-owned optimistic-concurrency token. Source bytes remain append-only in
-- knowledge_source_files; source replacement creates a new immutable row and
-- the existing provenance chain determines whether OCR/chunks are current.

ALTER TABLE public.knowledge_assets
    ADD COLUMN IF NOT EXISTS asset_revision BIGINT NOT NULL DEFAULT 1
        CHECK (asset_revision > 0);

CREATE OR REPLACE FUNCTION public.bump_knowledge_asset_revision()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $function$
BEGIN
    -- Clients never choose the revision. Even a caller that can UPDATE the table
    -- receives exactly the next monotonic value for every governed mutation.
    NEW.asset_revision := OLD.asset_revision + 1;
    RETURN NEW;
END;
$function$;

DROP TRIGGER IF EXISTS trg_bump_knowledge_asset_revision
    ON public.knowledge_assets;
CREATE TRIGGER trg_bump_knowledge_asset_revision
BEFORE UPDATE ON public.knowledge_assets
FOR EACH ROW
EXECUTE FUNCTION public.bump_knowledge_asset_revision();

-- The manager metadata mutation is the only SchoolManager write path for an
-- existing asset row. RLS intentionally keeps general knowledge_assets UPDATE
-- PlatformAdmin-only; this definer function re-checks actor identity, role,
-- school scope and expected revision before changing the exact target row.
CREATE OR REPLACE FUNCTION public.manager_update_knowledge_asset_metadata(
    p_asset_id UUID,
    p_expected_revision BIGINT,
    p_title TEXT,
    p_description TEXT,
    p_subject TEXT,
    p_grade TEXT,
    p_language TEXT,
    p_template_type TEXT,
    p_tags JSONB
)
RETURNS TABLE (
    asset_revision BIGINT,
    status TEXT,
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
    old_title TEXT;
    old_description TEXT;
    old_subject TEXT;
    old_grade TEXT;
    old_language TEXT;
    old_template_type TEXT;
    old_tags JSONB;
    current_source UUID;
    normalized_title TEXT := btrim(COALESCE(p_title, ''));
    normalized_description TEXT := NULLIF(btrim(COALESCE(p_description, '')), '');
    normalized_subject TEXT := NULLIF(btrim(COALESCE(p_subject, '')), '');
    normalized_grade TEXT := NULLIF(btrim(COALESCE(p_grade, '')), '');
    normalized_language TEXT := btrim(COALESCE(p_language, ''));
    normalized_template_type TEXT := NULLIF(btrim(COALESCE(p_template_type, '')), '');
    normalized_tags JSONB := COALESCE(p_tags, '{}'::jsonb);
    changed_fields TEXT[] := ARRAY[]::TEXT[];
    vector_sensitive BOOLEAN := FALSE;
    invalidate_vectors BOOLEAN := FALSE;
    has_current_ocr BOOLEAN := FALSE;
    target_status public.knowledge_asset_status;
    next_revision BIGINT;
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

    IF char_length(normalized_title) NOT BETWEEN 1 AND 255
       OR char_length(normalized_language) NOT BETWEEN 1 AND 32
       OR char_length(COALESCE(normalized_description, '')) > 8000
       OR char_length(COALESCE(normalized_subject, '')) > 255
       OR char_length(COALESCE(normalized_grade, '')) > 64
       OR char_length(COALESCE(normalized_template_type, '')) > 255
       OR jsonb_typeof(normalized_tags) <> 'object'
       OR octet_length(normalized_tags::text) > 16384 THEN
        RAISE EXCEPTION 'knowledge_asset_metadata_invalid' USING ERRCODE = '22023';
    END IF;

    SELECT asset.asset_revision,
           asset.status,
           asset.title,
           asset.description,
           asset.subject,
           asset.grade,
           asset.language,
           asset.template_type,
           asset.tags,
           asset.current_source_file_id
    INTO old_revision,
         old_status,
         old_title,
         old_description,
         old_subject,
         old_grade,
         old_language,
         old_template_type,
         old_tags,
         current_source
    FROM public.knowledge_assets AS asset
    WHERE asset.id = p_asset_id
      AND asset.school_id = actor_school
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'knowledge_asset_not_found' USING ERRCODE = 'P0002';
    END IF;
    IF p_expected_revision IS NULL OR p_expected_revision <> old_revision THEN
        RAISE EXCEPTION 'knowledge_asset_revision_conflict' USING ERRCODE = '40001';
    END IF;

    IF old_title IS DISTINCT FROM normalized_title THEN
        changed_fields := array_append(changed_fields, 'title');
        vector_sensitive := TRUE;
    END IF;
    IF old_description IS DISTINCT FROM normalized_description THEN
        changed_fields := array_append(changed_fields, 'description');
    END IF;
    IF old_subject IS DISTINCT FROM normalized_subject THEN
        changed_fields := array_append(changed_fields, 'subject');
        vector_sensitive := TRUE;
    END IF;
    IF old_grade IS DISTINCT FROM normalized_grade THEN
        changed_fields := array_append(changed_fields, 'grade');
        vector_sensitive := TRUE;
    END IF;
    IF old_language IS DISTINCT FROM normalized_language THEN
        changed_fields := array_append(changed_fields, 'language');
        vector_sensitive := TRUE;
    END IF;
    IF old_template_type IS DISTINCT FROM normalized_template_type THEN
        changed_fields := array_append(changed_fields, 'template_type');
        vector_sensitive := TRUE;
    END IF;
    IF old_tags IS DISTINCT FROM normalized_tags THEN
        changed_fields := array_append(changed_fields, 'tags');
        vector_sensitive := TRUE;
    END IF;

    IF cardinality(changed_fields) = 0 THEN
        RETURN QUERY SELECT old_revision, old_status::text, FALSE;
        RETURN;
    END IF;

    invalidate_vectors := vector_sensitive
        AND old_status IN ('embedding_pending', 'embedded', 'published');
    target_status := old_status;

    IF invalidate_vectors THEN
        SELECT EXISTS (
            SELECT 1
            FROM public.knowledge_ocr_texts AS ocr
            JOIN public.knowledge_source_files AS source
              ON source.id = current_source
             AND source.asset_id = p_asset_id
            WHERE ocr.asset_id = p_asset_id
              AND ocr.source_file_id = source.id
              AND lower(ocr.source_sha256) = lower(source.sha256)
        ) INTO has_current_ocr;

        target_status := CASE
            WHEN has_current_ocr THEN 'ocr_ready'::public.knowledge_asset_status
            ELSE 'ocr_pending'::public.knowledge_asset_status
        END;

        UPDATE public.ingestion_jobs
        SET status = 'cancelled',
            finished_at = NOW(),
            error_message = 'Cancelled because retrieval metadata changed',
            updated_at = NOW()
        WHERE asset_id = p_asset_id
          AND status IN ('queued', 'running');

        UPDATE public.teacher_asset_selections
        SET enabled = FALSE,
            updated_at = NOW()
        WHERE asset_id = p_asset_id
          AND enabled = TRUE;

        DELETE FROM public.knowledge_chunks
        WHERE asset_id = p_asset_id;

        -- The lifecycle trigger accepts this otherwise-illegal reset only when
        -- it is executing as this definer function's owner with the exact
        -- transaction-local manager actor marker.
        PERFORM set_config(
            'app.knowledge_metadata_invalidation_actor',
            actor_id::text,
            true
        );
    END IF;

    UPDATE public.knowledge_assets AS asset
    SET title = normalized_title,
        description = normalized_description,
        subject = normalized_subject,
        grade = normalized_grade,
        language = normalized_language,
        template_type = normalized_template_type,
        tags = normalized_tags,
        status = target_status,
        published_at = CASE
            WHEN target_status = 'published' THEN asset.published_at
            ELSE NULL
        END,
        failure_reason = CASE
            WHEN invalidate_vectors THEN NULL
            ELSE asset.failure_reason
        END
    WHERE asset.id = p_asset_id
      AND asset.school_id = actor_school
      AND asset.asset_revision = old_revision
    RETURNING asset.asset_revision INTO next_revision;

    IF next_revision IS NULL THEN
        RAISE EXCEPTION 'knowledge_asset_revision_conflict' USING ERRCODE = '40001';
    END IF;

    INSERT INTO public.knowledge_audit_logs (
        actor_id, actor_role, action, target_type, target_id, school_id, details_json
    ) VALUES (
        actor_id,
        'SchoolManager',
        'knowledge_asset.metadata_updated',
        'knowledge_asset',
        p_asset_id,
        actor_school,
        jsonb_build_object(
            'changed_fields', to_jsonb(changed_fields),
            'before', jsonb_build_object(
                'title', old_title,
                'description', old_description,
                'subject', old_subject,
                'grade', old_grade,
                'language', old_language,
                'template_type', old_template_type,
                'tags', old_tags
            ),
            'after', jsonb_build_object(
                'title', normalized_title,
                'description', normalized_description,
                'subject', normalized_subject,
                'grade', normalized_grade,
                'language', normalized_language,
                'template_type', normalized_template_type,
                'tags', normalized_tags
            ),
            'previous_status', old_status::text,
            'status', target_status::text,
            'previous_revision', old_revision,
            'asset_revision', next_revision,
            'vectors_invalidated', invalidate_vectors
        )
    );

    RETURN QUERY SELECT next_revision, target_status::text, invalidate_vectors;
END;
$function$;

REVOKE ALL ON FUNCTION public.manager_update_knowledge_asset_metadata(
    UUID, BIGINT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, JSONB
) FROM PUBLIC;

-- Permit only the trusted manager metadata function to reset downstream states
-- after retrieval-visible metadata changes. Ordinary app-role UPDATE remains
-- constrained by RLS and the normal lifecycle state machine.
CREATE OR REPLACE FUNCTION public.validate_knowledge_asset_status_transition()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $function$
DECLARE
    metadata_actor TEXT := current_setting('app.knowledge_metadata_invalidation_actor', true);
    metadata_function_owner TEXT;
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;

    SELECT pg_catalog.pg_get_userbyid(procedure.proowner)
    INTO metadata_function_owner
    FROM pg_catalog.pg_proc AS procedure
    WHERE procedure.oid = 'public.manager_update_knowledge_asset_metadata(uuid,bigint,text,text,text,text,text,text,jsonb)'::regprocedure;

    IF metadata_actor IS NOT NULL
       AND metadata_actor = COALESCE(public.get_user_id()::text, '')
       AND current_user = metadata_function_owner
       AND OLD.status IN ('embedding_pending', 'embedded', 'published')
       AND NEW.status IN ('ocr_ready', 'ocr_pending') THEN
        NEW.published_at = NULL;
        NEW.archived_at = NULL;
        NEW.failure_reason = NULL;
        RETURN NEW;
    END IF;

    -- Existing source-replacement reset: only the nested source trigger may
    -- return a downstream asset to OCR review.
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
$function$;

-- Extend the already-hardened source trigger with explicit stale-chunk
-- invalidation. The old OCR/source rows remain immutable provenance, but they no
-- longer satisfy current-source joins once this pointer advances.
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
    END IF;

    RETURN NEW;
END;
$function$;

REVOKE ALL ON FUNCTION public.advance_knowledge_current_source() FROM PUBLIC;

-- Explicit same-logical-asset source replacement. The uploaded object reference
-- is new and opaque; the previous source row is never overwritten or deleted.
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
       OR p_original_file_url !~ '^storage://edutalent-knowledge-sources/[0-9a-f-]{36}/[0-9a-f-]{36}\\.pdf$'
       OR char_length(btrim(COALESCE(p_original_filename, ''))) NOT BETWEEN 1 AND 255
       OR p_mime_type <> 'application/pdf'
       OR p_file_size_bytes IS NULL
       OR p_file_size_bytes <= 0
       OR p_file_size_bytes > 20971520
       OR lower(COALESCE(p_sha256, '')) !~ '^[0-9a-f]{64}$'
       OR p_page_count IS NOT NULL AND p_page_count < 0 THEN
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
            SELECT 1 FROM public.knowledge_chunks AS chunk
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
    -- canonical pointer, invalidated stale chunks/jobs/selections and bumped the
    -- asset revision before this statement resumes.
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

COMMENT ON COLUMN public.knowledge_assets.asset_revision IS
    'Server-owned monotonic optimistic-concurrency token. Every governed asset UPDATE advances it.';
COMMENT ON FUNCTION public.manager_update_knowledge_asset_metadata(
    UUID, BIGINT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, JSONB
) IS
    'SchoolManager same-school metadata edit with optimistic concurrency and lifecycle-aware vector invalidation.';
COMMENT ON FUNCTION public.manager_replace_knowledge_source_revision(
    UUID, BIGINT, TEXT, TEXT, TEXT, BIGINT, TEXT, INTEGER, BOOLEAN
) IS
    'SchoolManager same-logical-asset immutable source replacement with optimistic concurrency and downstream invalidation.';
