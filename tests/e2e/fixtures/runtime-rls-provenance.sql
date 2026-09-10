-- Complete the synthetic Teacher-visible knowledge asset through the exact
-- governed lifecycle required by production RLS. The base seed contains an old
-- pre-provenance published placeholder; replace only that deterministic row and
-- rebuild it through source review -> verified OCR -> embedded provenance ->
-- publication. This makes browser evidence prove the hardened contract instead
-- of manufacturing a trusted published state.

BEGIN;
SET LOCAL app.user_id = 'b0000000-0000-0000-0000-0000000000a0';
SET LOCAL app.user_role = 'PlatformAdmin';
SET LOCAL app.school_id = 'a0000000-0000-0000-0000-0000000000a1';
SET LOCAL app.elevated_operation = 'false';

-- This fixture connection is the disposable PostgreSQL administrator. Remove
-- only the deterministic pre-provenance placeholder; application identities do
-- not receive DELETE permission through this path.
DELETE FROM public.knowledge_assets
WHERE id = 'f3000000-0000-0000-0000-0000000000a1'::uuid;

INSERT INTO public.knowledge_assets (
    id, school_id, title, status, created_by, published_at
) VALUES (
    'f3000000-0000-0000-0000-0000000000a1',
    'a0000000-0000-0000-0000-0000000000a1',
    'E2E Published Asset',
    'submitted',
    'b0000000-0000-0000-0000-0000000000a1',
    NULL
);

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
FROM source_bytes;

SELECT public.record_knowledge_source_review(
    'f3000000-0000-0000-0000-0000000000a1',
    'f4000000-0000-0000-0000-0000000000a1',
    convert_to('%PDF-e2e-published-source', 'UTF8')
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
);

UPDATE public.knowledge_assets
SET status = 'ocr_ready',
    reviewed_by = 'b0000000-0000-0000-0000-0000000000a0'
WHERE id = 'f3000000-0000-0000-0000-0000000000a1';

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
);

UPDATE public.knowledge_assets
SET status = 'embedding_pending'
WHERE id = 'f3000000-0000-0000-0000-0000000000a1';

UPDATE public.knowledge_assets
SET status = 'embedded'
WHERE id = 'f3000000-0000-0000-0000-0000000000a1';

UPDATE public.knowledge_assets
SET status = 'published'
WHERE id = 'f3000000-0000-0000-0000-0000000000a1';

DO $verify$
BEGIN
    IF NOT public.knowledge_asset_has_current_provenance(
        'f3000000-0000-0000-0000-0000000000a1'::uuid
    ) THEN
        RAISE EXCEPTION 'E2E published knowledge fixture lacks current governed provenance';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM public.knowledge_assets
        WHERE id = 'f3000000-0000-0000-0000-0000000000a1'::uuid
          AND status = 'published'::public.knowledge_asset_status
          AND published_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'E2E governed knowledge fixture did not reach published state';
    END IF;
END
$verify$;

COMMIT;
