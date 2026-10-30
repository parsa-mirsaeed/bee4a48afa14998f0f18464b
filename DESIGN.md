---
version: alpha
name: "EduTalent Aurora Workspace"
description: "An expressive, accessible bilingual command workspace for school learning and operations."
colors:
  primary: "#5741da"
  primaryHover: "#402bb9"
  primarySoft: "#f0ecff"
  background: "#f5f6fc"
  surface: "#ffffff"
  surfaceSubtle: "#f0f2fa"
  surfaceStrong: "#e8ebf6"
  text: "#1b2041"
  textMuted: "#616b87"
  border: "#e0e5f0"
  borderStrong: "#bfc8dc"
  focus: "#4d37cf"
  sidebar: "#14182e"
  sidebarText: "#b8c1dd"
  danger: "#b4233e"
  success: "#087449"
  warning: "#8c5700"
  info: "#1c59ba"
  darkBackground: "#101326"
  darkSurface: "#1b2039"
  darkText: "#f3f5ff"
typography:
  latin:
    fontFamily: "Poppins, Inter, system-ui, sans-serif"
  persian:
    fontFamily: "Yekan Bakh, YekanBakh, Vazirmatn, sans-serif"
  technical:
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace"
rounded:
  DEFAULT: "0.75rem"
  sm: "0.75rem"
  md: "1rem"
  lg: "1.25rem"
  xl: "1.5rem"
spacing:
  shellSidebar: "276px"
  shellTopbar: "82px"
  pageMax: "92rem"
  sectionGap: "30px"
components:
  sidebar: {}
  topbar: {}
  overview: {}
  button: {}
  panel: {}
  table: {}
  input: {}
  dialog: {}
---

# EduTalent — Aurora Workspace design system

## Overview

### Creative North Star

A modern learning command centre: an ink-blue navigation spine supporting a luminous, highly legible workspace. The product should feel energetic and advanced without making real school operations seem like a game.

**Audience:** platform administrators, school managers, teachers, students, and parents. Each needs immediate access to authorized actions and real information. Primary use is desktop with responsive tablet/mobile support.

**Register:** application/product across authenticated routes; the sign-in route may adopt the palette in a later visual slice. The user specifically requested an eye-catching radical remake. This direction intentionally replaces the prior muted visual approach while preserving its behavior requirements.

**Signature:** the orbital learning-path motif on overview headers. It is decorative, not an inferred completion metric. It appears once per overview; no unrelated animated blobs or gradients.

**Restraint:** data tables, forms, editing screens, permission errors, destructive actions, and long Persian text prioritize fidelity and legibility.

**Anti-references:** generic all-white dashboards; repeated frosted-glass cards; fake metric-heavy dashboards; animated startup sequences that delay access.

**Locale:** English (Poppins) and Persian (locally available Yekan Bakh, otherwise Vazirmatn); use logical inline/block CSS, RTL document direction, visible labels, and existing locale bindings. No assumptions about a school market beyond these product locales.

**Ownership:** existing Dioxus components in `packages/web/src/ui` are canonical behavior owners. For the first migration slice, `packages/web/assets/experience-2026.css` is the last-loaded presentation token adapter; `packages/web/assets/design-system.css` is the established primitives layer; `dashboard-remake.css` provides temporary legacy adapters. This document mirrors the accepted values rather than generating CSS. Consolidate tokens into the canonical layer in a later explicit migration, rather than changing the three sources independently.

## Colors

Light: fog background `#f5f6fc`, white surfaces, ink text `#1b2041`, muted text `#616b87`, purple action `#5741da`. Strongest contrast appears between the midnight sidebar `#14182e` and pale navigation text.

Dark: `#101326` app background and `#1b2039` surfaces; text `#f3f5ff`, primary `#b8a8ff`. Semantic danger, warning, success, and info remain distinct. Never encode a status only in color. Brand purple is not interchangeable with semantic danger.

