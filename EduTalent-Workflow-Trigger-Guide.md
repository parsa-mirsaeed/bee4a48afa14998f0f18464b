# EduTalent Workflow Trigger Guide

**Stage:** Stage 1 — Smart CI/CD and Build Optimization

This guide is the binding change-to-proof map for ordinary pull requests. The production-readiness plan, UI/UX plan, `AGENTS.md`, and `.github/FULL_VALIDATION.md` remain authoritative for security and final-release evidence.

## Core rules

1. `AI Change Proof / AI change gate` remains required on every ordinary PR head.
2. Evidence must match the exact current head SHA.
3. Requirements for mixed changes are the union of every affected class.
4. Unknown executable/configuration files fail closed or conservatively escalate; they never become docs-only silently.
5. Manual controls may escalate (`ci:db`, `ci:browser`, `ci:workspace`, `ci:package`, `ci:production`, `ci:operations`, `ci:appliance`, `full-validation`) but may never suppress required proof.
6. Final comprehensive workflows are not the iteration loop.

Legend: **R** required, **C** conditional, **F** final-review/release, **N** not routine for this change alone.

| Change | AI gate | Rust | PostgreSQL | Browser | Dependency audit | Package | Production | Operations | Appliance | Full Validation |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Docs only | R | N | N | N | N | N | N | N | N | C/F |
| CSS/static assets | R | C | N | C | N | N | N | N | N | F |
| Dioxus Web/UI Rust | R | R | C | C | N | N | N | N | N | F |
| Browser/auth interaction | R | R | C | R | N | N | C | N | N | F |
| API pure logic | R | R | C | C | N | N | N | N | N | F |
| API repository/query | R | R | R | C | N | N | N | N | N | F |
| Auth/authorization | R | R | R | C/R | N | C | C | N | N | F |
| Migration/RLS/DB role | R | R | R | C | N | C | C | N | C | R/F |
| AI Gateway internal | R | R | C | N | N | N | C | N | C | F |
| Gateway topology/egress | R | R | C | N | N | C | R | C | C | F |
| Worker/RAG | R | R | C/R | N | N | N | C | C | C | F |
| Rust dependency/workspace | R | R | C | C | R | C | C | N | C | F |
| Packaging definition | R | C | C | N | C | R | C | N | C/R | F |
| Production topology/security | R | C | C | C | C | C | R | C | C | R/F |
| Operations/readiness | R | C | C | N | C | C | C | R | C | R/F |
| Appliance/offline definition | R | C | C | C | C | R | C | C | R | R/F |
| Workflow/classifier policy | R | C | C | C | C | C | C | C | C | C/F |

## PostgreSQL decision

PostgreSQL is required when any answer below is yes:

1. migration/schema/RLS/DB role changed;
2. repository/query/data-access behavior changed;
3. authentication/authorization/tenant/object scope changed;
4. durable persisted queue/job state changed;
5. the smallest meaningful integration test requires real DB state.

PostgreSQL is **not** required merely because a file is under `packages/web` or `packages/ui`.

If coupling is ambiguous, escalate to PostgreSQL until a focused test boundary proves it is unnecessary.

## Browser decision

Browser proof is required for changed browser behavior such as login/session/logout, role navigation, routes, forms, client validation, mutation state, hydration/browser storage, accessibility interactions, RTL/LTR behavior, or a critical user journey.

It is not automatically required for backend-private implementation, internal workers, migrations with no browser contract change, or ordinary documentation.

Use focused journey tags rather than the complete browser matrix during iteration. The complete contracted critical-journey suite belongs to final review.

## Change classes

### Documentation only
Required: AI gate and configured docs/policy checks. Do not start Rust, PostgreSQL, browser, package, production, operations, or appliance solely for documentation.

### Web style/static assets
Required: AI gate and asset/build validation. Add focused visual/a11y browser proof only when layout, focus, contrast, RTL/LTR, responsive behavior, or interaction affordance can change. A `.rs` component is executable Web/UI Rust, not an asset-only change.

