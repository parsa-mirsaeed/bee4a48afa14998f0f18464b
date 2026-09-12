# PR-12 Manual Keyboard and Screen-Reader Acceptance

## Follow-up ownership

This record exists because production acceptance requires **human keyboard and screen-reader testing** in addition to automated WCAG checks. It is one evidence section of the dedicated manual/external production-acceptance follow-up.

Do not pre-fill a PASS result from CI. A human tester must complete and sign the record below against the exact installed frozen release candidate.

The cross-cutting record is `docs/security/manual-external-production-acceptance.md`.

## Clean-repository migration status

This record was rebuilt in the new public EduTalent repository after repository-history sanitation and security remediation.

- Repository: `parsa-mirsaeed/bee4a48afa14998f0f18464b`
- Migration baseline on `main` when this record was rebuilt: `5ed53ab35552859b52d5595a27de19ec733e8ca2`
- Frozen release source SHA: **PENDING**
- Final Release Acceptance workflow run: **PENDING fresh run in this repository**
- Final Release Acceptance artifact digest: **PENDING fresh artifact in this repository**
- Automated classification: **PENDING fresh exact-head validation**
- Automated contracted-production decision: **false until all required automated and human/external evidence is complete**

Historical source SHAs, workflow-run IDs, artifact digests, signatures, attestations, and other commit-addressed evidence from the retired repository are superseded. They must not be copied into this record as current evidence.

If testing finds a WCAG 2.2 AA defect, fix it in code with regression coverage before sign-off unless the production-readiness process explicitly permits a documented risk acceptance with owner, date, rationale, and review/expiry. If the candidate source or installed artifact changes in a way that could affect accessibility, rerun the impacted acceptance journeys.

## Human test record

Populate during the actual accessibility session. Do not pre-fill PASS from automated checks.

- Exact installed release/source SHA:
- Installed signed artifact/build digest:
- Tester:
- Date (UTC):
- Browser / version:
- Operating system:
- Screen reader / version:
- Keyboard-only pass completed: yes / no
- Screen-reader pass completed: yes / no

## Critical journeys

For every journey below, verify that the user can perceive the current context, reach and operate all controls without a pointer, understand status/error feedback, and recover/continue without focus loss.

### 1. Authentication and session termination

- Reach email, password and Sign In in visible/logical keyboard order.
- Sign in as an active role and confirm the role-appropriate dashboard is announced/understandable.
- Reach Sign Out by keyboard and activate it.
- Confirm the logged-out/login state is clear and protected content is no longer reachable.

Result / notes:

### 2. Role navigation

- Navigate a representative School Manager or Teacher dashboard using keyboard only.
- Confirm navigation labels, active context and page/section headings are understandable with the screen reader.
- Confirm no keyboard trap in desktop or responsive/mobile navigation controls.

Result / notes:

### 3. Student assignment submission

- As the seeded Student, reach an assigned item by keyboard.
- Open the assignment details and Start Assignment without a pointer.
- Enter representative text into the work editor.
- Submit and confirm the resulting status/transition is understandable.

Result / notes:

### 4. Teacher grading and feedback modal

- As the authorized Teacher, reach the submitted work by keyboard.
- Open Grade Submission.
- Confirm the dialog has an announced accessible name and focus enters the dialog.
- Tab and Shift+Tab through the dialog controls in a logical order.
- Enter a valid grade and feedback, save, and confirm the modal closes without leaving focus in an unusable state.
- Confirm the operation and any validation/error message are understandable to the screen reader.

Result / notes:

### 5. Student persisted grade view

- Return as the Student and navigate to Grades.
- Open class grade details.
- Confirm assignment title, grade/points and date are understandable in both English/LTR and Persian/RTL contexts.
- Confirm the grade-details dialog is keyboard operable and announced as a dialog.

Result / notes:

### 6. Persian / RTL pass

- Switch to Persian.
- Confirm document direction and navigation order remain usable.
- Confirm dates and numeric grades remain readable and are not visually or audibly reordered into misleading values.
- Re-check one modal flow for focus and announcement behavior in RTL.

Result / notes:

## Findings

List every defect with severity, reproduction steps and issue/PR reference. WCAG 2.2 AA findings must be fixed before acceptance unless the production-readiness process permits a documented risk acceptance with explicit owner, date, rationale, and review/expiry.

- Findings:
- Accepted risks (owner + date + rationale + review/expiry), if any:

## Sign-off

- Keyboard acceptance: PASS / FAIL
- Screen-reader acceptance: PASS / FAIL
- Tester name:
- Sign-off date:
- Final exact source SHA verified unchanged:
- Installed signed artifact/digest verified:
