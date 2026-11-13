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
