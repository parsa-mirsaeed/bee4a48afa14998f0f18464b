# AI grading: provider-native PDF input (implementation contract)

**Decision (2026-11-13):** Student PDF assignments should be passed as original PDF documents to a PDF-capable GPT/Claude-style provider, not processed by an EduTalent PDF-to-text/OCR pipeline. The provider's own native document processing is expected.

## Landed foundation
- Server-only `NativePdfGradingInput` wraps immutable original PDF bytes and binds them to school, submission, revision and attachment IDs.
- Construction requires a SHA-256 match, an exact PDF MIME type, non-nil binding identifiers, the storage size bound, and existing PDF structural checks. Original byte sequence is preserved.
- No `Debug` on the wrapper, no outbound request, no logging of bytes, no serialization into a client response.
- This is an **input validation contract only**, not yet end-to-end PDF inference. The current grading repository query intentionally rejects submissions with finalized attachments; the text-only worker remains opt-in and disabled by default.

## Next implementation PRs (must remain fail-closed between each)
1. **Authorized attachment read**: at worker/teacher generation time, select the current submitted PDF attachment by exact submission + custom assignment + enrolled student + school + revision. Read the private object under server authority, validate the stored MIME/size/hash against actual bytes, and freeze the authorized revision/teacher/approved rubric before and after inference. Never accept an external file URL or client storage key.
2. **Provider capability and transport**: enable only provider+model pairs whose official API supports PDF document input; implement adapter-specific native PDF formats and endpoint authentication. OpenAI and Anthropic payloads are not interchangeable, and OpenAI-compatible chat JSON does not imply native PDF support. Fail closed for an unsupported or unconfigured model. Strict gateway quotas, limits, no proxy, no redirects, no content logging, bounded provider output and origin verification continue to apply. Do not send student documents to arbitrary operator-supplied custom endpoints without independent policy review.
3. **Mock end-to-end evidence**: prove the exact PDF original reaches the mocked native document field, in the correct school context, with approved rubric and revision. Verify wrong-school access, stale revisions, tampering, unsupported model, non-PDF, oversize, malformed PDF, retry and provider errors. Assert no text extraction, no implicit publication, no student content in logs and no result reuse across revisions.
4. **Teacher UI and release evidence**: show teacher-facing availability/errors, control when AI suggestions are generated, and run focused browser checks plus PostgreSQL/Rust/security integration. Release approval remains contingent on the real teacher-curated quality benchmark and human governance in #138.

PDF content must be treated as **untrusted reference data**, not a source of system or rubric instructions. Native PDF input is not a claim that a model's PDF reading is infallible: multilingual handwriting and scanned PDFs require benchmark proof on the selected provider, and a teacher may reject/override suggestions.

## Workflow
The changed server-side module requires exact-head AI Change Proof and focused Rust tests; when authorized DB read and LLM gateway transport change, require the union of DB RLS/integration, Rust, gateway mock/provider payload tests, and browser proof for any affected Teacher UI. Do not merge from stale checks or treat an outer successful workflow with skipped substantive jobs as comprehensive validation.

## Provider payload adapter stage (not live dispatch)
- `ai_grading_pdf_provider_payload` builds native OpenAI Chat Completions `file` content and Anthropic Messages `document` content, using bytes verified by `NativePdfGradingInput` and a fixed non-sensitive filename. The original PDF bytes survive base64 round-trip unchanged; no extraction occurs in the application.
- This adapter is **not yet wired to the gateway**. Before forwarding anything, the gateway must choose an explicit approved provider/model capability, the matching endpoint and authentication, apply binary-size quotas (the old text-character quota is insufficient for base64), and have a mock provider test for authorization and byte fidelity.
- Verified public provider documentation: https://developers.openai.com/api/docs/guides/file-inputs and https://platform.claude.com/docs/en/build-with-claude/pdf-support (checked 2026-11-13). Provider-specific schemas and feature availability must be checked when integration is performed.

## Fail-closed provider/model selection (staged)
Native PDF payload builders now require a typed, **exact** provider/model capability instead of arbitrary model strings. The current deliberately narrow pilot pairs are `openai / gpt-4o` and `anthropic / claude-sonnet-4-5`. This is **not** deployment configuration or a guarantee of model access: the actual gateway must separately check its pinned HTTPS origin, configured provider identity, operator credentials, policy and model availability at runtime before making any request. Unsupported generic OpenAI-compatible, DeepSeek, mismatched or unknown models fail closed. Bounded nonempty instructions and output tokens are enforced at payload construction.

Official document API references (checked 2026-11-13): https://developers.openai.com/api/docs/guides/file-inputs and https://platform.claude.com/docs/en/build-with-claude/pdf-support.

## Fixed first-party OpenAI provider profile
The operator can explicitly configure `LLM_PROFILE=openai-gpt4o-v1` with matching `LLM_MODEL=gpt-4o` and the managed `LLM_API_KEY`. The gateway binds this exact profile to `https://api.openai.com/v1/`; it rejects a mismatched origin and does **not** accept arbitrary OpenAI-compatible endpoints as native PDF providers. Omitting `AI_LLM_BASE_URL` for this explicit profile defaults to the fixed OpenAI origin; existing DeepSeek deployments retain the previous default.

