# EduTalent Production Readiness — AI-Guided Implementation and Validation Plan

**Document status:** Authoritative engineering execution plan derived from the merged production-foundation audit  
**Baseline audit date:** 2026-08-03  
**Audited public-mirror merge commit:** `97b42c946bb0a837c0a5c309c549f3a8e384472f`  
**Current readiness classification:** **Safe for controlled staging and remediation; not accepted for real student data or an unrestricted school production contract.**

> This file is intentionally written to guide an AI engineering agent in future chats. It is not merely a backlog. Each pull request section defines the threat being removed, required implementation shape, required tests, the smallest sufficient CI workflow, workflows that must not run unnecessarily, and an exact exit gate.

---

## 1. How a future AI must use this document

A future AI must treat this file as a plan, not as proof that the repository still has the same state.

Before changing code, the AI must:

1. Resolve the authoritative repository named by the user at execution time. The previous work used a private authoritative repository and a public validation mirror; do not assume the mirror is the write target.
2. Read the repository's current binding instructions, especially:
   - `AGENTS.md`;
   - `.github/FULL_VALIDATION.md`;
   - all affected workflow files;
   - relevant README, ADR, threat-model, migration, deployment, operations, and package documents;
   - this plan and `Rules-of-Workflow.txt` when supplied.
3. Inspect current `main`, open pull requests, latest commits, exact branch head SHA, changed files, review threads, workflow results, and diagnostic artifacts.
4. Reconfirm every finding in this plan against the current exact head. Mark a finding `already resolved` only when code and meaningful tests prove it.
5. Implement one coherent PR at a time in the priority and dependency order below. Do not combine unrelated product features with security-boundary work.
6. Preserve all existing security invariants. Never weaken authorization, RLS, migration integrity, secret handling, Qdrant filtering, Docker isolation, TLS validation, backup verification, or workflow gates to obtain green CI.
7. Add the regression test before or with the fix. A code-only security fix is incomplete.
8. Commit to the appropriate feature branch, then validate only workflows associated with the new exact head SHA.
9. Inspect failures and artifacts. Never claim a skipped job passed, never reuse an older SHA, and never treat pre-checkout infrastructure failure as code success.
10. Resolve review threads only after their findings are fixed and covered by regression tests.
11. Use an expected-head guard for merge. Do not merge if the branch changes after validation.
12. Produce the completion report required in Section 18.

### 1.1 AI behavior constraints

The AI must not:

- assume a green build means authorization is correct;
- treat UUIDs as authorization controls;
- depend on frontend route guards for backend security;
- leave reachable `#[server]` stubs that return fake success;
- display fabricated student, grade, attendance, schedule, message, or report data as real;
- introduce public-CDN runtime code into an offline-first product;
- add broad lint allowances, ignored tests, catch-all error handling, or reduced scanner scope;
- run every expensive Docker/appliance workflow on every commit;
- invent contractual SLA, RPO, RTO, privacy, certification, or legal claims;
- claim ISO, OWASP, NIST, WCAG, SLSA, GDPR, or Swiss-FADP compliance solely because this plan references them.

### 1.2 Revalidation rule

At the start of every future PR, create a short evidence note containing:

```text
Repository:
Base branch:
Base SHA:
Feature branch:
Current head SHA:
PR number:
Relevant plan PR:
Finding still reproducible: yes/no
Affected files:
Required targeted workflow:
Heavy workflows intentionally deferred:
```

---

## 2. Priority model and release gates

### P0 — Production/security blocker

A P0 issue can enable unauthorized access or mutation, produce false API success, bypass account disablement, or invalidate tenant isolation. No real student data and no school pilot may begin while a confirmed P0 remains open.

### P1 — Contract/product blocker

A P1 issue prevents truthful, supportable operation of a contracted feature, browser/offline assurance, accessibility, recovery qualification, or target-host acceptance. A synthetic-data engineering pilot can continue, but production contract acceptance cannot.

### P2 — Operational maturity or optional product scope

A P2 issue is required if the corresponding feature is sold or enabled. It may be explicitly excluded from the first release if the UI, routes, API surface, documentation, and contract all remove or disable it honestly.

### P3 — Optimization and continuous improvement

A P3 issue improves maintainability, efficiency, observability, or future scale without blocking a narrowly scoped accepted release.

### Release gates

| Gate | Meaning | Permitted data/use |
|---|---|---|
| G0 — Development | Code may be incomplete; focused checks pass | Synthetic data only |
| G1 — Security-contained staging | All P0 PRs merged and authorization matrix green | Synthetic or approved non-sensitive test data |
| G2 — Feature-complete pilot candidate | Contracted P1 workflows are real; demo/no-op paths removed | Controlled school pilot after legal approval |
| G3 — Target-host accepted | Replacement-host restore, load, offline browser, operations and security acceptance completed | Limited production pilot |
| G4 — Contract-ready release | Technical, operational, privacy, support and contractual packages approved | Production under the signed scope |

No AI may advance the classification merely because all GitHub checks are green. The corresponding gate evidence must exist.

---

## 3. International standards and authoritative guidance baseline

Future AI agents must verify the then-current official versions before implementation. As of the audit date, use the following baseline:

| Standard/guidance | Application to EduTalent |
|---|---|
| OWASP ASVS 5.0.0 | Use Level 2 as the general web-application verification baseline and selected Level 3 rigor for tenant boundaries, administration, authentication, cryptography, sensitive student data, and high-value operations. Reference requirements with the version prefix when practical. |
| OWASP API Security Top 10 — 2023 | Especially API1 Broken Object Level Authorization, API2 Broken Authentication, API3 Broken Object Property Level Authorization, API4 Resource Consumption, API5 Broken Function Level Authorization, API8 Security Misconfiguration, API9 Inventory Management, and API10 Unsafe Consumption of APIs. |
| NIST SP 800-218 SSDF 1.1 | Prepare the organization, protect software, produce well-secured software, and respond to vulnerabilities. Each security defect must create a regression control, not only a one-time patch. |
| NIST SP 800-63-4 / SP 800-63B-4 | Authentication, authenticator lifecycle, session invalidation, recovery, account disablement, and future MFA decisions. |
| ISO/IEC 27001:2022 plus Amendment 1:2024 | Risk-managed information-security management, roles, change control, incident response, supplier and operations governance. This plan does not claim certification. |
| ISO/IEC 27701:2025 | Privacy information management, controller/processor responsibilities, data inventory, retention, rights and privacy operations. This plan does not claim certification. |
| GDPR Articles 25 and 32, where applicable | Data protection by design/default, data minimization, and security appropriate to risk. |
| Swiss FADP and FDPIC guidance, where applicable | Outsourced processing, processor instructions, international transfer assessment, breach and high-risk processing obligations. Obtain legal review. |
| WCAG 2.2 | Target conformance: WCAG 2.2 AA for the contracted browser UI, including focus visibility, target size, accessible authentication, predictable navigation, errors and keyboard operation. |
| SLSA v1.2 Build track | Preserve verifiable provenance and signed release artifacts; target hosted-build signed provenance equivalent to Build L2 or stronger for published artifacts. |
| CIS Docker Benchmark 1.8.0 | Target-host and daemon/container configuration review. Tailor findings explicitly; do not claim full conformance without assessment. |
| OWASP File Upload guidance | Allowlisted types, size limits, signature/content validation, safe names, storage isolation, malware scanning, parser isolation and authorization. |

### Standards interpretation rule

Standards guide the security objective and evidence. They do not dictate a blind implementation. For each mapped control, record:

- the risk;
- the implemented control;
- the automated evidence;
- the manual evidence;
- residual risk;
- owner and review date.

---

## 4. Binding architectural invariants

The following must remain true throughout all PRs:

1. EduTalent is an offline-first school appliance.
2. Only the local AI Gateway may have approved external AI egress.
3. Browser code, the app, PostgreSQL, Supabase services, Qdrant, Storage, and document workers have no general internet access.
4. External AI unavailability is not a core health failure.
5. One authoritative Supabase PostgreSQL database stores application, identity, authorization, audit, queue, and Storage metadata.
6. Tenant authorization is enforced server-side and, after PR-03, by transaction-scoped PostgreSQL RLS using a `NOBYPASSRLS` application role.
7. Every endpoint that accepts an object identifier must authorize the action on that exact object.
8. Frontend route guards are usability controls, never the authoritative security boundary.
9. Teachers cannot bypass the governed manager/admin knowledge-publication flow.
10. PostgreSQL authorization precedes Qdrant retrieval, and Qdrant receives exact authorized school/asset/publication filters.
11. Unpublished and archived assets are not retrievable.
12. All long-running or retryable AI/document jobs are durable, idempotent, observable, and recoverable.
13. Production never exposes internal service ports to the host except the approved gateway ports.
14. Secrets, student data, PDFs, dumps, private keys and generated environment files are never committed or uploaded as ordinary CI artifacts.
15. A signed release artifact must trace to the exact source and build identity through checksums, SBOM, signatures and provenance.
16. Production UI never shows plausible fictional data as if it belonged to a real user.
17. A visible control either works end to end, is clearly disabled as unavailable, or is absent.

---

## 5. Cost-aware CI and workflow architecture

The repository uses one physical self-hosted runner and one job runs at a time. Repeating checkout, toolchain installation, Docker builds and full-stack startup in multiple workflows creates queue time without additional evidence. CI must therefore be risk-directed.

### 5.1 Required workflow tiers

#### Tier A — Exact-head targeted proof on every ordinary PR commit

Use the existing `AI Change Proof` / `AI change gate` model as the single globally required PR check.

It must always run and must:

1. Compare the correct base and exact head.
2. Classify changed paths into at least:
   - `rust_workspace`;
   - `api`;
   - `web_ui`;
   - `auth_authorization`;
   - `database_migration`;
   - `browser_e2e`;
   - `production_topology`;
   - `operations`;
   - `packaging_release`;
   - `docs_only`.
3. Escalate workflow-policy changes to the complete PR diff so the classifier cannot be bypassed by editing its own rules.
4. Run the smallest targeted validation described for the PR below.
5. Write a small machine-readable evidence artifact with exact head SHA, classifier outputs, commands executed, job outcomes and intentional omissions.
6. Fail if a required targeted section was skipped.

Because the runner is serial, prefer one targeted-validation job per PR head where practical. Start PostgreSQL or browser infrastructure only when the classifier requires it. Do not start a PostgreSQL service container for a docs-only or UI-style-only change.

#### Tier B — Final exact-head validation

Run `Full Validation` only when implementation is stable and the `full-validation` label is applied or an equivalent protected final-review event is used.

It should prove:

- full migration replay and database invariants;
- full workspace/server compile;
- full correctness/suspicious Clippy policy;
- all unit and integration tests;
- the critical browser journey suite when product/auth/UI code is in scope;
- exact-head gate evidence.

Do not run this after every small commit.

#### Tier C — Specialized production proof

Run only when affected:

- `Production Foundation`: production Compose, gateway, database-role, network, secret/TLS, AI topology or production scripts.
- `Production Operations`: backup, restore, WAL/PITR, monitoring, alert, load, fault, upgrade or operations scripts.
- `Package`: Dockerfile, runtime image, release Compose, installer or package definitions.
- `Air-gapped Appliance`: image inventory, model artifacts, offline installer, manifest, SBOM, signatures, provenance, multi-architecture or no-pull startup.

These workflows must not run for ordinary API authorization or UI business-logic PRs.

#### Tier D — Final release/mirror proof

`Mirror Final Proof` or equivalent orchestration runs once on the stable exact release candidate, sequentially reusing caches. It must not duplicate heavy work already proven on a different SHA, and it must never accept older/canceled evidence.

### 5.2 Required workflow engineering rules

- Use `concurrency` keyed by workflow and PR/ref with `cancel-in-progress: true` for ordinary PR validation so stale commits do not consume the single runner.
- Do not cancel protected release publication after signing/publishing begins; use a separate release concurrency group.
- Use path classification inside the always-running required workflow. A path-filtered workflow should not be a globally required branch-protection check because a filtered skip can remain pending.
- Use reusable workflows or shared scripts for repeated Rust/database/browser setup; do not copy security gates between workflow files.
- Cache Rust by toolchain, `Cargo.lock`, target profile and validation scope.
- Reuse one BuildKit cache scope for runtime/package/appliance builds.
- Preinstall or persist the pinned browser runtime on the self-hosted runner; do not download browsers on every E2E run.
- Mock OpenAI/LLM behavior locally in PR tests. Never call paid or live external AI providers from routine CI.
- Upload compact evidence on success. Upload verbose logs, screenshots, traces and diffs on failure. Ordinary retention: 3–7 days; final/release evidence: 14–30 days according to policy.
- Keep security scanners pinned. Do not reduce severity or excluded paths to hide findings.
- A scheduled weekly operations/recovery drill is appropriate; it must not block unrelated PR commits.

### 5.3 Workflow selection table

| Change category | Every commit | Final-review only | Release only | Must not run routinely |
|---|---|---|---|---|
| API logic/auth | AI gate + targeted API/database tests | Full Validation | None | Package, appliance, operations |
| Migration/RLS/DB role | AI gate + migration/security tests | Full Validation + relevant DB live proof | None | Full appliance unless packaging changed |
| UI business logic | AI gate + web check + focused browser smoke | Full Validation + critical browser suite | None | Production operations, appliance |
| Docker/runtime/package | AI gate + package definitions + targeted build | Full Validation + complete package | Signed release proof | Unrelated DB recovery |
| Production Compose/security | AI gate + topology rendering | Production Foundation complete | Appliance only if inventory changed | Browser full suite unless UI changed |
| Backup/recovery/monitoring | AI gate + definition/regression tests | Production Operations complete | Release evidence as required | Full Rust/browser if untouched |
| Documentation only | AI gate + docs/link/secret lint | Optional editorial review | None | Rust, Docker, database, browser |

---

# 6. Ordered implementation PRs

The order is mandatory unless a current-code inspection proves a dependency already resolved. P0 work must be completed before feature expansion.

---

## PR-01 — P0: Assignment object-level and function-level authorization

### Why this is first

Assignment endpoints were observed accepting client-supplied assignment/student IDs without consistently proving the authenticated actor owns the assignment, teaches the class, belongs to the same school, or is allowed to perform the function. This is a direct OWASP API1/API5 risk and is amplified by the current elevated application database role.

### Required implementation

1. Introduce or reuse one server-only authorization abstraction, for example:

```rust
struct AuthorizedTeacher {
    user_id: Uuid,
    teacher_id: Uuid,
    school_id: Uuid,
    pool: Arc<PgPool>,
}
```

2. The abstraction must:
   - extract the authenticated `UserInfo` from request extensions;
   - require the exact canonical role `Teacher`;
   - resolve the active user, school and teacher record;
   - reject inactive users;
   - fail closed when any relationship is missing.
3. Replace ID-only repository methods used by server functions with actor-scoped methods. Examples:
   - `find_for_teacher(assignment_id, teacher_id, school_id)`;
   - `publish_for_teacher(...)`;
   - `update_for_teacher(...)`;
   - `delete_for_teacher(...)`;
   - `personalize_for_teacher_and_student(...)`.
4. Scope the SQL predicate itself. Do not fetch an object by ID and then rely only on an application comparison.
5. Publishing must verify:
   - the assignment belongs to the authenticated teacher;
   - the class belongs to the same school;
   - the teacher has an active teaching assignment to the class;
   - every fan-out student is actively enrolled in that class and school.
