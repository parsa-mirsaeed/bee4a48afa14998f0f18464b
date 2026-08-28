# EduTalent Agent Engineering Guide

**Engineered by Parsa Mirsaeed**

This file is the operating contract for every human or automated coding agent
working in this repository. Its purpose is to make implementation claims
verifiable. An agent must not describe a change as successful merely because the
code looks correct or because a commit was created. Success requires a green,
exact-head proof appropriate to the change.

## 1. Core rule

For every change:

1. Identify the behavior being changed.
2. Identify the affected packages and infrastructure.
3. Add or update the smallest meaningful tests.
4. Commit the implementation and tests together.
5. Wait for the workflows on the exact commit SHA.
6. Inspect every required job and its logs.
7. Fix failures rather than bypassing checks.
8. Report success only after the required exact-head gates are green.

A passing workflow on an older SHA is not evidence for a newer SHA.

## 2. Repository boundaries

EduTalent is a Rust/Dioxus full-stack workspace with PostgreSQL, Supabase,
Qdrant, Docker packaging, and a self-hosted production topology.

Important areas:

- `packages/api/`: backend domain, repositories, services, middleware, and
  server functions.
- `packages/web/`: Dioxus Web application and role-based views.
- `packages/ui/`: shared UI components.
- `migrations/` and `packages/api/migration/`: canonical database migrations.
- `scripts/ci/`: migration, classifier, evidence, and security verification
  scripts.
- `docker/`, `Dockerfile`, `compose*.yaml`, `edutalent`: build and packaging.
- `deploy/production/`: production Supabase, Caddy, Qdrant, TLS, role,
  operations, and network topology.
- `deploy/appliance/` and `scripts/appliance/`: air-gapped appliance material.
- `.github/workflows/`: objective implementation proof.

Do not weaken authorization, migration integrity, secret handling, runner
isolation, Qdrant filtering, offline boundaries, or production network
boundaries to make a test pass.

## 3. Dioxus 0.7 implementation rules

Use Dioxus 0.7 APIs and documentation only. Older examples are not valid for
this repository.

- Do not use removed `cx`, `Scope`, or `use_state` APIs.
- Components use `#[component]` and return `Element`.
- Use `use_signal` for local reactive state and `use_memo` for derived state.
- Component props must be owned values and implement `Clone` and `PartialEq`.
  Prefer `String`, `Vec<T>`, and `ReadOnlySignal<T>` over borrowed props.
- Use `#[get]` and `#[post]` server functions with stable explicit paths.
- Keep server-only imports, derives, validation, SQLx, and operating-system code
  behind the appropriate `server` feature gates.
- Keep the initial client render identical to the server render. Use
  `use_server_future` when data must be present during SSR and hydration.
- Run browser-only APIs after hydration, normally inside `use_effect`.
- Prefer direct `for` loops and conditional elements in `rsx!`; wrap iterator
  expressions in braces.
- Use `asset!` for repository assets and `document::Stylesheet` for stylesheet
  injection.
- Define routes through the repository's `Routable` enum and preserve
  role-based route authorization.

## 4. Validation levels

### Level 1: AI Change Proof

Workflow: `.github/workflows/ci.yml`

`AI Change Proof` runs on every ordinary pull-request update and is the minimum
exact-head proof required after an agent commit.

Its Stage-1 classifier evaluates the complete changed-file set and derives the
required proof for:

- Rust workspace/dependency changes;
- API/backend logic and data access;
- Web/UI logic and browser-sensitive behavior;
- database, migration, RLS, and role changes;
- dependency audit;
- packaging;
- production topology;
- production operations/readiness;
- air-gapped appliance definitions;
- workflow/classifier policy;
- documentation-only changes;
- unknown executable/configuration changes.

Unknown executable/configuration changes fail closed. Manual labels may only
escalate proof; they never suppress classifier-required work.

The workflow runs only the selected ordinary Rust/database/browser checks and,
when required, invokes the specialized owner workflows described below. It
creates exact-head classification and `ai-change-evidence` artifacts.

Required ordinary-PR merge check by repository policy:

- `AI change gate`

### Level 2: Specialized focused owner proof

Ordinary PRs do **not** rely on broad direct `pull_request.paths` filters in the
specialized workflows. `AI Change Proof` is the routing authority and invokes
the affected owner workflow through `workflow_call`.

The specialized owners are:

- Package — package definitions and targeted package proof when explicitly
  escalated;
- Production Foundation — rendered production topology/security definitions;
- Production Operations — operations definitions/regressions, dependency and
  production-configuration security, and its focused gate;
- Air-gapped Appliance — appliance definitions/tamper checks and its focused
  gate.

The escalation-only labels are:

- `ci:package`;
- `ci:production`;
- `ci:operations`;
- `ci:appliance`.

