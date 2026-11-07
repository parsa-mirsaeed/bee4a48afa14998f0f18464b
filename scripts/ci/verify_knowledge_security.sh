#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
    missing_rls_tables text[];
    missing_policies text[];
    insecure_policy_count integer;
    teacher_policy text;
    review_insert_policy text;
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM roles
        WHERE name = 'PlatformAdmin'::role_name
          AND permissions @> '{"publish_knowledge_assets": true}'::jsonb
    ) THEN
        RAISE EXCEPTION 'PlatformAdmin role row or least-privilege permissions are missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM roles
        WHERE name = 'PlatformAdmin'::role_name
          AND permissions ?| ARRAY['can_manage_users', 'can_manage_classes', 'manage_system_settings']
    ) THEN
        RAISE EXCEPTION 'PlatformAdmin role unexpectedly contains school-management permissions';
    END IF;

    SELECT array_agg(required_name ORDER BY required_name)
    INTO missing_rls_tables
    FROM (
        VALUES
            ('knowledge_assets'),
            ('knowledge_source_files'),
            ('knowledge_source_reviews'),
            ('knowledge_ocr_texts'),
            ('knowledge_ocr_revision_provenance'),
            ('knowledge_chunks'),
            ('teacher_asset_selections'),
            ('ingestion_jobs'),
            ('knowledge_audit_logs')
    ) AS required(required_name)
    WHERE NOT EXISTS (
        SELECT 1
        FROM pg_class relation
        JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relname = required_name
          AND relation.relrowsecurity
    );

    IF missing_rls_tables IS NOT NULL THEN
        RAISE EXCEPTION 'RLS is not enabled for governed knowledge tables: %', missing_rls_tables;
    END IF;

    SELECT array_agg(required_policy ORDER BY required_policy)
    INTO missing_policies
    FROM (
        VALUES
            ('knowledge_assets_scoped_select'),
            ('knowledge_assets_scoped_insert'),
            ('knowledge_assets_admin_update'),
            ('knowledge_source_files_scoped_select'),
            ('knowledge_source_files_scoped_insert'),
            ('knowledge_source_files_admin_write'),
            ('knowledge_source_reviews_admin_select'),
            ('knowledge_source_reviews_definer_insert'),
            ('knowledge_ocr_texts_admin_all'),
            ('knowledge_ocr_revision_provenance_admin_select'),
            ('knowledge_ocr_revision_provenance_trigger_insert'),
            ('knowledge_chunks_admin_all'),
            ('teacher_asset_selection_owner'),
            ('ingestion_jobs_admin_all'),
            ('knowledge_audit_logs_admin_select'),
            ('knowledge_audit_logs_actor_insert')
    ) AS required(required_policy)
    WHERE NOT EXISTS (
        SELECT 1
        FROM pg_policies
        WHERE schemaname = 'public'
          AND policyname = required_policy
    );

    IF missing_policies IS NOT NULL THEN
        RAISE EXCEPTION 'Missing governed knowledge policies: %', missing_policies;
    END IF;

    -- A worker's queue RLS depends on SELECT of its linked asset.
    -- Both the current source revision and verified review evidence must
    -- remain school-scoped, while the teacher's own policy stays intact.
    IF EXISTS (
        SELECT 1
        FROM (
            VALUES
                ('knowledge_assets', 'knowledge_assets_worker_select', FALSE),
                ('knowledge_source_files', 'knowledge_source_files_worker_select', TRUE),
                ('knowledge_source_reviews', 'knowledge_source_reviews_worker_select', TRUE)
        ) AS required(tablename, policyname, restrict_revision)
        LEFT JOIN pg_policies p
          ON p.schemaname = 'public'
         AND p.tablename = required.tablename
         AND p.policyname = required.policyname
        WHERE p.policyname IS NULL
           OR p.cmd <> 'SELECT'
           OR p.permissive <> 'PERMISSIVE'
           OR p.qual NOT LIKE '%system_job%'
           OR p.qual NOT LIKE '%get_elevated_operation()%'
           OR p.qual NOT LIKE '%get_school_id()%'
           OR (required.restrict_revision AND p.qual NOT LIKE '%current_source_file_id%')
    ) THEN
        RAISE EXCEPTION 'Missing or broadened scoped knowledge worker SELECT policies';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM pg_class relation
        JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relname IN (
              'knowledge_assets', 'knowledge_source_files', 'knowledge_source_reviews'
          )
          AND NOT (relation.relrowsecurity AND relation.relforcerowsecurity)
    ) THEN
        RAISE EXCEPTION 'Governed worker-related knowledge tables must retain FORCE RLS';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_class relation
        JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relname = 'knowledge_source_reviews'
          AND relation.relrowsecurity
          AND relation.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'Trusted source-review evidence must remain FORCE-RLS protected';
    END IF;

    SELECT with_check
    INTO review_insert_policy
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'knowledge_source_reviews'
      AND policyname = 'knowledge_source_reviews_definer_insert'
      AND cmd = 'INSERT';

    IF review_insert_policy IS NULL
       OR lower(review_insert_policy) NOT LIKE '%current_user%'
       OR lower(review_insert_policy) NOT LIKE '%session_user%'
       OR lower(review_insert_policy) NOT LIKE '%<>%' THEN
        RAISE EXCEPTION 'Trusted source-review INSERT policy is not restricted to SECURITY DEFINER execution: %', review_insert_policy;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'knowledge_source_reviews'
          AND cmd IN ('INSERT', 'ALL')
          AND policyname <> 'knowledge_source_reviews_definer_insert'
    ) THEN
        RAISE EXCEPTION 'Trusted source-review evidence has an unexpected direct INSERT/ALL policy';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_proc procedure
        JOIN pg_class relation
          ON relation.oid = 'public.knowledge_source_reviews'::regclass
        WHERE procedure.oid = 'public.record_knowledge_source_review(uuid,uuid,bytea)'::regprocedure
          AND procedure.prosecdef
          AND procedure.proowner = relation.relowner
    ) THEN
        RAISE EXCEPTION 'Source-review evidence function must be SECURITY DEFINER and owned by the evidence-table owner';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM information_schema.routine_privileges
        WHERE specific_schema = 'public'
          AND routine_name = 'record_knowledge_source_review'
          AND grantee = 'PUBLIC'
          AND privilege_type = 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'Source-review evidence function remains executable by PUBLIC';
    END IF;

    -- Check the actual extension location (pinned Supabase uses extensions,
    -- generic CI PostgreSQL uses public). Do not assume either is universal.
    IF to_regprocedure('extensions.digest(bytea,text)') IS NOT NULL THEN
        IF NOT EXISTS (
            SELECT 1 FROM pg_proc procedure
            WHERE procedure.oid = 'public.record_knowledge_source_review(uuid,uuid,bytea)'::regprocedure
              AND 'search_path=pg_catalog, extensions, public, pg_temp' = ANY(procedure.proconfig)
        ) THEN
            RAISE EXCEPTION 'Source-review SECURITY DEFINER cannot resolve extensions.pgcrypto';
        END IF;
    ELSIF to_regprocedure('public.digest(bytea,text)') IS NOT NULL THEN
        IF NOT EXISTS (
            SELECT 1 FROM pg_proc procedure
            WHERE procedure.oid = 'public.record_knowledge_source_review(uuid,uuid,bytea)'::regprocedure
              AND 'search_path=pg_catalog, public, pg_temp' = ANY(procedure.proconfig)
        ) THEN
            RAISE EXCEPTION 'Source-review SECURITY DEFINER cannot resolve public.pgcrypto';
        END IF;
    ELSE
        RAISE EXCEPTION 'Source-review pgcrypto digest is unavailable';
    END IF;

    IF to_regprocedure('extensions.digest(bytea,text)') IS NOT NULL THEN
        IF encode(extensions.digest(convert_to('test', 'UTF8'), 'sha256'), 'hex')
           <> '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08' THEN
            RAISE EXCEPTION 'extensions.pgcrypto SHA256 failed known-answer test';
        END IF;
    ELSIF encode(public.digest(convert_to('test', 'UTF8'), 'sha256'), 'hex')
          <> '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08' THEN
        RAISE EXCEPTION 'public.pgcrypto SHA256 failed known-answer test';
    END IF;

    SELECT COUNT(*)
    INTO insecure_policy_count
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename LIKE 'knowledge%'
      AND (COALESCE(qual, '') || COALESCE(with_check, '')) LIKE '%app.current_%';

    IF insecure_policy_count <> 0 THEN
        RAISE EXCEPTION 'Governed knowledge policies still reference obsolete app.current_* settings';
    END IF;

    SELECT qual
    INTO teacher_policy
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'knowledge_assets'
      AND policyname = 'knowledge_assets_scoped_select';

    IF teacher_policy NOT LIKE '%get_role()%'
       OR teacher_policy NOT LIKE '%get_school_id()%'
       OR teacher_policy NOT LIKE '%published%' THEN
        RAISE EXCEPTION 'Teacher asset visibility policy is not school- and publication-scoped: %', teacher_policy;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'material_embeddings'
          AND column_name = 'current_batch'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'material_embeddings'
          AND column_name = 'total_batches'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'material_embeddings'
          AND column_name = 'cancelled'
    ) THEN
        RAISE EXCEPTION 'Material embedding progress/cancellation columns are incomplete';
    END IF;

    IF pg_get_functiondef('prevent_teacher_document_ingestion()'::regprocedure)
       NOT LIKE '%JOIN roles%' THEN
        RAISE EXCEPTION 'Teacher document-ingestion guard does not resolve the canonical roles table';
    END IF;
