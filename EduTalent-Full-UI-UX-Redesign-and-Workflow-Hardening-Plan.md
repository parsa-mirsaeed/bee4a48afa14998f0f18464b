# EduTalent Full UI/UX Redesign and Workflow Hardening Plan

**Repository:** `parsa-mirsaeed/35c8f3cf6db363100f4e880c`  
**Audited baseline:** `main` at `e5d29ab7b4ed6139c4f5f56f786423b03aba0fe7`  
**Open behavioral prerequisite:** PR #24, `agent/fix-platform-knowledge-lifecycle`, head `fddb9c64afb1af93d1cc75cdadfcc7c46a816eef`  
**Purpose:** one implementation contract for the complete EduTalent UI/UX redesign plus the frontend/backend correctness defects discovered during hands-on exploration.

---

## 1. Executive decision

EduTalent should not receive another CSS-only facelift. The current codebase contains **two overlapping UI systems**, several large role-specific pages with bespoke markup, inconsistent validation/error handling, some live-looking placeholder data mixed with real API data, and a small number of frontend/backend contract defects that make otherwise polished screens untrustworthy.

The correct sequence is:

1. **PR-1 — Workflow correctness and truthful state/data contracts**
   - fix the functional and security defects;
   - remove fabricated live-looking data and unsafe placeholder fallbacks;
   - introduce stable domain/UI error states for the affected workflows;
   - make the existing product truthful before visual migration.

2. **PR-2 — Design system, responsive shell, routing/state architecture, authentication and shared interaction primitives**
   - establish the permanent UI foundation;
   - replace the legacy/glass primitive layer rather than stacking more overrides;
   - implement accessibility, RTL, validation, error and empty-state behavior once.

3. **PR-3 — Complete role-by-role visual/interaction redesign**
   - migrate every canonical School Manager, Teacher, Student, Parent and Platform Admin surface to PR-2 primitives;
   - apply the premium responsive concept consistently;
   - remove duplicate/obsolete UI implementations and residual inline styling.

PR #24 should be completed/merged before PR-3. Its **knowledge lifecycle behavior** is a good backend/UI behavioral baseline; PR-3 should restyle/localize it rather than reimplementing its state machine.

This plan does **not** replace the human/external production acceptance in PR #16.

---

# 2. Audit scope

## 2.1 Frontend architecture audited

### Application entry / routing / session
- `packages/web/src/main.rs`
- `packages/web/src/application/routing_service.rs`
- `packages/web/src/application/*`
- `packages/web/src/domain/*`

### Shared dashboard shell
- `packages/web/src/views/role_based/components/dashboard_layout.rs`
- `packages/web/src/views/role_based/components/header.rs`
- `packages/web/src/views/role_based/components/sidebar.rs`
- `packages/web/src/views/role_based/components/navigation.rs`
- `packages/web/src/views/role_based/components/role_guard.rs`
- `packages/web/src/views/role_based/components/error_boundary.rs`
- `packages/web/src/views/role_based/components/unavailable_feature.rs`
- `packages/web/src/views/role_based/components/loading_spinner.rs`

### Shared feature primitives
- `packages/web/src/views/role_based/shared/common.rs`
- `packages/web/src/views/role_based/shared/forms.rs`
- `packages/web/src/views/role_based/shared/tables.rs`
- `packages/web/src/views/role_based/shared/charts.rs`
- `packages/web/src/views/role_based/shared/profile_request.rs`
- `packages/web/src/components/validation.rs`
- `packages/web/src/components/skeleton.rs`
- `packages/web/src/components/dashboard_skeleton.rs`
- `packages/web/src/components/auth.rs`

### School Manager
- `packages/web/src/views/role_based/school_manager/dashboard.rs`
- `packages/web/src/views/role_based/school_manager/user_management.rs`
- `packages/web/src/views/role_based/school_manager/user_creation.rs`
- `packages/web/src/views/role_based/school_manager/class_management.rs`
- `packages/web/src/views/role_based/school_manager/knowledge_upload.rs`
- `packages/web/src/views/role_based/school_manager/requests.rs`
- `packages/web/src/views/role_based/school_manager/settings/mod.rs`
- `packages/web/src/views/role_based/school_manager/settings/profile.rs`
- `packages/web/src/views/role_based/school_manager/settings/security.rs`
- `packages/web/src/views/role_based/school_manager/settings/general.rs`
- `packages/web/src/views/role_based/school_manager/settings/notifications.rs`
- `packages/web/src/views/role_based/school_manager/reports.rs`

### Teacher
- `packages/web/src/views/role_based/teacher/dashboard.rs`
- `packages/web/src/views/role_based/teacher/classes.rs`
- `packages/web/src/views/role_based/teacher/assignments.rs`
- `packages/web/src/views/role_based/teacher/submissions.rs`
- `packages/web/src/views/role_based/teacher/students.rs`
- `packages/web/src/views/role_based/teacher/knowledge_assets.rs`
- `packages/web/src/views/role_based/teacher/personalization_status.rs`

### Student
- `packages/web/src/views/role_based/student/dashboard.rs`
- `packages/web/src/views/role_based/student/classes.rs`
- `packages/web/src/views/role_based/student/assignments.rs`
- `packages/web/src/views/role_based/student/grades.rs`
- `packages/web/src/views/role_based/student/schedule.rs`

### Parent
- `packages/web/src/views/role_based/parent/dashboard.rs`
- `packages/web/src/views/role_based/parent/children.rs`
- `packages/web/src/views/role_based/parent/communication.rs`
- `packages/web/src/views/role_based/parent/reports.rs`

### Knowledge / Platform Admin
- `packages/web/src/views/role_based/knowledge.rs`
- PR #24: `packages/web/src/views/role_based/platform_admin.rs`

### Localization
- `packages/web/src/i18n/locale.rs`
- `packages/web/src/i18n/provider.rs`
- `packages/web/src/i18n/translations.rs`
- `packages/web/src/i18n/ui_translations.rs`
- `packages/web/src/i18n/grading.rs`

### Styles
- `packages/web/assets/main.css`
- `packages/web/assets/dashboard-remake.css`

### Product truth tests
- `packages/web/src/product_truthfulness_tests.rs`

---

## 2.2 Backend boundaries audited

### Authorization / endpoint truth
- `packages/api/endpoint_authorization_manifest.psv`
- `packages/api/src/middleware/endpoint_authorization.rs`
- `packages/api/src/product_capabilities.rs`

### User provisioning / relationship management
- `packages/api/src/server_functions/user_management.rs`
- `packages/api/src/repositories/user_repository.rs`
- `packages/api/src/server_functions/admin_functions.rs`
- `packages/api/src/server_functions/profile_change_requests.rs`
- `packages/api/src/server_functions/user_preferences_functions.rs`
- `packages/api/src/models/user_preferences.rs`