These labels add owner proof even when the current diff would not select it.
There are no de-escalation labels.

### Level 3: Full Validation

Workflow: `.github/workflows/full-validation.yml`

The workflow is eligible on PR `labeled`, `synchronize`, and `reopened` events,
but heavy Full Validation runs on a PR only while the `full-validation` label is
present. It also runs completely on:

- pushes to `main`;
- the weekly schedule;
- manual dispatch.

Once a PR enters final review and its risk requires complete validation, apply
`full-validation` and keep it until merge. Every later commit invalidates the
older exact-head evidence and reruns the full proof.

Required final-review gate when Full Validation is selected:

- `Full validation gate`

### Level 4: Complete release/production proof

Complete package, production, operations, and appliance workflows are release
or final-candidate evidence. They are deliberately not the ordinary iteration
loop.

Depending on the affected release boundary, complete proof includes:

- Package runtime image/release bundle, packaged migration replay, SBOM and
  release evidence;
- Production Foundation PostgreSQL 17 migration/role proof and complete
  self-hosted production-stack startup/security smoke;
- Production Operations backup/recovery/PITR/Qdrant recovery, alerts, restart,
  load, recreation, and fail-closed evidence;
- Air-gapped Appliance complete offline build, registry-disabled startup,
  immutable manifest, model/SBOM/signature/provenance evidence, and required
  architecture proof.

The final release/acceptance workflows remain authoritative for the frozen
release candidate. Do not replace them with a successful ordinary PR check.

## 5. Change-to-test matrix

The binding detailed map is `EduTalent-Workflow-Trigger-Guide.md`. The rules
below describe the current implementation contract.

### Documentation only

Examples: `README.md`, ordinary `docs/**`, `AGENTS.md`, `SECURITY.md`, and
`LICENSE`.

Required: `AI change gate` plus any selected documentation/policy self-test.

Do not start PostgreSQL, Rust compile, browser, package, production, operations,
or appliance proof solely for ordinary documentation. A document that is itself
a CI trigger/policy contract may select the policy self-test.

### Web style/static assets

Examples: CSS, fonts, SVG/images, and static assets under Web/UI.

Do not infer Rust, PostgreSQL, or browser proof merely from the parent package.
Add focused browser/visual/accessibility proof only when the changed asset can
materially alter layout, focus, contrast, responsive behavior, RTL/LTR, or an
interactive affordance.

### Dioxus Web/shared UI Rust without database behavior

For ordinary client-side Web/UI Rust, the Stage-1 fast path uses the Web client
boundary rather than the server/SQLx boundary:

```bash
cargo check -p web --features web --target wasm32-unknown-unknown --locked --message-format=short
cargo clippy -p web --features web --target wasm32-unknown-unknown --bin web --locked --message-format=short -- \
  -A warnings -D clippy::correctness -D clippy::suspicious
cargo test -p web --locked
```

Changed Rust formatting is also required. PostgreSQL is not required merely
because the file is under `packages/web` or `packages/ui`.

Compilation/unit tests do not prove browser behavior. Login, session, routing,
navigation, forms, permission interactions, hydration/storage, RTL/LTR, or
other critical browser behavior requires focused browser proof selected by the
browser policy.

### API/backend logic

Examples: services, server functions, middleware, AI gateway internals, and
other server code under `packages/api/src/`.

The current API/server compile graph remains SQLx schema coupled. Until a
separately proven SQLx offline/build-contract change removes that dependency,
API/server and workspace Rust proof remains database-backed.

Typical affected server proof includes:

```bash
cargo check -p api --features server --all-targets --locked --message-format=short
cargo clippy -p api --features server --lib --tests --locked --message-format=short -- \
  -A warnings -D clippy::correctness -D clippy::suspicious
cargo test -p api --features server --lib --locked
cargo check -p web --features server --all-targets --locked --message-format=short
```

The dependent Web server compile remains required when the shared Rust/API
contract changes.

### Workspace/dependency configuration

Changes to root `Cargo.toml`, `Cargo.lock`, Rust toolchain/workspace features, or
other shared Rust dependency configuration require complete affected/workspace
proof and dependency audit. The current workspace/server compile path remains
PostgreSQL-backed because of SQLx compile-time validation.

Do not infer browser/package/appliance proof from a lockfile alone; those are
selected only when their actual boundary is affected or explicitly escalated.

### Database migrations, RLS, roles, or data access

Required:

1. Apply all migrations.
2. Replay all migrations.
3. Verify governed schema lifecycle.
4. Verify security invariants.
5. Verify affected RLS/role/authorization boundaries.
6. Export the verified schema where the workflow contract requires it.
7. Compile and test affected Rust packages against that schema.

