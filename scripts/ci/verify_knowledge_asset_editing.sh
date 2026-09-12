#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

app_role="edutalent_app_ci"
app_password="KnowledgeEditingProbe.Password_2026_safe"

DATABASE_ADMIN_URL="${DATABASE_URL}" \
DATABASE_APP_USER="${app_role}" \
DATABASE_APP_PASSWORD="${app_password}" \
    bash scripts/ci/configure_database_role.sh

uuid() {
    psql "${DATABASE_URL}" --quiet --tuples-only --no-align --set=ON_ERROR_STOP=1 \
        -c "SELECT gen_random_uuid();" | tail -n 1
}

school_a="$(uuid)"
school_b="$(uuid)"
manager_a="$(uuid)"
manager_b="$(uuid)"
platform_admin="$(uuid)"
asset_id="$(uuid)"
source_one="$(uuid)"
source_two="$(uuid)"
ocr_revision="$(uuid)"
job_id="$(uuid)"
probe_suffix="$(uuid)"

psql "${DATABASE_URL}" \
    --quiet \
    --set=ON_ERROR_STOP=1 \
    --set=school_a="${school_a}" \
    --set=school_b="${school_b}" \
    --set=manager_a="${manager_a}" \
    --set=manager_b="${manager_b}" \
    --set=platform_admin="${platform_admin}" \
    --set=asset_id="${asset_id}" \
    --set=source_one="${source_one}" \
    --set=ocr_revision="${ocr_revision}" \
    --set=job_id="${job_id}" \
    --set=probe_suffix="${probe_suffix}" <<'SQL'
INSERT INTO schools (id, name)
VALUES
    (:'school_a', 'Knowledge edit school A ' || :'probe_suffix'),
    (:'school_b', 'Knowledge edit school B ' || :'probe_suffix');

INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
VALUES
    (:'manager_a', 'Knowledge edit manager A', 'knowledge-edit-manager-a-' || :'probe_suffix' || '@example.test',
        (SELECT id FROM roles WHERE name::text = 'SchoolManager' LIMIT 1), :'school_a', TRUE, '{}'::jsonb),
    (:'manager_b', 'Knowledge edit manager B', 'knowledge-edit-manager-b-' || :'probe_suffix' || '@example.test',
        (SELECT id FROM roles WHERE name::text = 'SchoolManager' LIMIT 1), :'school_b', TRUE, '{}'::jsonb),
    (:'platform_admin', 'Knowledge edit platform admin', 'knowledge-edit-platform-' || :'probe_suffix' || '@example.test',
        (SELECT id FROM roles WHERE name::text = 'PlatformAdmin' LIMIT 1), :'school_a', TRUE, '{}'::jsonb);

INSERT INTO knowledge_assets (
    id, school_id, title, description, source_type, status, language,
    subject, grade, tags, created_by, published_at
) VALUES (
    :'asset_id', :'school_a', 'Knowledge edit published asset', 'Original description',
    'pdf', 'published', 'en', 'Mathematics', '8', '{"scope":"fixture"}'::jsonb,
    :'manager_a', NOW()
);

BEGIN;
SET LOCAL app.user_id = :'manager_a';
SET LOCAL app.user_role = 'SchoolManager';
SET LOCAL app.school_id = :'school_a';
SET LOCAL app.elevated_operation = 'false';
INSERT INTO knowledge_source_files (
    id, asset_id, original_file_url, original_filename, mime_type,
    file_size_bytes, sha256, is_scanned_pdf
)
SELECT
    :'source_one', :'asset_id',
    'storage://edutalent-knowledge-sources/' || :'school_a' || '/' || :'source_one' || '.pdf',
    'original.pdf', 'application/pdf', octet_length(source_bytes),
    lower(encode(digest(source_bytes, 'sha256'), 'hex')), FALSE
FROM (SELECT convert_to('%PDF-1.4\noriginal knowledge source\n%%EOF\n', 'UTF8') AS source_bytes) AS bytes;
COMMIT;

BEGIN;
SET LOCAL app.user_id = :'platform_admin';
SET LOCAL app.user_role = 'PlatformAdmin';
SET LOCAL app.school_id = :'school_a';
SET LOCAL app.elevated_operation = 'false';
SELECT record_knowledge_source_review(
    :'asset_id', :'source_one',
    convert_to('%PDF-1.4\noriginal knowledge source\n%%EOF\n', 'UTF8')
);
INSERT INTO knowledge_ocr_texts (
    asset_id, raw_text, clean_text, ocr_provider, ocr_verified_by,
    text_sha256, revision
) VALUES (
    :'asset_id', 'Verified original OCR', 'Verified original OCR', 'manual',
    :'platform_admin', lower(encode(digest(convert_to('Verified original OCR', 'UTF8'), 'sha256'), 'hex')),
    :'ocr_revision'
);
COMMIT;

INSERT INTO ingestion_jobs (id, asset_id, stage, status, requested_by)
VALUES (:'job_id', :'asset_id', 'embed', 'queued', :'platform_admin');
SQL