### Classes / enrollment
- `packages/api/src/server_functions/class_functions.rs`
- `packages/api/src/server_functions/class_section_functions.rs`
- `packages/api/src/server_functions/admin_functions.rs`
- class/enrollment repositories and RLS policies already hardened in PR #21

### Assignments / submissions
- `packages/api/src/server_functions/assignment_functions.rs`
- `packages/api/src/repositories/authorized_assignment_repository.rs`
- `packages/api/src/server_functions/dashboard_functions.rs`
- `packages/api/src/server_functions/submission_functions.rs`
- personalization server functions/repositories

### Parent scoped data
- `packages/api/src/server_functions/parent_scoped_functions.rs`
- legacy parent dashboard functions in `dashboard_functions.rs`

### Notifications
- `packages/api/src/server_functions/notification_functions.rs`
- notification repository

### Knowledge upload and lifecycle
- `packages/api/src/handlers/knowledge_upload.rs`
- `packages/api/src/server_functions/knowledge_functions.rs`
- `packages/api/src/repositories/knowledge_asset_repository.rs`
- `packages/api/src/server_functions/knowledge_audit_functions.rs`
- PR #24:
  - `packages/api/src/server_functions/admin_knowledge_review_functions.rs`
  - `packages/api/src/handlers/knowledge_source.rs`

---

# 3. Product truth: non-negotiable rules

## 3.1 Never combine placeholders with live data

A live application area must be one of these:

1. **Real data**
2. **Loading skeleton**
3. **First-use guide**
4. **Natural empty state**
5. **Filtered empty state**
6. **Error/degraded state**
7. **Capability unavailable state**

It must never show fabricated “example” metrics, people, activity, grades, percentages, trends or system health inside a region that otherwise looks live.

### Existing violations to remove

`school_manager/user_creation.rs` currently presents live-looking hardcoded sidebar content including:
- fake student/teacher/parent counts;
- fake “new this week / pending approval / engagement rate” values;
- fake recent activity entries;
- fake trends.

These must be deleted, not restyled.

`dashboard_functions.rs` also contains historical placeholder-derived values such as:
- student attendance `95.0`;
- class progress `75`;
- teacher class progress `60`;
- other “coming soon” zero values.

Capability gates currently reduce exposure, but production functions must not return fabricated domain values. Either:
- compute a real value from an authoritative source, or
- change the field to `Option<T>` / capability-off, or
- retire the endpoint.

`student/assignments.rs` currently falls back to `"100"` points when points are absent. This must become `None`, hidden, or localized “Not specified”; never a fake score.

---

## 3.2 Placeholder text is only a format hint

Allowed:
- `name@example.com`
- `STU001234`
- `e.g. 2026–2027`
- `Search students…`
- “Add your first class to begin enrollment.”

Not allowed:
- plausible fake current school counts;
- fabricated recent actions;
- fake named people inside a live table;
- fake percentages/trends;
- fake service-health states.

### Component contract

Introduce two different primitives:

- `GuideCard`: instructional, clearly not live data.
- `MetricCard<T>`: accepts only typed values from a loaded resource and optional provenance/help text.

Do not allow a `MetricCard` to be constructed from arbitrary example strings in production screens.

---

# 4. Release capability truth

The current product capabilities declare these disabled:
- attendance;
- timetable;
- grade trends;
- parent reports;
- parent-teacher communication;
- school-manager reports;
- derived academic metrics;
- synthetic system health.

UI consequences:
- do not create dashboard KPIs for these;
- do not show links/buttons that imply they work;
- if an unavailable page must exist, use a non-interactive `CapabilityUnavailableState`;
- do not show “0” for a capability that has no source domain;
- do not offer configuration for a delivery channel/feature that cannot actually operate unless explicitly labeled “preference saved; delivery not enabled” and that distinction is intentional.

---

# 5. Visual direction

The generated premium desktop/mobile concept is the **visual direction only**. Its illustrative metrics/content are not product requirements.

## 5.1 Design character
- professional education SaaS;
- calm, high-confidence, premium;
- warm white / neutral surfaces;
- restrained EduTalent violet/indigo accent;
- minimal decorative gradients;
- no glassmorphism as the foundational card style;
- high information clarity without density overload;
- strong typography;
- generous but purposeful spacing;
- subtle elevation only for floating layers;
- responsive by composition, not by shrinking desktop.

## 5.2 Core semantic tokens

Extend `dashboard-remake.css` custom properties into a complete token layer:

### Color
- background / surface / surface-raised / surface-subtle;
- text-primary / text-secondary / text-tertiary;
- border / border-strong;
- primary / primary-hover / primary-soft;
- success / warning / danger / info;
- disabled;
- focus-ring.

### Typography
- display;
- page title;
- section title;
- body;
- small/meta;
- label;
- numeric/metric;
- monospace only for IDs/hash/audit values.

### Spacing
Use an 8px-derived system:
`4, 8, 12, 16, 20, 24, 32, 40, 48, 64`.

### Control sizes
- compact: 36;
- default: 44;
- prominent: 48;
- minimum touch target: 44x44.

### Radius
- small 8;
- input 10;
- card 14–16;
- dialog 18–20.

### Motion
- 120–180ms utility transitions;
- 180–240ms drawer/dialog transitions;
- no decorative perpetual animation;
- `prefers-reduced-motion` must reduce/disable motion.

---

# 6. Universal application state model

Every resource-backed surface must explicitly implement this state union:

```text
Loading
Ready(Data)
FirstUseEmpty
NaturalEmpty
FilteredEmpty
ValidationError(FieldErrors)
PermissionDenied
NotFound
Conflict
RetryableUnavailable
SubsystemDegraded
Saving/Mutating
PartialSuccess
Success
CapabilityUnavailable
SessionExpired
```

## 6.1 State semantics

### Loading
- skeleton matching final layout;
- no spinner-only blank pages except tiny controls;
- do not announce fake zeros.

### FirstUseEmpty
For a tenant/user who has never created the relevant domain object.
Must contain:
- what this area does;
- the first action;
- what happens after that action;
- only an actionable CTA if the current role is allowed.

Example School Manager classes:
> “No classes yet. Create a class, then enroll students and assign a teacher.”

### NaturalEmpty
The user has data in the domain but the current queue is legitimately empty.
Example Teacher grading:
> “You’re caught up. There are no submissions waiting for grading.”

### FilteredEmpty
Data exists, but the current search/filter produced none.
Must include:
- filter/query summary;
- clear filters action.

