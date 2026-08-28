# Stage 1 Smart CI/CD results and architecture exit decision

**Stage:** Stage 1 — Smart CI/CD and Build Optimization  
**Baseline:** `main` at `5874da3bd5dab491d967d16fb6d4dff1f7bcf6ae`  
**Review baseline after S1-PR-05:** `main` at `d55206fe2e431a27e1e01449a8211ee6c1ba8acc`  
**Review date:** 2026-08-28

This document closes the Stage-1 optimization sequence with measured evidence. It does not replace the production-readiness or UI/UX plans. It records what Stage 1 changed, what the observed measurements support, the remaining bottlenecks, and whether a broader application-architecture migration is justified.

## 1. Exit decision

**Decision: stop the Stage-1 refactor after S1-PR-06 and return to the product/security production-readiness sequence. Do not approve a Rust/Dioxus-to-React migration on the current evidence. Do not add a Python production backend for CI speed.**

The evidence shows that broad over-triggering and repeated CI/tool work were material problems and that Stage 1 removed or reduced them without weakening exact-head proof. It does **not** show that the Rust/Dioxus frontend architecture is now the dominant cross-repository delivery bottleneck.

Future architecture migration requires new measured evidence. A React/TypeScript frontend proposal is justified only if, after ordinary CI classification, caching, focused browser proof, and build-boundary improvements are in place, frontend Rust compilation remains the dominant cost for representative UI work and the expected product-delivery benefit exceeds migration/security/supply-chain risk.

Python remains appropriate only for a separately justified ML/OCR/research workload. It must not become a second general application backend, authentication authority, or data-authority layer merely to make the stack polyglot.

## 2. What Stage 1 delivered

The merged Stage-1 sequence established these controls:

- S1-PR-00: recorded the pre-optimization baseline, change-to-proof guide, shadow classifier, fail-closed unknown classification, and evidence schema.
- S1-PR-01: promoted the classifier to control mode; removed the generic `Web => PostgreSQL` implication; split client-only Rust from database-backed server proof; retained database proof for API/server, auth, repository/query, RLS/migrations, and persisted worker boundaries.
- S1-PR-02: added focused browser journey selection, locked browser dependencies, pinned Dioxus CLI reuse, pinned Chromium reuse, and compact success evidence.
- S1-PR-03: split Rust cache domains by target/build mode and allowed safe same-repository PR cache saves without allowing cache hits to replace proof.
- S1-PR-04: split Docker/BuildKit gateway and Web source-build invalidation, preserved SQLx compile-time schema validation, kept the source stages parallel, isolated compile-only PostgreSQL ports, and added targeted exact-head runtime-image/migration proof.
- S1-PR-05: made `AI Change Proof` the ordinary-PR routing authority for Package, Production Foundation, Production Operations, and Air-gapped Appliance owner proof; removed broad direct PR path filters; preserved escalation-only labels and final-release orchestration.

The globally required ordinary-PR contract remains `AI Change Proof / AI change gate` on the exact current head.

## 3. Baseline observation

The preserved baseline is `docs/ci/STAGE1_BASELINE.md`.

Representative pre-Stage-1 UI head: `e131d1f42aa3898f780f37bd065752add96a6b0a`  
AI Change Proof run: `33085930807`  
Runner topology: GitHub-hosted `ubuntu-latest`

Observed wall times:

| Baseline job/run | Result | Wall time |
|---|---:|---:|
| AI Change Proof workflow | failure | 11m41s |
| Classify change impact | success | 6s |
| PostgreSQL migrations and invariants | success | 28s |
| Format changed Rust files | success | 8s |
| Affected Rust checks and tests | failure | 4m17s |
| Browser smoke critical journeys | failure | 11m19s |
| Browser-smoke script itself | failure | 10m25s |

The baseline browser lane spent about 48 seconds before the browser-smoke script began, including PostgreSQL initialization, Rust/WASM setup, Dioxus CLI, Node/browser dependencies, and Chromium installation/cache handling.

