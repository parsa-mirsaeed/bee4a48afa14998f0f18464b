#!/usr/bin/env bash
# EduTalent Local Runner Convenience Entrypoint
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${ROOT_DIR}/scripts/dev/run_and_verify.sh" "$@"
