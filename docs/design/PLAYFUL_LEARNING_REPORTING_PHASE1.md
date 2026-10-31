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

## Next phase candidates (not implemented here)
- Lesson-completion celebration based on real persisted state.
- Age-band visual preference after reliable grade is available.
- Drill-down reports and trend charts only after a verified time-series API
  exists (not synthetic demo data).
- Parent-teacher report sharing subject to authorization and privacy review.
