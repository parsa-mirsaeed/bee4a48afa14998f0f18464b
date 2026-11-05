-- Ensure a suggestion cannot claim a school other than the submission's
-- assignment school. RLS alone cannot enforce integrity for privileged writers.
CREATE OR REPLACE FUNCTION edutalent_internal.validate_ai_grading_suggestion_tenant()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    actual_school UUID;
BEGIN
    SELECT cs.school_id INTO actual_school
    FROM public.submissions s
    JOIN public.custom_assignments ca ON ca.id = s.custom_assignment_id
    JOIN public.assignments a ON a.id = ca.assignment_id
    JOIN public.class_sections cs ON cs.id = a.class_section_id
    WHERE s.id = NEW.submission_id;

    IF actual_school IS NULL OR actual_school IS DISTINCT FROM NEW.school_id THEN
        RAISE EXCEPTION 'AI suggestion school does not match submission'
            USING ERRCODE = '23514';
    END IF;

    -- A stored suggestion is an immutable model artifact. Review decisions
    -- may change status/reviewer/timestamp but never its original evidence.
    IF TG_OP = 'UPDATE' AND (
        NEW.submission_id IS DISTINCT FROM OLD.submission_id
        OR NEW.submission_revision IS DISTINCT FROM OLD.submission_revision
        OR NEW.school_id IS DISTINCT FROM OLD.school_id
        OR NEW.rubric_fingerprint IS DISTINCT FROM OLD.rubric_fingerprint
        OR NEW.contract_version IS DISTINCT FROM OLD.contract_version
        OR NEW.grade_scale IS DISTINCT FROM OLD.grade_scale
        OR NEW.suggested_grade IS DISTINCT FROM OLD.suggested_grade
        OR NEW.confidence IS DISTINCT FROM OLD.confidence
        OR NEW.feedback IS DISTINCT FROM OLD.feedback
        OR NEW.criteria IS DISTINCT FROM OLD.criteria
        OR NEW.created_at IS DISTINCT FROM OLD.created_at
    ) THEN
        RAISE EXCEPTION 'AI suggestion artifact is immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.validate_ai_grading_suggestion_tenant() FROM PUBLIC;
DROP TRIGGER IF EXISTS ai_grading_suggestion_tenant_integrity ON public.ai_grading_suggestions;
CREATE TRIGGER ai_grading_suggestion_tenant_integrity
BEFORE INSERT OR UPDATE ON public.ai_grading_suggestions
FOR EACH ROW EXECUTE FUNCTION edutalent_internal.validate_ai_grading_suggestion_tenant();