6. Student personalization must verify the target student is enrolled in the assignment's authorized class and school.
7. Preserve transactionality and idempotency of publication. Repeated publish must not create duplicate custom assignments.
8. Use explicit DTOs; do not return internal properties that are not needed by the caller.
9. Audit every assignment endpoint, including get, list, create, update, publish, personalize, archive/delete and teacher grading-related paths.
10. Return consistent `Unauthorized`, `Forbidden`, `NotFound`, and conflict errors without revealing cross-tenant existence.

### Required regression tests

Create a database-backed authorization matrix with at least:

- Teacher A can create/read/update/publish own assignment.
- Teacher A cannot read/update/publish Teacher B's assignment in the same school.
- Teacher A cannot access any assignment in School B.
- A student cannot call teacher assignment mutation endpoints.
- A parent cannot call teacher assignment mutation endpoints.
- A school manager cannot call teacher-only endpoints unless an explicit separate manager operation exists.
- An inactive teacher is denied.
- A teacher cannot personalize for a non-enrolled student.
- A teacher cannot personalize for a student in another school.
- A random existing UUID and random nonexistent UUID both fail without cross-tenant disclosure.
- Repeated publish is idempotent or returns a documented conflict without duplicates.
- Concurrent publish attempts preserve one fan-out set.

### Smallest sufficient workflow

Run on every PR-01 head:

- change classifier;
- rustfmt for changed Rust files;
- migrations applied and replayed against PostgreSQL;
- `cargo check -p api --features server --all-targets --locked`;
- targeted Clippy for API;
- API unit tests;
- assignment authorization integration test suite;
- `cargo check -p web --features server --locked` only because server-function signatures affect generated client stubs;
- AI change gate evidence.

Do **not** run:

- production image build;
- Production Foundation complete stack;
- Production Operations;
- Package complete bundle;
- Air-gapped Appliance;
- multi-architecture builds.

### Exit gate

All authorization cases pass on the exact head; repository methods no longer expose ID-only mutations used by public server functions; no unresolved review thread identifies a missing assignment scope.

---

## PR-02 — P0: Session lifecycle, inactive-account enforcement and production API cleanup

### Risks removed

- A user disabled after login may continue through access-token or refresh-token middleware.
- Notification functions use inconsistent token-passing patterns despite HttpOnly-cookie authentication.
- Legacy submission server endpoints can return fake success without persistence or authorization.
- Development-only endpoints may remain reachable in production.

### Required implementation

1. In authentication middleware, after validating a token:
   - fetch the user from PostgreSQL;
   - require `is_active = true`;
   - verify school and role are still valid;
   - reject or clear cookies on inactive/deleted user.
2. Apply the same check after refresh-token rotation before inserting request identity.
3. Add administrative session revocation semantics:
   - preferred: store an account/session version or `disabled_at` and reject tokens issued before it;
   - at minimum, database active-state checking on every authenticated request and refresh.
4. Ensure logout removal cookies preserve the original security attributes needed for reliable deletion.
5. Refactor notification functions to derive identity from middleware request extensions. Remove `auth_token` parameters from browser-callable server functions.
6. Remove or fully implement legacy submission endpoints. Production must not expose endpoints that return `created`, `updated`, empty data or success without performing the operation.
7. Remove the production `echo` server function or compile it only under an explicit development/test feature that is absent from the release image.
8. Add a server-function inventory test that fails when a production endpoint is registered without an authorization classification.
9. Standardize error responses and avoid logging passwords, tokens or sensitive auth-provider bodies.
10. Add rate-limit design for login and refresh. If not implemented here, create a linked P1 task with an explicit release gate.

### Required tests

- Login succeeds for active provisioned user.
- Login fails for inactive user.
- User logs in, admin disables account, next authenticated request fails.
- Disabled account cannot refresh.
- Deleted account cannot refresh.
- Cookie rotation keeps HttpOnly, Secure, SameSite and path attributes.
- Logout invalidates both access and refresh cookies.
- Notification list/summary/read operations work via cookie-derived identity and cannot accept another user's token or ID.
- Legacy submission routes are absent or execute real authorized behavior.
- Production route inventory contains no `echo` or development stub.
- Invalid auth-provider response does not leak provider body to the client/log artifact.

### Smallest sufficient workflow

- AI change gate;
- API/web targeted compile;
- PostgreSQL-backed auth/session integration tests;
- local mock Supabase Auth/JWKS/refresh service—no live external service;
- focused browser smoke: login, refresh, disable account, logout;
- route/API inventory test.

Do **not** run production/appliance workflows unless Docker or production auth configuration is changed. If cookie or Caddy security headers change, run only production topology rendering, not the complete appliance.

### Exit gate

An account disablement takes effect on the next request and refresh; browser-callable production stubs are gone; all user-scoped notification operations use the server session.

---

## PR-03 — P0: Transaction-scoped RLS and `NOBYPASSRLS` application role

### Why this is a separate architecture PR

The current production design documents a long-running application role with `BYPASSRLS`. A helper sets transaction-local authorization context through a pool, but subsequent queries may execute on another connection. This cannot be treated as reliable defense in depth.

### Required design

1. Change the long-running application role to:
   - `NOSUPERUSER`;
   - `NOCREATEDB`;
   - `NOCREATEROLE`;
   - `NOREPLICATION`;
   - `NOINHERIT` as appropriate;
   - **`NOBYPASSRLS`**.
2. Introduce a request-scoped database transaction abstraction:

```rust
pub struct AuthorizedTx<'a> {
    actor: AuthorizedActor,
    tx: Transaction<'a, Postgres>,
}
```

3. Begin the transaction, set transaction-local context, then execute all protected queries through that same transaction.
4. Never call `SET LOCAL` through a pool and then issue protected queries through the pool.
5. Define canonical context values: user ID, role, school ID and any needed elevated operation flag.
6. Ensure pooled connections return without tenant context after commit/rollback.
7. Review every RLS policy and force RLS on browser/client tenant tables. Define explicit policies for the application role rather than relying on ownership.
8. Separate operations that genuinely require elevated migration/admin authority into one-shot containers or tightly scoped functions; never grant that authority to the web/worker runtime.
9. Update database-role configurator, migration scripts, production checks, ADR and threat model.
10. Add a safe migration/upgrade path:
    - apply policies and grants;
    - test with new role;
    - switch app role;
    - rollback procedure if a legitimate operation is blocked.

### Required tests

- Same pooled connection cannot leak School A context into School B request.
- Concurrent School A and B transactions return only their own records.
- Context is absent after commit and rollback.
- Application role reports `rolbypassrls = false`.
- Application role cannot create roles, databases, schemas or change migration registry.
- Direct IDs cannot retrieve another tenant through every RLS-protected table.
- Background worker sets an explicit authorized system/job context rather than bypassing RLS.
- Migration and database-role configuration remain idempotent.
- Production database-check proves the live app uses the `NOBYPASSRLS` role.

### Smallest sufficient workflow

Every commit:

- migration apply and replay;
- schema/RLS verification scripts;
- database role configurator regression;
- API compile/tests;
- dedicated concurrent RLS integration suite;
- production Compose **definition rendering only** if role environment/commands changed.

Final-review only:

- Full database validation;
- full Rust validation;
- live Production Foundation database-role proof.

Do **not** run backup/PITR, full package or appliance on every commit. Run Package definition validation only if Docker entrypoint/image commands changed.

### Exit gate

The live production app and worker use a `NOBYPASSRLS` role; all protected queries execute through the transaction carrying the actor context; cross-tenant concurrent tests pass.

---

## PR-04 — P0/P1: Repository-wide endpoint authorization inventory and negative matrix

### Objective

Known assignment defects are evidence of a systemic risk. This PR prevents the same class from remaining in class, user, school, invite, report, grade, submission, knowledge, notification and administration modules.

### Required implementation

1. Inventory every browser-callable route and `#[server]` function into a version-controlled manifest containing:
   - endpoint name/path;
   - allowed roles;
   - tenant scope;
   - object scope;
   - write/read classification;
   - rate/resource classification;
   - audit requirement;
   - implementation owner.