initial_revision="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --set=ON_ERROR_STOP=1 \
    --set=asset_id="${asset_id}" -c "SELECT asset_revision FROM knowledge_assets WHERE id = :'asset_id';" | tail -n 1)"
initial_source="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --set=ON_ERROR_STOP=1 \
    --set=asset_id="${asset_id}" -c "SELECT current_source_file_id FROM knowledge_assets WHERE id = :'asset_id';" | tail -n 1)"

safe_result="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --field-separator='|' \
    --set=ON_ERROR_STOP=1 --set=app_role="${app_role}" --set=manager_a="${manager_a}" \
    --set=school_a="${school_a}" --set=asset_id="${asset_id}" --set=revision="${initial_revision}" <<'SQL' | grep -E '^[0-9]+\|' | tail -n 1
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = :'manager_a';
SET LOCAL app.user_role = 'SchoolManager';
SET LOCAL app.school_id = :'school_a';
SET LOCAL app.elevated_operation = 'false';
SELECT asset_revision, status, vectors_invalidated
FROM manager_update_knowledge_asset_metadata(
    :'asset_id', :'revision', 'Knowledge edit published asset', 'Safe presentation update',
    'Mathematics', '8', 'en', NULL, '{"scope":"fixture"}'::jsonb
);
COMMIT;
SQL
)"

IFS='|' read -r safe_revision safe_status safe_invalidated <<<"${safe_result}"
if [[ "${safe_status}" != "published" || "${safe_invalidated}" != "f" ]]; then
    echo "Safe metadata edit changed lifecycle unexpectedly: ${safe_result}" >&2
    exit 1
fi

safe_source="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --set=ON_ERROR_STOP=1 \
    --set=asset_id="${asset_id}" -c "SELECT current_source_file_id FROM knowledge_assets WHERE id = :'asset_id';" | tail -n 1)"
if [[ "${safe_source}" != "${initial_source}" ]]; then
    echo "Metadata edit replaced the governed source" >&2
    exit 1
fi

expect_role_failure() {
    local label="$1"
    local actor_id="$2"
    local school_id="$3"
    local sql="$4"
    if psql "${DATABASE_URL}" --quiet --set=ON_ERROR_STOP=1 \
        --set=app_role="${app_role}" --set=actor_id="${actor_id}" --set=school_id="${school_id}" \
        >/tmp/edutalent-knowledge-edit-denied.log 2>&1 <<SQL
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = :'actor_id';
SET LOCAL app.user_role = 'SchoolManager';
SET LOCAL app.school_id = :'school_id';
SET LOCAL app.elevated_operation = 'false';
${sql};
COMMIT;
SQL
    then
        echo "Knowledge edit boundary unexpectedly succeeded: ${label}" >&2
        cat /tmp/edutalent-knowledge-edit-denied.log >&2
        exit 1
    fi
}

expect_role_failure \
    "stale optimistic revision" "${manager_a}" "${school_a}" \
    "SELECT * FROM manager_update_knowledge_asset_metadata('${asset_id}', ${initial_revision}, 'Stale title', 'Safe presentation update', 'Mathematics', '8', 'en', NULL, '{\"scope\":\"fixture\"}'::jsonb)"

expect_role_failure \
    "cross-school manager mutation" "${manager_b}" "${school_b}" \
    "SELECT * FROM manager_update_knowledge_asset_metadata('${asset_id}', ${safe_revision}, 'Cross school', 'Safe presentation update', 'Mathematics', '8', 'en', NULL, '{\"scope\":\"fixture\"}'::jsonb)"

expect_role_failure \
    "direct manager lifecycle mutation" "${manager_a}" "${school_a}" \
    "UPDATE knowledge_assets SET status = 'archived' WHERE id = '${asset_id}'"

expect_role_failure \
    "manager verified OCR overwrite" "${manager_a}" "${school_a}" \
    "UPDATE knowledge_ocr_texts SET raw_text = 'manager overwrite', clean_text = 'manager overwrite' WHERE asset_id = '${asset_id}'"

sensitive_result="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --field-separator='|' \
    --set=ON_ERROR_STOP=1 --set=app_role="${app_role}" --set=manager_a="${manager_a}" \
    --set=school_a="${school_a}" --set=asset_id="${asset_id}" --set=revision="${safe_revision}" <<'SQL' | grep -E '^[0-9]+\|' | tail -n 1
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = :'manager_a';
SET LOCAL app.user_role = 'SchoolManager';
SET LOCAL app.school_id = :'school_a';
SET LOCAL app.elevated_operation = 'false';
SELECT asset_revision, status, vectors_invalidated
FROM manager_update_knowledge_asset_metadata(
    :'asset_id', :'revision', 'Knowledge edit published asset', 'Safe presentation update',
    'Physics', '8', 'en', NULL, '{"scope":"fixture"}'::jsonb
);
COMMIT;
SQL
)"

IFS='|' read -r sensitive_revision sensitive_status sensitive_invalidated <<<"${sensitive_result}"
if [[ "${sensitive_status}" != "ocr_ready" || "${sensitive_invalidated}" != "t" ]]; then
    echo "Retrieval-sensitive edit did not require reprocessing: ${sensitive_result}" >&2
    exit 1