Token map: `colors.primary` → `--et-color-primary` → `--et-accent` legacy alias → `Button`, action cards, and selected navigation. `colors.background` → `--et-color-bg` / `--et-bg` → dashboard content. `colors.sidebar` → `.et-sidebar`. `colors.focus` → `--et-color-focus` → focus-visible. The alias mapping is defined once in `experience-2026.css`.

## Typography

**Font delivery:** Yekan Bakh is a commercial font. The locale CSS currently prefers an installed Yekan Bakh family only; without licensed webfont assets it falls back to bundled Vazirmatn. Do not claim a universal font switch or include unlicensed font binaries/CDNs. Add properly licensed WOFF2 files and verify Persian RTL/mobile/desktop screenshots before considering the typeface rollout complete.

Poppins for English UI, locally installed Yekan Bakh preferred with self-hosted Vazirmatn fallback for Persian UI, monospace for technical identifiers as supported by existing i18n conventions. Page title 19px in shell; overview title scales from 30px to 44px; section heading 17–20px; list text 13–14px. Persian headings disable artificially compressed letter spacing. Preserve native scripts, wrapping, and reading order.

## Layout

Desktop: 276px persistent inline-start navigation spine, 82px header, independently scrolling content region with a maximum width of 92rem. Tablet at up to 1100px and mobile at up to 767px reuse the existing drawer/navigation and focus lifecycle; do not create separate route trees.

Use 15–30px gaps according to density. The mobile work area uses 16px inline padding, and overview illustrations retreat to protect text. Do not place a decorative overlay over interactive areas.

## Elevation & Depth

The sidebar is solid ink, and product surfaces are predominantly opaque. Use gentle shadows for actionable tiles, menu overlays, and overview hero. Tables, rows, and form groups use subtle borders. Motion is limited to clear hover feedback and respects `prefers-reduced-motion`.

## Shapes

12px controls, 16–19px panels, 24px hero, square-leaning 12–15px avatar/brand icons. Status chips retain their semantic, compact shape. Do not introduce unlimited pill styles.

## Components

### Foundational states

Retain the shared `Button`, `DataState`, `InlineAlert`, `Popover`, `SidebarNav`, `Dialog`, and form controls. Existing async, permission, error, retry, empty, and busy behavior is authoritative. Disabled actions must remain visibly disabled; no made-up metrics or unauthorised shortcuts.

### Buttons and actions

Primary is solid purple with legible foreground; secondary is outlined; destructive remains semantically red. Ensure 44px interaction targets and keyboard-visible focus. Action-card hover lift is 3px only when motion is allowed.

### Navigation and data display

The selected route is identified by a filled purple row and `aria-current="page"`. The sidebar is shared across all five roles; navigation content remains server-authorization-aware. Tables retain their semantic headings and responsive overflow strategy.

### Forms and overlays

Preserve canonical validation, focus, Escape, async state, and localized labels. Overlay layers remain above the topbar. Input focus uses a clearly visible ring; never obscure it with decoration.

### Iconography

Keep the existing Material Icons Outlined font; do not add an external icon CDN or fetch fonts at runtime. Every icon-only control retains a localized accessible name.

### Motion

Short 130–170ms hover and color transitions only; no motion required to understand state. Reduced-motion mode disables lift and transitions.

### Content and data visualization

Use task and domain vocabulary from the product. Show real authorized data, explicit unknown/empty states, and actionable next steps. No decorative pseudo-data or fabricated progress.

## Do's and Don'ts

- **Do:** use the shared shell and canonical UI components for every role.
- **Do:** test RTL, narrow viewports, dark mode, keyboard navigation, and reduced motion.
- **Do:** preserve offline operation and CSP/no-external-origin requirements.
- **Don't:** visually imply features disabled in `product_capabilities`.
- **Don't:** replace scoped Dioxus/server-function behavior with a parallel React application.
- **Don't:** modify protected domain, database, or authorization behavior to achieve a visual effect.
