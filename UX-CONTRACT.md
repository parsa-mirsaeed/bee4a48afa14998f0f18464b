# EduTalent UI interaction contract

This contract documents the existing, approved observable behavior that the Aurora redesign must preserve. Visual values live in [DESIGN.md](DESIGN.md); this file is not a token source. Product capability truth is maintained by `docs/release/feature-matrix.md` and server-side permission checks.

## Canonical owners

| Capability | Owner | Required behavior |
| --- | --- | --- |
| Role and section routing | `application/routing_service.rs`, `src/main.rs` | Authenticated and authorized sections only; back/forward and reload retain location |
| Dashboard shell and drawer | `views/role_based/components/dashboard_layout.rs` | One role-aware navigation model at every viewport; no competing mobile navigation tree |
| Sidebar destinations | `ui/navigation.rs::SidebarNav` | Exactly one current item, `aria-current="page"`; navigation restores focus to main |
| Header notifications | `views/role_based/components/header.rs`, `ui::Popover` | Live unread state, busy and failure states, Escape closure, named accessible trigger |
| Shared buttons, forms, alerts, overlays | `packages/web/src/ui/` | Canonical semantics, validation, pending state, localized feedback, keyboard and focus |
| Authorization and data | Server functions and API authorization | Denied access stays denied regardless of UI route/display |

## Navigation and focus

Desktop navigation is a persistent sidebar. At compact widths, the same sidebar opens as a modal-like drawer with a closing control and overlay. Opening moves focus to close; Escape or overlay closes and restores focus to the menu trigger. Selecting a destination closes the drawer, updates the URL, and focuses `#dashboard-main-content`. Preserve the independent `.et-dashboard-content` scroll region so the sidebar never grows with long pages.

## Async, errors, and feedback

Every data-dependent view must distinguish loading, usable, empty, permission denied, and failed states. Errors provide a comprehensible, localized recovery path when safe. Mutations cannot be repeated while pending; destructive changes require the existing business-specific confirmation and server authorization. Never declare success before the server confirms it. Do not expose raw secrets or backend error payloads in UI notices.

## Forms, lists, and tables

Inputs retain entered values on recoverable validation errors, show associated text errors, and expose keyboard-visible focus. Interactive data rows need a real action. Search/filter/sort are exposed only where backed by a real dataset contract, with URL state preserved when supported. Do not turn a list into fake cards or imply unsupported bulk actions.

## Language, themes, and accessibility

English and Persian UI are supported, including native RTL document flow and correct LTR presentation for technical identifiers. Use the existing localization API rather than CSS-injected language labels. Preserve light/dark semantic hierarchy, 44px target preference, focus visibility, reduced-motion handling, heading order, semantic tables, and accessible names for icon-only controls. Aim for WCAG 2.2 AA.

## Proof requirements

At minimum, exercise one representative workflow per role; desktop 1440px, tablet 768–1024px, mobile 390px; English/Persian; light/dark; keyboard and reduced-motion. Run focused browser tests and exact-head CI checks. Static CSS inspection does not prove visual or interaction quality.

## Migration rule

Existing shared owners remain canonical until explicitly replaced with equivalent behavior and tests. Aurora's last-loaded CSS adapter is a transitional presentation layer, not permission to fork or duplicate the behavior of shared controls.
