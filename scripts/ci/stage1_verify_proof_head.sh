#!/usr/bin/env bash
# Never label merge-preview or stale source as exact-head evidence.
set -euo pipefail
expected="${PROOF_HEAD_SHA:?PROOF_HEAD_SHA is required}"
actual="$(git rev-parse HEAD)"
if [[ ! "${expected}" =~ ^[0-9a-f]{40}$ ]] || [[ "${actual}" != "${expected}" ]]; then
  echo "Proof rejected: checked out ${actual}, expected ${expected}." >&2
  exit 1
fi
echo "Verified proof checkout: ${actual}"