END
$$;

-- Exercise the teacher document-ingestion boundary inside a rolled-back fixture.
BEGIN;
DO $$
DECLARE
    school_uuid uuid;
    subject_uuid uuid;
    class_uuid uuid;
    teacher_user_uuid uuid;
    teacher_role_uuid uuid;
BEGIN
    INSERT INTO schools (name)
    VALUES ('Knowledge security probe ' || gen_random_uuid()::text)
    RETURNING id INTO school_uuid;

    INSERT INTO subjects (code, name)
    VALUES ('KSP-' || substr(gen_random_uuid()::text, 1, 8), 'Knowledge Security Probe')
    RETURNING id INTO subject_uuid;

    INSERT INTO class_sections (school_id, subject_id, name, term)
    VALUES (school_uuid, subject_uuid, 'Security Probe', 'CI')
    RETURNING id INTO class_uuid;

    SELECT id INTO teacher_role_uuid
    FROM roles
    WHERE name = 'Teacher'::role_name;

    INSERT INTO users (name, email, role_id, school_id)
    VALUES (
        'Knowledge Security Probe Teacher',
        'knowledge-security-' || gen_random_uuid()::text || '@example.invalid',
        teacher_role_uuid,
        school_uuid
    )
    RETURNING id INTO teacher_user_uuid;

    BEGIN
        INSERT INTO class_materials (
            class_section_id, title, material_type, created_by
        ) VALUES (
            class_uuid, 'Blocked teacher PDF', 'document', teacher_user_uuid
        );
        RAISE EXCEPTION 'Teacher document ingestion guard accepted a document';
    EXCEPTION
        WHEN insufficient_privilege THEN NULL;
    END;

    INSERT INTO class_materials (
        class_section_id, title, material_type, external_link, created_by
    ) VALUES (
        class_uuid, 'Allowed teacher link', 'link', 'https://example.invalid', teacher_user_uuid
    );
