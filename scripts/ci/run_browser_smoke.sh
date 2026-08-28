#!/usr/bin/env bash
# Stage-1 Tier-1 browser smoke on an exact head. E2E_GREP may narrow the
# existing tagged suite; an absent selector intentionally falls back to all
# @smoke journeys.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

: "${E2E_GREP:=@smoke}"
export E2E_GREP
exec bash scripts/ci/run_browser_e2e.sh