### Error
- no raw `ServerFnError`;
- stable user message;
- optional retry;
- correlation/reference ID only if backend provides a safe one.

### Degraded
When a subsystem is unavailable but the rest of the page works:
- preserve usable data;
- disable only dependent action;
- explain which operation is temporarily unavailable;
- retry control.

### Capability unavailable
No enabled-looking controls.
Never masquerade as empty data.

---

# 7. Error contract architecture

## 7.1 Problem

Current screens frequently do:
```text
"Failed: {e}"
"Unable to load: {error}"
"Submission failed: {error}"
```

This leaks backend implementation strings into the product and gives poor action guidance.

## 7.2 Required API error shape

For redesigned/touched APIs, expose a stable serializable error contract:

```rust
struct UiError {
    code: UiErrorCode,
    field_errors: Vec<FieldError>,
    retryable: bool,
    safe_detail: Option<String>,
}
```

Example codes:
- `auth.invalid_credentials`
- `auth.session_expired`
- `user.duplicate_email`
- `user.relationship_invalid`
- `user.no_school_scope`
- `class.name_required`
- `assignment.no_eligible_students`
- `assignment.not_found`
- `assignment.not_owner`
- `knowledge.storage_unavailable`
- `knowledge.invalid_pdf`
- `knowledge.file_too_large`
- `knowledge.lifecycle_conflict`
- `profile.current_password_invalid`
- `profile.password_policy_failed`
- `network.retryable`

Backend logs keep real technical diagnostics; browser receives only safe, stable codes/details.

---

# 8. Navigation architecture

## 8.1 Current issue
Role dashboards use `active_section` signals inside one canonical dashboard route. This means:
- weak deep-linkability;
- refresh resets section;
- browser history does not represent section changes;
- harder mobile/back behavior.

## 8.2 Target
Prefer nested role-aware routes while retaining `/dashboard` as canonical entry:

```text
/dashboard
/dashboard/users
/dashboard/classes
/dashboard/knowledge
/dashboard/settings
...
```

Role guard chooses valid destinations. Backend authorization remains authoritative.

If nested routing is too risky for PR-2, the minimum acceptable fallback is:
- synchronize active section into query/hash state;
- preserve history;
- restore section on refresh;
- no duplicate role-specific unguarded routes.

## 8.3 Navigation action contract

Each navigation item needs:
- localized accessible name;
- `aria-current=page` when active;
- hidden when capability is false;
- disabled is not used for unavailable product features unless there is explanatory value;
- mobile drawer closes after navigation;
- focus moves to page heading;
- browser back restores previous destination;
- direct URL role mismatch renders a safe permission state, not a blank screen.

---

# 9. Responsive shell specification

## Desktop >= 1200
- persistent right sidebar in RTL / left in LTR using logical CSS;
- 240–264px navigation width;
- 64–72px top bar;
- content max-width ~1440;
- 24–32px page gutters;
- optional context panel only when useful.

## Tablet 768–1199
- collapsible sidebar or drawer;
- two-column cards where appropriate;
- tables may preserve table mode with horizontal scroll only if fields genuinely need it.

## Mobile < 768
- top bar + drawer for full navigation;
- optional bottom nav for the 3–4 highest frequency role tasks only;
- do not maintain two conflicting active navigation models;
- forms one column;
- dialogs become full-width sheets when content-heavy;
- tables become cards/list rows when meaningful;
- destructive actions remain explicit;
- no tiny multi-selects.

---

# 10. Shared design-system components (PR-2)

Create a canonical `ui/` or equivalent module; do not keep adding feature-local primitives.

Required primitives:

### Structure
- `AppShell`
- `PageHeader`
- `Section`
- `Card`
- `Panel`
- `Divider`
- `Stack`
- `Grid`

### Navigation
- `SidebarNav`
- `MobileNavDrawer`
- `Breadcrumbs` where deep hierarchy exists
- `Tabs`
- `SegmentedControl`

### Actions
- `Button`
- `IconButton`
- `DropdownMenu`
- `SplitButton` only where justified
- `DestructiveAction`

### Forms
- `Field`
- `TextField`
- `EmailField`
- `PasswordField`
- `TextArea`
- `Select`
- `Combobox`
- `MultiSelect`
- `Checkbox`
- `Switch`
- `RadioGroup`
- `DateInput`
- `FileDropzone`

### Feedback
- `Toast`
- `InlineAlert`
- `StatusBanner`
- `Progress`
- `Skeleton`
- `DataState`
- `GuideCard`

### Data
- `MetricCard`
- `DataList`
- `DataTable`
- `MobileDataCard`
- `StatusBadge`
- `MetadataList`

### Layers
- `Dialog`
- `Drawer`
- `Popover`
- `ConfirmDialog`

## Primitive accessibility contract
- generated unique IDs;
- every label references the exact input ID;
- `aria-describedby` includes hint + error IDs;
- field error uses `aria-invalid`;
- dialogs trap focus, close with Escape where safe, and return focus;
- destructive modal does not close on accidental backdrop click while mutation runs;
- forms use semantic `<form>` submit;
- required validation is app-localized and server-authoritative;
- do not rely on browser-native untranslated validation bubbles.

---

# 11. Authentication / session redesign

Files:
- `packages/web/src/views/login.rs`
- `packages/web/src/main.rs`
- auth/session application hooks.

## Login states
- idle;
- field validation;
- submitting;
- invalid credentials;
- inactive/unavailable account;
- session/service unavailable;
- success redirect;
- already-authenticated redirect.

## UX
- simple premium split/centered layout;
- no decorative blobs as core design;
- email/password labels explicitly bound to controls;
- password reveal button;
- language switcher;
- loading stays inside submit button;
- recovery capability remains truthful.

## Forgot password
Current email reset is unavailable. The redesigned screen must not imply an email will be sent.

## Session-expiry behavior
- detect 401/session-unavailable;
- show “Session expired” state;
- return to login without dumping raw errors;
- preserve safe intended destination if supported.

---

# 12. Header / notifications

Files:
- `header.rs`
- `notification_functions.rs`.

## Bell states
- loading summary;
- 0 unread;
- unread count;
- fetch error with retry;
- popover open/closed;
- mutation pending;
- mark-one failure;
- mark-all failure.

## Desktop
Popover aligned by logical inline end.

## Mobile
Use a sheet/full-width panel.

## Notification item
Only render navigation affordance if notification has a real supported destination. Otherwise item is read-only but can be marked read.

Do not reuse unrelated translations for notification errors.

---

# 13. School Manager redesign

## 13.1 Overview

Source:
`school_manager/dashboard.rs`

### Goal
Operations-first home:
- first-use onboarding;
- immediate tasks;
- recent real domain changes;
- only real metrics.

