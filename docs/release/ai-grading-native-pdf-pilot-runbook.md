# Native PDF AI grading — staging pilot acceptance and rollback

**Status: NOT APPROVED.** This is an operational runbook and an evidence
template, not permission to expose real student originals or activate a model.
Binding: original PDF bytes sent as native document input, no EduTalent OCR /
PDF-to-text extraction, and only an assigned Teacher accepts official grades.
Track all missing evidence at [AI grading readiness #138](https://github.com/parsa-mirsaeed/bee4a48afa14998f0f18464b/issues/138).
Manual/external signoff remains consolidated in PR #2.

## 1. Frozen pilot candidate prerequisites

An operator must record, **before** enabling any switch:

- immutable Git SHA, base SHA, migration set/checksums, deployment image
  digests, prompt+rubric policy SHA-256, exact OpenAI `gpt-4o` model profile
  and fixed first-party HTTPS origin;
- successful exact-head `AI Change Proof`, **complete** Full Validation
  with actual DB/Rust/desktop/mobile browser/final gate jobs (not skipped),
  focused dependency-security, production topology, operational restore and
  privacy controls for changed boundaries;
- student→private original→revision-bound queue→authenticated PDF gateway→
  model mock→pending private suggestion→Teacher accept/reject→student/parent
  outcome with duplicate/stale/revoked/cross-school negative tests;
- approved, de-identified *real-model* teacher-scored benchmark including
  English/Farsi, subject×grade×language×document type, scanned/handwritten
  and adversarial instructions; frozen sample manifest SHA and independently
  adjudicated severe/high-confidence errors; the offline synthetic CLI alone
  is **not** a release attestation;
- assessed DPIA/consent/legal basis, cross-border vendor processing review,
  student/parent transparency, retention/erasure policy, incident contacts,
  support and escalation ownership, operator signoff.

**If any item is missing, remain OFF.** The school opt-in table is empty
after deployment and has no request-scoped mutation policy.

## 2. Financial controls — separate guarantees

The native PDF gateway already applies per-school in-memory hourly quota,
1 MB original-file limit, fixed model, limited output tokens and a no-retry
outbound request. In-memory quotas are **not durable across replicas**.
The durable admission reservation tracks per-school UTC-day original bytes,
attempts, and operator-supplied reserved micro-USD allocations in PostgreSQL.
It starts DENY ALL if no approved budget policy exists.

The reservation is committed independently BEFORE provider egress. Every
potentially charged attempt consumes capacity, including timeout/retry;
automatic job retries must not bypass exhaustion. The reservation figure is
**not provider invoice truth**. Obtain an independently measured bound on
PDF page processing, image representations and output tokens, supplier
account/org spending hard stops, invoice telemetry and alert thresholds before
a pilot. If the supplier does not supply an enforceable spending cap, document
that residual risk and do not claim a hard monetary budget.

## 3. Activation — isolated staging only

1. Use an isolated synthetic-data school. No production student's original
   is sent to a model during smoke testing.
2. Restore an encrypted, known-good PostgreSQL/storage backup in a disposable
   environment. Confirm the target DB role has no `BYPASSRLS`, raw public
   access to protected originals, or write access to opt-in/budget policy.
3. Verify native PDF bucket is private, original SHA/size are immutable, and
   access is scoped to the assigned Teacher and current submission revision.
4. In a separately reviewed **owner-executed migration** (never via a
   Teacher/Student/Parent API), create the exact school's budget policy with
   validated daily requests/bytes/reservation units. Record approval ID,
   SQL digest and rollback SQL; no generic UI toggle is authorized.
5. Enable the single synthetic school's `ai_grading_pdf_school_optins`
   record only in an owner-executed, audited SQL migration. FORCE RLS must
   be restored **within the transaction before COMMIT**; the runtime actor
   guard trigger must remain active. Verify other schools stay OFF.
6. Configure the internal gateway using only the pinned first-party profile
   `openai-gpt4o-v1`, approved model `gpt-4o`, first-party HTTPS origin and
   managed secrets. Keep unrestricted OpenAI-compatible/custom origins denied.
   Enable `AI_NATIVE_PDF_GATEWAY_ENABLED` only for the isolated staging
   gateway, then `AI_GRADING_WORKER_ENABLED` on one isolated worker *after*
   the exact-head tests and budget gate pass.
7. Run serial positive and negative E2E against isolated, synthetic fixtures.
   Assert no provider content in logs, no unreviewed final grades, idempotent
   exact-revision jobs, bounded failures and retry exhaustion. Verify the
   Teacher's accept/reject decision is the only publication path.
8. Record actual provider request ID, tokens, charges, latency and output
   status only in restricted operational evidence; redact/delete student
   content. Reconcile supplier charges against local reservations. Record
   any overrun or mismatch as a release blocker.

## 4. Stop, rollback and incident response

On unauthorized egress, budget overrun, RLS regression, corrupted original,
model-quality failure, leaked content, sustained queue failures or any
unexpected grade publication:

1. Set `AI_GRADING_WORKER_ENABLED=false` and
   `AI_NATIVE_PDF_GATEWAY_ENABLED=false` in the deployment; terminate
   running workers/providers to stop in-flight requests (config changes
   alone do not cancel in-flight calls).
2. In a separately audited operator SQL transaction, turn the pilot school's
   PDF opt-in OFF and restore FORCE RLS before commit; verify the guard
   trigger still denies Teacher-context writes.
3. Disable/revoke provider credentials or supplier-side spend authorization
   if outbound calls might continue. Alert operations, school privacy lead
   and incident owner without copying original student data into tickets.
4. Freeze queue claims, reconcile running leases, cancel/recover jobs using
   existing guarded queue APIs; avoid deleting suggestion/audit evidence.
   Verify no unapproved grade changed.
5. Preserve protected audit/provenance logs only under the approved retention
   schedule, perform DB+private Storage consistency checks, and rehearse
   restore and cleanup in the isolated environment.
6. Reopen only with a new exact-head tested candidate, documented root-cause
   remediation, independent model/privacy signoff and an operator decision.

## 5. Final handover/evidence record

Keep a restricted machine-readable or signed record of: exact candidate head,
all workflow run IDs and substantive jobs, RLS negative cases, original-byte
integrity, provider mock and real pilot evaluations, calibration/severe-error
per cohort, benchmark hash, prompt/rubric hash, approved school ID, configured
budget terms and observed external billing, gateway/worker switches, rollback
rehearsal, privacy approval, Teacher acceptance and decision owners.

**Release classification is `NOT APPROVED` unless all applicable entries
above have independently inspectable evidence.** A successful focused PR or
an outer Full Validation wrapper with skipped jobs never changes this.