END
$$;
ROLLBACK;

SELECT 'governed knowledge security invariants verified' AS result;
SQL

bash scripts/ci/verify_transaction_scoped_rls.sh
bash scripts/ci/verify_class_enrollment_rls.sh
bash scripts/ci/verify_knowledge_asset_editing.sh

# Exercise terminal AI grading review transitions on every database security run.

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

# Fail closed if a later migration accidentally exposes unreviewed AI grading
# evidence through Data API roles or introduces a permissive RLS policy.
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
    runtime_role text;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relname = 'ai_grading_suggestions'
          AND c.relrowsecurity AND c.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'AI grading suggestions must retain FORCE RLS';
    END IF;

    IF (
        SELECT COUNT(*) FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'ai_grading_suggestions'
    ) <> 2 OR (
        SELECT COUNT(*) FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'ai_grading_suggestions'
          AND policyname IN (
              'ai_grading_suggestion_teacher_select',
              'ai_grading_suggestion_teacher_insert'
          )
    ) <> 2 THEN
        RAISE EXCEPTION 'AI grading suggestions must have exactly the two audited teacher policies';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname = 'public' AND tablename = 'ai_grading_suggestions'
          AND cmd IN ('UPDATE', 'DELETE', 'ALL')
    ) THEN
        RAISE EXCEPTION 'AI suggestions must not allow direct review/update/delete';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'public.ai_grading_suggestions'::regclass
          AND tgname = 'ai_grading_suggestion_authority_guard'
          AND tgenabled <> 'D'
    ) THEN
        RAISE EXCEPTION 'AI grading suggestion authoritative rubric guard missing';
    END IF;

    FOREACH runtime_role IN ARRAY ARRAY['anon', 'authenticated'] LOOP
        IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = runtime_role)
           AND (
               has_table_privilege(runtime_role, 'public.ai_grading_suggestions', 'SELECT')
               OR has_table_privilege(runtime_role, 'public.ai_grading_suggestions', 'INSERT')
               OR has_table_privilege(runtime_role, 'public.ai_grading_suggestions', 'UPDATE')
               OR has_table_privilege(runtime_role, 'public.ai_grading_suggestions', 'DELETE')
           ) THEN
            RAISE EXCEPTION 'AI grading suggestion privileges leaked to %', runtime_role;
        END IF;
    END LOOP;