### First-use checklist
Derived from real queries:
1. Create first class.
2. Create/provision staff/students.
3. Enroll students.
4. Upload governed knowledge if needed.

No fake completion percentages.

---

## 13.2 User directory

Source:
`user_management.rs`

### Desktop
- page header + primary “Add user”;
- search + role/status filter;
- responsive real data table.

### Mobile
- user cards with name/role/status and overflow menu.

### States
- loading;
- first user/first staff guide;
- filtered empty;
- directory load failure;
- mutation pending;
- partial refresh failure.

### Search
Debounce 250–400ms; do not issue server query for every keystroke.

### Deactivate/reactivate/update
These endpoints remain disabled on current manifest. Do not display enabled controls until their server contract is supported.

---

## 13.3 User Creation Hub

Source:
`school_manager/user_creation.rs`  
Backend:
`user_management.rs`, `user_repository.rs`.

### Replace current layout
Remove fake “Current statistics / Quick tips / Recent activity” live-looking sidebar.

Use:
- role tabs;
- compact step indicator;
- form card;
- contextual `GuideCard` beside it.

### Student flow
Fields:
- first name;
- last name;
- email;
- phone according to authoritative required rule;
- DOB if required by real schema;
- Student ID;
- grade;
- enrollment date;
- academic year;
- optional/required parent link according to product contract;
- class enrollment as a typed relationship.

### Teacher flow
- first name;
- last name;
- email;
- phone;
- employee ID;
- department;
- hire date;
- qualification;
- subjects;
- class assignments.

Replace `<select multiple>` with searchable `MultiSelect`.
Selections render chips and explicit count.

### Parent flow
- full name;
- email;
- phone;
- Parent ID;
- associated students through typed `MultiSelect`.

### Relationship transport
Do not bury relationship UUIDs in arbitrary metadata JSON.
Add typed request fields:
```rust
teacher_class_ids: Vec<Uuid>
teacher_subject_ids: Vec<Uuid>
student_class_id: Option<Uuid>
student_parent_user_id: Option<Uuid>
parent_student_user_ids: Vec<Uuid>
```

Server still validates same-school/active constraints.

### Temporary credentials
**Current defect:** browser creates an 8-character UUID fragment and success banner exposes it.

Target:
- server generates cryptographically strong temporary credential or uses invitation/reset flow;
- browser cannot choose credential;
- no credential in logs;
- return once in typed success response only if invitation is unavailable;
- show dedicated one-time credential panel:
  - masked by default;
  - reveal;
  - copy;
  - “shown only once” warning;
- force change at first login if supported.

### Success states
- account created and relationship(s) created;
- account created but downstream optional action failed = partial success with exact next step;
- never claim welcome email sent unless a provider actually sent it.

### Browser validation
No native English “Please fill out this field”.
Fields use localized app-level validation and matching required marker.

---

## 13.4 Class Management

Source:
`class_management.rs`

### Preserve PR #21 behavior
- list failure remains visible;
- create success is explicit;
- create + refresh failure is partial success.

### New layout
- classes table/cards;
- filter/search;
- grid/list preference only if both layouts are meaningfully maintained;
- empty state CTA “Create first class.”

### Class detail
Desktop: right-side drawer or large dialog.  
Mobile: full-screen sheet.

Sections:
- class identity;
- subject/term/teacher;
- enrollment;
- assignments summary where available.

### Enroll student
States:
- loading candidates;
- no eligible un-enrolled students;
- candidate select;
- enrolling;
- success;
- conflict/already enrolled;
- school-scope failure;
- refresh failure.

### Unenroll
Require confirmation with consequences.

---

## 13.5 Knowledge Submission

Frontend:
`knowledge_upload.rs`  
Backend:
`handlers/knowledge_upload.rs`.

### Upload form
- drag/drop + browse;
- accepted type = PDF only;
- maximum size 20 MiB visibly shown;
- title required;
- subject/grade optional if schema allows;
- description optional;
- selected file preview card;
- remove/replace file;
- upload progress if transport supports meaningful progress; otherwise pending state.

### Storage readiness
Add a safe SchoolManager-only readiness endpoint:
```text
Ready
UnavailableRetryable
Misconfigured
```
Do not expose bucket secrets/internal URLs.

Page behavior:
- `Ready`: enable upload.
- `UnavailableRetryable`: preserve form fields, disable submit, show retry.
- `Misconfigured`: non-retryable operator-facing guidance without secrets.

Server remains authoritative even if UI readiness says Ready.

### Successful upload
Show:
- title;
- filename;
- `submitted` lifecycle status;
- statement: “Uploaded for platform review. OCR, embedding and publication have not occurred.”

---

## 13.6 Requests

Source:
`school_manager/requests.rs`

Current raw JSON payload diff is not user-friendly.

Target:
- requestor/user identity;
- changed fields rendered as old → requested value;
- requested timestamp;
- approve/reject buttons;
- confirm destructive/sensitive change where appropriate;
- decision pending;
- decision success;
- already decided conflict;
- safe failure.

Do not show raw JSON unless a development/admin diagnostics view explicitly requires it.

---

## 13.7 Settings

### Profile
- loading;
- load error + retry;
- clean form;
- dirty state;
- save pending;
- save success;
- save failure;
- email read-only with explanation if changes require request process.

Current profile update logs success only to console. Must show user feedback.

### Profile change request
Migrate `profile_request.rs` away from inline styles and raw server errors.

### Password/security — critical PR-1 correction
Current contradiction:
- Profile page says password changes are unavailable;
- Security page exposes a live password-change flow;
- backend `change_admin_password(new_password)` changes via admin API;
- UI collects `current_password` but **does not send or verify it**.

Choose one coherent secure contract:

**Preferred:** re-authenticate current password through the configured identity provider, then change.
- current password required;
- new password policy server-side;
- confirm client-side;
- session/security policy defined;
- success clears fields;
- error codes are safe.

If re-auth cannot be supported now:
- disable the Security password form;
- show the same truthful unavailable state everywhere.

### General settings
Current UI offers languages not supported by actual `Locale` (`Locale` supports only Farsi and English).
Target options must be generated from `Locale::all()`.

Timezone/date/time values must be validated server-side against an allowlist.

### Notification preferences
Only show/configure channels actually implemented.
Persisted preference must not imply delivery exists.

Switch primitive:
- `role=switch` / checked semantics;
- keyboard accessible;
- RTL-independent.

---

# 14. Teacher redesign

## 14.1 Overview
Keep real stats from existing APIs.
Remove hardcoded English loading/error/status strings.
Primary actions:
- grading;
- assignments;
- classes;
- knowledge.

First-use:
- if no classes assigned, explain that a School Manager must assign classes.

