#!/usr/bin/env bash
# Reset the dedicated browser-test database before migrations/fixtures are applied.
#
# This is intentionally destructive and must fail closed for ordinary databases.
# The canonical CI database is `edutalent_ci`. A non-canonical database must both
# look explicitly test-owned and require a one-shot acknowledgement.
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL is required for browser fixture reset}"

current_database="$(psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -Atqc 'SELECT current_database()')"
if [[ -z "${current_database}" ]]; then
  echo "Unable to resolve browser fixture database name; refusing destructive reset." >&2
  exit 1
fi

if [[ "${current_database}" != "edutalent_ci" ]]; then
  if [[ "${E2E_ALLOW_FIXTURE_RESET:-}" != "1" ]]; then
    echo "Refusing browser fixture reset for database '${current_database}'." >&2
    echo "Expected dedicated database 'edutalent_ci'." >&2
    exit 1
  fi

  # The override is not a bypass for arbitrary databases: require an explicit
  # test-owned database name as a second independent safety signal.
  if [[ ! "${current_database}" =~ (^|[_-])(ci|e2e|test)([_-]|$) ]]; then
    echo "Refusing destructive reset for non-test database '${current_database}' even with E2E_ALLOW_FIXTURE_RESET=1." >&2
    echo "Use a dedicated database whose name contains a standalone ci/e2e/test marker." >&2
    exit 1
  fi
fi

# Resetting the dedicated schema before migrations is simpler and safer than a
# growing FK-ordered delete list. It guarantees browser-created users,
# enrollments, submissions, audit rows, and other mutable fixture state cannot
# leak into the next run on a reused CI database.
psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 <<'SQL'
DROP SCHEMA IF EXISTS public CASCADE;
CREATE SCHEMA public AUTHORIZATION CURRENT_USER;
GRANT ALL ON SCHEMA public TO public;
SQL

relation_count="$(psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -Atqc "SELECT COUNT(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND c.relkind IN ('r','p','v','m','S','f')")"
if [[ "${relation_count}" != "0" ]]; then
  echo "Browser fixture reset did not leave an empty public schema; refusing to continue." >&2
  exit 1
fi

echo "reset dedicated browser fixture database '${current_database}'"
