#!/usr/bin/env bash
# PR-12 harness self-check: the harness must be pinned, present, reset-safe, and
# internally consistent before any browser journey is trusted. Fails closed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

test -f tests/e2e/package.json
test -f tests/e2e/playwright.config.ts
test -f tests/e2e/fixtures/seed.sql
test -f tests/e2e/fixtures/mock-idp.mjs
test -f tests/e2e/fixtures/network-policy.ts
test -f tests/e2e/fixtures/console-guard.ts
test -f scripts/ci/reset_browser_fixture_db.sh
test -d tests/e2e/specs

! grep -R --line-number -E '"(@playwright/test|@axe-core/playwright|playwright)"[[:space:]]*:[[:space:]]*"(latest|\*|\^|~)' tests/e2e/package.json
grep -q '"@playwright/test": "1.49.1"' tests/e2e/package.json
grep -q '"@axe-core/playwright": "4.10.2"' tests/e2e/package.json
grep -q 'E2E_ALLOWED_ORIGINS' tests/e2e/playwright.config.ts

grep -q "alg: 'ES256'" tests/e2e/fixtures/mock-idp.mjs
grep -q "kid: KID" tests/e2e/fixtures/mock-idp.mjs
grep -q "email," tests/e2e/fixtures/mock-idp.mjs
grep -q 'SUPABASE_JWT_ISSUER="http://127.0.0.1:9100/auth/v1"' scripts/ci/run_browser_e2e.sh

grep -q 'dx bundle --web --release --package web' scripts/ci/run_browser_e2e.sh
grep -q 'target/dx/web/release/web' scripts/ci/run_browser_e2e.sh
grep -q 'current_database.*!=.*edutalent_ci' scripts/ci/reset_browser_fixture_db.sh
grep -q 'E2E_ALLOW_FIXTURE_RESET' scripts/ci/reset_browser_fixture_db.sh
grep -q 'standalone ci/e2e/test marker' scripts/ci/reset_browser_fixture_db.sh
grep -q 'DROP SCHEMA IF EXISTS public CASCADE' scripts/ci/reset_browser_fixture_db.sh

reset_line="$(grep -n 'bash scripts/ci/reset_browser_fixture_db.sh' scripts/ci/run_browser_e2e.sh | cut -d: -f1)"
migration_line="$(grep -n 'bash scripts/ci/apply_migrations.sh' scripts/ci/run_browser_e2e.sh | cut -d: -f1)"
seed_line="$(grep -n 'tests/e2e/fixtures/seed.sql' scripts/ci/run_browser_e2e.sh | cut -d: -f1)"
test -n "${reset_line}"
test -n "${migration_line}"
test -n "${seed_line}"
test "${reset_line}" -lt "${migration_line}"
test "${migration_line}" -lt "${seed_line}"

grep -q "status IN ('Submitted'::custom_status, 'Graded'::custom_status)" tests/e2e/fixtures/seed.sql
grep -q 'graded submissions require coherent graded_at' tests/e2e/fixtures/seed.sql
grep -q 'guided draft must have no generated custom assignment or submission' tests/e2e/fixtures/seed.sql
grep -q 'browser-created e2e-pr1 accounts leaked into fresh baseline' tests/e2e/fixtures/seed.sql

echo "browser harness verification passed"
