-- The source-review evidence table is protected from the long-running
-- NOBYPASSRLS application role by RLS and intentionally has no INSERT policy.
-- The byte-verifying SECURITY DEFINER function is owned by the migration/table
-- owner and is the sole evidence-minting path. FORCE RLS would also subject that
-- non-superuser owner to the no-INSERT policy in hardened production, making the
-- protected function unusable even though CI's PostgreSQL superuser bypasses it.

ALTER TABLE public.knowledge_source_reviews ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.knowledge_source_reviews NO FORCE ROW LEVEL SECURITY;

COMMENT ON TABLE public.knowledge_source_reviews IS
    'Append-only trusted source-review evidence. RLS blocks direct application-role writes; record_knowledge_source_review is the byte-verifying SECURITY DEFINER insert boundary.';