Never edit an already-applied protected migration silently. Checksum and replay
protection are security and operational guarantees.

### Authentication, authorization, and governed knowledge

Required as affected:

- API/server Rust tests;
- allowed and denied authorization cases;
- exact-object and cross-school denial cases where applicable;
- database/RLS security invariants;
- dependent Web server compile;
- focused browser proof for browser-reachable auth/session behavior;
- governed retrieval/ingestion proof.

Preserve these invariants:

- database authorization precedes vector retrieval;
- Qdrant filters include school, publication state, and exact authorized asset
  IDs;
- unpublished or archived assets are not retrievable;
- teachers cannot bypass the governed ingestion boundary;
- duplicate active ingestion jobs remain prevented;
- migration/bootstrap credentials never reach long-running services.

### Browser harness/behavior

Use the selector in `tests/e2e/select_smoke.py` and the existing journey tags.
Known narrow changes should run focused journeys. Ambiguous/harness changes and
manual browser escalation must fail conservative to representative complete
smoke rather than selecting zero tests.

Browser dependencies use the committed lockfile. Dioxus CLI and Chromium are
version-pinned and may be restored from cache, but a cache hit never replaces
the browser journey itself.

### Packaging

The central classifier selects Package owner proof for packaging definitions.
Focused definition validation is ordinary PR evidence. Targeted runtime image
and migration replay may be added with `ci:package` when that proof is needed.

Complete image/release archive, packaged migration replay, SBOM,
signature/provenance, and release trust evidence belong to stable final review
or release proof.

### Production topology/security

The central classifier selects Production Foundation for executable/configured
production topology. Ordinary Markdown guidance under production remains
ordinary documentation unless it is an explicit operations/policy owner file.

Focused proof renders the pinned topology with isolated validation secrets and
enforces fail-closed security/network/runtime invariants. Complete stack startup
and PostgreSQL role/migration proof belong to final/release evidence.

### Production operations/readiness

The Operations owner includes the actual `deploy/production/operations/**`
implementation, `deploy/production/edutalent-operations`, readiness entry
points, and the owned Operations ADR/threat-model contract.

Focused proof runs definitions/regressions, bounded load/capacity preflight,
RustSec/dependency and production-configuration security, and the Operations
gate. Complete backup/recovery/PITR/Qdrant recovery/load/restart evidence belongs
to final/release proof.

### Air-gapped appliance

The central classifier selects Appliance owner proof for appliance definitions,
installer/build scripts, and owned workflow boundaries. Focused definition,
lock/inventory, workflow-isolation, and tamper checks are ordinary affected PR
proof.

Complete offline bundle, no-pull startup, model/SBOM/signature/provenance and
native architecture proof belongs to the stable final candidate.

### Workflow/classifier/policy

Required: AI gate plus workflow syntax/static checks, classifier fixtures,
legacy-delta regression tests, browser selector tests, specialized-trigger
contract tests, and evidence-contract tests as selected.

If a workflow can alter proof selection, evaluate the complete PR changed-file
set. A workflow change may add/escalate evidence but must not silently reduce
it.

## 6. Test design rules

- Prefer behavior tests over implementation-detail tests.
- A production bug fix must include a regression test when practical.
- New backend behavior should have API, service, or repository tests in the same
  change.
- New authorization behavior requires both allowed and denied cases.
- Migration changes require first-run and replay/idempotence proof.
- Do not delete, ignore, or weaken a failing test without explaining the
  obsolete requirement.
- Do not use `#[ignore]`, broad Clippy allowances, or catch-all error handling as
  substitutes for a fix.
- Use deterministic fixtures. Do not commit real student, teacher, school,
  customer, credential, document, or production identifiers.
- Never make external AI or network availability a health requirement for the
  core offline school system.
- Cache/tool reuse may reduce setup work but must never decide whether required
  proof executes.

## 7. Agent execution protocol

Before editing:

```bash
git status --short
git branch --show-current
git fetch --all --tags --prune
```

During implementation:

- keep the change scoped;
- inspect existing abstractions before adding new ones;
- preserve feature-gated client/server compilation;
- update tests and documentation together;
- do not commit generated build output, `.env`, private keys, database dumps,
  PDFs, or runtime secrets.

Before committing, run the narrowest local checks available. CI remains the
source of truth because it provides a clean, recorded environment.

After committing:

1. Record the exact head SHA.
2. Inspect workflow runs for that SHA.
3. Verify `AI change gate`.
4. Read the exact-head classification/evidence and verify every selected
   Package, Production, Operations, or Appliance owner result.
5. Verify database/browser/Rust proof when selected; do not infer success from a
   skipped job.
6. For stable final review, add `full-validation` when required and verify the
   complete final-review gate.
7. Run complete package/production/operations/appliance release proof when the
   authoritative release plan requires it.
