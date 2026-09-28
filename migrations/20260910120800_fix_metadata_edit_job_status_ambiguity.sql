-- Fix #34 metadata-edit invalidation under PL/pgSQL RETURN TABLE semantics.
--
-- manager_update_knowledge_asset_metadata returns an output column named
-- `status`.  Its ingestion-job cancellation predicate therefore must qualify
-- the table's status column; otherwise PostgreSQL treats the unqualified name
-- as ambiguous between the output variable and ingestion_jobs.status.

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

        UPDATE public.ingestion_jobs AS job
        SET status = 'cancelled',
            finished_at = NOW(),
            error_message = 'Cancelled because retrieval metadata changed',
            updated_at = NOW()
        WHERE job.asset_id = p_asset_id
          AND job.status IN ('queued', 'running');

        UPDATE public.teacher_asset_selections AS selection
        SET enabled = FALSE,
            updated_at = NOW()
        WHERE selection.asset_id = p_asset_id
          AND selection.enabled = TRUE;

        DELETE FROM public.knowledge_chunks AS chunk
        WHERE chunk.asset_id = p_asset_id;

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