---

## 14.2 Classes
Source:
`teacher/classes.rs`

Remove class-name-length gradient selection.

Use consistent class cards:
- name;
- subject;
- term;
- active student count;
- actions menu.

Actions:
- overview;
- students;
- materials;
- grading.

Each subview needs loading/error/empty/ready.

AI/material vectorization statuses must be fully localized:
- checking;
- queued;
- processing;
- cancelling;
- cancelled;
- completed;
- failed;
- retry if allowed.

Never print provider/internal error directly.

---

## 14.3 Assignments

Frontend:
`teacher/assignments.rs`  
Backend:
`assignment_functions.rs`, `authorized_assignment_repository.rs`.

### Current discovered publish defect
Publishing a valid draft with zero active enrolled students returns repository `NotFound(EnrolledStudents)`, server maps it to generic `"Not found"`, UI renders raw server error.

### Target contract
Return `assignment.no_eligible_students`.

UI:
> “This assignment cannot be published because the class has no active enrolled students.”

If Teacher cannot enroll:
> “Ask a School Manager to enroll at least one student.”

If role can navigate to relevant class:
provide safe navigation CTA.

### Assignment list
Filters must actually filter:
- Draft;
- Published/Active;
- Closed/Completed if domain supports it;
- otherwise do not render fake filter tabs.

### Actions
- create draft;
- view details;
- edit draft;
- publish;
- view submissions;
- delete draft.

Delete requires confirmation.

Publish button:
- disabled/pending during mutation;
- server authoritative;
- optional preflight `eligible_student_count` may improve UX but must never replace backend enforcement.

---

## 14.4 Students
Source:
`teacher/students.rs`

- debounced search;
- filtered empty state;
- student card/table;
- view profile;
- view persisted grades;
- email link only where displaying email is authorized.

No decorative fake academic status.

---

## 14.5 Submissions / grading
Source:
`teacher/submissions.rs`

Verify the “Pending / All” filter semantics:
- current resource is `get_pending_submissions_for_teacher`;
- if “All” does not fetch all, either implement proper backend query or remove the tab.

Grading:
- Farsi scale 0–20;
- English scale 0–100;
- backend remains normalized;
- field validation;
- save pending;
- save success;
- conflict if submission changed;
- safe retry.

---

## 14.6 Teacher knowledge
Source:
`teacher/knowledge_assets.rs`.

- only published same-school assets;
- Available / Enabled states;
- toggle pending;
- safe failure;
- explicit text: enabling permits governed generation selection; it does not republish the asset.

Localize all text.

---

# 15. Student redesign

## 15.1 Overview
- work due next;
- enrolled classes;
- grades entry point;
- no disabled attendance/timetable metrics.

First use:
> “You are not enrolled in a class yet. Contact your school administrator.”

---

## 15.2 Classes
Source:
`student/classes.rs`.

One reusable class component shared stylistically with Teacher but student actions differ:
- tasks;
- materials;
- grades.

No decorative hash gradients.

---

## 15.3 Assignments
Source:
`student/assignments.rs`.

States:
- All / Pending / Submitted / Graded;
- loading;
- filtered empty;
- natural empty;
- overdue;
- detail;
- working/submitting;
- submitted;
- grade/feedback available.

Remove fake `100`-point fallback.

Clickable cards must have correct semantics:
- either card is one button/link;
- or card has independent buttons;
- do not combine clickable container with nested controls in an inaccessible manner.

Submission:
- prevent accidental duplicate submit;
- preserve text on retryable failure;
- show persisted success state.

---

## 15.4 Grades
Source:
`student/grades.rs`.

Keep the current truthfulness principle:
- only persisted assignment grades;
- no aggregate GPA/trends unless authoritative source exists.

Keep `bdi dir=ltr` for numeric/date isolation in Persian.

---

# 16. Parent redesign

Backend source:
`parent_scoped_functions.rs`.

## 16.1 Overview
Show only:
- linked children count;
- real class/enrollment context;
- safe navigation.

No synthetic GPA/report/communication metrics if disabled.

## 16.2 First-use no-child state
Distinguish:
- no child linked;
- child loading failed.

First-use copy:
> “No student is linked to this parent account yet. School administration must link a student before academic information appears.”

Do not imply the Parent can self-link if no such endpoint exists.

## 16.3 My Children
Cards:
- child name;
- grade level if real;
- enrollment/class count;
- assignments;
- persisted grades.

Actions:
- Grades;
- Assignments.

Disabled capabilities such as attendance/communication/reports remain absent or clearly unavailable.

## 16.4 PR-1 relationship regression
The create-parent workflow must be proven end to end:
1. manager chooses student;
2. typed student user UUID reaches server;
3. server verifies same school/active;
4. `students.parent_id` is updated;
5. new parent logs in;
6. scoped parent query returns that child;
7. cross-school parent does not see child.

---

# 17. Platform Admin redesign

**Behavioral baseline:** PR #24 after it is green/merged.

Do not revert to `knowledge.rs` old loose-button implementation.

## 17.1 Knowledge review
Lifecycle:
1. Submitted / source review
2. Verified OCR
3. Embedding queued
4. Embedded
5. Published
6. Archived
7. Failed with legal recovery path

Buttons are state-derived:
- Review source PDF only when private source review is available;
- Attach/Update OCR only in legal OCR states;
- Queue/Retry embedding only with verified OCR;
- Publish only when embedded;
- Archive/Withdraw with confirmation;
- archived = history/read-only.

## 17.2 Audit
Replace raw internal table presentation with:
- actor role;
- human-readable event;
- asset;
- timestamp;
- expandable technical details if appropriate.

Raw UUID/hash/JSON should be secondary details, not the primary interface.

---

# 18. Localization / RTL contract

Actual supported locales: **Farsi and English only**.

## Rules
- all canonical user-facing strings use translation keys;
- no feature-local `if is_fa { ... } else { ... }` except unavoidable formatting logic;
- no hardcoded English loading/error/status text;
- no settings dropdown options for unsupported languages;
- logical CSS properties (`margin-inline`, `inset-inline`, etc.);
- icons with directional meaning mirror in RTL;
- numeric IDs/emails/dates use `bdi`/direction isolation where appropriate;
- server error codes map to locale strings client-side.

---

# 19. Accessibility contract

Target: WCAG 2.2 AA for redesigned surfaces.

Must include:
- keyboard-only completion of all Tier-1 workflows;
- visible focus;
- 44px touch targets;
- proper headings;
- labels explicitly bound to controls;
- form errors linked with `aria-describedby`;
- `aria-live` for mutation result/status;
- dialog focus trap + Escape + focus return;
- popover/drawer semantics;
- `aria-current` navigation;
- table header associations;
- switch semantics;
- reduced motion;
- no color-only status meaning;
- RTL screen-reader order follows DOM order.