### Dioxus Web/UI Rust
Required: changed Rust formatting, Web check, targeted Clippy, Web tests, AI gate. Add browser proof when visible interaction changes. Do not start PostgreSQL solely because the source lives under Web/UI.

### API pure logic
Required: API Rust check/Clippy/tests and AI gate. Keep dependent Web compile while current shared Rust contract coupling cannot be safely excluded. PostgreSQL is not required for truly non-DB logic.

### API repository/query/data access
Required: API Rust plus PostgreSQL-backed affected integration/security proof and AI gate.

### Authentication/authorization
Required: API Rust, allowed/denied cases, exact-object and cross-school denial where applicable, PostgreSQL security proof, dependent Web compile, AI gate. Add focused browser proof when browser-reachable auth/session behavior changes. Never skip DB-before-Qdrant authorization proof when affected.

### Migration/RLS/DB role
Required: apply, replay/idempotence, schema/security verification, RLS/role proof, affected Rust, AI gate. Never silently edit an already-applied protected migration. Production/package/appliance definition proof is conditional on whether their owned runtime/inventory changes.

### AI Gateway
Internal quota/circuit-breaker/limit/provider-normalization changes require focused Rust/fault/concurrency tests and AI gate; no browser. Egress/configuration/topology changes additionally require focused Production Foundation and package/appliance definition proof when those manifests are affected.

### Durable workers/RAG/knowledge
Run affected Rust worker/service tests, fault/retry/idempotency tests, PostgreSQL when persisted queue/auth state changes, and authorization proof when retrieval/publication scope changes. Browser is required only for changed user workflow behavior.

### Dependencies/workspace
Changes to Cargo/workspace/toolchain/lockfiles require dependency audit plus affected/workspace compile and tests. PostgreSQL/browser/package/appliance are conditional on the dependency boundary affected; they are not automatic merely because a lockfile changed.

### Browser harness
Harness/config/dependency changes require harness verification and representative critical smoke. Use PostgreSQL only for selected journeys requiring real auth/data state.

### Packaging
Relevant PR commits run package-definition/static checks and targeted builds as needed. Complete image/release archive, packaged migration replay, SBOM/signature/provenance belong to final/release proof, not unrelated UI/API iteration.

### Production topology/security
Relevant commits run rendered topology/security definition proof. Add DB/browser/package/operations/appliance proof only for the corresponding changed boundary. Complete Production Foundation is final-candidate evidence.

### Operations/readiness
Relevant commits run definition/regression/security checks. Complete recovery, backup, PITR, Qdrant recovery, failure, alert, load, restart and recreation evidence is final-review/release proof as defined by the production plan.

### Air-gapped appliance
Relevant commits run definition/inventory/tamper checks. Complete offline bundle, no-pull startup, model/SBOM/signature/provenance and native architecture proof belongs to the stable final candidate.

### Workflow/classifier/policy
Required: AI gate plus workflow syntax/static checks, classifier fixtures and gate/evidence self-tests. If the workflow can alter proof selection, evaluate the complete PR changed-file set. A workflow change may add/escalate evidence but must not silently reduce it.

## Full Validation

Do not use Full Validation as an iteration loop. Use it when the PR is stable and in final review according to `.github/FULL_VALIDATION.md`. A new commit invalidates previous final exact-head evidence.

## Mixed examples

`packages/web/src/views/login.rs` + `packages/api/src/middleware/auth.rs` requires Web Rust + API Rust + PostgreSQL auth/security + focused browser auth/session + AI gate.

`Dockerfile` + `packages/api/src/ai_gateway_runtime.rs` + production Compose requires gateway Rust + package definition + production topology definition + appliance definition when inventory changes + AI gate; complete specialized builds wait for stable final review/release.

## Evidence statement

Every implementation report must identify the exact SHA, categories, required proof, executed proof, skipped/omitted sections and why, and specialized workflows required. Never say “all tests passed” when only targeted tests ran.

> Risk-directed does not mean less rigorous. It means rigorous proof exactly where the change can cause a regression.