This only registers a **text/chat capable fixed profile** at the existing gateway boundary; it does **not yet** implement a native PDF endpoint, upload, document processing dispatch, or automatic grading. Do not enable the grading worker or transmit PDF documents until the separate authenticated PDF request and provider tests are merged.

## Authenticated native PDF gateway route (staged, default OFF)
A dedicated internal `POST /v1/grading/pdf` contract accepts **only base64 of an original PDF**, its SHA-256, immutable submission/attachment/revision UUIDs, and bounded teacher-approved grading prompt content. School identity is taken from the authenticated `x-edutalent-school-id` header, not from a client field. An exact gateway-internal bearer token is mandatory.

Safety constraints: `AI_NATIVE_PDF_GATEWAY_ENABLED` defaults to false; only the explicitly configured first-party `openai-gpt4o-v1` profile can send a PDF. Generic/custom/DeepSeek profiles fail closed. For the initial pilot the PDF is limited to **1,000,000 original bytes** (encoded request body max 1,500,000 bytes). The gateway verifies byte integrity and PDF structure, charges the encoded document bytes plus instructions to the school quota, uses direct/no-proxy HTTPS with redirects forbidden, enforces output and response limits and does not retry a possibly charged PDF inference request. Content/file names/base64 must never be logged. Mock integration tests must prove identical provider-bound PDF bytes and unauthorized rejection.

**Not yet a user-ready feature:** the application-side RLS-authorized finalized attachment read and worker/Teacher handoff are still absent, so the route is not invoked by the Student assignment flow. Do not enable this switch outside an isolated staging proof; it is not permission to auto-grade or publish student grades. The default text-only grading worker remains disabled.

OpenAI official Chat Completions PDF file input requirements: https://developers.openai.com/api/docs/guides/file-inputs (PDF-only file content part, no external file URL).

## Authorized source stage — #145 (application-side, still disconnected from worker)
`ai_grading_pdf_source` implements a dedicated **Teacher-scoped RLS query** for an exact submitted assignment, submission UUID/revision, currently approved rubric UUID, and exactly **one** submitted PDF attachment under the same school and student. The query rejects stale/graded/unsubmitted assignments, inactive teachers or students, missing enrollment/teaching assignment and revoked rubric. It does not weaken the existing text-only attachment rejection.

The private original is retrieved server-side only via the verified Supabase Storage bucket using a storage key **derived from authorized metadata** (never a client URL). The original file size must match recorded metadata, SHA-256 must match, framing must be valid, and the current authorized metadata must still match immediately after the asynchronous read. The gateway's initial 1 MB PDF limit applies. This stage returns an in-memory binary document and never serializes original bytes to a browser, logs contents or extracts text.

**This is not end-to-end inference**: the text-only worker does not yet call this source. A separate worker/API PR must reauthorize submission revision and rubric **again after LLM inference**, and persist a teacher-review suggestion only after exact-authority verification. A disabled-by-default flag, operator cost limits, and full mock/browser E2E acceptance are prerequisites for a controlled pilot.

## Native PDF application-to-gateway adapter (staged)
The application LLM client now has a server-only entry point for `AuthorizedNativePdf`, a document obtained from the Teacher-RLS source rather than a caller-provided URL. Before egress, the client revalidates school/submission/revision/attachment identity, original size/hash, the exact `openai-gpt4o-v1` model and the teacher-approved rubric. It sends the original bytes as base64 only to the pinned internal `/v1/grading/pdf` gateway and validates the single resulting suggestion against the approved rubric and revision. Synthetic gateway tests assert exact byte fidelity, school headers and scoring evidence.

**Still separate:** the active durable worker and Teacher HTTP grading suggestion flow do not yet call the new adapter. Authorization must be rechecked **after** the LLM reply and before suggestion persistence. The worker and native PDF gateway remain default-OFF, and no AI grade is automatically published.

## Teacher-authorized PDF suggestion integration (stage #147)
The existing audited teacher grading suggestion service now checks current approved rubric first, selects the existing text-only authorized source for text submissions, and otherwise explicitly selects a **single-PDF** source under the same live Teacher RLS context and current revision. For PDF submissions it reads the original from private storage and sends it through the opt-in native PDF LLM client, then **re-reads the exact PDF metadata and the approved rubric after inference**. Stale, revoked, graded, wrong-school or changed originals cause rejection before persistence. The same DB-backed suggestion record, idempotency and audited Teacher accept/reject controls apply. The teacher's current pending suggestion reader also supports an authorized PDF submission. The free-form teacher grading preview remains text-only.

**Activation remains blocked**: no automatic worker/queue enablement is authorized; existing database enqueue behavior for submitted attachment revisions remains unchanged until separately reviewed queue and worker end-to-end proof. The native PDF gateway and worker stay OFF by default. The model produces only a pending suggestion, never an official grade.