---

# 20. Security / privacy UX

- never display raw provider/API/database errors;
- never expose Storage URLs/keys;
- never use real personal data in placeholder/example content;
- temporary credentials never in generic toasts/logs;
- destructive actions need consequence copy;
- role/capability decisions remain server-authoritative;
- UI hiding is convenience, never the security boundary;
- cross-school object identifiers must fail closed.

---

# 21. Product-truthfulness enforcement

Strengthen `product_truthfulness_tests.rs`.

## Add structural tests
1. Canonical UI files cannot contain banned fake-live metric fixtures.
2. `MetricCard` must be sourced from resource result/typed data.
3. No fallback like `unwrap_or("100")` for academic values.
4. No raw `format!("{error}")` in production user-facing error containers on canonical role views.
5. Visible server-function calls must correspond to non-Disabled endpoint-manifest entries.
6. Capability-disabled endpoints/features cannot have active controls.
7. Supported-language selector values must equal `Locale::all()`.
8. No canonical role page imports the retired old manager knowledge URL submission UI.
9. No old `glassmorphism`/`glass-card` primitive usage remains after PR-3 in canonical views.
10. No inline `style:` on canonical role UI after migration except genuinely data-driven style values.

---

# 22. PR-1 — Workflow correctness and truthful state/data contracts

## 22.1 Objective
Before the full redesign, fix correctness/security defects exposed by real usage and make the data/state contract trustworthy.

## 22.2 Required implementation

### A. Provisioning
Files:
- `school_manager/user_creation.rs`
- `user_management.rs`
- `user_repository.rs`
- related DTO/model definitions.

Changes:
- server-generated temporary credential or invitation flow;
- typed relationship fields;
- replace native multi-select parsing;
- authoritative required-field schema;
- remove duplicate metadata keys;
- one-time credential success object;
- remove fake sidebar stats/activity;
- remove inert enabled Bulk Import action;
- parent link E2E;
- teacher class/subject persistence E2E;
- student class enrollment E2E.

### B. Assignment publish
Files:
- `teacher/assignments.rs`
- `assignment_functions.rs`
- `authorized_assignment_repository.rs`.

Changes:
- stable `NoEligibleStudents` domain error;
- actionable localized UI;
- preserve not-found/unauthorized distinction internally without leaking objects;
- publish succeeds after eligible enrollment.

### C. Knowledge storage
Files:
- `knowledge_upload.rs`
- `handlers/knowledge_upload.rs`
- endpoint manifest if new readiness endpoint added.

Changes:
- readiness contract;
- stable error codes;
- form preserved on retryable failure;
- no raw error text.

### D. Parent
- prove exact manager-create → parent-login → child-visible contract;
- reconcile or retire legacy parent dashboard functions that compare the wrong identifier domain.

### E. Settings security
Files:
- `settings/security.rs`
- `settings/profile.rs`
- `admin_functions.rs`.

Decision:
- implement real current-password reauthentication before admin password change, **or**
- disable the live password form consistently.

Never collect a current password and ignore it.

### F. Truthfulness cleanup
- remove fake user-creation metrics/activity;
- remove `100` points fallback;
- remove/quarantine backend placeholder attendance/progress values;
- restrict settings language list to Farsi/English;
- remove/label notification settings that do not map to real delivery capability.

## 22.3 PR-1 tests

### Required unit/integration
- provisioning role allowlist;
- typed relationship validation;
- no privileged role provisioning;
- duplicate email;
- parent relation success/cross-school rejection;
- teacher assignment relation success/cross-school rejection;
- Auth compensation on DB failure;
- assignment no-eligible-student typed error;
- assignment publish success after enrollment;
- storage readiness states;
- upload PDF limits/type/signature/hash tests;
- password reauth policy if enabled.

### Browser E2E
Only these critical journeys:
1. manager create Student;
2. manager create Teacher;
3. manager create Parent linked to student;
4. parent login sees child;
5. teacher draft detail loads;
6. publish with no students gives actionable state;
7. after enrollment publish succeeds;
8. manager knowledge storage unavailable state;
9. manager knowledge upload success.

### Static
- product truthfulness scanner;
- endpoint manifest/UI consistency.

## 22.4 PR-1 workflows
Required:
- **AI Change Proof** classifier;
- changed Rust formatting;
- affected API/web check + lint + tests;
- PostgreSQL/RLS invariants only because touched relationship/assignment queries require it;
- browser critical journeys above.

Not required unless automatic path policy selects them:
- appliance build;
- full package;
- AI provider proof;
- full production operations.

## 22.5 PR-1 merge gate
- exact-head targeted checks green;
- no unresolved P1/P2 review blockers;
- no raw technical errors in touched browser paths;
- no fake-live data in touched paths;
- merge, then branch PR-2 from new `main`.

---

# 23. PR-2 — Design system and global interaction architecture

## 23.1 Objective
Create the permanent UI foundation without changing feature business semantics.

## 23.2 Files
Primary:
- `packages/web/assets/dashboard-remake.css`
- new `packages/web/src/ui/*` or equivalent;
- `shared/common.rs`
- `shared/forms.rs`
- `shared/tables.rs`
- `components/validation.rs`
- `components/error_boundary.rs`
- `dashboard_layout.rs`
- `header.rs`
- `sidebar.rs`
- `routing_service.rs`
- `login.rs`
- `main.rs`
- i18n translation files.

## 23.3 Implementation
- semantic tokens;
- typography/spacing/elevation;
- all primitives listed in §10;
- nested/deep-link navigation or history-safe section state;
- responsive desktop/tablet/mobile shell;
- auth/session states;
- notification popover/sheet;
- error/data-state system;
- localized validation;
- dialog/drawer/popover accessibility;
- theme/dark mode if retained;
- RTL from logical properties;
- one state/event convention for async buttons.

## 23.4 PR-2 tests
- component unit tests;
- compile/lint;
- navigation role matrix;
- direct route authorization;
- desktop/mobile browser shell;
- keyboard drawer/dialog/popover;
- axe/WCAG automated checks;
- RTL/LTR screenshots;
- reduced-motion CSS test where feasible.

## 23.5 PR-2 workflows
Required:
- AI Change Proof;
- web/frontend Rust check/test;
- browser smoke;
- accessibility/browser harness.

Not required:
- DB migration validation unless an API contract unexpectedly changes;
- appliance/AI/backup workflows.

## 23.6 Merge gate
- primitives used by shell/auth/notifications;
- no new legacy glass/inline styles in PR-2 surfaces;
- desktop + mobile critical shell journeys green;
- exact-head merge-ready.

