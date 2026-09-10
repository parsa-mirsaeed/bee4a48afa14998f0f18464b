-- Complete the synthetic Teacher-visible knowledge asset with the exact governed
-- provenance chain required by production RLS.  The base seed intentionally
-- creates deterministic actors/assets; this follow-up runs after all migrations
-- so it cannot accidentally model a pre-hardening published row as trustworthy.

BEGIN;
SET LOCAL app.user_id = 'b0000000-0000-0000-0000-0000000000a0';
SET LOCAL app.user_role = 'PlatformAdmin';
SET LOCAL app.school_id = 'a0000000-0000-0000-0000-0000000000a1';
SET LOCAL app.elevated_operation = 'false';

WITH source_bytes AS (
    SELECT convert_to('%PDF-e2e-published-source', 'UTF8') AS bytes
)
INSERT INTO public.knowledge_source_files (
    id,
    asset_id,
    original_file_url,
    original_filename,
    mime_type,
    file_size_bytes,
    sha256,
    is_scanned_pdf
)
SELECT
    'f4000000-0000-0000-0000-0000000000a1',
    'f3000000-0000-0000-0000-0000000000a1',
    'storage://edutalent-knowledge-sources/a0000000-0000-0000-0000-0000000000a1/f4000000-0000-0000-0000-0000000000a1.pdf',
    'e2e-published-source.pdf',
    'application/pdf',
    octet_length(bytes),
    lower(encode(digest(bytes, 'sha256'), 'hex')),
    FALSE
FROM source_bytes
ON CONFLICT (id) DO NOTHING;

SELECT public.record_knowledge_source_review(
    'f3000000-0000-0000-0000-0000000000a1',
    'f4000000-0000-0000-0000-0000000000a1',
    convert_to('%PDF-e2e-published-source', 'UTF8')
)
WHERE NOT EXISTS (
    SELECT 1
    FROM public.knowledge_source_reviews
    WHERE asset_id = 'f3000000-0000-0000-0000-0000000000a1'
      AND source_file_id = 'f4000000-0000-0000-0000-0000000000a1'
);

INSERT INTO public.knowledge_ocr_texts (
    asset_id,
    raw_text,
    clean_text,
    ocr_provider,
    ocr_verified_by,
    text_sha256
)
VALUES (
    'f3000000-0000-0000-0000-0000000000a1',
    'E2E published governed knowledge text',
    'E2E published governed knowledge text',
    'e2e-reviewed-fixture',
    'b0000000-0000-0000-0000-0000000000a0',
    lower(encode(digest(convert_to('E2E published governed knowledge text', 'UTF8'), 'sha256'), 'hex'))
)
ON CONFLICT (asset_id) DO NOTHING;

INSERT INTO public.knowledge_chunks (
    asset_id,
    chunk_index,
    text,
    token_count,
    embedding_provider,
    embedding_model,
    vector_id,
    metadata_json
)
VALUES (
    'f3000000-0000-0000-0000-0000000000a1',
    0,
    'E2E published governed knowledge text',
    5,
    'e2e-fixture',
    'e2e-fixture-model',
    'e2e-published-governed-f3000000-a1-0',
    '{"fixture":true,"provenance":"governed"}'::jsonb
)
ON CONFLICT (asset_id, chunk_index) DO NOTHING;

DO $verify$
BEGIN
    IF NOT public.knowledge_asset_has_current_provenance(
        'f3000000-0000-0000-0000-0000000000a1'::uuid
    ) THEN
        RAISE EXCEPTION 'E2E published knowledge fixture lacks current governed provenance';
    END IF;
END
$verify$;

COMMIT;