2. Build a shared deny-by-default authorization layer. Public functions must explicitly opt into `Public` or a named policy.
3. Add repository query conventions:
   - actor-scoped methods for sensitive operations;
   - no public ID-only mutation methods;
   - explicit DTOs;
   - pagination and maximum limits;
   - school/ownership predicates in SQL.
4. Add a test that compares discovered server endpoints to the manifest and fails on an unclassified endpoint.
5. Build negative tests for each role pair and tenant pair, prioritizing reads/writes involving:
   - users and invitations;
   - classes/enrollments;
   - assignments/submissions/grades;
   - parent-child links;
   - reports;
   - knowledge assets/jobs/search;
   - notification records;
   - platform administration.
6. Verify object-property authorization: clients cannot set role, school, publication, grading, ownership, status or audit fields not explicitly allowed.
7. Verify maximum page sizes, query lengths and mutation payload sizes.
8. Record residual exceptions with owner and expiry. No permanent undocumented exception.

### Required workflow

- AI gate;
- endpoint inventory generation/diff check;
- full API targeted tests;
- PostgreSQL authorization matrix;
- web server compile;
- no Docker/package/appliance.

At final review, run Full Validation because this PR touches broad API security boundaries.

### Exit gate

Every production endpoint is classified and covered by at least one positive and one negative authorization assertion; no unclassified endpoint can merge.

---

## PR-05 — P1: Durable assignment personalization queue

### Problem

Assignment publication starts AI personalization with an in-process background task. A container restart can lose the task or leave only some students personalized.

### Required implementation

1. Add a durable `assignment_personalization_jobs` table or generalize the existing durable knowledge-job infrastructure.
2. Required job fields:
   - stable job ID;
   - school ID;
   - assignment ID;
   - target student or class scope;
   - status;
   - attempt count;
   - available-at/backoff;
   - lease owner and heartbeat;
   - last error code/safe summary;
   - created/started/completed timestamps;
   - idempotency key;
   - model/profile/version.
3. Publishing commits assignment state, custom-assignment fan-out and job enqueue atomically.
4. Worker claims jobs with bounded leases and `SKIP LOCKED` or equivalent safe concurrency.
5. Make personalization idempotent per assignment/student/profile.
6. Provider/gateway failure returns the job to retry with bounded exponential backoff and terminal policy.
7. A restart reclaims stale leases.
8. Expose safe operator metrics and a school-scoped status in the UI without prompts, secrets or raw provider bodies.
9. Preserve core operation when AI is unavailable. Publication must not fail because the provider is down unless the product requirement explicitly makes personalization mandatory.
10. Add cancellation/retry semantics for archived or superseded assignments.

### Tests

- Atomic publication and enqueue.
- Duplicate publication does not duplicate jobs.
- Worker restart reclaims stale job.
- Provider timeout/429/invalid JSON/outage retries safely.
- Partial class completion resumes remaining students.
- Cross-school worker/query isolation.
- Job cannot personalize a student no longer authorized/enrolled without a defined policy.
- No prompt/provider secret in logs or job error fields.

### Smallest sufficient workflow

- AI gate;
- migrations and replay;
- API worker integration tests using a local mock AI Gateway;
- fault/restart unit/integration test;
- web compile only if status UI/server DTO changes.

Do not run full Production Operations or appliance. Run AI Gateway targeted tests only; no real provider and no paid API.

### Exit gate

No assignment personalization depends on `tokio::spawn` survival; restart/outage evidence proves eventual, bounded, idempotent completion.

---

## PR-06 — P1: Product truthfulness, canonical routing and placeholder/no-op containment

### Objective

Before implementing large product domains, remove the risk of showing fictional data or exposing controls that pretend to work.

### Required implementation

1. Make `/dashboard` and direct role routes use one canonical role-aware dashboard implementation.
2. Remove student/parent “under development” routes when a real dashboard exists; redirect or render the canonical component.
3. Create a feature-capability registry controlled server-side and included in deployment/release metadata.
4. For incomplete features, choose exactly one:
   - remove navigation and route;
   - render a clearly labelled unavailable page with no fictional data;
   - enable only under a non-production demo feature that cannot be present in release artifacts.
5. Replace all hardcoded user-specific values, dates, names, scores, GPA, attendance, schedule, messages and report history with real data or explicit unavailable/empty states.
6. Add a static regression scanner for release source that detects:
   - `onclick: move |_| {}` and equivalent no-op handlers;
   - “coming soon” in enabled production feature paths;
   - known fictional names/dates;
   - hardcoded academic metrics;
   - placeholder server values;
   - development-only routes/endpoints.
7. Allow documented exceptions only in test fixtures, Storybook/demo assets, translations or explicitly excluded modules.
8. Error states must distinguish unavailable, no data, forbidden and load failure without leaking internals.
9. Ensure disabled controls are not keyboard-focus traps and include an accessible explanation.

### Tests

- Every role's direct route reaches canonical dashboard or valid access denied.
- Release build contains no known fictional student/person data.
- No enabled button has a no-op handler.
- Incomplete feature routes are unavailable and absent from navigation.
- Empty/error/loading states render correctly.
- Persian/RTL and English labels remain coherent.

### Smallest sufficient workflow

- AI gate;
- web format/check/clippy/tests;
- static placeholder/no-op scanner;
- focused Playwright smoke for role routes/navigation/disabled features;
- API compile only if capability DTO/server function changes.

Do not run database, production, package or appliance unless the feature registry changes schema or runtime packaging.

### Exit gate

The production UI is truthful: no fabricated user data, no misleading successful action, and no duplicate placeholder role route.

---

## PR-07 — P1: Attendance domain and workflows

### Scope decision

Attendance must be implemented if it appears in the contracted product. Otherwise remove it through PR-06 and explicitly exclude it from the contract. Never keep the current fixed 95% value.

### Required domain model

At minimum define:

- school calendar and instructional days;
- class/session/period identity;
- student enrollment at the event date;
- attendance status enum: present, absent, late, excused, school activity, unknown as approved;
- recorded by, source and timestamp;
- correction history and reason;
- optional manager approval policy;
- timezone and local-date semantics;
- absence notes with privacy classification;
- aggregation rules and grading/report impact.

### Required implementation

1. Migrations with tenant keys, indexes, uniqueness and audit history.
2. Teacher can record attendance only for classes they teach and students enrolled for that date.
3. School manager can correct/approve according to explicit policy.
4. Student and parent can read only authorized records.
5. Parent-child link must be verified in the database query.
6. Aggregates derive from records and school calendar, never constants.
7. Define handling for enrollment changes, deleted classes, holidays, duplicate sessions, timezone transitions and late corrections.
8. Audit every correction without exposing sensitive notes broadly.
9. UI includes empty/error/loading, correction history as authorized, and accessible input controls.
10. Reports consume the same canonical attendance service.

### Tests

- Complete role/tenant matrix.
- Duplicate attendance prevention.
- Teacher cannot record unrelated class/student.
- Parent cannot see unrelated child.
- Date/timezone and calendar boundaries.
- Correction audit and immutable original event.
- Aggregation accuracy.
- Concurrent update conflict behavior.
- Migration replay and rollback.

### Smallest sufficient workflow

- AI gate;
- migrations + attendance schema/security verification;
- API tests;
- web tests;
- focused browser journey: teacher records, manager corrects, student/parent reads;
- no production/appliance.

Final review: Full Validation and critical browser suite.

### Exit gate

All attendance values displayed by the product are derived from authorized records with an auditable correction history.

---

## PR-08 — P1: Timetable, school calendar and student schedule

### Required implementation

1. Model academic terms, school days, holidays, periods, rooms, class meetings, teacher assignments and exceptional events.
2. Define recurrence and exception behavior without storing only presentation strings.
3. Resolve schedules using school timezone and local dates.
4. Student schedule queries must verify enrollment and publication/visibility.
5. Teacher schedule must verify teaching assignment.
6. Manager mutations must be school-scoped and detect room/teacher/class conflicts.
7. Assignment due dates and examinations may appear as related events but remain distinct domain records.
8. Remove all fixed March 2025 schedule and important-date data.
9. Add calendar/timetable indexes and bounded date-range queries.
10. Define export format only if included in release scope.

