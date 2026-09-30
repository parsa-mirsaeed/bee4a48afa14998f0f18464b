-- PR-12 synthetic tenant fixture (plan §8). Deterministic IDs; no real data.
-- Loaded into the local CI PostgreSQL before the production-like server starts.
--
-- Schools A and B, a PlatformAdmin, managers/teachers/students/parents for both
-- schools, active and inactive accounts, classes, published assignments,
-- submissions/grades, and published knowledge assets. Separate desktop/mobile
-- assignment IDs keep stateful acceptance deterministic across Playwright projects.

BEGIN;

INSERT INTO schools (id, name) VALUES
  ('a0000000-0000-0000-0000-0000000000a1', 'E2E School A'),
  ('a0000000-0000-0000-0000-0000000000b1', 'E2E School B')
ON CONFLICT (id) DO NOTHING;

INSERT INTO users (id, name, email, role_id, school_id, is_active) VALUES
  ('b0000000-0000-0000-0000-0000000000a0', 'E2E Platform Admin',  'e2e-admin@example.test',    (SELECT id FROM roles WHERE name = 'PlatformAdmin'),  'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000a1', 'E2E Manager A',      'e2e-manager-a@example.test',(SELECT id FROM roles WHERE name = 'SchoolManager'), 'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b1', 'E2E Manager B',      'e2e-manager-b@example.test',(SELECT id FROM roles WHERE name = 'SchoolManager'), 'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a2', 'E2E Teacher A',      'e2e-teacher-a@example.test',(SELECT id FROM roles WHERE name = 'Teacher'),       'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b2', 'E2E Teacher B',      'e2e-teacher-b@example.test',(SELECT id FROM roles WHERE name = 'Teacher'),       'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a3', 'E2E Student A',      'e2e-student-a@example.test',(SELECT id FROM roles WHERE name = 'Student'),       'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b3', 'E2E Student B',      'e2e-student-b@example.test',(SELECT id FROM roles WHERE name = 'Student'),       'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a4', 'E2E Parent A',       'e2e-parent-a@example.test', (SELECT id FROM roles WHERE name = 'Parent'),        'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b4', 'E2E Parent B',       'e2e-parent-b@example.test', (SELECT id FROM roles WHERE name = 'Parent'),        'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a5', 'E2E Parent Empty',   'e2e-parent-empty@example.test', (SELECT id FROM roles WHERE name = 'Parent'),     'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000a9', 'E2E Inactive Teacher','e2e-inactive@example.test',(SELECT id FROM roles WHERE name = 'Teacher'),       'a0000000-0000-0000-0000-0000000000a1', false)
ON CONFLICT (id) DO NOTHING;

INSERT INTO teachers (id, user_id, school_id) VALUES
  ('c0000000-0000-0000-0000-0000000000a2', 'b0000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-0000000000a1'),
  ('c0000000-0000-0000-0000-0000000000b2', 'b0000000-0000-0000-0000-0000000000b2', 'a0000000-0000-0000-0000-0000000000b1')
ON CONFLICT (id) DO NOTHING;

INSERT INTO students (id, user_id, school_id, parent_id) VALUES
  ('c0000000-0000-0000-0000-0000000000a3', 'b0000000-0000-0000-0000-0000000000a3', 'a0000000-0000-0000-0000-0000000000a1', 'b0000000-0000-0000-0000-0000000000a4'),
  ('c0000000-0000-0000-0000-0000000000b3', 'b0000000-0000-0000-0000-0000000000b3', 'a0000000-0000-0000-0000-0000000000b1', 'b0000000-0000-0000-0000-0000000000b4')
ON CONFLICT (id) DO NOTHING;

INSERT INTO subjects (id, code, name) VALUES
  ('d0000000-0000-0000-0000-0000000000a1', 'E2EMATH', 'E2E Mathematics')
ON CONFLICT (id) DO NOTHING;

INSERT INTO class_sections (id, school_id, subject_id, name, term) VALUES
  ('e0000000-0000-0000-0000-0000000000a1', 'a0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Class A1', 'E2E-Term'),
  ('e0000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Empty Class A', 'E2E-Term'),
  ('e0000000-0000-0000-0000-0000000000b1', 'a0000000-0000-0000-0000-0000000000b1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Class B1', 'E2E-Term')
ON CONFLICT (id) DO NOTHING;

