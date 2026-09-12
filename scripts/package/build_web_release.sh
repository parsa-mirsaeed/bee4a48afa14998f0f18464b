#!/usr/bin/env bash
# Shared browser/image release contract. Dioxus 0.7.2 defaults debug_symbols to
# true and logs (but swallows) Binaryen failures. Reject that fallback explicitly.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"
log="target/web-release-build.log"
evidence="target/web-release-evidence.txt"
bundle_dir="target/dx/web/release/web"
mkdir -p target
rm -f "${evidence}"
# Only disposable bundle outputs; retain Cargo/tool caches. An old WASM file
# must never stand in for missing output from this invocation.
rm -rf "${bundle_dir}"
dx --version
started="$(date +%s)"
dx bundle --web --release --package web --debug-symbols false 2>&1 | tee "${log}"
if grep -Eiq 'wasm-opt.*(failed|error|abort|panic)|failed to (run|create) wasm-opt' "${log}"; then
  echo 'Release rejected: wasm-opt did not complete successfully.' >&2
  exit 1
fi
find "${bundle_dir}/public" -type f -name '*.wasm' -print0 > target/web-release-wasm-files.list
mapfile -d '' wasm_files < target/web-release-wasm-files.list
if (( ${#wasm_files[@]} == 0 )); then
  echo 'Release rejected: no fresh browser WASM was produced.' >&2
  exit 1
fi
for wasm in "${wasm_files[@]}"; do
  if [[ "$(od -An -tx1 -N8 "${wasm}" | tr -d ' \n')" != '0061736d01000000' ]] || [[ "$(wc -c < "${wasm}")" -le 8 ]]; then
    echo 'Release rejected: invalid or empty browser WASM.' >&2
    exit 1
  fi
done
{
  echo 'release_wasm_debug_symbols=false'
  echo "build_seconds=$(( $(date +%s) - started ))"
  for wasm in "${wasm_files[@]}"; do
    echo "wasm_bytes=$(wc -c < "${wasm}") path=${wasm}"
    sha256sum "${wasm}"
  done
} | tee "${evidence}"