### Tests

- Cross-school and cross-role authorization.
- Term/date range and timezone behavior.
- Holiday/exception overrides.
- Conflict detection.
- Enrollment effective-date behavior.
- Empty week and no-class day.
- Responsive/RTL schedule rendering.

### Smallest sufficient workflow

- AI gate;
- migrations/API/web targeted validation;
- focused schedule browser tests;
- no Docker/appliance.

### Exit gate

Every student schedule item comes from the canonical authorized timetable/calendar service; no static dates remain in production paths.

---

## PR-09 — P1: Reports, deterministic exports and parent authorization

### Required implementation

1. Define the exact report catalogue for the first release. Do not advertise behavior/standardized-test/attendance reports without source data.
2. Introduce a report request/snapshot model containing:
   - school;
   - requesting actor;
   - authorized subject/student/class;
   - report type/version;
   - period and timezone;
   - source data watermark;
   - generation status;
   - output checksum, media type and size;
   - created/expiry timestamps.
3. Authorize parents through the parent-child relationship at query and download time.
4. Authorize managers/teachers by school, role and object scope.
5. Generate reports asynchronously when expensive; make jobs durable and idempotent.
6. Generate deterministic PDF/CSV/XLSX only for explicitly supported formats. Pin fonts/templates/libraries in the release.
7. Store output in private local Storage with short-lived authorized download or server streaming.
8. Record generation and download audit events.
9. Apply retention/deletion policy and regenerate rather than serving stale unauthorized artifacts.
10. Connect every Generate/Export/Download Again button to real state and error handling.
11. Remove fictional report history and names.
12. Protect against spreadsheet formula injection in CSV/XLSX exports.

### Tests

- Parent cannot request/download unrelated child report.
- School A cannot access School B report ID.
- Role/type authorization matrix.
- Stable output checksum for fixed fixture data.
- Private storage; direct unauthenticated URL fails.
- Expired/revoked relationship prevents download.
- Formula-injection strings are neutralized.
- Empty report and large report bounds.
- Browser generation/download/error flow.

### Smallest sufficient workflow

- AI gate;
- migrations/API/report unit and integration tests;
- deterministic fixture export tests without full Docker image;
- focused browser report journey;
- web tests.

Run Package definition validation only if the runtime image adds native/font/export dependencies. Defer complete image build to final review for that PR.

### Exit gate

All enabled report controls produce authorized, auditable, deterministic output; unsupported report types are absent.

---

## PR-10 — P2 unless contracted: Parent/teacher communication

### Product decision first

Messaging creates significant privacy, safeguarding, moderation, retention and support obligations. Either implement this PR fully or remove communication from the first release and contract through PR-06.

### Required implementation when included

1. Model conversation/thread, participants, message, school, child context, read state, archive state, retention, sender and timestamps.
2. Determine allowed communication pairs and school policy. Deny arbitrary recipient IDs.
3. Verify parent-child and teacher-class relationships at send and read time.
4. Add server-side body length, content and attachment policy.
5. If attachments are allowed, use the governed upload/quarantine pipeline; otherwise prohibit them.
6. Add audit events and abuse/reporting controls appropriate to the jurisdiction and school policy.
7. Decide immutable retention versus edit/delete behavior.
8. Provide notification semantics without exposing content in insecure channels.
9. Replace all hardcoded example messages and empty handlers.
10. Add rate limits and anti-automation controls for sends.

### Tests

- Complete participant authorization matrix.
- Cross-school and unrelated-parent denial.
- Recipient manipulation.
- Oversized/content/attachment rejection.
- Rate limits.
- Archive/read state is user-scoped.
- Retention and audit.
- Browser compose/send/reply/archive/error flow.

### Smallest sufficient workflow

- AI gate;
- migrations/API/web tests;
- focused messaging browser suite;
- no production/appliance unless local mail/notification infrastructure changes.

### Exit gate

Messaging is either fully authorized, persisted and auditable, or completely absent from the production release scope.

---

## PR-11 — P1: Browser offline integrity, CSP and frontend supply chain

### Problem

The login UI was observed dynamically importing executable browser code from a public CDN. That conflicts with the offline-first appliance, weakens supply-chain control and complicates a strict Content Security Policy.

### Required implementation

1. Remove all runtime CDN/module/font/script/style imports.
2. Bundle required assets into the signed release or remove nonessential effects.
3. Produce an inventory of browser network destinations and enforce zero external destinations in offline mode.
4. Add a restrictive CSP at the gateway/application level:
   - default deny;
   - no arbitrary remote scripts;
   - avoid `unsafe-eval`;
   - minimize or eliminate `unsafe-inline` through nonces/hashes or bundled styles;
   - restrict connect, image, font, frame, object and base origins.
5. Add security headers appropriate to the deployment: HSTS after domain/TLS acceptance, frame restrictions, MIME sniffing protection, referrer policy and permissions policy.
6. Ensure Dioxus hydration and server functions operate under CSP.
7. Pin frontend dependencies and include them in SBOM/provenance.
8. Browser tests must run with outbound networking blocked and fail on unexpected requests.
9. Add a CSP report-only development mode only if it cannot weaken production.
10. Verify no provider keys, secret keys, internal tokens or private service URLs are present in WASM/static assets.

### Tests

- Login and all critical UI render with external DNS/network disabled.
- Request interception reports zero unexpected origins.
- CSP has no violations on critical routes.
- WASM/static secret scan.
- Progressive enhancement login still works before hydration.
- No UI dependency is downloaded on first start.

### Smallest sufficient workflow

- AI gate;
- web build/check/tests;
- offline Playwright smoke with request allowlist;
- static asset secret/origin scan;
- Caddy/production definition rendering if headers change.

Run Package definition validation if asset packaging/Dockerfile changes. Do not run full appliance on each commit; run complete no-pull appliance once at final review of the stable PR if the artifact inventory changed.

### Exit gate

Critical browser journeys work with all external browser network access blocked, and the release has an enforced CSP with no runtime public-CDN dependency.

---

## PR-12 — P1: Browser E2E, WCAG 2.2 AA, RTL and product acceptance suite

### Objective

Compilation and unit tests do not prove that a user can operate the product. Establish browser evidence without making every commit run an expensive full matrix.

### Required implementation

1. Add a pinned Playwright-based E2E project or an equivalent deterministic browser harness.
2. Provide synthetic seed fixtures for at least:
   - PlatformAdmin;
   - SchoolManager A and B;
   - Teacher A and B;
   - Student A and B;
   - Parent A and B;
   - active and inactive accounts;
   - classes, assignments, submissions, grades and knowledge assets.
3. Implement two browser tiers:
   - **PR smoke:** login, role landing, one critical changed-feature path, authorization denial, logout;
   - **final suite:** all contracted workflows, roles, browsers/viewports/languages and failure cases.
4. Tests must manipulate direct URLs and object IDs to prove backend denial, not only hidden buttons.
5. Add automated accessibility checks, then manual keyboard/screen-reader acceptance for critical journeys.
6. Target WCAG 2.2 AA, including focus order/visibility, target size, accessible authentication, labels, errors, status announcements, contrast and keyboard operation.
7. Test English and Persian/RTL layouts, date/number direction and modal/focus behavior.
8. Capture screenshots, trace and console/network logs only on failure for routine PRs; preserve final evidence.
9. Fail on browser console errors, unhandled promise/WASM errors and unexpected external network calls.
10. Ensure tests run against the production-like server build, not a mock-only UI.

### Smallest sufficient workflow

For ordinary UI/auth PR commits:

- one browser engine;
- desktop plus one mobile viewport only when responsive code changes;
- only tagged `@smoke` and changed-feature tests;
- local synthetic database;
- failure artifacts.

For full-validation label:

- complete critical journey suite;
- English and Persian/RTL;
- desktop and mobile;
- accessibility scan;
- offline network policy.

Do not run Docker appliance, backup/PITR or multi-architecture builds for browser-only changes.

### Exit gate

Every contracted feature has at least one end-to-end positive journey and relevant negative authorization journey; WCAG 2.2 AA issues have been fixed or explicitly risk-accepted with owner/date.

---

## PR-13 — P1: Target-host hardening, deployment qualification and maintenance automation

### Objective

CI proves a model environment. A school contract requires evidence on the actual supported host and operational procedures.

### Required implementation and evidence

1. Define supported host baseline:
   - Linux distribution/version;
   - kernel and filesystem requirements;
   - Docker Engine and Compose versions;
   - rootless Docker expectations;
   - CPU/RAM/storage minimum and tested capacity;
   - time synchronization;
   - DNS/TLS requirements.
2. Create host-preflight checks for:
   - disk layout/free space/inodes;
   - filesystem encryption expectation;
   - required ports and firewall;
   - DNS resolution;
   - clock skew;
   - Docker configuration;
   - CPU/memory limits;
   - backup mount and permissions;
   - operator account and secret modes.
3. Review the target against the current CIS Docker Benchmark and record tailored pass/fail/not-applicable evidence.
4. Record each container's:
   - immutable image digest;
   - user/group;
   - capability set;
   - writable paths;
   - read-only root state;
   - PID/resource limits;
   - restart policy;
   - health check;
   - networks and published ports;
   - seccomp/AppArmor profile as applicable.
5. Decide high availability honestly. The existing single server is not HA. Either:
   - contract it as single-node with documented interruptions and recovery targets; or
   - design and prove a separate HA architecture.
6. Install and validate systemd timers/services for monitoring, backups, restore verification and WAL handling.
7. Establish off-host encrypted backup copy and passphrase escrow separate from backup media.
8. Run on a clean replacement host:
   - installation;
   - production validation;
   - live security checks;
   - full encrypted backup;
   - restore;
   - PostgreSQL PITR;
   - Qdrant recovery/reindex choice;
   - app/database restart;
   - disk-full/low-capacity fail closed;
   - expired/near-expiry TLS;
   - corrupted config;
   - failed migration rollback;
   - AI/Qdrant/provider outage;
   - load/soak using expected school scale.
9. Measure real RPO/RTO. Do not convert provisional objectives into contractual guarantees without results.
10. Document patching, certificate rotation, key rotation, Supabase/Qdrant/model upgrade and rollback procedures.

### Workflow strategy

Ordinary commits to host-preflight/Compose/security definitions:

- AI gate;
- shell/Python tests;
- rendered topology/security checks;
- no complete package unless image/runtime definitions changed.

Final stable PR:

- complete Production Foundation;
- complete Production Operations;
- Package only if runtime/package affected;
- Air-gapped Appliance only if appliance inventory/installer affected.

Replacement-host qualification is manual/controlled acceptance evidence and must not be simulated solely in CI.

### Exit gate

A named release is installed and restored on a clean supported host; actual RPO/RTO/load results and residual risks are signed off; single-node/HA status is explicit.

---

## PR-14 — P1 before contract: Documentation, privacy, security response and contract-readiness package

### Technical documentation

1. Reconcile all README/ADR/threat-model drift. Remove statements that contradict the implemented AI gateway or air-gapped appliance.
2. Create one release-versioned operator manual covering:
   - connected AI, degraded AI and fully offline modes;
   - supported host;
   - installation;
   - validation;
   - onboarding;
   - backup/restore;
   - monitoring/alerts;
   - patch/upgrade/rollback;
   - key/certificate rotation;
   - incident response;
   - offboarding and secure deletion.
3. Create administrator, teacher, parent and student guides only for enabled features.
4. Publish an exact feature matrix: implemented, disabled, optional and excluded.
5. Document API/server-function inventory and security architecture.

### Privacy and governance package

With qualified legal/privacy review, prepare:

- personal-data and processing-activity inventory;
- data-flow diagram including local services and external AI providers;
- controller/processor role allocation;
- data-processing agreement inputs;
- subprocessor/provider list;
- data location and international transfer assessment;
- retention/deletion schedule;
- data subject request procedures;
- breach/incident notification procedure;
- DPIA trigger and template for high-risk processing;
- AI use notice, purpose limitation and human oversight;
- school instructions for lawful account provisioning and parental/student notices;
- end-of-contract export, return and deletion process.

### Security organization package

Aligning with NIST SSDF and ISO/IEC 27001-style governance, define:

- vulnerability disclosure contact/process;
- supported-version policy;
- severity and remediation SLA;
- incident roles and escalation;
- security-update signing and release process;
- supplier/dependency review;
- access review and operator onboarding/offboarding;
- evidence retention;
- risk register with owner/expiry;
- penetration-test remediation process.

### Commercial scope package

Prepare for counsel/business approval:

- explicit proprietary software deployment/use grant;
- contracted features and exclusions;
- support hours and escalation;
- maintenance windows;
- availability definition;
- accepted backup/RPO/RTO responsibilities;
- customer hardware/network responsibilities;
- data export/offboarding;
- security incident cooperation;
- AI provider responsibility and outage behavior;
- acceptance procedure and warranty limitations.

### Smallest sufficient workflow

- AI change gate in docs-only mode;
- Markdown/style/link validation using local allowlisted checks;
- secret/PII pattern scan;
- verify feature matrix against server endpoint/capability manifest;
- no Rust, database, Docker, browser or appliance workflows unless code/config is also changed.

### Exit gate

No contradictory production documentation; the enabled product scope, privacy responsibilities, operational responsibility and support boundaries can be reviewed and signed without relying on undocumented assumptions.

---

## PR-15 — P0 release gate: Final exact-head production acceptance

This is not a feature PR. It is the release-candidate proof after all required preceding PRs are merged.

### Entry criteria

- No open P0.
- All contracted P1 features implemented or explicitly excluded and absent.
- All review threads resolved.
- Security/privacy/operational residual risks documented with owner and acceptance.
- Release candidate is frozen except fixes.

### Required exact-head automated sequence

Run expensive work sequentially to maximize the single runner and shared caches:

1. AI Change Proof gate.
2. Full database validation.
3. Full Rust compile, lint and tests.
4. Full critical browser/offline/accessibility suite.
5. Production Foundation complete proof.
6. Production Operations recovery/backup/load/security proof.
7. Package complete image/bundle proof.
8. Air-gapped Appliance:
   - exact image inventory;
   - SBOM;
   - signatures;
   - provenance;
   - model artifacts;
   - no-pull startup;
   - amd64 and native arm64 custom image build as required;
   - registry-disabled browser/core smoke.
9. Mirror Final Proof or equivalent exact-head evidence aggregation.

Do not start a second redundant heavy build on the same SHA. Orchestration must dispatch or reuse fresh exact-head run IDs and reject older/canceled/skipped required evidence.

### Required manual/external evidence

- independent authorization/security review;
- target-host replacement restore;
- penetration test and remediation disposition;
- browser keyboard/screen-reader check;
- school-scale load/soak;
- privacy/legal/contract sign-off;
- operator acceptance and incident rehearsal.

### Release decision

Classify exactly one:

- `not accepted`;
- `safe to continue developing`;
- `ready for final validation`;
- `ready for limited pilot`;
- `ready for contracted production`.

Green CI alone cannot yield the last classification.

---

# 7. Cross-cutting implementation checklists

## 7.1 Backend authorization checklist

For every server function and route:

- [ ] Is authentication required or explicitly public?
- [ ] Is active-account status checked?
- [ ] Is the function-level role checked server-side?
- [ ] Is school/tenant scope resolved from the authenticated database identity, not client input?
- [ ] Is object ownership/membership checked for the exact client-supplied ID?
- [ ] Is the SQL query scoped by actor/school/object?
- [ ] Are sensitive object properties allowlisted for read/write?
- [ ] Is the query executed in the request's transaction-scoped RLS context?
- [ ] Are response size, input size and pagination bounded?
- [ ] Does error behavior avoid cross-tenant existence disclosure?
- [ ] Is an audit event required and emitted?
- [ ] Are positive and negative multi-user/multi-school tests present?

