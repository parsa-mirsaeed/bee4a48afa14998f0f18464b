# EduTalent phase 2: mobile bottom navigation

**Scope:** Shared authenticated Dioxus layout and a single new CSS asset.
The phase-2 branch is stacked on phase-1's tested head. Do not merge phase 2
before phase 1 is integrated, nor use phase-1 CI as phase-2 proof.

## Contract
- Up to four prominent, touch-first role shortcuts at widths <=767 CSS px.
- Student: overview, classes, assignments, grades.
- Teacher: overview, classes, assignments, grading/submissions.
- Parent: overview, children, AI assistance; reports only if enabled by
  the current production capability and present in the canonical navigation.
- School manager: overview, users, classes, knowledge submissions.
- Platform admin: knowledge assets, audit, personalization.
- Actual destinations are always intersected with the canonical role-aware
  navigation and section access matrix. Other destinations remain in the
  existing drawer; no additional route or privilege is introduced.
- Fixed glass-style surface, icon+label, current-page semantics, focus-visible,
  keyboard navigation, visible contrast, reduced motion and mobile safe areas.
- Reserve scrollable content space, keep the drawer above the bottom bar,
  hide the bottom bar while the drawer is open, and preserve tablet/desktop.
- Compact-height mobile view suppresses the bar in favor of reachable form
  controls (including resized virtual-keyboard viewports).

## Proof required per EduTalent Workflow Trigger Guide
Changed: Dioxus UI Rust, stylesheet, localized product copy, browser navigation
behavior, smoke tests and documentation. Required: rustfmt, Web wasm compile,
targeted Web Clippy/tests, focused browser proof, exact-head AI change gate.
No API, persistence, SQL, RLS, permissions or database changes: PostgreSQL not
required by the ordinary change classifier solely for this feature.

Browser smoke includes student navigation/history, teacher+drawer lifecycle,
parent report gating, manager/admin links, 320/375/390/430/768 breakpoints,
Persian RTL, dark mode, keyboard, visible focus, reduced motion and height 440.
Screenshots are CI artifacts, not proof until the job passes on the exact SHA.

Only when stable: apply full-validation label, mark ready for review and inspect
the complete exact-head results from .github/FULL_VALIDATION.md (including
package, production, operations, mirror and appliance when selected).
Skipped final jobs are *not* passing final proof. No merge in this phase.
