#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

app_role="edutalent_app_ci"
app_password="KnowledgeEditingProbe.Password_2026_safe"
DATABASE_ADMIN_URL="${DATABASE_URL}" DATABASE_APP_USER="${app_role}" DATABASE_APP_PASSWORD="${app_password}" \
  bash scripts/ci/configure_database_role.sh

uuid() { psql "${DATABASE_URL}" -Atqc 'SELECT gen_random_uuid();'; }
school_a="$(uuid)"; school_b="$(uuid)"; manager_a="$(uuid)"; manager_b="$(uuid)"
platform_admin="$(uuid)"; asset_id="$(uuid)"; source_one="$(uuid)"; source_two="$(uuid)"
ocr_revision="$(uuid)"; job_id="$(uuid)"; chunk_id="$(uuid)"; probe_suffix="$(uuid)"

psql "${DATABASE_URL}" --quiet --set=ON_ERROR_STOP=1 \
  --set=school_a="${school_a}" --set=school_b="${school_b}" \
  --set=manager_a="${manager_a}" --set=manager_b="${manager_b}" \
  --set=platform_admin="${platform_admin}" --set=asset_id="${asset_id}" \
  --set=source_one="${source_one}" --set=ocr_revision="${ocr_revision}" \
  --set=chunk_id="${chunk_id}" --set=job_id="${job_id}" --set=probe_suffix="${probe_suffix}" <<'SQL'
INSERT INTO schools (id, name) VALUES
  (:'school_a', 'Knowledge edit school A ' || :'probe_suffix'),
  (:'school_b', 'Knowledge edit school B ' || :'probe_suffix');
INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata) VALUES
  (:'manager_a', 'Knowledge edit manager A', 'knowledge-edit-a-' || :'probe_suffix' || '@example.test',
    (SELECT id FROM roles WHERE name::text='SchoolManager' LIMIT 1), :'school_a', TRUE, '{}'::jsonb),
  (:'manager_b', 'Knowledge edit manager B', 'knowledge-edit-b-' || :'probe_suffix' || '@example.test',
    (SELECT id FROM roles WHERE name::text='SchoolManager' LIMIT 1), :'school_b', TRUE, '{}'::jsonb),
  (:'platform_admin', 'Knowledge edit platform admin', 'knowledge-edit-admin-' || :'probe_suffix' || '@example.test',
    (SELECT id FROM roles WHERE name::text='PlatformAdmin' LIMIT 1), :'school_a', TRUE, '{}'::jsonb);
INSERT INTO knowledge_assets (
  id, school_id, title, description, source_type, status, language, subject, grade, tags, created_by
) VALUES (
  :'asset_id', :'school_a', 'Knowledge edit published asset', 'Original description', 'pdf',
  'submitted', 'en', 'Mathematics', '8', '{"scope":"fixture"}'::jsonb, :'manager_a'
);

BEGIN;
SET LOCAL app.user_id=:'manager_a'; SET LOCAL app.user_role='SchoolManager';
SET LOCAL app.school_id=:'school_a'; SET LOCAL app.elevated_operation='false';
INSERT INTO knowledge_source_files (
  id, asset_id, original_file_url, original_filename, mime_type, file_size_bytes, sha256, is_scanned_pdf
)
SELECT :'source_one', :'asset_id',
  'storage://edutalent-knowledge-sources/' || :'school_a' || '/' || :'source_one' || '.pdf',
  'original.pdf', 'application/pdf', octet_length(bytes), lower(encode(digest(bytes,'sha256'),'hex')), FALSE
FROM (SELECT convert_to('%PDF-1.4\noriginal knowledge source\n%%EOF\n','UTF8') bytes) source;
COMMIT;

