# Native PDF grading — 2026-11-18 engineering handoff and release disposition

**Status: ENGINEERING HANDOFF READY; PRODUCTION RELEASE NO-GO / NOT APPROVED.**
**Frozen source reference:** `main` at `2249d93175a89fd81fab6a634d0092c4a17d4adb`. **No switch or school opt-in is authorized by this document.**

Binding sources: [issue #138](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/issues/138); [manual/external acceptance PR #2](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/pull/2) (OPEN/DRAFT); [staging pilot and rollback runbook](ai-grading-native-pdf-pilot-runbook.md) (NOT APPROVED). This handoff inventories the evidence and outstanding decisions; it does not replace the runbook or qualified human approval.

## 2026-11-24 update — assessment feature delivery

PR [#168](https://github.com/pemerald0/bee4a48afa14998f0f18464b/pull/168)
merged into main at `20cf81a9f875833dbbe2013b609de8bc1fd435bb`.
Its exact reviewed head `98512e157aa40fe27211db9b842fc3ad48c9ebb1` passed
[real gateway acceptance](https://github.com/pemerald0/bee4a48afa14998f0f18464b/actions/runs/45229525026),
[AI change proof with production foundation](https://github.com/pemerald0/bee4a48afa14998f0f18464b/actions/runs/45229525125),
[Full Validation](https://github.com/pemerald0/bee4a48afa14998f0f18464b/actions/runs/45229524893)
and [Dependency Security](https://github.com/pemerald0/bee4a48afa14998f0f18464b/actions/runs/45225979372).
The continuous synthetic PDF journey uses the actual application, durable
budget and queue, real gateway and TLS-verified local first-party mock
provider. It verifies Teacher-reviewed accept/reject and Student/Parent
visibility in English desktop and Persian mobile. This proves integration,
not real-model grading quality or supplier charges.

**Next-update deployment contract:** an installation with a configured
connected chat provider can use Teacher-initiated text grading. Native PDF
grading additionally requires an exact approved provider/model pair, gateway switch and
the existing approved school-specific opt-in/budget. Automatic processing
requires the application worker switch. The deployment templates preserve
the previous installation's default-OFF behavior and allow operators to
explicitly select the new capabilities; upgrading the image does not
automatically authorize external processing.

### Operator-selected chat provider: GapGPT

For this update the preferred text and native-PDF assessment provider is GapGPT at
`https://api.gapgpt.ir/v1/`, using the existing operator-owned custom
profile contract. The app continues to call only its authenticated internal
gateway; the gateway sends requests to `/v1/chat/completions`. The existing native PDF file adapter is extended only for this exact
provider/model pair; no automatic failover.

Configure the existing private `deploy/production/.env.edutalent`:

```dotenv
AI_LLM_MODE=connected
AI_LLM_BASE_URL=https://api.gapgpt.ir/v1/
LLM_PROFILE=custom-v1:gpt-6-luna
LLM_MODEL=gpt-6-luna
AI_GRADING_WORKER_ENABLED=true
AI_NATIVE_PDF_GATEWAY_ENABLED=true
```

Set `LLM_API_KEY` separately to a newly issued secret. Never commit it.
For an operator-selected text-only backup, change both model settings and disable the PDF gateway:

```dotenv
LLM_PROFILE=custom-v1:claude-haiku-5-5
LLM_MODEL=claude-haiku-5-5
AI_NATIVE_PDF_GATEWAY_ENABLED=false
```

Keep the endpoint and managed key as above; disable native PDF for the backup. The active
profile is installation-wide, including other chat features. The backup
is a deliberate configuration switch/restart, not silent failover.
Preserve the previous profile and model for rollback. Provider acceptance
requires a synthetic rubric-grading call with a valid JSON suggestion,
exact requested model in the response and correct Teacher review; a plain
greeting response or a successful Compose render is not inference proof.

**Native PDF capability evidence:** GapGPT's OpenAI-compatible API documentation
and the operator's successful synthetic original-PDF retrieval through
`gpt-6-luna` confirm basic file input. The new exact-origin route uses the
same file content part, bounded completion tokens, strict JSON validation,
immutable revision and durable budget controls. Dedicated CI tests exercise
the local TLS mock for the selected provider; real rubric-scoring compatibility
and provider response-model identity remain a separate operator acceptance
check. Haiku remains text-only. Do not bypass existing school opt-in,
budget or Teacher review; do not silently switch models or convert PDFs.

### Existing native-PDF profile (separate selection)

For an approved installation, the relevant entries in its existing private
`deploy/production/.env.edutalent` are:

```dotenv
AI_LLM_MODE=connected
AI_LLM_BASE_URL=https://api.openai.com/v1/
LLM_PROFILE=openai-gpt4o-v1
LLM_MODEL=gpt-4o
AI_GRADING_WORKER_ENABLED=true
AI_NATIVE_PDF_GATEWAY_ENABLED=true
```

Supply the real `LLM_API_KEY` through the existing managed private gateway
configuration, not this document. Preserve the existing embedding profile
and all other installation settings. The gateway and worker switches may
be enabled independently; neither bypasses the school PDF opt-in or durable
budget. Follow the [activation and rollback runbook](ai-grading-native-pdf-pilot-runbook.md)
for operator authorization and the school-specific policy. Use the existing
`bash edutalent production-validate` before `production-up`, and verify
Teacher review and actual publication with approved data. Never put student
originals, credentials or model prompts in public evidence.

Release acceptance is limited to the agreed assessment update; it does not
restart unrelated platform hardening or replace the qualified approval
requirements for real student data. Issue #138's administrative closure
does not constitute a signed release decision; PR #2 remains the record
for applicable independent acceptance. Final candidate packaging and
activation evidence must refer to the actual deployed candidate.

## Verified exact-source engineering evidence

| Boundary | Workflow / PR | Observed result | Scope limitation |
| --- | --- | --- | --- |
| `main` full validation | [#43825328876](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/actions/runs/43825328876) | SUCCESS: full database, full Rust compile/lint/tests, final browser acceptance and aggregate gate | Automated acceptance only |
| Dependency advisory audit | [#43877637319](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/actions/runs/43877637319) | SUCCESS | Not a penetration test or vendor review |
| Package | [#43836874860](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/actions/runs/43836874860) | SUCCESS: definition and Docker/release bundle | Targeted runtime image/migration replay was SKIPPED |
| Production foundation | [#44289157154](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/actions/runs/44289157154) | SUCCESS: topology, pinned PG17 migrations/roles, production-stack startup | Does not replace operator release signoff |
| Native PDF HTTP mock transport | [PR #159](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/pull/159) | MERGED: original PDF bytes, revision/rubric, provider-native request and valid private suggestion through real authenticated gateway | No Student/worker/browser chain in the same test |
| Native PDF provider-egress denial | [PR #160](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/pull/160) | MERGED: unauthenticated, unscoped and tampered originals do not reach mock provider | Focused negative matrix |
| Teacher decision / Student + Parent | [PR #161](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/pull/161), [AI #43762162425](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/actions/runs/43762162425), [Full #43762162342](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/actions/runs/43762162342) | MERGED and GREEN: English desktop accepts 18/20, Persian mobile rejects; real Student/linked Parent grade visibility asserts | Synthetic *pre-seeded* suggestion; no real inference in that browser test |

**Important:** Workflow checks on these SHAs do not automatically apply to subsequent changes. Complete final proof must be rerun for the eventual frozen release candidate.

## Release P0 — must be closed before any pilot activation

- [x] **Single continuous positive staging journey** (synthetic data only): Student original PDF upload/finalization → immutable SHA/size and submission revision → explicit Teacher rubric → operator-scoped single-school opt-in and committed durable budget reservation → exactly one durable queue claim by bounded worker → real internal gateway → local mock provider with exact original binary → validated pending private suggestion → assigned Teacher accept/reject → Student/linked Parent official grade only on acceptance. PR #168 proves the continuous synthetic integration in CI; actual target-host staging acceptance and any external approval remain separate. Capture distinct job/lease/audit identities, not private content.
- [ ] **Adversarial integrated checks**: duplicate enqueue and retry; stale revision/rubric; revoked school opt-in or Teacher authorization before/after storage/provider request; cross-school/cross-role isolation; missing/unsupported/encrypted/malformed/oversize PDF; sibling attachment ambiguity; provider failure/timeouts/budget exhaustion; expired worker leases and recovery; no Student/Parent access to private suggestion and no automatic grade publication.
- [ ] **Independent teacher-scored model-quality evidence**: approved de-identified English/Farsi dataset across subjects/grades, scanned/handwritten and adversarial material, frozen benchmark/model/prompt/rubric hashes, preregistered MAE and severe/high-confidence-error thresholds, independently adjudicated results and signoffs. Local synthetic tests are not pedagogical validation.
- [ ] **Financial hard limits**: supplier-enforced spend caps or documented unacceptable residual risk; document native PDF page/image/token cost envelope, vendor invoices vs durable reservation, multi-replica quota semantics, per-school alerts and stop owner. In-memory rate limiting and reserved micro-USD are *not* invoice truth.
- [ ] **Security, privacy and governance**: DPIA/lawful basis/consent and child/parent transparency, vendor cross-border processing contract, retention/erasure, independent access/security review, penetration test, human accessibility/keyboard/screen-reader acceptance, incident/support ownership and approvals in PR #2.
- [ ] **Operational release proof**: isolated PostgreSQL+private Storage restore, least-privilege role/no BYPASSRLS, FORCE RLS after owner-executed policy changes, queue lease recovery, emergency stop and credential revocation, restricted evidence trail, clean release host, signed final frozen candidate and explicit operator decision.
- [ ] **Final decision**: record named engineering, independent Teacher, finance/vendor, privacy/legal, security, SRE/operations and release-authority approvers with evidence IDs, dated dispositions and exact candidate artifacts. **Do not close #138 or merge #2 until actually satisfied.**

## Default-OFF / stop controls

- `AI_GRADING_WORKER_ENABLED=false` by default.
- `AI_NATIVE_PDF_GATEWAY_ENABLED=false` by default.
- School native-PDF opt-in/budget are default-deny; any exception requires separate reviewed **owner-only migration**, explicit synthetic staging school, protected role context, and `FORCE RLS` restored before commit. Do not create runtime policy-writer APIs.
- No production student's original is permitted in staging smoke. No application-side OCR/PDF-to-text on native-document inputs. Model-generated evidence remains non-authoritative; only Teacher acceptance publishes.
- On any security, integrity, spending or grade incident: disable both flags, stop active workers/in-flight provider calls, revoke credentials/spend authorization, operator-revoke the school opt-in in audited RLS-safe transaction, freeze/resolve queue leases without destroying evidence, reconcile storage/DB and actual charges, and require independent approval before restart. See [binding rollback runbook](ai-grading-native-pdf-pilot-runbook.md).

## Handoff ownership

| Gate owner | Assigned person / signed evidence | Status |
| --- | --- | --- |
| Engineering + continuous synthetic native-PDF integration | PR #168 exact-head acceptance evidence above | PASSED (synthetic CI; target-host signoff separate) |
| Independent pedagogical EN/FA quality assessor | UNASSIGNED | BLOCKED |
| School privacy lead / legal / consent | UNASSIGNED | BLOCKED |
| Finance / supplier hard spending authority | UNASSIGNED | BLOCKED |
| Security / penetration / accessibility | UNASSIGNED | BLOCKED |
| SRE release operator / restore and rollback | UNASSIGNED | BLOCKED |
| Final independent release authority | UNASSIGNED | **NO-GO** |

**Handoff action:** assign the owners and P0 tasks in Issue #138; store qualified external acceptance in PR #2 and protected evidence channels; verify an exact frozen release source SHA, package digests and complete final-release workflows; only then consider an **explicitly limited synthetic-school staging pilot**. This document is not an approval to enable production.