---

# 24. PR-3 — Complete role-screen redesign

## 24.1 Objective
Migrate every canonical authenticated page to PR-2 primitives and the premium responsive design.

## 24.2 Migration order inside the PR

### Slice 1 — School Manager
- overview;
- user directory;
- creation flows;
- class management;
- knowledge upload;
- requests;
- settings/profile/security/general/notifications.

### Slice 2 — Teacher
- overview;
- classes;
- assignments;
- students;
- submissions/grading;
- knowledge.

### Slice 3 — Student
- overview;
- classes;
- assignments;
- grades.

### Slice 4 — Parent
- overview;
- children.

### Slice 5 — Platform Admin
- PR #24 knowledge review;
- audit;
- profile.

### Slice 6 — cleanup
- remove obsolete duplicate knowledge components;
- remove unused legacy cards/forms/modals;
- remove old inline styles;
- remove canonical `glass-card` dependencies;
- keep only explicit unavailable-feature components for capability-off surfaces.

## 24.3 PR-3 responsive acceptance
For every role:
- 1440px desktop;
- ~1024px tablet;
- 390px mobile;
- Farsi RTL;
- English LTR.

## 24.4 PR-3 browser acceptance journeys

### School Manager
- navigate all visible sections;
- first-use empty states;
- create user;
- create class;
- enroll student;
- upload PDF;
- settings save.

### Teacher
- class list;
- student list;
- assignment draft/detail/publish;
- grading;
- knowledge toggle.

### Student
- assignment filters;
- assignment detail/work/submit;
- classes;
- grades.

### Parent
- linked child;
- assignments;
- grades;
- no-child first-use state.

### Platform Admin
- source review;
- OCR;
- embed;
- publish;
- archive confirmation;
- audit.

## 24.5 PR-3 tests/workflows
Required:
- changed Rust format/check/test;
- browser final role journeys desktop/mobile;
- axe/WCAG automated suite;
- keyboard-only automated coverage where feasible;
- RTL/LTR product acceptance;
- product-truthfulness scanner;
- visual screenshot regression against approved redesign ledger.

API/database tests only if PR-3 changes a backend contract. Pure visual migration must not trigger unnecessary DB/appliance proofs manually.

## 24.6 Merge gate
- zero canonical old glass/inline feature UIs;
- no fake data;
- no raw server errors;
- all role critical journeys green;
- responsive screenshots approved;
- no unresolved P1/P2 review threads.

---

# 25. File-by-file migration matrix

| File / area | Current problem | Target PR |
|---|---|---|
| `main.rs` | bespoke route/session fallback copy | PR-2 |
| `routing_service.rs` | internal section-state navigation, hardcoded labels | PR-2 |
| `dashboard_layout.rs` | good base but must become full responsive app shell | PR-2 |
| `header.rs` | mixed loading/error copy, weak notification state | PR-2 |
| `sidebar.rs` | localized/async signout gaps | PR-2 |
| `error_boundary.rs` | inline styles, raw errors, inert “Go Dashboard” | PR-2 |
| `unavailable_feature.rs` | old glass style; behavior is correct | PR-2/3 |
| `shared/common.rs` | legacy glass Card, static modal title ID, incomplete layer contract | PR-2 |
| `shared/forms.rs` | labels not bound to input IDs, native validation leakage | PR-2 |
| `shared/tables.rs` | stringly table, generic empty, arbitrary metric strings | PR-2 |
| `components/validation.rs` | inline styles, hardcoded English, incomplete real validation | PR-2 |
| `login.rs` | legacy gradient/glass, bespoke unavailable modal | PR-2 |
| `school_manager/user_creation.rs` | fake metrics/activity, client temp password, multi-select/required defects | PR-1 then PR-3 |
| `school_manager/user_management.rs` | raw mutation errors, no debounce, old directory layout | PR-1/3 |
| `school_manager/class_management.rs` | correct PR21 state behavior but old styling/bespoke select | PR-3 |
| `school_manager/knowledge_upload.rs` | no readiness state, native file control/raw errors | PR-1/3 |
| `school_manager/requests.rs` | inline styles/raw JSON/raw errors | PR-3 |
| `settings/profile.rs` | silent load/save errors; password contradiction | PR-1/3 |
| `settings/security.rs` | current password collected but ignored | PR-1/3 |
| `settings/general.rs` | unsupported languages; inline styles/raw errors | PR-1/3 |
| `settings/notifications.rs` | persisted options may imply unavailable delivery; inaccessible custom switch | PR-1/3 |
| `teacher/dashboard.rs` | hardcoded status/loading strings | PR-3 |
| `teacher/classes.rs` | bespoke cards/modals, fake decorative color hash, raw errors | PR-3 |
| `teacher/assignments.rs` | raw publish error, ambiguous actions, delete confirmation gap | PR-1/3 |
| `teacher/students.rs` | old cards/raw errors | PR-3 |
| `teacher/submissions.rs` | possibly misleading All filter; old forms | PR-1/3 |
| `teacher/knowledge_assets.rs` | hardcoded English/raw errors | PR-3 |
| `student/dashboard.rs` | status localization / mixed copy | PR-3 |
| `student/classes.rs` | decorative gradients/raw errors | PR-3 |
| `student/assignments.rs` | fake 100-point fallback, nested clickable controls/raw errors | PR-1/3 |
| `student/grades.rs` | correct truth model, old styling/hardcoded copy | PR-3 |
| `parent/dashboard.rs` | simple state UX | PR-3 |
| `parent/children.rs` | no-link guidance/localization improvements | PR-1/3 |
| `knowledge.rs` | duplicate/obsolete manager + old admin paths | PR-3 cleanup |
| PR24 `platform_admin.rs` | behavior good; hardcoded English/old glass | PR-3 |
| `dashboard-remake.css` | override layer over legacy UI rather than final component system | PR-2/3 |
| `product_truthfulness_tests.rs` | token scanner too narrow | PR-1/2/3 |

---

# 26. Backend contract migration matrix

| Backend file | Required change |
|---|---|
| `user_management.rs` | typed relationships, server-generated credential/invite result, stable errors |
| `user_repository.rs` | preserve same-school relationship enforcement; regression tests |
| `assignment_functions.rs` | stable assignment publish error codes |
| `authorized_assignment_repository.rs` | explicit no-eligible-students condition |
| `handlers/knowledge_upload.rs` | stable upload/readiness errors; safe readiness support |
| `parent_scoped_functions.rs` | keep as canonical parent read path |
| `dashboard_functions.rs` | retire/quarantine placeholder-derived legacy values; prevent accidental parent-ID misuse |
| `admin_functions.rs` | password reauth or disable; safe profile result errors |
| `notification_functions.rs` | already bounded/scoped; expose stable safe codes if needed |
| `user_preferences_functions.rs` | validate supported locale/timezone/date/time/channel values |
| `product_capabilities.rs` | remain UI truth source for disabled capabilities |
| `endpoint_authorization_manifest.psv` | remain authoritative action availability inventory |
| PR24 `admin_knowledge_review_functions.rs` | preserve source-review metadata + OCR readiness behavior |