BEGIN;
SET LOCAL app.user_id=:'platform_admin'; SET LOCAL app.user_role='PlatformAdmin';
SET LOCAL app.school_id=:'school_a'; SET LOCAL app.elevated_operation='false';
SELECT record_knowledge_source_review(:'asset_id', :'source_one', convert_to('%PDF-1.4\noriginal knowledge source\n%%EOF\n','UTF8'));
INSERT INTO knowledge_ocr_texts (
  asset_id, raw_text, clean_text, ocr_provider, ocr_verified_by, text_sha256, revision
) VALUES (
  :'asset_id', 'Verified original OCR', 'Verified original OCR', 'manual', :'platform_admin',
  lower(encode(digest(convert_to('Verified original OCR','UTF8'),'sha256'),'hex')), :'ocr_revision'
);
UPDATE knowledge_assets SET status='ocr_ready' WHERE id=:'asset_id';
UPDATE knowledge_assets SET status='embedding_pending' WHERE id=:'asset_id';
INSERT INTO knowledge_chunks (
  id, asset_id, chunk_index, text, token_count, embedding_provider, embedding_model, vector_id, metadata_json
) VALUES (
  :'chunk_id', :'asset_id', 0, 'Verified original OCR', 3, 'fixture', 'fixture',
  'knowledge-edit-' || :'probe_suffix', '{"fixture":true}'::jsonb
);
UPDATE knowledge_assets SET status='embedded' WHERE id=:'asset_id';
UPDATE knowledge_assets SET status='published' WHERE id=:'asset_id';
COMMIT;
INSERT INTO ingestion_jobs (id, asset_id, stage, status, requested_by)
VALUES (:'job_id', :'asset_id', 'embed', 'queued', :'platform_admin');
SQL

initial_revision="$(psql "${DATABASE_URL}" -At --set=ON_ERROR_STOP=1 \
  -c "SELECT asset_revision FROM knowledge_assets WHERE id='${asset_id}';")"
initial_source="$(psql "${DATABASE_URL}" -At --set=ON_ERROR_STOP=1 \
  -c "SELECT current_source_file_id FROM knowledge_assets WHERE id='${asset_id}';")"

manager_query() {
  local actor="$1" school="$2" sql="$3"
  psql "${DATABASE_URL}" -At --field-separator='|' --set=ON_ERROR_STOP=1 \
    --set=app_role="${app_role}" --set=actor="${actor}" --set=school="${school}" <<SQL
SET ROLE :"app_role";
BEGIN;
SET LOCAL app.user_id=:'actor'; SET LOCAL app.user_role='SchoolManager';
SET LOCAL app.school_id=:'school'; SET LOCAL app.elevated_operation='false';
${sql}
COMMIT;
SQL
}
expect_failure() {
  local label="$1" actor="$2" school="$3" sql="$4"
  if manager_query "$actor" "$school" "$sql" >/tmp/knowledge-edit-denied.log 2>&1; then
    echo "Knowledge edit boundary unexpectedly succeeded: ${label}" >&2
    cat /tmp/knowledge-edit-denied.log >&2; exit 1
  fi
}

safe_result="$(manager_query "${manager_a}" "${school_a}" "SELECT asset_revision,status,vectors_invalidated FROM manager_update_knowledge_asset_metadata('${asset_id}',${initial_revision},'Knowledge edit published asset','Safe presentation update','Mathematics','8','en',NULL,'{\"scope\":\"fixture\"}'::jsonb);" | grep -E '^[0-9]+\|' | tail -1)"
IFS='|' read -r safe_revision safe_status safe_invalidated <<<"${safe_result}"
[[ "${safe_status}" == published && "${safe_invalidated}" == f ]] || { echo "Safe edit changed lifecycle: ${safe_result}" >&2; exit 1; }
[[ "$(psql "${DATABASE_URL}" -At -c "SELECT current_source_file_id FROM knowledge_assets WHERE id='${asset_id}';")" == "${initial_source}" ]] || { echo 'Safe edit replaced source' >&2; exit 1; }

expect_failure 'stale optimistic revision' "${manager_a}" "${school_a}" "SELECT * FROM manager_update_knowledge_asset_metadata('${asset_id}',${initial_revision},'Stale','Safe presentation update','Mathematics','8','en',NULL,'{\"scope\":\"fixture\"}'::jsonb);"
expect_failure 'cross-school manager mutation' "${manager_b}" "${school_b}" "SELECT * FROM manager_update_knowledge_asset_metadata('${asset_id}',${safe_revision},'Cross school','Safe presentation update','Mathematics','8','en',NULL,'{\"scope\":\"fixture\"}'::jsonb);"
expect_failure 'direct manager lifecycle mutation' "${manager_a}" "${school_a}" "UPDATE knowledge_assets SET status='archived' WHERE id='${asset_id}';"
expect_failure 'manager verified OCR overwrite' "${manager_a}" "${school_a}" "UPDATE knowledge_ocr_texts SET raw_text='manager overwrite',clean_text='manager overwrite' WHERE asset_id='${asset_id}';"

