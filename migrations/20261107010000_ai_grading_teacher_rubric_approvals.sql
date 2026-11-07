-- Teacher-approved rubric authority for an individual custom assignment.
-- AI personalization rubrics are never authoritative until the assigned,
-- active teacher explicitly approves a strict rubric in a school-scoped request.
-- Historical approvals are immutable and revocation is terminal.
CREATE TABLE public.ai_grading_rubric_approvals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    custom_assignment_id UUID NOT NULL REFERENCES public.custom_assignments(id) ON DELETE RESTRICT,
    school_id UUID NOT NULL REFERENCES public.schools(id) ON DELETE RESTRICT,
    teacher_id UUID NOT NULL REFERENCES public.teachers(id) ON DELETE RESTRICT,
    grade_scale SMALLINT NOT NULL CHECK (grade_scale IN (20, 100)),
    criteria JSONB NOT NULL CHECK (
        jsonb_typeof(criteria) = 'array'
        AND jsonb_array_length(criteria) BETWEEN 1 AND 20
    ),
    rubric_fingerprint TEXT NOT NULL CHECK (rubric_fingerprint ~ '^[0-9a-f]{64}$'),
    approved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    CHECK (revoked_at IS NULL OR revoked_at >= approved_at)
);
CREATE UNIQUE INDEX ai_grading_rubric_one_active
    ON public.ai_grading_rubric_approvals(custom_assignment_id)
    WHERE revoked_at IS NULL;
CREATE INDEX ai_grading_rubric_teacher_active
    ON public.ai_grading_rubric_approvals(school_id, teacher_id, approved_at DESC)
    WHERE revoked_at IS NULL;

ALTER TABLE public.ai_grading_rubric_approvals ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ai_grading_rubric_approvals FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.ai_grading_rubric_approvals FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'anon') THEN
        REVOKE ALL ON public.ai_grading_rubric_approvals FROM anon;
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'authenticated') THEN
        REVOKE ALL ON public.ai_grading_rubric_approvals FROM authenticated;
    END IF;
END $$;

-- No direct student/parent visibility and no public Data API grants.
CREATE POLICY ai_grading_rubric_teacher_select
ON public.ai_grading_rubric_approvals FOR SELECT USING (
    public.get_role() = 'Teacher'
    AND NOT public.get_elevated_operation()
    AND school_id = public.get_school_id()
    AND EXISTS (
        SELECT 1 FROM public.teachers t
        WHERE t.id = teacher_id AND t.user_id = public.get_user_id()
          AND t.school_id = school_id
    )
);
CREATE POLICY ai_grading_rubric_teacher_insert
ON public.ai_grading_rubric_approvals FOR INSERT WITH CHECK (
    public.get_role() = 'Teacher'
    AND NOT public.get_elevated_operation()
    AND school_id = public.get_school_id()
    AND revoked_at IS NULL
    AND EXISTS (
        SELECT 1 FROM public.teachers t
        WHERE t.id = teacher_id AND t.user_id = public.get_user_id()
          AND t.school_id = school_id
    )
);
CREATE POLICY ai_grading_rubric_teacher_revoke
ON public.ai_grading_rubric_approvals FOR UPDATE
USING (
    public.get_role() = 'Teacher'
    AND NOT public.get_elevated_operation()
    AND school_id = public.get_school_id()
    AND revoked_at IS NULL
    AND EXISTS (
        SELECT 1 FROM public.teachers t
        WHERE t.id = teacher_id AND t.user_id = public.get_user_id()
          AND t.school_id = school_id
    )
)
WITH CHECK (
    public.get_role() = 'Teacher'
    AND NOT public.get_elevated_operation()
    AND school_id = public.get_school_id()
    AND revoked_at IS NOT NULL
    AND EXISTS (
        SELECT 1 FROM public.teachers t
        WHERE t.id = teacher_id AND t.user_id = public.get_user_id()
          AND t.school_id = school_id
    )
);

