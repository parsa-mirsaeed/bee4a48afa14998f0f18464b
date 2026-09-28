-- Persist the exact vectorization contract selected for governed knowledge jobs.
--
-- Existing ingestion rows are intentionally left NULL: historical jobs predate
-- selectable vectorization profiles and must not be assigned guessed provenance.
-- Every newly configured embedding job writes the complete snapshot atomically.

ALTER TABLE public.ingestion_jobs
    ADD COLUMN IF NOT EXISTS embedding_profile TEXT,
    ADD COLUMN IF NOT EXISTS embedding_provider TEXT,
    ADD COLUMN IF NOT EXISTS embedding_model TEXT,
    ADD COLUMN IF NOT EXISTS embedding_dimensions INTEGER,
    ADD COLUMN IF NOT EXISTS embedding_collection TEXT,
    ADD COLUMN IF NOT EXISTS chunk_size INTEGER,
    ADD COLUMN IF NOT EXISTS chunk_overlap INTEGER;

DO $migration$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'public.ingestion_jobs'::regclass
          AND conname = 'ingestion_job_embedding_configuration'
    ) THEN
        ALTER TABLE public.ingestion_jobs
            ADD CONSTRAINT ingestion_job_embedding_configuration
            CHECK (
                (
                    embedding_profile IS NULL
                    AND embedding_provider IS NULL
                    AND embedding_model IS NULL
                    AND embedding_dimensions IS NULL
                    AND embedding_collection IS NULL
                    AND chunk_size IS NULL
                    AND chunk_overlap IS NULL
                )
                OR
                (
                    embedding_profile IS NOT NULL
                    AND embedding_provider IS NOT NULL
                    AND embedding_model IS NOT NULL
                    AND embedding_dimensions IS NOT NULL
                    AND embedding_collection IS NOT NULL
                    AND chunk_size IS NOT NULL
                    AND chunk_overlap IS NOT NULL
                    AND (
                        (
                            embedding_profile = 'openai-v1'
                            AND embedding_provider = 'openai'
                            AND embedding_model = 'text-embedding-3-small'
                            AND embedding_dimensions = 1536
                            AND embedding_collection = 'edutalent_openai_v1'
                        )
                        OR
                        (
                            embedding_profile = 'local-bge-v1'
                            AND embedding_provider = 'local'
                            AND embedding_model = 'BAAI/bge-small-en-v1.5'
                            AND embedding_dimensions = 384
                            AND embedding_collection = 'edutalent_materials_local_v1'
                        )
                    )
                    AND chunk_size BETWEEN 100 AND 20000
                    AND chunk_overlap >= 0
                    AND chunk_overlap <= chunk_size / 2
                )
            );
    END IF;
END
$migration$;

COMMENT ON COLUMN public.ingestion_jobs.embedding_profile IS
    'Immutable registered embedding profile selected when a governed embedding job is queued.';
COMMENT ON COLUMN public.ingestion_jobs.embedding_collection IS
    'Qdrant collection contract captured when the embedding job is queued.';
COMMENT ON COLUMN public.ingestion_jobs.chunk_size IS
    'Character chunk size captured at queue time for deterministic retries.';
COMMENT ON COLUMN public.ingestion_jobs.chunk_overlap IS
    'Character overlap captured at queue time for deterministic retries.';