sensitive_result="$(manager_query "${manager_a}" "${school_a}" "SELECT asset_revision,status,vectors_invalidated FROM manager_update_knowledge_asset_metadata('${asset_id}',${safe_revision},'Knowledge edit published asset','Safe presentation update','Physics','8','en',NULL,'{\"scope\":\"fixture\"}'::jsonb);" | grep -E '^[0-9]+\|' | tail -1)"
IFS='|' read -r sensitive_revision sensitive_status sensitive_invalidated <<<"${sensitive_result}"
[[ "${sensitive_status}" == ocr_ready && "${sensitive_invalidated}" == t ]] || { echo "Sensitive edit did not require reprocessing: ${sensitive_result}" >&2; exit 1; }
post_sensitive="$(psql "${DATABASE_URL}" -At --field-separator='|' \
  -c "SELECT COALESCE(published_at::text,''),(SELECT status::text FROM ingestion_jobs WHERE id='${job_id}'),(SELECT count(*) FROM knowledge_chunks WHERE asset_id='${asset_id}') FROM knowledge_assets WHERE id='${asset_id}';")"
IFS='|' read -r published job_status chunk_count <<<"${post_sensitive}"
[[ -z "${published}" && "${job_status}" == cancelled && "${chunk_count}" == 0 ]] || { echo "Sensitive invalidation failed: ${post_sensitive}" >&2; exit 1; }

replacement_result="$(manager_query "${manager_a}" "${school_a}" "SELECT asset_revision,status,source_file_id,vectors_invalidated FROM manager_replace_knowledge_source_revision('${asset_id}',${sensitive_revision},'storage://edutalent-knowledge-sources/${school_a}/${source_two}.pdf','replacement.pdf','application/pdf',49,lower(encode(digest(convert_to('%PDF-1.4\\nreplacement knowledge source\\n%%EOF\\n','UTF8'),'sha256'),'hex')),NULL,FALSE);" | grep -E '^[0-9]+\|' | tail -1)"
IFS='|' read -r replacement_revision replacement_status replacement_source replacement_invalidated <<<"${replacement_result}"
[[ "${replacement_status}" == ocr_pending && "${replacement_source}" == "${source_two}" ]] || { echo "Source replacement failed: ${replacement_result}" >&2; exit 1; }

state="$(psql "${DATABASE_URL}" -At --field-separator='|' \
  -c "SELECT (SELECT count(*) FROM knowledge_source_files WHERE asset_id='${asset_id}'),(SELECT count(*) FROM knowledge_ocr_texts WHERE asset_id='${asset_id}'),(SELECT count(*) FROM knowledge_ocr_revision_provenance WHERE revision='${ocr_revision}'),(SELECT count(*) FROM knowledge_audit_logs WHERE target_id='${asset_id}' AND action='knowledge_asset.metadata_updated'),(SELECT count(*) FROM knowledge_audit_logs WHERE target_id='${asset_id}' AND action='knowledge_asset.source_replaced');")"
IFS='|' read -r source_count active_ocr provenance metadata_audits source_audits <<<"${state}"
[[ "${source_count}" == 2 && "${active_ocr}" == 0 && "${provenance}" == 1 && "${metadata_audits}" -ge 2 && "${source_audits}" == 1 ]] || { echo "Source/OCR/audit invariant failed: ${state}" >&2; exit 1; }

psql "${DATABASE_URL}" --quiet --set=ON_ERROR_STOP=1 --set=platform_admin="${platform_admin}" --set=school_a="${school_a}" --set=asset_id="${asset_id}" <<'SQL'
BEGIN;
SET LOCAL app.user_id=:'platform_admin'; SET LOCAL app.user_role='PlatformAdmin';
SET LOCAL app.school_id=:'school_a'; SET LOCAL app.elevated_operation='false';
UPDATE knowledge_assets SET status='archived' WHERE id=:'asset_id';
COMMIT;
SQL
archived_revision="$(psql "${DATABASE_URL}" -At \
  -c "SELECT asset_revision FROM knowledge_assets WHERE id='${asset_id}';")"
expect_failure 'archived source replacement' "${manager_a}" "${school_a}" "SELECT * FROM manager_replace_knowledge_source_revision('${asset_id}',${archived_revision},'storage://edutalent-knowledge-sources/${school_a}/$(uuid).pdf','forbidden.pdf','application/pdf',10,repeat('a',64),NULL,FALSE);"

echo 'knowledge asset editing/versioning invariants verified'