## 7.2 UI connection checklist

For every visible control:

- [ ] Does it call a real authorized server operation?
- [ ] Does success mean the operation committed?
- [ ] Does it present loading, empty, error and forbidden states?
- [ ] Does it prevent duplicate submission while preserving retry?
- [ ] Does it refresh/invalidate affected data?
- [ ] Does direct URL/API manipulation remain secure?
- [ ] Is it keyboard operable with visible focus?
- [ ] Is its label/status announced accessibly?
- [ ] Does it work in English and Persian/RTL?
- [ ] Does it work with browser external networking blocked?
- [ ] Is all displayed personal/academic data real and authorized?

## 7.3 Database/migration checklist

- [ ] Migration is transactional where PostgreSQL permits.
- [ ] Migration registry checksum/integrity remains protected.
- [ ] Replay is idempotent according to repository policy.
- [ ] Rollback/failure leaves no partial object.
- [ ] Tenant keys and indexes exist.
- [ ] Uniqueness prevents duplicate business events/jobs.
- [ ] RLS is enabled/forced as required.
- [ ] Policies are tested for every role/tenant pair.
- [ ] Runtime role remains least privilege and `NOBYPASSRLS`.
- [ ] Migration/admin authority is absent from long-running app/worker.
- [ ] Backup/restore implications are documented.

## 7.4 Offline and supply-chain checklist

- [ ] No runtime CDN or unapproved external origin.
- [ ] No image/model pull on first startup.
- [ ] All artifacts have checksums and immutable manifest entries.
- [ ] SBOM covers custom images and release filesystem.
- [ ] Provenance identifies source and builder.
- [ ] Signatures verify the expected workflow/repository identity.
- [ ] Browser static assets contain no secrets/internal credentials.
- [ ] External AI destinations/models are fixed and allowlisted.
- [ ] Redirects and unsafe provider responses are rejected.
- [ ] Release uses no floating `latest` tag.

## 7.5 Operations and maintenance checklist

- [ ] Separate encrypted backup destination.
- [ ] Passphrase escrow separate from media.
- [ ] Continuous WAL reception and off-host copy.
- [ ] Daily full backup and verification.
- [ ] Weekly logical restore.
- [ ] Monthly PITR/Qdrant drill.
- [ ] Quarterly replacement-host restore.
- [ ] Disk/TLS/WAL/backup/database/core/Qdrant/AI alerts.
- [ ] Patch, certificate, key, Supabase, Qdrant and model upgrade runbooks.
- [ ] Measured RPO/RTO and school-specific capacity.
- [ ] Incident owner, escalation and evidence preservation.

---

# 8. Proposed reusable test assets

Future AI should prefer reusable security fixtures over copy-pasted tests.

### Synthetic tenant fixture

```text
School A
  Manager A
  Teacher A1, Teacher A2
  Student A1, Student A2
  Parent A1 linked only to Student A1
  Class A1 taught by Teacher A1
  Class A2 taught by Teacher A2

School B
  Manager B
  Teacher B1
  Student B1
  Parent B1 linked only to Student B1
  Class B1 taught by Teacher B1

Global
  PlatformAdmin
  inactive accounts for every major role
```

### Authorization test vocabulary

Every sensitive operation should use consistent test naming:

```text
allows_<role>_<action>_own_<object>
denies_<role>_<action>_same_school_unrelated_<object>
denies_<role>_<action>_cross_school_<object>
denies_inactive_<role>_<action>
denies_unclassified_role_<action>
does_not_disclose_cross_tenant_existence
```

### Browser tags

```text
@smoke
@auth
@authorization
@student
@teacher
@parent
@manager
@platform-admin
@rtl
@mobile
@offline
@accessibility
@final
```

The AI change classifier should select only relevant tags on ordinary commits and all critical tags at final validation.

---

# 9. Workflow command guidance

Use repository-defined commands when present. Do not blindly add parallel tools that duplicate existing scripts.

Typical targeted commands, adjusted to current repository policy:

```bash
cargo check -p api --features server --all-targets --locked
cargo clippy -p api --features server --lib --tests --locked -- \
  -A warnings -D clippy::correctness -D clippy::suspicious
cargo test -p api --features server --lib --locked
cargo check -p web --features server --all-targets --locked
cargo clippy -p web --features server --all-targets --locked -- \
  -A warnings -D clippy::correctness -D clippy::suspicious
cargo test -p web --features server --locked
bash scripts/ci/apply_migrations.sh
```

Add focused scripts rather than one giant opaque command, for example:

```text
scripts/ci/verify_endpoint_inventory.py
scripts/ci/verify_authorization_matrix.sh
scripts/ci/verify_rls_transaction_isolation.sh
scripts/ci/verify_no_production_placeholders.py
scripts/ci/verify_browser_asset_origins.py
scripts/ci/verify_release_capabilities.py
```

Each script must:

- use `set -euo pipefail` where shell is used;
- produce deterministic output;
- fail closed;
- avoid secrets/PII;
- write a small JSON evidence file when useful;
- have its own regression test where policy parsing is nontrivial.

---

# 10. Workflow anti-waste rules by PR

| PR | Required targeted proof | Full Validation? | Production Foundation? | Operations? | Package? | Appliance? |
|---|---|---:|---:|---:|---:|---:|
| PR-01 Assignment authorization | API + DB auth matrix + web stub compile | Final PR head | No | No | No | No |
| PR-02 Sessions/API cleanup | API auth + mock IdP + browser auth smoke | Final PR head | Definition only if headers/config changed | No | No | No |
| PR-03 NOBYPASSRLS | Migration/RLS/concurrency + live DB role | Yes | Complete at final head | No | Definitions if entrypoint changed | No |
| PR-04 Endpoint inventory | API full authorization matrix | Yes | No | No | No | No |
| PR-05 Durable personalization | Migration + worker + mock gateway fault tests | Final PR head | No | No | No | No |
| PR-06 Truthful UI | Web + scanner + focused browser | Final PR head | No | No | No | No |
| PR-07 Attendance | Migration + API/web + focused browser | Yes | No | No | No | No |
| PR-08 Timetable | Migration + API/web + focused browser | Yes | No | No | No | No |
| PR-09 Reports | API/export + browser | Yes | No | No | Only if runtime deps changed | No |
| PR-10 Messaging | Migration + API/web + browser | Yes | No | No | Only if runtime deps changed | No |
| PR-11 Offline/CSP | Web offline/CSP + asset scan | Yes | Definitions if gateway changed | No | Definitions/build if packaging changed | Final head only if inventory changed |
| PR-12 E2E/a11y | Smoke per commit; full tagged suite final | Yes | No | No | No | No |
| PR-13 Host qualification | Definition checks per commit | Relevant Rust only | Complete final | Complete final | If runtime changed | If appliance changed |
| PR-14 Docs/contract | Docs lint + inventory consistency | No | No | No | No | No |
| PR-15 Release gate | All exact-head gates | Yes | Yes | Yes | Yes | Yes |

A future AI must explain any deviation from this table before triggering extra heavy workflows.

---

# 11. Definition of done for a PR

A PR is not done until all applicable items are true:

- [ ] Current exact head SHA recorded.
- [ ] Scope matches one plan PR.
- [ ] Required implementation complete.
- [ ] Regression tests included.
- [ ] No unrelated refactor or generated artifact.
- [ ] AI change gate green on exact head.
- [ ] Required targeted jobs green on exact head.
- [ ] Skips are intentional and allowed by this plan.
- [ ] Failed logs and artifacts inspected and corrected.
- [ ] Relevant docs/threat model/ADR updated.
- [ ] Review threads resolved.
- [ ] Residual risk recorded.
- [ ] Final-review workflow run only when stable.
- [ ] Merge uses expected-head guard.