END $$;
SQL

# Verify the production revision trigger is installed and fails closed for an
# unknown submission, without creating or modifying production student records.
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;
CREATE TEMP TABLE ai_revision_fixture
    (LIKE public.ai_grading_suggestions INCLUDING DEFAULTS INCLUDING CONSTRAINTS);
CREATE TRIGGER ai_revision_fixture_guard
    BEFORE INSERT OR UPDATE ON ai_revision_fixture
    FOR EACH ROW EXECUTE FUNCTION edutalent_internal.guard_ai_grading_suggestion_revision();

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'public.ai_grading_suggestions'::regclass
          AND tgname = 'ai_grading_suggestion_revision_guard'
          AND tgenabled <> 'D'
    ) THEN
        RAISE EXCEPTION 'Production AI grading revision guard missing or disabled';
    END IF;

    BEGIN
        INSERT INTO ai_revision_fixture
            (submission_id, submission_revision, school_id, rubric_fingerprint,
             contract_version, grade_scale, suggested_grade, confidence,
             feedback, criteria)
        VALUES
            (gen_random_uuid(), gen_random_uuid(), gen_random_uuid(), repeat('d', 64),
             1, 100, 75, 0.7, 'stale revision fixture', '[{}]'::jsonb);
        RAISE EXCEPTION 'Expected rejection of unknown submission revision';
    EXCEPTION
        WHEN check_violation THEN NULL;
    END;
END $$;
ROLLBACK;
SQL