INSERT INTO teaching_assignments (class_section_id, teacher_id) VALUES
  ('e0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a2'),
  ('e0000000-0000-0000-0000-0000000000a2', 'c0000000-0000-0000-0000-0000000000a2'),
  ('e0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b2')
ON CONFLICT DO NOTHING;

INSERT INTO enrollments (class_section_id, student_id) VALUES
  ('e0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a3'),
  ('e0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b3')
ON CONFLICT DO NOTHING;

INSERT INTO class_materials (
  id, class_section_id, title, description, material_type, is_required,
  display_order, created_by, created_at, updated_at
) VALUES (
  'f5000000-0000-0000-0000-0000000000a1',
  'e0000000-0000-0000-0000-0000000000a1',
  'E2E Class Material A1',
  'Synthetic class material for localized date acceptance.',
  'other',
  false,
  0,
  'b0000000-0000-0000-0000-0000000000a2',
  TIMESTAMPTZ '2026-09-10 13:45:00+00',
  TIMESTAMPTZ '2026-09-10 13:45:00+00'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO assignments (id, teacher_id, class_section_id, subject_id, title, body, due_at, status, published_at) VALUES
  ('f0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Assignment A1', 'Synthetic body', NOW() + INTERVAL '7 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a2', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Submission Journey Desktop', 'Desktop stateful acceptance body', NOW() + INTERVAL '8 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a3', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Submission Journey Mobile', 'Mobile stateful acceptance body', NOW() + INTERVAL '9 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a4', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Authorization Submission A', 'School A authorization probe', NOW() + INTERVAL '10 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a5', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a2', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Guided Publish Draft', 'Must remain draft until an active student is enrolled.', NOW() + INTERVAL '11 days', 'Draft', NULL),
  ('f0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b2', 'e0000000-0000-0000-0000-0000000000b1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Authorization Submission B', 'School B authorization probe', NOW() + INTERVAL '10 days', 'Published', NOW())
ON CONFLICT (id) DO NOTHING;

INSERT INTO custom_assignments (
  id, assignment_id, student_id, due_at, status, submitted_at, graded_at
) VALUES
  ('f1000000-0000-0000-0000-0000000000a1', 'f0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '7 days', 'Graded', NOW() - INTERVAL '2 days', NOW() - INTERVAL '1 day'),
  ('f1000000-0000-0000-0000-0000000000a2', 'f0000000-0000-0000-0000-0000000000a2', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '8 days', 'Assigned', NULL, NULL),
  ('f1000000-0000-0000-0000-0000000000a3', 'f0000000-0000-0000-0000-0000000000a3', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '9 days', 'Assigned', NULL, NULL),
  ('f1000000-0000-0000-0000-0000000000a4', 'f0000000-0000-0000-0000-0000000000a4', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '10 days', 'Submitted', NOW() - INTERVAL '1 day', NULL),
  ('f1000000-0000-0000-0000-0000000000b1', 'f0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b3', NOW() + INTERVAL '10 days', 'Submitted', NOW() - INTERVAL '1 day', NULL)
ON CONFLICT DO NOTHING;

-- Existing browser journeys represent assignments whose personalization is
-- already ready. Persist a small deterministic payload so the delivery-truth
-- contract does not turn unrelated submission/localization acceptance into a
-- synthetic AI outage.
UPDATE custom_assignments AS custom_assignment
SET prompt_ctx = jsonb_build_object(
    'personalized_assignment',
    jsonb_build_object(
      'title', assignment.title,
      'body', assignment.body,
      'scope', jsonb_build_object(
        'type', 'default',
        'deliverables', '[]'::jsonb
      ),
      'estimated_difficulty', 'medium',
      'personalization_notes', 'E2E deterministic personalized fixture'
    )
)
FROM assignments AS assignment
WHERE assignment.id = custom_assignment.assignment_id
  AND custom_assignment.id IN (
    'f1000000-0000-0000-0000-0000000000a1',
    'f1000000-0000-0000-0000-0000000000a2',
    'f1000000-0000-0000-0000-0000000000a3',
    'f1000000-0000-0000-0000-0000000000a4',
    'f1000000-0000-0000-0000-0000000000b1'
  );

UPDATE assignment_personalization_jobs
SET status = 'succeeded',
    processing_stage = 'ready',
    completed_at = COALESCE(completed_at, NOW()),
    last_error_code = NULL,
    last_error_summary = NULL
WHERE assignment_id IN (
  'f0000000-0000-0000-0000-0000000000a1',
  'f0000000-0000-0000-0000-0000000000a2',
  'f0000000-0000-0000-0000-0000000000a3',
  'f0000000-0000-0000-0000-0000000000a4',
  'f0000000-0000-0000-0000-0000000000b1'
);

-- Dedicated truth-state fixtures. These are intentionally separate from the
-- existing stateful submission journeys.
INSERT INTO assignments (
  id, teacher_id, class_section_id, subject_id, title, body, due_at, status, published_at
) VALUES
  (
    'f0000000-0000-0000-0000-0000000000c1',
    'c0000000-0000-0000-0000-0000000000a2',
    'e0000000-0000-0000-0000-0000000000a1',
    'd0000000-0000-0000-0000-0000000000a1',
    'E2E Personalization Preparing',
    'PREPARING SOURCE MUST NOT BE SHOWN',
    NOW() + INTERVAL '2 days',
    'Published',
    NOW()
  ),
  (
    'f0000000-0000-0000-0000-0000000000c2',
    'c0000000-0000-0000-0000-0000000000a2',
    'e0000000-0000-0000-0000-0000000000a1',
    'd0000000-0000-0000-0000-0000000000a1',
    'E2E Personalization Unavailable',
    'UNAVAILABLE SOURCE MUST NOT BE SHOWN',
    NOW() + INTERVAL '3 days',
    'Published',
    NOW()
  ),
  (
    'f0000000-0000-0000-0000-0000000000c3',
    'c0000000-0000-0000-0000-0000000000a2',
    'e0000000-0000-0000-0000-0000000000a1',
    'd0000000-0000-0000-0000-0000000000a1',
    'E2E Personalization Fallback',
    'E2E FALLBACK ORIGINAL CONTENT',
    NOW() + INTERVAL '4 days',
    'Published',
    NOW()
  ),
  (
    'f0000000-0000-0000-0000-0000000000c4',
    'c0000000-0000-0000-0000-0000000000a2',
    'e0000000-0000-0000-0000-0000000000a1',
    'd0000000-0000-0000-0000-0000000000a1',
    'E2E Personalization Running',
    'RUNNING SOURCE MUST NOT BE SHOWN',
    NOW() + INTERVAL '5 days',
    'Published',
    NOW()
  ),
  (
    'f0000000-0000-0000-0000-0000000000c5',
    'c0000000-0000-0000-0000-0000000000a2',
    'e0000000-0000-0000-0000-0000000000a1',
    'd0000000-0000-0000-0000-0000000000a1',
    'E2E Personalization Source Ready',
    'READY SOURCE MUST NOT BE SHOWN',
    NOW() + INTERVAL '6 days',
    'Published',
    NOW()
  )
ON CONFLICT (id) DO NOTHING;

-- Preparing and unavailable jobs snapshot the default require-personalized
-- school policy.
INSERT INTO custom_assignments (
  id, assignment_id, student_id, due_at, status
) VALUES
  (
    'f1000000-0000-0000-0000-0000000000c1',
    'f0000000-0000-0000-0000-0000000000c1',
    'c0000000-0000-0000-0000-0000000000a3',
    NOW() + INTERVAL '2 days',
    'Assigned'
  ),
  (
    'f1000000-0000-0000-0000-0000000000c2',
    'f0000000-0000-0000-0000-0000000000c2',
    'c0000000-0000-0000-0000-0000000000a3',
    NOW() + INTERVAL '3 days',
    'Assigned'
  ),
  (
    'f1000000-0000-0000-0000-0000000000c4',
    'f0000000-0000-0000-0000-0000000000c4',
    'c0000000-0000-0000-0000-0000000000a3',
    NOW() + INTERVAL '5 days',
    'Assigned'
  ),
  (
    'f1000000-0000-0000-0000-0000000000c5',
    'f0000000-0000-0000-0000-0000000000c5',
    'c0000000-0000-0000-0000-0000000000a3',
    NOW() + INTERVAL '6 days',
    'Assigned'
  )
ON CONFLICT DO NOTHING;

-- Keep the queued fixture deterministic while the real worker is running:
-- it remains truthfully queued but is not eligible for claim during browser
-- acceptance.
UPDATE assignment_personalization_jobs
SET available_at = NOW() + INTERVAL '1 day'
WHERE assignment_id = 'f0000000-0000-0000-0000-0000000000c1';

UPDATE assignment_personalization_jobs
SET status = 'failed',
    processing_stage = 'failed',
    completed_at = NOW(),
    last_error_code = 'gateway_unavailable',
    last_error_summary = 'AI gateway is temporarily unavailable'
WHERE assignment_id = 'f0000000-0000-0000-0000-0000000000c2';

UPDATE assignment_personalization_jobs
SET status = 'running',
    processing_stage = 'retrieving_context',
    attempt_count = 1,
    started_at = NOW() - INTERVAL '10 seconds',
    lease_owner = 'f9000000-0000-0000-0000-000000000001',
    -- Keep this synthetic running state outside stale-recovery during a long
    -- browser suite without disabling the real worker.
    heartbeat_at = NOW() + INTERVAL '1 hour',
    completed_at = NULL,
    last_error_code = NULL,
    last_error_summary = NULL
WHERE assignment_id = 'f0000000-0000-0000-0000-0000000000c4';

UPDATE custom_assignments
SET prompt_ctx = jsonb_build_object(
    'personalized_assignment',
    jsonb_build_object(
      'title', 'E2E Personalized Ready',
      'body', 'E2E GENERATED PERSONALIZED CONTENT',
      'scope', jsonb_build_object(
        'type', 'default',
        'deliverables', '[]'::jsonb
      ),
      'estimated_difficulty', 'medium',
      'personalization_notes', 'E2E generated personalization ready fixture'
    )
)
WHERE id = 'f1000000-0000-0000-0000-0000000000c5';

UPDATE assignment_personalization_jobs
SET status = 'succeeded',
    processing_stage = 'ready',
    completed_at = NOW(),
    lease_owner = NULL,
    heartbeat_at = NULL,
    last_error_code = NULL,
    last_error_summary = NULL,
    generated_content_changed = TRUE
WHERE assignment_id = 'f0000000-0000-0000-0000-0000000000c5';

-- Snapshot fallback as an explicit policy decision at enqueue time, then return
-- the school default to require-personalized for subsequent fixtures.
UPDATE assignment_personalization_school_policies
SET delivery_policy = 'allow_original_fallback'
WHERE school_id = 'a0000000-0000-0000-0000-0000000000a1';

INSERT INTO custom_assignments (
  id, assignment_id, student_id, due_at, status
) VALUES (
  'f1000000-0000-0000-0000-0000000000c3',
  'f0000000-0000-0000-0000-0000000000c3',
  'c0000000-0000-0000-0000-0000000000a3',
  NOW() + INTERVAL '4 days',
  'Assigned'
)
ON CONFLICT DO NOTHING;

UPDATE assignment_personalization_jobs
SET status = 'failed',
    processing_stage = 'failed',
    completed_at = NOW(),
    last_error_code = 'provider_unconfigured',
    last_error_summary = 'AI personalization provider is not configured'
WHERE assignment_id = 'f0000000-0000-0000-0000-0000000000c3';

UPDATE assignment_personalization_school_policies
SET delivery_policy = 'require_personalized'
WHERE school_id = 'a0000000-0000-0000-0000-0000000000a1';

INSERT INTO submissions (
  id, custom_assignment_id, student_id, content, grade, grade_scale, graded_by,
  submitted_at
) VALUES
  ('f2000000-0000-0000-0000-0000000000a1', 'f1000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a3', '{"text":"synthetic"}'::jsonb, 18.00, 20, 'c0000000-0000-0000-0000-0000000000a2', NOW() - INTERVAL '2 days'),
  ('f2000000-0000-0000-0000-0000000000a4', 'f1000000-0000-0000-0000-0000000000a4', 'c0000000-0000-0000-0000-0000000000a3', '{"text":"school-a authorization submission"}'::jsonb, NULL, 100, NULL, NOW() - INTERVAL '1 day'),
  ('f2000000-0000-0000-0000-0000000000b1', 'f1000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b3', '{"text":"school-b authorization submission"}'::jsonb, NULL, 100, NULL, NOW() - INTERVAL '1 day')
ON CONFLICT (id) DO NOTHING;

INSERT INTO knowledge_assets (id, school_id, title, status, created_by, published_at) VALUES
  ('f3000000-0000-0000-0000-0000000000a1', 'a0000000-0000-0000-0000-0000000000a1', 'E2E Published Asset', 'published', 'b0000000-0000-0000-0000-0000000000a1', NOW()),
  ('f3000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-0000000000a1', 'E2E Verified OCR Asset', 'ocr_ready', 'b0000000-0000-0000-0000-0000000000a1', NULL),
  ('f3000000-0000-0000-0000-0000000000b1', 'a0000000-0000-0000-0000-0000000000b1', 'E2E School B Asset', 'published', 'b0000000-0000-0000-0000-0000000000b1', NOW())
ON CONFLICT (id) DO NOTHING;

-- The browser environment intentionally has no matching private Storage object.
-- We still seed a legitimate historical review using synthetic bytes so OCR is
-- valid, while clicking Review source proves the live missing-object path is
-- rendered as bounded EduTalent UI rather than raw provider output.
SELECT set_config('app.user_id', 'b0000000-0000-0000-0000-0000000000a0', true);
SELECT set_config('app.user_role', 'PlatformAdmin', true);
SELECT set_config('app.school_id', 'a0000000-0000-0000-0000-0000000000a1', true);

WITH source_bytes AS (
  SELECT convert_to('%PDF-e2e-reviewed-source', 'UTF8') AS bytes
)
INSERT INTO knowledge_source_files (
  id, asset_id, original_file_url, original_filename, mime_type,
  file_size_bytes, sha256, is_scanned_pdf
)
SELECT
  'f4000000-0000-0000-0000-0000000000a2',
  'f3000000-0000-0000-0000-0000000000a2',
  'storage://edutalent-knowledge-sources/a0000000-0000-0000-0000-0000000000a1/f4000000-0000-0000-0000-0000000000a2.pdf',
  'e2e-reviewed-source.pdf',
  'application/pdf',
  octet_length(bytes),
  lower(encode(digest(bytes, 'sha256'), 'hex')),
  FALSE
FROM source_bytes
ON CONFLICT (id) DO NOTHING;

SELECT record_knowledge_source_review(
  'f3000000-0000-0000-0000-0000000000a2',
  'f4000000-0000-0000-0000-0000000000a2',
  convert_to('%PDF-e2e-reviewed-source', 'UTF8')
);

INSERT INTO knowledge_ocr_texts (
  asset_id, raw_text, clean_text, ocr_provider, ocr_verified_by, text_sha256
) VALUES (
  'f3000000-0000-0000-0000-0000000000a2',
  'E2E preverified OCR text',
  'E2E preverified OCR text',
  'e2e-manual-review',
  'b0000000-0000-0000-0000-0000000000a0',
  repeat('a', 64)
)
ON CONFLICT (asset_id) DO NOTHING;

-- Dedicated read-only accessibility fixture: never submitted by other journeys.
-- Its nearer deadline preserves the five-row Teacher overview baseline.
INSERT INTO assignments (
  id, teacher_id, class_section_id, subject_id, title, body, due_at, status, published_at
) VALUES (
  'f0000000-0000-0000-0000-0000000000a6',
  'c0000000-0000-0000-0000-0000000000a2',
  'e0000000-0000-0000-0000-0000000000a1',
  'd0000000-0000-0000-0000-0000000000a1',
  'E2E Accessibility Pending', 'Read-only editor accessibility acceptance',
  NOW() + INTERVAL '1 day', 'Published', NOW()
) ON CONFLICT (id) DO NOTHING;

INSERT INTO custom_assignments (
  id, assignment_id, student_id, due_at, status, assigned_at
) VALUES (
  'f1000000-0000-0000-0000-0000000000a6',
  'f0000000-0000-0000-0000-0000000000a6',
  'c0000000-0000-0000-0000-0000000000a3',
  NOW() + INTERVAL '1 day', 'Assigned', NOW()
) ON CONFLICT (id) DO NOTHING;
UPDATE custom_assignments AS custom_assignment
SET prompt_ctx = jsonb_build_object(
  'personalized_assignment',
  jsonb_build_object(
    'title', assignment.title,
    'body', assignment.body,
    'scope', jsonb_build_object('type', 'default', 'deliverables', '[]'::jsonb),
    'estimated_difficulty', 'medium',
    'personalization_notes', 'E2E deterministic accessibility fixture'
  )
)
FROM assignments AS assignment
WHERE custom_assignment.id = 'f1000000-0000-0000-0000-0000000000a6'
  AND assignment.id = custom_assignment.assignment_id;

UPDATE assignment_personalization_jobs
SET status = 'succeeded',
    processing_stage = 'ready',
    completed_at = COALESCE(completed_at, NOW()),
    lease_owner = NULL,
    heartbeat_at = NULL,
    last_error_code = NULL,
    last_error_summary = NULL
WHERE assignment_id = 'f0000000-0000-0000-0000-0000000000a6';

-- The deterministic fixture must itself obey the application state model. Keep
-- these assertions beside the seed so an invalid baseline fails before a
-- browser journey can disguise it as a product regression.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM custom_assignments
    WHERE status IN ('Submitted'::custom_status, 'Graded'::custom_status)
      AND submitted_at IS NULL
  ) THEN
    RAISE EXCEPTION 'E2E invariant: Submitted/Graded custom assignments require submitted_at';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM custom_assignments
    WHERE status = 'Graded'::custom_status
      AND (graded_at IS NULL OR graded_at < submitted_at)
  ) THEN
    RAISE EXCEPTION 'E2E invariant: Graded custom assignments require coherent graded_at';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM submissions s
    JOIN custom_assignments ca ON ca.id = s.custom_assignment_id
    WHERE ca.status IN ('Submitted'::custom_status, 'Graded'::custom_status)
      AND s.submitted_at IS NULL
  ) THEN
    RAISE EXCEPTION 'E2E invariant: persisted Submitted/Graded work requires submission submitted_at';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM custom_assignments ca
    LEFT JOIN submissions s ON s.custom_assignment_id = ca.id
    WHERE ca.status = 'Graded'::custom_status
      AND (s.id IS NULL OR s.grade IS NULL OR s.submitted_at IS NULL)
  ) THEN
    RAISE EXCEPTION 'E2E invariant: Graded custom assignments require persisted graded submissions';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM custom_assignments ca
    LEFT JOIN submissions s ON s.custom_assignment_id = ca.id
    WHERE ca.status IN ('Submitted'::custom_status, 'Graded'::custom_status)
      AND s.id IS NULL
  ) THEN
    RAISE EXCEPTION 'E2E invariant: Submitted/Graded custom assignments require a persisted submission';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM custom_assignments ca
    LEFT JOIN submissions s ON s.custom_assignment_id = ca.id
    WHERE ca.id IN (
      'f1000000-0000-0000-0000-0000000000a2'::uuid,
      'f1000000-0000-0000-0000-0000000000a3'::uuid
    )
      AND (ca.status <> 'Assigned'::custom_status OR ca.submitted_at IS NOT NULL OR ca.graded_at IS NOT NULL OR s.id IS NOT NULL)
  ) THEN
    RAISE EXCEPTION 'E2E invariant: desktop/mobile submission journeys must start Assigned with no submission';
  END IF;

  IF EXISTS (
    SELECT 1 FROM enrollments
    WHERE class_section_id = 'e0000000-0000-0000-0000-0000000000a2'::uuid
  ) THEN
    RAISE EXCEPTION 'E2E invariant: guided-publish class must start empty';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM custom_assignments ca
    LEFT JOIN submissions s ON s.custom_assignment_id = ca.id
    WHERE ca.assignment_id = 'f0000000-0000-0000-0000-0000000000a5'::uuid
      AND (ca.id IS NOT NULL OR s.id IS NOT NULL)
  ) THEN
    RAISE EXCEPTION 'E2E invariant: guided draft must have no generated custom assignment or submission';
  END IF;

  IF EXISTS (
    SELECT 1 FROM users
    WHERE email IN (
      'e2e-pr1-student@example.test',
      'e2e-pr1-teacher@example.test',
      'e2e-pr1-parent@example.test'
    )
  ) THEN
    RAISE EXCEPTION 'E2E invariant: browser-created e2e-pr1 accounts leaked into fresh baseline';
  END IF;