---

# 27. First-use guide library

Every first-use guide must answer:
1. What is this?
2. Why is it empty?
3. What is the next allowed action?
4. Who can perform that action?

Examples:

### School Manager / Users
> “No school users yet. Add a teacher, student or parent to begin. Accounts are created only for this school.”

CTA: `Add user`

### School Manager / Classes
> “No classes yet. Create a class, then enroll students and assign a teacher.”

CTA: `Create class`

### Teacher / Classes
> “No classes are assigned to you yet. A School Manager must assign a class before teaching workflows appear.”

No CTA if Teacher cannot assign.

### Teacher / Grading
> “You’re caught up. No submissions are waiting for grading.”

No fake activity.

### Student / Classes
> “You are not enrolled in a class yet. Contact your school administrator.”

### Parent / Children
> “No student is linked to this parent account yet. School administration must link a student before academic information appears.”

### Platform Admin / Knowledge
> “No manager submissions are waiting for review.”

### Knowledge storage degraded
> “Knowledge storage is temporarily unavailable. Existing submissions remain visible; new PDF uploads are paused.”

CTA: `Retry storage check`

---

# 28. Button behavior standard

Every mutating button has:

```text
Idle
→ Pressed
→ Pending/disabled
→ Success
   or
→ Field error
→ Domain conflict
→ Retryable failure
→ Non-retryable failure
```

Rules:
- no double submission;
- pending label is specific (“Creating…”, “Publishing…”, not generic “Loading”);
- success tells what changed;
- partial success tells what persisted and what did not;
- destructive actions require confirmation unless trivially reversible;
- disabled buttons need a reason if visible;
- buttons never exist solely as decoration.

---

# 29. Modal / drawer behavior standard

### Dialog
Use for:
- create/edit forms;
- confirmations;
- focused grading.

### Drawer
Use for:
- class/user details;
- assignment detail where context should remain visible.

### Mobile sheet
Heavy dialogs/drawers become full-screen or near-full-screen.

All:
- focus trap;
- initial focus is meaningful;
- Escape closes unless mutation in irreversible pending stage;
- close button has localized label;
- focus returns to trigger;
- body scroll lock;
- unique title/description IDs.

---

# 30. Visual regression and fidelity ledger

For PR-2/3 store approved screenshots/artifacts for:
- School Manager desktop/mobile RTL/LTR;
- Teacher desktop/mobile;
- Student desktop/mobile;
- Parent desktop/mobile;
- Platform Admin desktop/mobile;
- login;
- dialog;
- empty state;
- error/degraded state.

Visual review checks:
- no clipping;
- no horizontal page overflow;
- no tiny controls;
- consistent card radius/spacing;
- logical RTL mirroring;
- no raw browser default select/multi-select where custom control is required;
- no giant unused blank region caused by fixed-width form columns;
- no fake data.

---

# 31. Data provenance rule for metrics and summaries

Any numeric metric shown in a dashboard must have an explicit source definition:

```text
Metric name
Source endpoint/query
Scope
Time window
Null/empty semantics
Freshness
Permission
```

If any of these is undefined, the metric does not ship.

Examples:
- “Total school users” can ship if sourced from actual school-scoped user count.
- “Engagement rate 94%” cannot ship without an authoritative event model and time window.
- “Attendance 95%” cannot ship while attendance capability is false.
- “Progress 75%” cannot ship from a constant.

---

# 32. Performance rules

- debounce server-backed search;
- avoid re-fetching every tab when data can be cached safely;
- preserve cache invalidation after mutations;
- skeleton only the region being loaded;
- do not block the whole dashboard for one failed widget;
- pagination/virtualization for large user/student lists;
- no unnecessary animation;
- mobile bundle should not load unused decorative assets.

---

# 33. Implementation discipline

For each PR:
1. branch from latest merged `main`;
2. keep scope exactly to that PR;
3. no historical migration edits;
4. no weakening endpoint authorization/RLS;
5. no fake local bypasses;
6. run only classifier-selected/relevant checks;
7. inspect exact-head CI;
8. resolve review blockers;
9. merge only when exact head is green;
10. branch next PR only after merge.

---

# 34. Definition of done

The redesign is complete only when:

- all canonical role pages use one shared design system;
- desktop/tablet/mobile layouts are intentional;
- Farsi RTL and English LTR both pass critical journeys;
- no real-looking placeholder data exists in live regions;
- no backend placeholder values appear as product metrics;
- first-use guides are distinct from live data;
- raw `ServerFnError`/DB/provider text never reaches users;
- required markers, field semantics, frontend validation and server validation agree;
- parent relationship creation is proven end to end;
- assignment publish no-student condition is actionable;
- knowledge storage degradation is explicit before submit;
- password flow is coherent and secure;
- unsupported languages/features are not configurable as if active;
- Platform Admin lifecycle is state-gated;
- keyboard/automated WCAG checks are green;
- no canonical old glass/inline UI remains;
- product truthfulness tests prevent regressions;
- PR #16 remains responsible for legitimate human/external production acceptance.

---

# 35. AI execution checklist

When this file is given to another implementation chat/agent:

1. Check current `main` and PR #24 state.
2. If #24 is green/merge-ready, merge it first; do not duplicate its lifecycle work.
3. Start PR-1 from latest `main`.
4. Implement only §22.
5. Run only §22.3/22.4 gates.
6. Merge PR-1 when exact-head green.
7. Start PR-2 from new `main`.
8. Implement only §23.
9. Run only §23.4/23.5 gates.
10. Merge PR-2 when exact-head green.
11. Start PR-3 from new `main`.
12. Migrate role surfaces in §24.2 without business-contract drift.
13. Run only §24.4/24.5 gates.
14. Merge PR-3 when exact-head green.
15. Re-run a final targeted browser role matrix on merged `main`.
16. Do not mark PR #16/human production acceptance complete without genuine external evidence.

---

## Final architecture principle

**EduTalent should never look more certain than its data is.**

The final UI must make the distinction between:
- real data,
- no data yet,
- feature not enabled,
- temporary subsystem failure,
- insufficient permission,
- validation error,
- completed workflow

immediately understandable.

That truthfulness, combined with the premium responsive design system, is the core requirement of this redesign.