The important baseline defect was not merely raw Rust speed: broad Web/UI changes selected PostgreSQL and browser proof by default, so presentation churn could pay database, Rust server, WASM/tool, and browser costs together.

## 4. Post-optimization observed measurements

These are recorded observations, not universal SLO claims. Different PRs have different changed-file sets, so cross-PR comparisons are directional unless explicitly described as the same-head warm comparison.

### 4.1 Same-head cold-to-warm cache evidence

S1-PR-03 exact head: `89a2be19fd235fe44247196179b949fac5a1246f`.

| AI Change Proof run | Result | Wall time | Notes |
|---|---:|---:|---|
| `33192233160` | success | 8m50s | cache-establishing/cold representative run |
| `33193738325` | success | 5m47s | same-head warm run |

The same-head warm run was about **34.5% faster** than the cache-establishing run. The real browser proof still executed and passed; cache/tool reuse did not substitute for tests.

In warm run `33193738325`:

- browser job: 5m06s;
- focused browser-smoke step: 4m09s;
- pinned Dioxus CLI restore succeeded and the install step was skipped;
- pinned Chromium restore succeeded and the install step was skipped;
- the Rust cache action completed successfully, but the available GitHub step metadata does not expose a reliable `cache-hit` value, so no unsupported hit-rate claim is made.

### 4.2 Directional baseline-to-post-optimization browser comparison

The representative baseline browser job was 11m19s. The warm post-optimization browser job above was 5m06s, a directional reduction of about **54.9%**. The baseline browser-smoke script was 10m25s versus 4m09s in the warm sample, a directional reduction of about **60.2%**.

This is **not a controlled A/B benchmark** because the changed-file sets and exact test selections differ. It is evidence that the new focused-selection/tool-reuse path can complete materially faster while still executing real browser proof.

The overall baseline workflow was 11m41s versus 5m47s for the warm sample, a directional reduction of about **50.5%**. Again, do not interpret that number as a universal speedup for every PR class.

### 4.3 Specialized-owner routing evidence

S1-PR-05 exact head: `cf40dcd79b7a735dc984e6ae2de918520b5b18d0`  
AI Change Proof run: `33200208743`  
Workflow wall time: 3m50s, success.

That PR changed workflow/classifier ownership. The central classifier selected the four affected specialized owners and the following unrelated ordinary product lanes were skipped:

- changed-Rust formatting;
- PostgreSQL migrations/invariants;
- Web client Rust checks;
- database-backed Rust checks;
- browser smoke.

Selected focused owner evidence passed:

- Package definition proof;
- Production Foundation topology/security definition proof;
- Production Operations definition/regression/security proof and gate;
- Air-gapped Appliance definition proof and gate.

Complete package release, complete production stack/migrations, complete operations recovery/PITR, and complete appliance builds remained skipped because they are final/release evidence, not ordinary iteration proof.

This is the desired Stage-1 behavior: fewer irrelevant jobs, not fewer required security checks.

### 4.4 Targeted package proof remains intentionally heavy

S1-PR-04 exact head: `8d72a4190b230d394d6f475a18228d53f2b60d4e`  
Package run: `33198435034`  
Workflow wall time: 10m42s, success.

The `Targeted runtime image and migration replay` job took 10m24s. Its exact-head runtime image build step took 9m57s, about **95.7%** of that job and about **93.0%** of the workflow wall time.

That is not an ordinary UI/API feedback-path regression. It is a package-specific escalation proof that builds the real runtime image, verifies its content contract, and replays packaged migrations. Stage 1 intentionally keeps this evidence heavy when packaging is actually affected.

## 5. Remaining bottlenecks

### 5.1 Repeated security-tool provisioning

In S1-PR-05 run `33200208743`, Production Operations job `Audit dependencies and production configuration` took 3m09s. The pinned `cargo-audit` installation step took 2m49s; the actual RustSec audit took about 3s and the repository production-configuration policy check about 13s.

The install therefore consumed about **89.4%** of that job and about **73.5%** of the complete 3m50s AI Change Proof wall time.

