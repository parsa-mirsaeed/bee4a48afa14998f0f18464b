# Stage 1 CI/CD baseline

Baseline source: `main` at `5874da3bd5dab491d967d16fb6d4dff1f7bcf6ae`.

This record exists before S1-PR-01 changes any proof-selection rule.

## Current AI Change Proof behavior

The current `.github/workflows/ci.yml` classifies broad `packages/web` / `packages/ui` changes as Web/Rust and starts PostgreSQL whenever `api == true || web == true || database == true`. Browser smoke is also selected broadly for Web/UI changes and installs the WASM target, Dioxus CLI, Node/browser dependencies and Chromium before the critical browser journey.

This baseline is intentionally preserved while the Stage-1 classifier runs in shadow mode.

## Observed exact-head sample

Representative UI branch head: `e131d1f42aa3898f780f37bd065752add96a6b0a`, AI Change Proof run `33085930807` (2026-08-27).

Observed job wall time from GitHub timestamps:

| Job | Result | Approx. wall time |
|---|---:|---:|
| Classify change impact | success | 6 s |
| PostgreSQL migrations and invariants | success | 28 s |
| Format changed Rust files | success | 8 s |
| Affected Rust checks and tests | failure | 4 m 17 s |
| Browser smoke critical journeys | failure | 11 m 19 s |
| AI change gate | failure | 4 s |

The browser job spent about 48 seconds before the browser-smoke script began, including PostgreSQL initialization, Rust/WASM setup, Dioxus CLI, Node/browser dependencies and Chromium installation/cache handling. The selected browser script then ran for about 10 m 25 s before failure.

These are baseline observations, not performance targets and not proof that all future runs have the same timing.

## Runner topology observed

The sampled AI Change Proof jobs used GitHub-hosted `ubuntu-latest` runners. Repository documentation also defines configurable self-hosted runner pools. Stage 1 must therefore record the actual runner and optimize for the active topology instead of assuming one runner model.

## S1-PR-00 safety rule

`stage1_change_classifier.py` is SHADOW ONLY in this PR. Its decision cannot skip or add existing AI Change Proof Rust/database/browser work. S1-PR-01 may wire it into proof selection only after fixture comparison and exact-head review.

## Historical-record note

This file intentionally preserves the pre-optimization state and wording. The implemented Stage-1 result, post-optimization observations, remaining bottlenecks, and architecture exit decision are recorded in `docs/ci/STAGE1_RESULTS.md`.
