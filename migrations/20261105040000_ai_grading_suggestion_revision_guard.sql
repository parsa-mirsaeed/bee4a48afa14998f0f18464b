-- Reject stale AI suggestions both when first recorded and when reviewed.
-- Lock the submission against concurrent revision changes while checking.
-- Existing tenant integrity, immutable evidence, terminal review and FORCE RLS
-- remain in effect; this function grants no runtime table access.
CREATE OR REPLACE FUNCTION edutalent_internal.guard_ai_grading_suggestion_revision()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    current_revision UUID;
BEGIN
    SELECT s.revision INTO current_revision
    FROM public.submissions s
    WHERE s.id = NEW.submission_id
    FOR SHARE;

    IF current_revision IS NULL
       OR NEW.submission_revision IS DISTINCT FROM current_revision THEN
        RAISE EXCEPTION 'AI grading suggestion references a stale submission revision'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.guard_ai_grading_suggestion_revision() FROM PUBLIC;

CREATE TRIGGER ai_grading_suggestion_revision_guard
BEFORE INSERT OR UPDATE ON public.ai_grading_suggestions
FOR EACH ROW
EXECUTE FUNCTION edutalent_internal.guard_ai_grading_suggestion_revision();