# Guard the teacher-approved rubric authority against accidental Data API
# exposure, new permissive role policies, or a disabled integrity trigger.
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
    external_role TEXT;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relname = 'ai_grading_rubric_approvals'
          AND c.relrowsecurity AND c.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'Teacher rubric authority must retain FORCE RLS';
    END IF;
    IF (
        SELECT COUNT(*) FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'ai_grading_rubric_approvals'
          AND policyname IN (
              'ai_grading_rubric_teacher_select',
              'ai_grading_rubric_teacher_insert',
              'ai_grading_rubric_teacher_revoke'
          )
    ) <> 3 OR (
        SELECT COUNT(*) FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'ai_grading_rubric_approvals'
    ) <> 3 THEN
        RAISE EXCEPTION 'Teacher rubric authority policies changed unexpectedly';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'public.ai_grading_rubric_approvals'::regclass
          AND tgname = 'ai_grading_rubric_approval_guard'
          AND tgenabled <> 'D'
    ) THEN
        RAISE EXCEPTION 'Teacher rubric authority guard missing or disabled';
    END IF;
    FOREACH external_role IN ARRAY ARRAY['anon', 'authenticated'] LOOP
        IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = external_role)
           AND (
               has_table_privilege(external_role, 'public.ai_grading_rubric_approvals', 'SELECT')
               OR has_table_privilege(external_role, 'public.ai_grading_rubric_approvals', 'INSERT')
               OR has_table_privilege(external_role, 'public.ai_grading_rubric_approvals', 'UPDATE')
               OR has_table_privilege(external_role, 'public.ai_grading_rubric_approvals', 'DELETE')
           ) THEN
            RAISE EXCEPTION 'Teacher rubric authority leaked privileges to %', external_role;
        END IF;
    END LOOP;
END $$;
SQL

# Durable AI grading jobs are a private work queue, never a Data API. Fail
# if direct role access, RLS policies or the atomic finalization triggers
# are accidentally introduced/disabled.
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
    external_role TEXT;
    required_trigger TEXT;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname='public' AND c.relname='ai_grading_jobs'
          AND c.relrowsecurity AND c.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'Durable AI grading job queue must retain FORCE RLS';
    END IF;
    IF (
        SELECT COUNT(*) FROM pg_policies
        WHERE schemaname='public' AND tablename='ai_grading_jobs'
    ) <> 3 OR (
        SELECT COUNT(*) FROM pg_policies
        WHERE schemaname='public' AND tablename='ai_grading_jobs'
          AND policyname IN (
              'ai_grading_jobs_owner_select',
              'ai_grading_jobs_owner_insert',
              'ai_grading_jobs_owner_update'
          )
          AND (
              COALESCE(qual, '') LIKE '%pg_get_userbyid%'
              OR COALESCE(with_check, '') LIKE '%pg_get_userbyid%'
          )
    ) <> 3 OR EXISTS (
        SELECT 1 FROM pg_policies
        WHERE schemaname='public' AND tablename='ai_grading_jobs'
          AND cmd IN ('DELETE', 'ALL')
    ) THEN
        RAISE EXCEPTION 'Private queue needs exactly three owner-scoped RLS policies';
    END IF;
    FOREACH external_role IN ARRAY ARRAY['anon', 'authenticated'] LOOP
        IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname=external_role)
            AND (
                has_table_privilege(external_role, 'public.ai_grading_jobs', 'SELECT')
                OR has_table_privilege(external_role, 'public.ai_grading_jobs', 'INSERT')
                OR has_table_privilege(external_role, 'public.ai_grading_jobs', 'UPDATE')
                OR has_table_privilege(external_role, 'public.ai_grading_jobs', 'DELETE')
            ) THEN
            RAISE EXCEPTION 'Private AI grading jobs leaked privileges to %', external_role;
        END IF;
    END LOOP;
    FOREACH required_trigger IN ARRAY ARRAY[
        'submissions_enqueue_ai_grading',
        'ai_grading_approval_enqueue'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgname=required_trigger AND NOT tgisinternal
              AND tgenabled <> 'D'
        ) THEN
            RAISE EXCEPTION 'Missing atomic AI grading enqueue trigger %', required_trigger;
        END IF;
    END LOOP;
END $$;
SQL
