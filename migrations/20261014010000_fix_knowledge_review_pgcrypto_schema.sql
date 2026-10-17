-- The pinned Supabase PostgreSQL installation keeps pgcrypto under
-- extensions; generic CI PostgreSQL installs it under public. Use only the
-- already-installed trusted digest schema. Do not rewrite applied migration
-- checksums or change review authorization, bytes, audit and FORCE-RLS logic.
DO $$
BEGIN
    IF to_regprocedure('public.record_knowledge_source_review(uuid,uuid,bytea)') IS NULL THEN
        RAISE EXCEPTION 'Governed source-review procedure is missing';
    END IF;

    IF to_regprocedure('extensions.digest(bytea,text)') IS NOT NULL THEN
        IF EXISTS (
            SELECT 1 FROM pg_namespace namespace
            CROSS JOIN LATERAL aclexplode(
                COALESCE(namespace.nspacl, acldefault('n', namespace.nspowner))
            ) entry
            WHERE namespace.nspname = 'extensions'
              AND entry.grantee = 0
              AND entry.privilege_type = 'CREATE'
        ) THEN
            RAISE EXCEPTION 'Unsafe PUBLIC CREATE privilege on extensions schema';
        END IF;

        ALTER FUNCTION public.record_knowledge_source_review(uuid,uuid,bytea)
            SET search_path = pg_catalog, extensions, public, pg_temp;
    ELSIF to_regprocedure('public.digest(bytea,text)') IS NOT NULL THEN
        -- Generic CI PostgreSQL has pgcrypto in public, so do not require
        -- an unrelated extensions schema. The original function already
        -- resolves the digest under this path.
        ALTER FUNCTION public.record_knowledge_source_review(uuid,uuid,bytea)
            SET search_path = pg_catalog, public, pg_temp;
    ELSE
        RAISE EXCEPTION 'Required pgcrypto digest is not installed';
    END IF;
END
$$;
