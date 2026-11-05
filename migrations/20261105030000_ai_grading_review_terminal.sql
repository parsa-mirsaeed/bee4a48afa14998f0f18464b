-- Review decisions are terminal. This is a defense-in-depth invariant, not
-- a grant to review suggestions: the table remains inaccessible to runtime
-- roles until a separately audited teacher-authorized endpoint exists.
CREATE OR REPLACE FUNCTION edutalent_internal.guard_ai_grading_suggestion_review()
RETURNS trigger
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF OLD.status <> 'pending_review' THEN
        RAISE EXCEPTION 'AI grading review is terminal'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.status NOT IN ('accepted', 'rejected')
        OR NEW.reviewed_by IS NULL
        OR NEW.reviewed_at IS NULL
        OR NEW.reviewed_at < OLD.created_at
    THEN
        RAISE EXCEPTION 'AI grading review requires a final decision and reviewer'
            USING ERRCODE = '23514';
    END IF;

    -- Do not allow changing any part of the model artifact. The existing
    -- tenant-integrity trigger independently enforces this on every update.
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.guard_ai_grading_suggestion_review() FROM PUBLIC;

CREATE TRIGGER ai_grading_suggestion_review_terminal
BEFORE UPDATE ON public.ai_grading_suggestions
FOR EACH ROW EXECUTE FUNCTION edutalent_internal.guard_ai_grading_suggestion_review();