fi

post_sensitive="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --field-separator='|' \
    --set=ON_ERROR_STOP=1 --set=asset_id="${asset_id}" --set=job_id="${job_id}" <<'SQL' | grep -E '^(|[a-z])' | tail -n 1
SELECT
    COALESCE(published_at::text, ''),
    (SELECT status::text FROM ingestion_jobs WHERE id = :'job_id')
FROM knowledge_assets
WHERE id = :'asset_id';
SQL
)"
IFS='|' read -r sensitive_published job_status <<<"${post_sensitive}"
if [[ -n "${sensitive_published}" || "${job_status}" != "cancelled" ]]; then
    echo "Sensitive edit did not withdraw publication/cancel work: ${post_sensitive}" >&2
    exit 1
fi

replacement_result="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --field-separator='|' \
    --set=ON_ERROR_STOP=1 --set=app_role="${app_role}" --set=manager_a="${manager_a}" \
    --set=school_a="${school_a}" --set=asset_id="${asset_id}" --set=revision="${sensitive_revision}" \
    --set=source_two="${source_two}" <<'SQL' | grep -E '^[0-9]+\|' | tail -n 1
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id = :'manager_a';
SET LOCAL app.user_role = 'SchoolManager';
SET LOCAL app.school_id = :'school_a';
SET LOCAL app.elevated_operation = 'false';
SELECT asset_revision, status, source_file_id, vectors_invalidated
FROM manager_replace_knowledge_source_revision(
    :'asset_id', :'revision',
    'storage://edutalent-knowledge-sources/' || :'school_a' || '/' || :'source_two' || '.pdf',
    'replacement.pdf', 'application/pdf', 49,
    lower(encode(digest(convert_to('%PDF-1.4\nreplacement knowledge source\n%%EOF\n', 'UTF8'), 'sha256'), 'hex')),
    NULL, FALSE
);
COMMIT;
SQL
)"
IFS='|' read -r replacement_revision replacement_status replacement_source replacement_invalidated <<<"${replacement_result}"
if [[ "${replacement_status}" != "ocr_pending" || "${replacement_source}" != "${source_two}" ]]; then
    echo "Source replacement did not advance the same logical asset: ${replacement_result}" >&2
    exit 1
fi

replacement_state="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --field-separator='|' \
    --set=ON_ERROR_STOP=1 --set=asset_id="${asset_id}" --set=ocr_revision="${ocr_revision}" <<'SQL' | grep -E '^[0-9]+\|' | tail -n 1
SELECT
    (SELECT COUNT(*) FROM knowledge_source_files WHERE asset_id = :'asset_id'),
    (SELECT COUNT(*) FROM knowledge_ocr_texts WHERE asset_id = :'asset_id'),
    (SELECT COUNT(*) FROM knowledge_ocr_revision_provenance WHERE revision = :'ocr_revision'),
    (SELECT COUNT(*) FROM knowledge_audit_logs WHERE target_id = :'asset_id' AND action = 'knowledge_asset.metadata_updated'),
    (SELECT COUNT(*) FROM knowledge_audit_logs WHERE target_id = :'asset_id' AND action = 'knowledge_asset.source_replaced');
SQL
)"
IFS='|' read -r source_count active_ocr_count provenance_count metadata_audit_count source_audit_count <<<"${replacement_state}"
if [[ "${source_count}" != "2" || "${active_ocr_count}" != "0" || "${provenance_count}" != "1" \
   || "${metadata_audit_count}" -lt 2 || "${source_audit_count}" != "1" ]]; then
    echo "Source/OCR/audit provenance invariant failed: ${replacement_state}" >&2
    exit 1
fi

# Archive through the canonical PlatformAdmin lifecycle and prove source
# replacement is terminal after archive.
psql "${DATABASE_URL}" --quiet --set=ON_ERROR_STOP=1 \
    --set=platform_admin="${platform_admin}" --set=school_a="${school_a}" --set=asset_id="${asset_id}" <<'SQL'
BEGIN;
SET LOCAL app.user_id = :'platform_admin';
SET LOCAL app.user_role = 'PlatformAdmin';
SET LOCAL app.school_id = :'school_a';
SET LOCAL app.elevated_operation = 'false';
UPDATE knowledge_assets SET status = 'archived' WHERE id = :'asset_id';
COMMIT;
SQL

archived_revision="$(psql "${DATABASE_URL}" --quiet --tuples-only --no-align --set=ON_ERROR_STOP=1 \
    --set=asset_id="${asset_id}" -c "SELECT asset_revision FROM knowledge_assets WHERE id = :'asset_id';" | tail -n 1)"
expect_role_failure \
    "archived source replacement" "${manager_a}" "${school_a}" \
    "SELECT * FROM manager_replace_knowledge_source_revision('${asset_id}', ${archived_revision}, 'storage://edutalent-knowledge-sources/${school_a}/$(uuid).pdf', 'forbidden.pdf', 'application/pdf', 10, repeat('a', 64), NULL, FALSE)"

echo "knowledge asset editing/versioning invariants verified"