---

# 12. Production pilot minimum scope recommendation

The fastest safe commercial path is not to implement every visible concept simultaneously. A limited first release should include only fully proven workflows.

Recommended minimum contracted pilot:

- local authentication and administratively provisioned users;
- school manager user/class administration;
- teacher assignment creation/publication;
- student assignment viewing/submission;
- teacher grading/feedback when fully authorized;
- governed knowledge ingestion and teacher retrieval when final acceptance passes;
- basic student/parent read-only academic views backed by real data;
- backups, restore, monitoring and operator procedures.

Exclude until completed:

- attendance;
- full timetable/calendar;
- parent/teacher messaging;
- advanced trends and analytics;
- behavior reports;
- standardized-test reports;
- any control still marked coming soon or backed by demo data.

Exclusion means removed/disabled in code, documentation and contract—not merely described verbally.

---

# 13. Independent review requirements

Before Gate G4, obtain independent evidence for:

1. Application/API penetration testing focused on multi-tenant authorization.
2. Review of request-scoped RLS and runtime database privileges.
3. Browser CSP/offline and supply-chain review.
4. Container/host hardening assessment.
5. Backup media, restore and key/passphrase handling.
6. Privacy/data-flow/DPIA and contract review for the school jurisdiction.
7. Accessibility review of critical journeys.

The engineering team must track every finding with severity, owner, fix/acceptance, evidence and expiry.

---

# 14. Current known findings to reconfirm

The audit that produced this plan identified these concrete examples. Future AI must reconfirm them before acting:

- assignment operations scoped by object ID without complete actor ownership/school checks;
- runtime app role documented with `BYPASSRLS`;
- RLS context helper not demonstrably pinning subsequent queries to the same transaction/connection;
- legacy submission server functions returning fake success/empty results;
- inactive-account check present at login but not consistently in middleware/refresh;
- notification server functions accepting auth-token arguments;
- assignment personalization launched via non-durable in-process task;
- direct student/parent role routes displaying placeholders despite real dashboard components;
- hardcoded GPA, credits, attendance and trend values;
- attendance backend returning fixed placeholder values;
- hardcoded March 2025 student schedule;
- parent reports and communication containing fictional people/data and no-op controls;
- manager report export buttons not connected;
- login page dynamically importing a public-CDN graphics dependency;
- no comprehensive browser E2E acceptance proof;
- production documentation contradicting later implemented AI/appliance capabilities.

Do not copy old line numbers into new reports. Fetch current files and cite the current exact head.

---

# 15. Risk register seed

| ID | Risk | Initial severity | Primary PR | Release treatment |
|---|---|---:|---|---|
| R-01 | Assignment ID manipulation allows unauthorized read/write/publish/personalize | Critical | PR-01 | Must fix |
| R-02 | Elevated runtime DB role bypasses RLS | Critical | PR-03 | Must fix |
| R-03 | Transaction-local context not tied to protected queries | Critical | PR-03 | Must fix |
| R-04 | Fake-success legacy production APIs | High | PR-02 | Must remove/fix |
| R-05 | Disabled account remains usable | High | PR-02 | Must fix |
| R-06 | Non-durable assignment AI work lost on restart | High | PR-05 | Must fix if AI personalization enabled |
| R-07 | Fictional academic data shown as real | High | PR-06/07/08/09 | Must remove/fix |
| R-08 | Runtime CDN violates offline/supply-chain boundary | High | PR-11 | Must fix |
| R-09 | No browser-level acceptance | High | PR-12 | Must establish |
| R-10 | Single-node service represented as HA/SLA | High contractual | PR-13/14 | Must disclose or redesign |
| R-11 | Privacy/controller/processor duties incomplete | High contractual | PR-14 | Must complete with counsel |
| R-12 | Documentation drift causes operator error | Medium/High | PR-14 | Must reconcile |

---

# 16. Evidence retention and artifact policy

### Ordinary PR evidence

Retain briefly:

- classifier and gate JSON;
- changed-file list;
- exact rustfmt diff on failure;
- focused test logs on failure;
- browser screenshot/trace only on failure;
- no database dump containing data beyond synthetic schema fixtures.

### Final/release evidence

Retain according to release/security policy:

- exact-head gate summaries;
- migration/RLS security evidence;
- browser/accessibility/offline reports;
- topology/security scans;
- backup/restore/PITR/Qdrant evidence;
- load/soak metrics;
- SBOM, checksums, signatures and provenance;
- artifact digests and verification policy;
- risk acceptance and manual sign-off.

Never upload:

- real student/customer data;
- real environments or secrets;
- TLS private keys;
- backup passphrases;
- plaintext backups;
- real school PDFs;
- provider credentials or raw prompts.

---

# 17. Merge and release rules

1. Merge only after all required checks are green on the exact head.
2. An intentionally skipped job is acceptable only when the classifier says the domain is not required and the gate verifies the skip.
3. A required skipped/canceled job is failure.
4. A workflow on an older SHA is irrelevant.
5. Required review threads must be resolved.
6. Use expected-head merge protection.
7. Protected release tags are created only from an accepted commit.
8. No floating `latest` release tag.
9. Published images/artifacts must be signed and have provenance/SBOM/checksum evidence.
10. A release is not contract-ready until manual target-host, privacy, support and acceptance requirements are complete.

---

# 18. Mandatory AI completion report

For every implemented PR, report exactly:

```text
Plan item:
Repository:
Branch:
PR number:
Base SHA:
Final exact head SHA:

Files changed:
- ...

Behavior implemented:
- ...

Security invariants preserved:
- ...

Tests added/updated:
- ...

Workflows/jobs actually run on final SHA:
- Workflow / Job — success|failure|intentional skip

Heavy workflows intentionally not run and why:
- ...

Artifacts/logs inspected:
- ...

Review threads:
- resolved/open

Remaining risks/manual checks:
- ...

Classification:
- safe to continue developing | ready for final validation | ready to merge | ready for limited pilot | ready for contracted production
```

Do not use `completed`, `all tests passed`, `merge-ready`, or `production-ready` without the evidence above.

---

# 19. Final execution order summary

1. **PR-01:** Assignment authorization containment.
2. **PR-02:** Session lifecycle and reachable API cleanup.
3. **PR-03:** `NOBYPASSRLS` and transaction-scoped RLS.
4. **PR-04:** Repository-wide endpoint authorization inventory/matrix.
5. **PR-05:** Durable assignment personalization.
6. **PR-06:** Remove/disable fictional, placeholder and no-op production UI.
7. **PR-07:** Attendance, only if contracted.
8. **PR-08:** Timetable/calendar, only if contracted.
9. **PR-09:** Reports/exports, only supported report types.
10. **PR-10:** Messaging, only if explicitly included.
11. **PR-11:** Browser offline integrity, CSP and frontend supply chain.
12. **PR-12:** Browser E2E, WCAG 2.2 AA and RTL acceptance.
13. **PR-13:** Target-host hardening, operations and measured recovery/load.
14. **PR-14:** Documentation, privacy, security-response and contract package.
15. **PR-15:** Final exact-head production acceptance and release proof.

P0 items are not negotiable. Optional P1/P2 product domains may be excluded only by removing them from the enabled release and contract. The final release workflow is deliberately expensive; ordinary PR workflows are deliberately targeted.

---

## Appendix A — Source-derived versus standards-derived content

### Source/codebase-derived baseline

The concrete findings and architecture in this plan are derived from the audited EduTalent repository state, the merged production operations work, `Plan-V1.txt`, `Rules-of-Workflow.txt`, production documentation, workflow definitions and inspected Rust/Dioxus code as of the baseline date.

### Standards/research-derived baseline

The control framing and acceptance targets use official OWASP, NIST, W3C, ISO, SLSA, CIS, EU and Swiss authority guidance listed in Section 3. A future AI must verify official current versions and jurisdiction applicability before relying on them.
