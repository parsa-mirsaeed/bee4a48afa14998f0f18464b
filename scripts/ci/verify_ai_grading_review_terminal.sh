#!/usr/bin/env bash
set -euo pipefail
: "${DATABASE_URL:?DATABASE_URL must be set}"

# Exercise the production trigger against an isolated, rollback-only fixture.
# LIKE copies CHECK constraints but deliberately does not copy foreign keys;
# no school, submission, or teacher production rows are touched.
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;
CREATE TEMP TABLE ai_review_fixture
    (LIKE public.ai_grading_suggestions INCLUDING DEFAULTS INCLUDING CONSTRAINTS);
CREATE TRIGGER ai_review_fixture_terminal
    BEFORE UPDATE ON ai_review_fixture
    FOR EACH ROW EXECUTE FUNCTION edutalent_internal.guard_ai_grading_suggestion_review();

DO $$
DECLARE
    accepted_id uuid;
    rejected_id uuid;
    invalid_id uuid;
    test_case text;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relname = 'ai_grading_suggestions'
          AND c.relrowsecurity AND c.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'AI grading suggestion table must enforce RLS';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'public.ai_grading_suggestions'::regclass
          AND tgname = 'ai_grading_suggestion_review_terminal'
          AND tgenabled <> 'D'
    ) THEN
        RAISE EXCEPTION 'Production terminal review trigger missing or disabled';
    END IF;

    INSERT INTO ai_review_fixture
        (submission_id, submission_revision, school_id, rubric_fingerprint,
         contract_version, grade_scale, suggested_grade, confidence,
         feedback, criteria)
    VALUES
        (gen_random_uuid(), gen_random_uuid(), gen_random_uuid(), repeat('a', 64),
         1, 100, 85, 0.8, 'review fixture', '[{}]'::jsonb)
    RETURNING id INTO accepted_id;

    INSERT INTO ai_review_fixture
        (submission_id, submission_revision, school_id, rubric_fingerprint,
         contract_version, grade_scale, suggested_grade, confidence,
         feedback, criteria)
    VALUES
        (gen_random_uuid(), gen_random_uuid(), gen_random_uuid(), repeat('b', 64),
         1, 100, 65, 0.6, 'review fixture', '[{}]'::jsonb)
    RETURNING id INTO rejected_id;

    INSERT INTO ai_review_fixture
        (submission_id, submission_revision, school_id, rubric_fingerprint,
         contract_version, grade_scale, suggested_grade, confidence,
         feedback, criteria)
    VALUES
        (gen_random_uuid(), gen_random_uuid(), gen_random_uuid(), repeat('c', 64),
         1, 100, 75, 0.7, 'review fixture', '[{}]'::jsonb)
    RETURNING id INTO invalid_id;

    UPDATE ai_review_fixture
    SET status = 'accepted', reviewed_by = gen_random_uuid(), reviewed_at = now()
    WHERE id = accepted_id;
    UPDATE ai_review_fixture
    SET status = 'rejected', reviewed_by = gen_random_uuid(), reviewed_at = now()
    WHERE id = rejected_id;

    FOREACH test_case IN ARRAY ARRAY[
        'repeat_accept', 'reverse_accept', 'repeat_reject',
        'reverse_reject', 'premature_timestamp', 'missing_reviewer',
        'missing_timestamp', 'pending_mutation'
    ] LOOP
        BEGIN
            CASE test_case
                WHEN 'repeat_accept' THEN
                    UPDATE ai_review_fixture SET reviewed_at = now() WHERE id = accepted_id;
                WHEN 'reverse_accept' THEN
                    UPDATE ai_review_fixture SET status = 'rejected' WHERE id = accepted_id;
                WHEN 'repeat_reject' THEN
                    UPDATE ai_review_fixture SET reviewed_at = now() WHERE id = rejected_id;
                WHEN 'reverse_reject' THEN
                    UPDATE ai_review_fixture SET status = 'accepted' WHERE id = rejected_id;
                WHEN 'premature_timestamp' THEN
                    UPDATE ai_review_fixture SET status = 'accepted',
                        reviewed_by = gen_random_uuid(),
                        reviewed_at = created_at - interval '1 second'
                    WHERE id = invalid_id;
                WHEN 'missing_reviewer' THEN
                    UPDATE ai_review_fixture SET status = 'accepted',
                        reviewed_at = now() WHERE id = invalid_id;
                WHEN 'missing_timestamp' THEN
                    UPDATE ai_review_fixture SET status = 'rejected',
                        reviewed_by = gen_random_uuid() WHERE id = invalid_id;
                WHEN 'pending_mutation' THEN
                    UPDATE ai_review_fixture SET feedback = 'tampered'
                    WHERE id = invalid_id;
            END CASE;
            RAISE EXCEPTION 'Expected review guard rejection for %', test_case;
        EXCEPTION
            WHEN check_violation THEN NULL;
        END;
    END LOOP;
END $$;
ROLLBACK;
SQL
