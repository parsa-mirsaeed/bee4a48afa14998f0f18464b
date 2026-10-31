# EduTalent — playful student experiences & interactive reporting (phase 1)

Scope: student, parent and teacher overview only. No change to authorization,
API/database, persistence, grades, submission workflows or release capabilities.

## Why these interactions
- **Grades 1–4:** generous touch targets, clear pictograms, playful colors and a
  single primary next step. All meaningful progress remains text-readable.
- **Grades 5–9:** the same accessible tasks and data, with a restrained
  information hierarchy; do not force cartoon-heavy visuals on older pupils.
- **Parents:** select a child in an in-page navigator and inspect only existing,
  authorization-scoped grade-level and enrolled-class counts. Other details
  retain their existing scoped route.
- **Teachers:** an interactive submission overview using persisted submitted
  and expected counts for published assignments; draft assignments excluded
  from aggregation. A filter isolates published work with incomplete submission
  counts. These counts are not attainment scores or grades.

This is a **shared, age-inclusive v1**. Grade detection and individual
age-band modes are intentionally not inferred from account metadata. Before
introducing per-grade theming, require a validated server-side grade source
and age-specific usability tests.

## Behavior, honesty, inclusion
- No artificial coins, grades, streaks, trophies or auto-generated achievements.
- Submission progress measures submitted/graded assignments versus all loaded
  assignments; pending includes overdue per canonical presentation state.
- The family switcher uses only the current parent's scoped children API.
- No new parent reporting route is advertised when the capability is disabled.
- RTL/LTR, dark mode, responsive layout, visible focus, keyboard controls,
  semantic progress and `prefers-reduced-motion` are required.
- All animations are lightweight, decorative, and optional; never obscure a
  task or substitute for feedback.
- Browser smoke should cover all three roles, including toggles and navigation.

## Phase 2 priority — mobile bottom navigation (not implemented in phase 1)
This is a confirmed product requirement, not an optional visual experiment.

- Build a **modern, touch-first bottom navigation bar** for narrow mobile viewports
  (start at widths up to 767 CSS px; validate breakpoints against existing drawer).
- Use the existing authorized role navigation and routing: for students prioritize
  overview, classes, assignments, grades; for parents overview and children plus
  only enabled destinations; for teachers overview, classes, assignments and
  grading/submissions. Never surface a disabled or unauthorized route.
- Show up to four primary destinations (five only if validated in usability tests);
  put secondary items in an accessible More menu or the existing drawer.
- Use readable icon + text pairs, a persistent active indicator with
  `aria-current="page"`, 44×44 px or larger touch targets, and keyboard-visible focus.
- Respect right-to-left Persian and left-to-right English icon/label ordering,
  mobile safe areas (`env(safe-area-inset-bottom)`), dark mode, reduced motion,
  and dynamic text sizing.
- Keep the content reachable: add adequate bottom scroll padding; avoid collision
  with form submit controls, virtual keyboards, dialogs and accessibility overlays.
- Preserve existing tablet/desktop layout and mobile drawer lifecycle; bottom nav
  should complement rather than duplicate the entire navigation tree.
- Test route transitions, correct active state, role scoping, viewport widths
  320/375/390/430/768 px, RTL/LTR, focus, keyboard, screen readers, overlay
  stacking, reduced motion and iOS/Android safe areas.

**Delivery:** separate PR and exact-head browser proof following the
EduTalent Workflow Trigger Guide. Do not merge as part of phase 1.

## Next phase candidates (not implemented here)
- Lesson-completion celebration based on real persisted state.
- Age-band visual preference after reliable grade is available.
- Drill-down reports and trend charts only after a verified time-series API
  exists (not synthetic demo data).
- Parent-teacher report sharing subject to authorization and privacy review.
