-- The pinned Supabase PG17 runtime installs pgcrypto in the extensions schema.
-- Retain the exact source bytes, digest verification, SECURITY DEFINER owner,
-- RLS and append-only audit contract from the original migration.
-- Do not modify previously applied migration checksums.
DO $$
BEGIN
    IF to_regprocedure('public.record_knowledge_source_review(uuid,uuid,bytea)') IS NULL THEN
        RAISE EXCEPTION 'Governed source-review procedure is missing';
    END IF;
    IF to_regprocedure('extensions.digest(bytea,text)') IS NULL THEN
        RAISE EXCEPTION 'Pinned pgcrypto digest must be installed in extensions';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_namespace namespace
        CROSS JOIN LATERAL aclexplode(COALESCE(namespace.nspacl, acldefault('n', namespace.nspowner))) entry
        WHERE namespace.nspname = 'extensions'
          AND entry.grantee = 0
          AND entry.privilege_type = 'CREATE'
    ) THEN
        RAISE EXCEPTION 'Unsafe PUBLIC CREATE privilege on extensions schema';
    END IF;
END
$$;

ALTER FUNCTION public.record_knowledge_source_review(uuid,uuid,bytea)
    SET search_path = pg_catalog, extensions, public, pg_temp;
