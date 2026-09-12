# Release WASM optimization contract (#64)

Browser acceptance and Docker packaging use `scripts/package/build_web_release.sh`.
It invokes the pinned Dioxus CLI with `--debug-symbols false`; optimization stays
mandatory. Normal development builds are unaffected.

The workspace Cargo release profile is not the complete post-processing policy.
In Dioxus CLI v0.7.2, `TargetArgs.debug_symbols` defaults to `true`, independently
of Cargo's `debug = false`. `BuildRequest` passes it to wasm-bindgen's debug
retention and to `WasmOptConfig.debug`. The latter selects Binaryen's
`--debuginfo` versus `--strip-debug`. That CLI pins Binaryen version 123.
The effective wasm-bindgen version comes from the locked application dependency.

Primary implementation references:

- [CLI flag and default](https://github.com/DioxusLabs/dioxus/blob/v0.7.2/packages/cli/src/cli/target.rs)
- [WASM post-processing](https://github.com/DioxusLabs/dioxus/blob/v0.7.2/packages/cli/src/build/request.rs)
- [Binaryen arguments, version and failure behavior](https://github.com/DioxusLabs/dioxus/blob/v0.7.2/packages/cli/src/wasm_opt.rs)

The CLI also logs a failing optimizer and returns success with the original
module. The shared wrapper rejects that fallback, preserves the complete build
log, rejects missing/invalid WASM, and emits output SHA-256, size and elapsed
build time only after those checks pass. It clears disposable bundle output
before building, preserving Cargo and tool caches, so an older module cannot
stand in for a missing new build. Tests cover nonzero build exit, swallowed
optimizer failure, missing output, invalid output and successful fresh output.

Diagnostics remain available through Playwright traces, console/network records,
exact source SHA, and retained build logs. Release WASM does not retain DWARF.
Use a separate local diagnostic build when DWARF is required; never substitute
its artifact for the exact release candidate.

The wrapper checks do not replace authenticated browser acceptance or final
Package checksum/image/startup proof. #64 is complete only after the new exact
head passes those checks and its real output identity/size/timing are recorded.
Historical successful bundles with an optimizer crash are functional fallback
evidence, not successful optimization evidence.