END
$$;

-- Dedicated attachment actors keep other role journeys' lists/counts independent.
INSERT INTO schools(id,name) VALUES('a0000000-0000-0000-0000-0000000000c1','E2E Attachment School') ON CONFLICT DO NOTHING;
INSERT INTO users(id,name,email,role_id,school_id,is_active) VALUES
 ('b0000000-0000-0000-0000-0000000000c2','E2E Attachment Teacher','e2e-attachment-teacher@example.test',(SELECT id FROM roles WHERE name='Teacher'),'a0000000-0000-0000-0000-0000000000c1',true),
 ('b0000000-0000-0000-0000-0000000000c3','E2E Attachment Student','e2e-attachment-student@example.test',(SELECT id FROM roles WHERE name='Student'),'a0000000-0000-0000-0000-0000000000c1',true)
ON CONFLICT DO NOTHING;
INSERT INTO teachers(id,user_id,school_id) VALUES('c0000000-0000-0000-0000-0000000000c2','b0000000-0000-0000-0000-0000000000c2','a0000000-0000-0000-0000-0000000000c1') ON CONFLICT DO NOTHING;
INSERT INTO students(id,user_id,school_id) VALUES('c0000000-0000-0000-0000-0000000000c3','b0000000-0000-0000-0000-0000000000c3','a0000000-0000-0000-0000-0000000000c1') ON CONFLICT DO NOTHING;
INSERT INTO class_sections(id,school_id,subject_id,name,term) VALUES('e0000000-0000-0000-0000-0000000000c1','a0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment Class','E2E-Term') ON CONFLICT DO NOTHING;
INSERT INTO teaching_assignments(class_section_id,teacher_id) VALUES('e0000000-0000-0000-0000-0000000000c1','c0000000-0000-0000-0000-0000000000c2') ON CONFLICT DO NOTHING;
INSERT INTO enrollments(class_section_id,student_id) VALUES('e0000000-0000-0000-0000-0000000000c1','c0000000-0000-0000-0000-0000000000c3') ON CONFLICT DO NOTHING;
-- Independent file-submission journeys: per-project and per-language IDs.
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000101','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 101','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000101','f0600000-0000-0000-0000-000000000101','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000102','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 102','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000102','f0600000-0000-0000-0000-000000000102','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000103','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 103','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000103','f0600000-0000-0000-0000-000000000103','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000104','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 104','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000104','f0600000-0000-0000-0000-000000000104','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000190','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 190','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000190','f0600000-0000-0000-0000-000000000190','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000201','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 201','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000201','f0600000-0000-0000-0000-000000000201','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000202','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 202','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000202','f0600000-0000-0000-0000-000000000202','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000203','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 203','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000203','f0600000-0000-0000-0000-000000000203','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000204','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 204','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000204','f0600000-0000-0000-0000-000000000204','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;
INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,due_at,status,published_at) VALUES('f0600000-0000-0000-0000-000000000290','c0000000-0000-0000-0000-0000000000c2','e0000000-0000-0000-0000-0000000000c1','d0000000-0000-0000-0000-0000000000a1','E2E Attachment 290','Submit your work.',NOW()+INTERVAL '14 days','Published',NOW()) ON CONFLICT DO NOTHING;
INSERT INTO custom_assignments(id,assignment_id,student_id,due_at,status) VALUES('f1600000-0000-0000-0000-000000000290','f0600000-0000-0000-0000-000000000290','c0000000-0000-0000-0000-0000000000c3',NOW()+INTERVAL '14 days','Assigned') ON CONFLICT DO NOTHING;

-- Attachment acceptance exercises submission/storage, not AI availability.
-- Snapshot deterministic ready content so delivery truth does not block
-- unrelated attachment journeys while the browser gateway is offline.
UPDATE custom_assignments AS custom_assignment
SET prompt_ctx = jsonb_build_object(
  'personalized_assignment',
  jsonb_build_object(
    'title', assignment.title,
    'body', assignment.body,
    'scope', jsonb_build_object('type', 'default', 'deliverables', '[]'::jsonb),
    'estimated_difficulty', 'medium',
    'personalization_notes', 'E2E deterministic attachment fixture'
  )
)
FROM assignments AS assignment
JOIN class_sections AS class_section
  ON class_section.id = assignment.class_section_id
WHERE assignment.id = custom_assignment.assignment_id
  AND class_section.school_id = 'a0000000-0000-0000-0000-0000000000c1';

UPDATE assignment_personalization_jobs
SET status = 'succeeded',
    processing_stage = 'ready',
    completed_at = COALESCE(completed_at, NOW()),
    lease_owner = NULL,
    heartbeat_at = NULL,
    last_error_code = NULL,
    last_error_summary = NULL
WHERE school_id = 'a0000000-0000-0000-0000-0000000000c1';

COMMIT;
