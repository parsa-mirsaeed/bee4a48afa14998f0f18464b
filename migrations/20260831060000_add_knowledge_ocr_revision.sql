-- Verified OCR is edited by platform administrators. A fresh opaque revision
-- prevents a stale browser tab from silently replacing another review.
ALTER TABLE public.knowledge_ocr_texts
    ADD COLUMN IF NOT EXISTS revision UUID NOT NULL DEFAULT gen_random_uuid();

UPDATE public.knowledge_ocr_texts
SET revision = gen_random_uuid()
WHERE revision IS NULL;
