-- ONE disposable synthetic school, CI-only. Never run on a deployed database.
-- Browser runner requires CI=true, exact-head source and disposable edutalent_ci.
-- This transaction is executed as migration owner, outside all app RLS actors.
BEGIN;
ALTER TABLE public.ai_grading_pdf_school_optins NO FORCE ROW LEVEL SECURITY;
INSERT INTO public.ai_grading_pdf_school_optins (school_id,enabled)
VALUES ('a0000000-0000-0000-0000-0000000000c1',TRUE);
ALTER TABLE public.ai_grading_pdf_school_optins FORCE ROW LEVEL SECURITY;
INSERT INTO edutalent_internal.ai_grading_pdf_budget_policy (
  school_id, max_requests_per_utc_day, max_input_bytes_per_utc_day,
  reserved_microusd_per_request, max_reserved_microusd_per_utc_day
) VALUES (
  'a0000000-0000-0000-0000-0000000000c1',2,2000000,250000,500000
);
DO $$
BEGIN
  IF (SELECT count(*) FROM public.ai_grading_pdf_school_optins WHERE enabled) <> 1
     OR (SELECT relforcerowsecurity FROM pg_class
         WHERE oid='public.ai_grading_pdf_school_optins'::regclass) IS NOT TRUE
     OR (SELECT count(*) FROM edutalent_internal.ai_grading_pdf_budget_policy) <> 1 THEN
    RAISE EXCEPTION 'synthetic PDF school opt-in/budget isolation failed';
  END IF;
END $$;
COMMIT;
