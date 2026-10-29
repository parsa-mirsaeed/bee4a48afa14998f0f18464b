# Aurora Workspace — EduTalent UI/UX redesign execution roadmap

**Status:** Phase 1 implementation branch in progress; subsequent phases are planned, not claimed complete.
**Scope:** authenticated EduTalent web UI, across platform administrator, school manager, teacher, student, and parent roles.
**Implementation stack:** Rust/Dioxus 0.7, existing shared UI components, Tailwind/CSS, existing localization.
**Design specification:** [DESIGN.md](../../DESIGN.md).
**Existing evidence:** [Dashboard UI/UX Remake Implementation Plan](../../Dashboard-UI-UX-Remake-Implementation-Plan.md), [Full UI/UX Redesign and Workflow Hardening Plan](../../EduTalent-Full-UI-UX-Redesign-and-Workflow-Hardening-Plan.md), role-specific guides in `docs/release`, product truth in `docs/release/feature-matrix.md`.

## Decision and scope

The latest user brief explicitly requests an extreme, stylish, modern, eye-catching redesign. Aurora Workspace evolves the earlier calm-enterprise visual direction into a high-contrast, vivid but operationally credible workspace. It **does not** change supported product features or their permission boundaries. Do not port the Dioxus application to React merely to imitate React-like polish.

Primary UX outcome: in every role, a user can identify their current location, the next task, the real state of that task, and a safe route to continue or recover. The visual design should enhance rather than obscure these answers.

Non-negotiable: no fabricated data/metrics, no fake actions, no front-end auth bypass, no new third-party assets/CDNs, no external dependency required for air-gapped operation, no English-only labels, no regression of current focus/keyboard behavior.

## Current UI / code inventory

| Area | Current owner | Decision |
| --- | --- | --- |
| Routes + role capability gating | `packages/web/src/main.rs`, `application/routing_service.rs` | Preserve |
| Shared shell | `views/role_based/components/dashboard_layout.rs` | Reuse, restyle |
| Navigation | `components/sidebar.rs`, `ui/navigation.rs` | Reuse, clarify hierarchy |
| Notification state | `components/header.rs`, shared Popover | Preserve API, focus/async behavior |
| Shared UI | `packages/web/src/ui/` | Migrate centrally, do not duplicate |
| Prior global styling | `assets/dashboard-remake.css`, `assets/design-system.css`, `assets/design-system-compat.css` | Transitional compatibility |
| Final presentation adapter for Phase 1 | `assets/experience-2026.css` | Aurora colors, shell, hero, shared surfaces |
| Localization | `packages/web/src/i18n/` | English/Persian and RTL remain first-class |
| Acceptance | `tests/e2e/specs/ui-foundation*.spec.ts` | Extend without weakening |

The principal risks are CSS cascade collisions during transition, responsive drawer geometry, insufficient dark/RTL contrast, over-styling data-heavy views, and introducing display-only features absent from the actual product.

## Phase 1 — Shared visual foundation and shell (started here)

**Implementation**
1. Create `DESIGN.md` with exact light/dark tokens, type, density, and token ownership.
2. Introduce the last-loaded Aurora CSS adapter without deleting existing shared behavior.
3. Make the sidebar a distinct midnight navigation spine; preserve the canonical role-aware `SidebarNav` and authorization rules.
4. Refine topbar hierarchy with a localized Workspace label; retain working notification control.
5. Convert overview introductions into a single branded command-stage motif; make action cards, stats, panels, fields, and tables feel like one system.
6. Preserve LTR/RTL logical layout, compact breakpoints, independently scrolling dashboard content, keyboard focus, and reduced-motion behavior.
7. Add focused browser regression checks for loaded visual tokens, navigation, responsive layout, and accessibility.

**Exit criteria:** all five roles use a consistent shell without introducing fake controls; same destination on browser back/forward; no mobile off-screen sidebar; no layout overflow at 390px or 768px; focus-visible and reduced-motion survive; exact-head CI proof green. A stylesheet alone does not demonstrate that all goals are met.

## Phase 2 — Role-specific overview and information architecture

Build vertical slices on top of the shared foundation, with real server-backed data only.

| Role | First visual/task priorities | Verification |
| --- | --- | --- |
| Platform admin | knowledge review queue, published vs pending assets, OCR source review and audit | permission, revision conflict, document source display |
| School manager | users, class administration, governed knowledge submissions, settings | create/edit validation, denied school scope, list state |
| Teacher | active classes, assignments, submissions, knowledge access | grading/review handoff, no phantom actions |
| Student | current assignments, enrolled classes, grades where supported | pending/due states, assignment navigation |
| Parent | children and their learning context, academic chat | child scoping, chat failure/retry, no excluded reports |

For every overview: define one primary action, priority queue or latest work, and real contextual empty/retry state. Avoid repeating giant action-card grids when a list or table carries the task better.

**Exit criteria:** representative task reaches its real next step with localized labels and clear loading/empty/error state, and remains correct under reload, keyboard use, RTL, and narrow screens.

## Phase 3 — Dense workflows and shared interactions

Prioritize by risk: privileged edits/knowledge publication first, then school management, teacher submissions/assignment forms, student task surfaces, and parent child/chat flows.

