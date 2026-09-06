# Dioxus fullstack transport patch

Upstream: DioxusLabs/dioxus, dioxus-fullstack 0.7.2 (MIT OR Apache-2.0).
Source archive: https://static.crates.io/crates/dioxus-fullstack/dioxus-fullstack-0.7.2.crate
SHA-256: `54150804265defdb21a6f2d8914a45316a1e7fb70ab22c30cf836e8fe2f8081b`

The published crate is retained verbatim except for `src/magic.rs`: the
`RequestDecodeResult` response-body read propagates `RequestError` with `?`
instead of panicking with `unwrap()`. This changes no authorization, response
schema, successful decoding, or server error handling. See EduTalent issue #61.

The API transport regression tests serve both complete and truncated HTTP
bodies and call this exact decoder. The truncated body must return a request
error after a successful HTTP response, not panic. Authenticated browser
console/error guards remain strict and retries remain disabled.

The workspace patch and lockfile preserve version 0.7.2 and its existing
dependency resolution. The crate's own Cargo.lock and editor metadata are
upstream reference material; the workspace Cargo.lock controls builds.

Remove this patch only when an upstream release contains the response-body
error propagation fix and passes the same regression tests. Do not mass-format
upstream files; compare against the archive to review the single-line delta.
