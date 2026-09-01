-- Trusted source-review evidence remains FORCE-RLS protected. The long-running
-- application role must not be able to INSERT evidence directly, while the
-- byte-verifying SECURITY DEFINER function must remain usable even when the
-- migration/table owner is a hardened non-superuser.
--
-- Inside a SECURITY DEFINER call, `current_user` becomes the function owner but
-- `session_user` remains the caller. Requiring both the migration-owner identity
-- and a distinct session identity therefore permits only the trusted definer
-- execution path; direct SQL by either the application role or the table owner
-- does not satisfy this policy. The owner is embedded as a literal at migration
-- time so the policy itself contains no catalog subquery.

ALTER TABLE public.knowledge_source_reviews ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.knowledge_source_reviews FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS knowledge_source_reviews_definer_insert
    ON public.knowledge_source_reviews;
DO $policy$
DECLARE
    review_owner TEXT := current_user;
BEGIN
    EXECUTE format(
        'CREATE POLICY knowledge_source_reviews_definer_insert ON public.knowledge_source_reviews FOR INSERT WITH CHECK (current_user <> session_user AND current_user = %L)',
        review_owner
    );
END
$policy$;

COMMENT ON TABLE public.knowledge_source_reviews IS
    'Append-only trusted source-review evidence. FORCE RLS blocks direct application writes; only the byte-verifying SECURITY DEFINER context may insert evidence.';
