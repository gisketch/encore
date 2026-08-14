# Paper & Grain Design System Foundation

> Status: **APPROVED** (self-grilled 2026-08-13; decisions below).
> Source of truth for visuals: Claude Design project "Encore Mockups"
> (turn 1 direction note, turn 2 dark variants, tweaks-panel props).

## Problem and Outcome

The current shell uses a one-off dark "glass rail" palette defined ad hoc in
`app.css`. The approved design direction is "warm cream paper, grain texture,
pastel accents" with a warm-charcoal dark mode. Every current and future
surface (action bar, settings, library, editor) must draw from one shared
token layer so restyles and theme switches are single-point changes.

Outcome: a single CSS custom-property token layer plus typography and texture
assets that all Encore surfaces consume, with light, dark, and system
appearance resolution.

## In Scope

- CSS custom properties for the full palette, both themes:
  - Light: canvas `#e9e1cf`, surface `#faf6ec`, raised surface `#fffdf6`,
    inset surface `#f3ecda`, ink `#37322a`, secondary ink `#4a4438`,
    muted `#8c8371`, faint `#a39a86`, hairline `rgba(70,58,35,.12–.16)`.
  - Dark: canvas `#262019` / window `#2b251d`, surface `#322c22`, raised
    `#3b3428`, ink `#ece4d2`, secondary `#d8cfba`, muted `#9a8f7a`,
    hairline `rgba(236,228,210,.10–.14)`.
  - Accent: `--accent`, default green `#7a9b6d` (user decision 2026-08-13;
    the mockups render the orange option `#dd7a55` — apply the green token,
    not the mockup's literal orange); derived tints via
    `color-mix(in oklab, …)` exactly as in the mockups (e.g. 14% tint on
    light surfaces, 22% on dark), success/health green `#93b28a`.
- Grain texture: the inline-SVG `feTurbulence` data-URI from the mockups as
  `--grain`, applied as `background-image` on window and bar surfaces.
  Intensity is a numeric opacity parameter (mockup range 0–120 mapped to
  opacity/1000, default 55).
- Typography: bundle "Instrument Sans" (UI) and "Spline Sans Mono"
  (numeric/status/kbd text) as local font assets — no network font loading
  (local-only constraint). Weights per mockups: Sans 400–700, Mono 400–600.
- Shape and depth language: pill radii (full-height rounding on bars and
  controls), soft warm shadows (`rgba(60,45,20,…)` light /
  `rgba(0,0,0,…)` dark), `inset 0 1px` highlight on floating surfaces.
- Appearance resolution: Light / Dark / System modes. System follows
  `prefers-color-scheme`; explicit choice overrides it. The resolved theme
  applies to every Encore window consistently.
- Motion tokens: keep `motion` package usage; `recpulse` recording pulse and
  existing entrance animation restyled to token colors;
  `prefers-reduced-motion` continues to disable/zero animations.
- Replacement of the legacy palette variables (`--rail`, `--signal`,
  `--mint`, `--coral`, `--fog`, etc.) across existing components.

## Out of Scope

- The per-surface layouts themselves (separate specs: action bar, settings,
  library, editor).
- User-configurable accent color and grain intensity in Settings. The token
  layer must make them trivially swappable (they were tweaks-panel props in
  the design canvas: accent options `#dd7a55` / `#7a9b6d` / `#c9973f` /
  `#6d8fa8`, grain 0–120), but exposing UI for them is a later decision.
- Windows theming.

## Acceptance Criteria

- All rendered surfaces derive colors, fonts, radii, and shadows from the
  shared tokens; no component-local hex values that duplicate a token.
- Switching appearance Light → Dark → System immediately restyles every open
  Encore window without relaunch; System tracks the OS setting live.
- The recording accent, status greens, and text hierarchy match the mockup
  values in both themes.
- The app renders correctly with no network access (fonts and grain are
  bundled/inline).
- Reduced-motion preference still suppresses pulse/entrance animation.

## Implementation Constraints and Settled Decisions

- Tokens live in one stylesheet layer loaded before component styles; theme
  switching is a root-level attribute/class swap, not per-component logic.
- The grain data-URI is generated from one source of truth so intensity can
  become a parameter later without touching consumers.
- Appearance preference persistence belongs to the Settings spec; until that
  lands, System is the default behavior.

## Expected Validation

- Visual smoke of each surface in light and dark against the mockups.
- A check (grep-level or lint) that legacy palette variables are gone.
- Manual toggle of macOS appearance while Encore runs (System mode).

## Grilled Decisions (2026-08-13)

- Fonts: Instrument Sans and Spline Sans Mono are OFL-licensed; bundle them
  as local woff2 assets under the frontend asset tree with `@font-face`
  declarations in the token stylesheet. No runtime network fetch. License
  files ship alongside the fonts.
- Theme mechanism: a `data-theme="light" | "dark"` attribute on the root
  element selects the palette. "System" removes the attribute and a
  `prefers-color-scheme` media-query block supplies the dark palette —
  Tauri's WKWebView tracks macOS appearance live, so no Rust-side
  appearance event is needed.
- Token file: one `theme.css` (tokens, `@font-face`, grain, keyframes)
  loaded before component styles. Components consume only `var(--…)`.
- Grain: applied as a tiled `background-image` on surface elements (never
  `background-attachment: fixed`), so it moves with the window and cannot
  shimmer during drags. Default opacity parameter 55 (mockup scale /1000).
- Accent and grain intensity stay hardcoded tokens (green `#7a9b6d`, 55);
  no settings UI in this migration. The accent is green per user decision
  (2026-08-13), chosen from the mockup tweaks-panel options.
- The accent and the health/success green `#93b28a` are now close in hue.
  They stay distinct tokens: `--accent` drives actions and the recording
  pulse; the health green drives only the small status dots (buffer badge,
  local indicator). If contrast between them proves too weak in visual
  smoke, adjust the health green toward a cooler/desaturated value — never
  the accent.

## Risks

- Visual drift between mockup `color-mix()` tints and shipped values —
  mitigated by side-by-side visual smoke against the mockups per surface.