-- Server validation is necessary but not sufficient: enforce the authoritative
-- assignment/teacher/student/school link, immutable evidence and terminal
-- revocation even against a privileged writer. Only an active Teacher with
-- the matching bounded transaction context can approve or revoke.
CREATE OR REPLACE FUNCTION edutalent_internal.guard_ai_grading_rubric_approval()
RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    actual_school UUID;
    actual_teacher UUID;
    authorized BOOLEAN;
    item JSONB;
    criterion_id TEXT;
    pts NUMERIC;
    possible_sum NUMERIC := 0;
    identifiers TEXT[] := ARRAY[]::TEXT[];
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'Teacher rubric approval evidence cannot be deleted'
            USING ERRCODE = '23514';
    END IF;
    IF public.get_role() IS DISTINCT FROM 'Teacher'
        OR public.get_elevated_operation()
        OR public.get_user_id() IS NULL
        OR public.get_school_id() IS NULL THEN
        RAISE EXCEPTION 'Teacher rubric approval requires a bounded teacher session'
            USING ERRCODE = '42501';
    END IF;

    SELECT cs.school_id, a.teacher_id,
           EXISTS (
               SELECT 1 FROM public.teachers t
               JOIN public.users u ON u.id = t.user_id
               JOIN public.roles r ON r.id = u.role_id
               JOIN public.teaching_assignments ta
                 ON ta.teacher_id = t.id AND ta.class_section_id = a.class_section_id
               JOIN public.students st ON st.id = ca.student_id
               JOIN public.users student_user ON student_user.id = st.user_id
               JOIN public.enrollments e
                 ON e.student_id = st.id AND e.class_section_id = a.class_section_id
               WHERE t.id = a.teacher_id
                 AND t.user_id = public.get_user_id()
                 AND t.school_id = cs.school_id
                 AND u.school_id = cs.school_id
                 AND u.is_active AND r.name::text = 'Teacher'
                 AND st.school_id = cs.school_id
                 AND student_user.school_id = cs.school_id
                 AND student_user.is_active
           )
    INTO actual_school, actual_teacher, authorized
    FROM public.custom_assignments ca
    JOIN public.assignments a ON a.id = ca.assignment_id
    JOIN public.class_sections cs ON cs.id = a.class_section_id
    WHERE ca.id = NEW.custom_assignment_id AND a.status = 'Published'::public.assignment_status;

    IF actual_school IS NULL OR NOT COALESCE(authorized, FALSE)
       OR NEW.school_id IS DISTINCT FROM actual_school
       OR NEW.teacher_id IS DISTINCT FROM actual_teacher
       OR public.get_school_id() IS DISTINCT FROM actual_school THEN
        RAISE EXCEPTION 'Teacher rubric approval does not belong to the assigned teacher and school'
            USING ERRCODE = '42501';
    END IF;

    IF TG_OP = 'UPDATE' THEN
        IF OLD.revoked_at IS NOT NULL
            OR NEW.id IS DISTINCT FROM OLD.id
            OR NEW.custom_assignment_id IS DISTINCT FROM OLD.custom_assignment_id
            OR NEW.school_id IS DISTINCT FROM OLD.school_id
            OR NEW.teacher_id IS DISTINCT FROM OLD.teacher_id
            OR NEW.grade_scale IS DISTINCT FROM OLD.grade_scale
            OR NEW.criteria IS DISTINCT FROM OLD.criteria
            OR NEW.rubric_fingerprint IS DISTINCT FROM OLD.rubric_fingerprint
            OR NEW.approved_at IS DISTINCT FROM OLD.approved_at
            OR NEW.revoked_at IS NULL THEN
            RAISE EXCEPTION 'Teacher rubric approval is immutable; only one revocation is allowed'
                USING ERRCODE = '23514';
        END IF;
        -- Revocation time is server-authoritative even for a direct DB caller.
        NEW.revoked_at := transaction_timestamp();
        RETURN NEW;
    END IF;

    -- Approval time is server-authoritative, never a caller-provided history.
    NEW.approved_at := transaction_timestamp();
    IF NEW.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'New rubric approvals cannot start revoked'
            USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.custom_assignments ca
        WHERE ca.id = NEW.custom_assignment_id AND ca.graded_at IS NOT NULL
    ) OR EXISTS (
        SELECT 1 FROM public.submissions s
        WHERE s.custom_assignment_id = NEW.custom_assignment_id
          AND (s.grade IS NOT NULL OR s.graded_by IS NOT NULL
               OR s.grade_scale <> NEW.grade_scale)
    ) THEN
        RAISE EXCEPTION 'Cannot approve a graded or scale-mismatched submission'
            USING ERRCODE = '23514';
    END IF;

    -- Never store structurally malformed or grade-scale-inconsistent criteria.
    -- Strict Unicode identifier checks also run in the shared Rust contract.
    FOR item IN SELECT value FROM jsonb_array_elements(NEW.criteria) AS element(value) LOOP
        IF jsonb_typeof(item) IS DISTINCT FROM 'object'
           OR item ? 'criterion_id' IS NOT TRUE
           OR item ? 'possible_points' IS NOT TRUE
           OR (SELECT count(*) FROM jsonb_object_keys(item)) <> 2
           OR jsonb_typeof(item->'criterion_id') IS DISTINCT FROM 'string'
           OR jsonb_typeof(item->'possible_points') IS DISTINCT FROM 'number' THEN
            RAISE EXCEPTION 'Invalid rubric criterion shape' USING ERRCODE = '23514';
        END IF;
        criterion_id := item->>'criterion_id';
        pts := (item->>'possible_points')::NUMERIC;
        IF criterion_id = '' OR length(criterion_id) > 100
           OR criterion_id <> btrim(criterion_id)
           OR criterion_id = ANY(identifiers)
           OR pts <= 0 THEN
            RAISE EXCEPTION 'Invalid or duplicate rubric criterion' USING ERRCODE = '23514';
        END IF;
        identifiers := array_append(identifiers, criterion_id);
        possible_sum := possible_sum + pts;
    END LOOP;
    IF abs(possible_sum - NEW.grade_scale) > 0.000001 THEN
        RAISE EXCEPTION 'Rubric points do not match grade scale' USING ERRCODE = '23514';
    END IF;

    -- JSONB's normalized representation is canonical inside PostgreSQL.
    NEW.rubric_fingerprint := encode(sha256(convert_to(NEW.criteria::TEXT, 'UTF8')), 'hex');
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION edutalent_internal.guard_ai_grading_rubric_approval() FROM PUBLIC;

CREATE TRIGGER ai_grading_rubric_approval_guard
BEFORE INSERT OR UPDATE OR DELETE ON public.ai_grading_rubric_approvals
FOR EACH ROW EXECUTE FUNCTION edutalent_internal.guard_ai_grading_rubric_approval();
