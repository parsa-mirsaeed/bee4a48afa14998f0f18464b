# Native PDF grading — 2026-11-18 engineering handoff and release disposition

**Status: ENGINEERING HANDOFF READY; PRODUCTION RELEASE NO-GO / NOT APPROVED.**
**Frozen source reference:** `main` at `2249d93175a89fd81fab6a634d0092c4a17d4adb`. **No switch or school opt-in is authorized by this document.**

Binding sources: [issue #138](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/issues/138); [manual/external acceptance PR #2](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/pull/2) (OPEN/DRAFT); [staging pilot and rollback runbook](ai-grading-native-pdf-pilot-runbook.md) (NOT APPROVED). This handoff inventories the evidence and outstanding decisions; it does not replace the runbook or qualified human approval.

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

- [ ] **Single continuous positive staging journey** (synthetic data only): Student original PDF upload/finalization → immutable SHA/size and submission revision → explicit Teacher rubric → operator-scoped single-school opt-in and committed durable budget reservation → exactly one durable queue claim by bounded worker → real internal gateway → local mock provider with exact original binary → validated pending private suggestion → assigned Teacher accept/reject → Student/linked Parent official grade only on acceptance. Capture distinct job/lease/audit identities, not private content.
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
| Engineering + continuous synthetic native-PDF integration | UNASSIGNED | BLOCKED |
| Independent pedagogical EN/FA quality assessor | UNASSIGNED | BLOCKED |
| School privacy lead / legal / consent | UNASSIGNED | BLOCKED |
| Finance / supplier hard spending authority | UNASSIGNED | BLOCKED |
| Security / penetration / accessibility | UNASSIGNED | BLOCKED |
| SRE release operator / restore and rollback | UNASSIGNED | BLOCKED |
| Final independent release authority | UNASSIGNED | **NO-GO** |

**Handoff action:** assign the owners and P0 tasks in Issue #138; store qualified external acceptance in PR #2 and protected evidence channels; verify an exact frozen release source SHA, package digests and complete final-release workflows; only then consider an **explicitly limited synthetic-school staging pilot**. This document is not an approval to enable production.