This is a tooling/provisioning bottleneck, not evidence for an application rewrite. A later low-risk CI maintenance change may preinstall or safely cache the pinned audit binary, provided version pinning, provenance/trust, fork safety, and proof semantics remain unchanged.

### 5.2 SQLx compile-time PostgreSQL coupling

S1-PR-04 attempted to remove PostgreSQL from source compilation and proved that the current API/server graph still requires a real schema for SQLx compile-time validation. Stage 1 therefore kept a minimum compile-only PostgreSQL prerequisite in the two source-build stages and isolated their ports so BuildKit can run them in parallel.

Removing this remaining coupling requires a separately proven SQLx offline-metadata/build-contract change. It must not be achieved by disabling compile-time query verification or weakening migration/schema proof.

This is a candidate only if future measurements show compile-time database setup is again a dominant feedback-path cost.

### 5.3 Browser work is now more focused, but real product proof still costs minutes

The warm representative browser job still took 5m06s, of which the focused browser-smoke step took 4m09s. That is real release-like Dioxus/browser execution, not merely setup overhead.

This remains worth measuring by journey and change class, but it is not currently sufficient evidence for a frontend framework migration. A migration would add API-contract, auth/session, routing, localization/RTL, accessibility, JS supply-chain, offline packaging, and dual-stack cutover risk.

### 5.4 Package image compilation remains expensive when it is legitimately selected

The targeted image build remains around ten minutes in the observed package proof. That should be optimized with BuildKit/cache/source boundaries where safe, but it belongs to packaging changes and final/release proof rather than every product edit.

## 6. Architecture decision criteria after Stage 1

Do **not** reopen the frontend architecture question from preference or framework fashion. Reconsider it only with a controlled measurement set that includes representative:

- CSS/static-only changes;
- ordinary Dioxus component changes without browser behavior;
- browser-interaction changes;
- API pure-logic changes;
- API/data-access/auth changes;
- package changes;
- cold and warm runs on the active runner topology.

A Stage-2 frontend migration proposal should demonstrate that, after Stage-1 classification and cache behavior, Rust frontend compilation itself is the sustained dominant cost and that the expected reduction is large enough to justify the migration and supply-chain/security cost.

Until that evidence exists, the preferred architecture remains:

- Rust for the application/security/data/concurrency core and current Dioxus product surface;
- PostgreSQL/RLS as the security/data substrate;
- no second general backend;
- no React rewrite solely for CI speed;
- no Python production service unless a narrow ML/OCR/research workload independently requires it.

## 7. CI operating policy after Stage 1

1. `AI Change Proof / AI change gate` is the globally required ordinary-PR exact-head gate.
2. The Stage-1 classifier evaluates the complete changed-file set and fails closed for unknown executable/configuration changes.
3. Ordinary PR specialized proof is selected centrally and executed through the owner reusable workflows for Package, Production Foundation, Production Operations, and Air-gapped Appliance.
4. `ci:db`, `ci:browser`, `ci:workspace`, `ci:package`, `ci:production`, `ci:operations`, and `ci:appliance` may only add proof; they never suppress classifier-required proof.
5. Full Validation is not the normal iteration loop. On PRs, heavy Full Validation runs when `full-validation` is present; pushes to `main`, schedule, and manual dispatch retain complete validation behavior.
6. Complete package/production/operations/appliance evidence belongs to stable final review or release acceptance according to the production-readiness plan.
7. Cache hits and reusable artifacts may reduce work but never replace an affected test or security proof.
8. Every merge decision uses the current exact head; a new commit invalidates older evidence.

## 8. Stage-1 closure

S1-PR-06 is documentation/measurement/decision work. It must not smuggle in a framework migration or relax any production-readiness requirement.

After S1-PR-06 merges, Stage 1 is closed. Continue the authoritative production-readiness and UI/UX implementation plans using the risk-directed CI contract now documented in `AGENTS.md` and `EduTalent-Workflow-Trigger-Guide.md`.
