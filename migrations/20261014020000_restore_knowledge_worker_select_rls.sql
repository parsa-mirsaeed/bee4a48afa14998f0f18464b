-- Restore bounded system_job reads dropped by
-- 20260901090000_bind_knowledge_ocr_to_reviewed_source.sql.
-- Use additive SELECT-only policies to preserve teacher provenance
-- restrictions and all existing DML grants / FORCE RLS.
-- DROP IF EXISTS supports production hosts with the equivalent manual repair.

DROP POLICY IF EXISTS knowledge_assets_worker_select ON public.knowledge_assets;
CREATE POLICY knowledge_assets_worker_select ON public.knowledge_assets
    AS PERMISSIVE FOR SELECT
    USING (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND school_id = public.get_school_id()
    );

DROP POLICY IF EXISTS knowledge_source_files_worker_select ON public.knowledge_source_files;
CREATE POLICY knowledge_source_files_worker_select ON public.knowledge_source_files
    AS PERMISSIVE FOR SELECT
    USING (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1 FROM public.knowledge_assets AS asset
            WHERE asset.id = knowledge_source_files.asset_id
              AND asset.school_id = public.get_school_id()
              AND asset.current_source_file_id = knowledge_source_files.id
        )
    );

DROP POLICY IF EXISTS knowledge_source_reviews_worker_select ON public.knowledge_source_reviews;
CREATE POLICY knowledge_source_reviews_worker_select ON public.knowledge_source_reviews
    AS PERMISSIVE FOR SELECT
    USING (
        public.get_role() = 'system_job'
        AND public.get_elevated_operation()
        AND EXISTS (
            SELECT 1 FROM public.knowledge_assets AS asset
            WHERE asset.id = knowledge_source_reviews.asset_id
              AND asset.school_id = public.get_school_id()
              AND asset.current_source_file_id = knowledge_source_reviews.source_file_id
        )
    );