- Normalize tables: headers, search, filter/sort/paging as API supports, selection semantics only where implemented, no invented bulk operations.
- Normalize forms: labels, constraints, field errors, first-invalid focus, safe async submit, duplicate-submit protection.
- Normalize overlays: focus trap and return, Escape, scroll collision, explicit confirmations for irreversible actions.
- Normalize feedback: localized success, actionable inline errors, retry, loading skeleton or state, consistent disabled/busy style.
- Inventory legacy screen-local UI before replacing it; adapt through `packages/web/src/ui` rather than making another component family.

**Exit criteria:** per-flow tests cover success, failure, denied access, and interruption. Keyboard and locale behavior match shared contracts. No API or domain rules changed solely for redesign.

## Phase 4 — Responsive, RTL, dark, accessibility and operational proof

Matrix: five roles × desktop 1440, tablet 768/1024, mobile 390 × English/Persian × light/dark, with focused representative journeys to avoid an unbounded test suite. Test minimum 44px interactive targets, semantic headings, keyboard focus order, drawer close/return focus, overlay focus, large text, and prefers-reduced-motion.

Run the repository's relevant Rust Web checks, Playwright browser checks, Axe coverage, and GitHub Actions exact-head AI Change Proof. Confirm the changed head SHA's checks; do not infer green from an earlier run.

**Exit criteria:** no critical accessibility, authorization, focus, or mobile-blocking regressions; no unexpected network origins; screenshot comparison confirms intended visual direction; existing production/no-CDN boundaries preserved.

## Phase 5 — Token consolidation and controlled rollout

Once Phase 1–4 behavior is proven:
- migrate legacy declarations from `dashboard-remake.css` to canonical components/tokens;
- replace compatibility aliases only after every consumer is inspected;
- remove obsolete CSS with a comparison of computed styles and browser journeys;
- obtain stakeholder review of role workflows and Persian text/layout;
- merge through review after exact-head checks are green.

## Tracking and acceptance evidence

Maintain a change ledger: `route | role | old component | canonical owner | behavior risk | visual state | test | review status`. PR descriptions should name the affected routes, screenshot evidence, verified breakpoints/locales, risk, and CI commit SHA. Do not mark later phases complete because the shared CSS looks polished.

### Current implementation and QA ledger

The current draft branch has progressed beyond shell styling into all five role overviews and the first dense manager-directory workflow:

- **Implemented (not yet visually signed off):** role-specific teacher grading priority, student assignment queue, parent academic-assistance entry, manager first-action hierarchy, admin knowledge-review stage presentation.
- **Implemented:** canonical shared table/tab/form/dialog/alert/data-state styling; manager directory layout, semantic filter names, table headings, 44px row action targets, and RTL-aware search/filter icons.
- **Implemented:** accessible mounted focus for compact navigation instead of timing-sensitive requestAnimationFrame code; 44px high-contrast close control.
- **Regression tests added:** role-specific navigation, mobile drawer focus under reduced motion, 44px directory row actions, in-panel table overflow at 390px, and Persian logical icon positioning.
- **Previous CI evidence:** Web Rust checks and most browser journeys passed; the reduced-motion focus check failed on an older head. A later branch's full validation result alone is not permission to merge. Re-run and inspect exact latest-head `AI Change Proof`, including browser smoke.
- **Outstanding:** confirm new focus behavior in browser, review desktop/tablet/mobile screenshots in LTR/RTL and light/dark, audit user CRUD pending states/error recovery, complete other dense workflows, consolidate legacy CSS, obtain reviewer approval.

A passing compiler does not imply keyboard correctness; a passing screenshot does not imply authorization safety. This ledger tracks implementation, not a release claim.

### Next implementation ledger — manager class workflows

- **Implemented:** class inventory has responsive grid/list treatment and a two-state semantic view switch; class cards are keyboard activatable with Enter and Space and expose focus outlines.
- **Implemented:** create-class subject selector has an actual label association, required state, and a focused treatment rather than borderless glassmorphism; form submission remains the existing guarded server action.
- **Implemented:** class enrollment selector has a real label, and the existing roster-removal actions gain student-specific accessible names and 44px targets; the API and authorization are unchanged.
- **Tests added (verification pending exact-head CI):** keyboard opens class details, switch mode semantics, create subject labeling, enrollment select labeling, roster icon target dimensions.
- **Focus remediation:** removed timing-sensitive reliance on the initial Dioxus mount and added bounded post-render retries that stop on success or drawer dismissal. Only the latest exact-head browser run can validate the fix.
- **Release gate remains open:** do not merge before Rust, Playwright, accessible focus, RTL/light/dark screenshot review, and reviewer sign-off. Class removal and role permission lifecycle require further operational review.

### Phase 1 implementation ledger

- [x] Repository, role routes, product capability matrix entry points, and current design-system owners identified.
- [x] Aurora palette/typography/layout direction set, including bilingual and dark-state tokens.
- [x] Standalone CSS adapter introduced after existing canonical/compat styles.
- [x] Sidebar navigation hierarchy and bilingual header label changed.
- [x] Durable `DESIGN.md` recorded.
- [ ] Automated visual/accessibility proof on running Dioxus app.
- [ ] Desktop/tablet/mobile screenshots reviewed in both directions.
- [ ] Exact-head CI gates confirmed green.
- [ ] Phase 2 role-content redesign.