8. Never claim that skipped work passed.

A job that failed before checkout is infrastructure failure, not code success.
A job that was skipped is not proof unless the classifier explicitly determined
that it was irrelevant or a complete/final mode was not selected.

## 8. Evidence and reporting

Every implementation report must state:

- exact commit SHA;
- files or systems changed;
- classifier categories/affected boundaries when relevant;
- tests added or changed;
- workflows and jobs that ran;
- pass, fail, or skip status;
- any check not run and why;
- remaining risks or manual verification.

Preferred completion statement:

> Exact head `<sha>` passed `AI change gate`; API unit tests and dependent Web
> compile passed; classifier evidence did not require package, production,
> operations, appliance, or browser proof.

Never write “all tests passed” unless every test implied by the sentence
actually ran on the exact SHA.

Stage-1 before/after measurements and the architecture exit decision are
recorded in `docs/ci/STAGE1_RESULTS.md`. Treat those values as observations from
the named runs, not universal performance guarantees.

## 9. Runner topology and tool provisioning

The Stage-1 measurements in `docs/ci/STAGE1_BASELINE.md` and
`docs/ci/STAGE1_RESULTS.md` were observed on GitHub-hosted runners. Current
ordinary workflows use explicit GitHub-hosted labels such as `ubuntu-latest`,
`ubuntu-24.04`, and the required hosted architecture runner where applicable.

Do not assume undocumented self-hosted repository variables control the active
workflow unless the workflow itself proves that contract.

If self-hosted runners are introduced later, they require a separate security
and reliability review. At minimum:

- repository-scoped trust;
- dedicated Linux user/VM;
- no personal SSH/GPG credentials;
- no untrusted public-fork code;
- isolated work/cache directories;
- one heavy job per appropriately sized runner unless measured otherwise;
- rootless or otherwise reviewed Docker boundary for Docker jobs;
- pinned, reproducible toolchain image/provisioning;
- exact same test/security semantics as the hosted path.

Stage-1 measurements identified repeated tool installation as a remaining
optimization opportunity. In particular, a focused Operations security run
spent most of its wall time installing pinned `cargo-audit`. A future tool-cache
or prebuilt-runner change is acceptable only if the pinned version/trust model
and proof behavior remain unchanged.

## 10. Branch and merge policy

For ordinary development:

1. create or update a feature branch from the intended current base;
2. open a PR;
3. require exact-head `AI change gate`;
4. require every classifier-selected specialized owner proof;
5. fix failures instead of adding skip/de-escalation behavior;
6. apply `full-validation` for stable final review when required by the change;
7. verify `Full validation gate` when selected;
8. run complete Package/Production/Operations/Appliance proof when the release
   contract requires it;
9. merge only the exact validated head.

A force-push, rebase, conflict resolution, label change that adds proof, or new
commit invalidates previous evidence as applicable and must be revalidated.

## 11. Security and publication

This repository is proprietary and all rights are reserved. Follow `LICENSE`.

Never expose repository or runner secrets, generated production environment
files, Supabase secret keys, JWT signing material, database passwords, Qdrant
keys, TLS private keys, personal data, private documents, vector payloads, or
host-specific credentials.

Run secret scanning before public visibility or source releases:

```bash
gitleaks detect --source . --redact --no-banner --log-opts="--all"
```

## 12. Definition of done

A change is done only when:

- the implementation satisfies the requested behavior;
- relevant tests exist;
- exact-head required workflows are green;
- classifier-selected owner proof is green;
- evidence is inspectable;
- diagnostics do not expose secrets or personal data;
- documentation is updated;
- no known regression is hidden by skipped or weakened validation.

When these conditions are not met, report the implementation as incomplete or
partially validated.

## 13. Stage-1 architecture guardrail

Stage 1 optimized proof selection, browser/tool reuse, Rust cache domains,
Docker/BuildKit source boundaries, and specialized workflow ownership without a
product-framework rewrite.

The measured exit decision is recorded in `docs/ci/STAGE1_RESULTS.md`:

- keep the current Rust/Dioxus application architecture;
- do not initiate a React/TypeScript rewrite solely for CI/CD speed;
- do not add a Python general application backend solely for CI/CD speed;
- keep Rust as the security/data/concurrency/application core;
- treat SQLx compile-time PostgreSQL coupling and repeated tool provisioning as
  focused future optimization candidates, not justification for a broad
  migration.

A Stage-2 frontend architecture proposal requires a new controlled measurement
set proving that Rust frontend compilation itself is the sustained dominant
feedback-path bottleneck after the Stage-1 controls are active. The proposal
must account for auth/session, explicit API contracts, routing, localization,
RTL/LTR, accessibility, offline packaging, browser supply chain, security, and
cutover risk before migration is approved.
