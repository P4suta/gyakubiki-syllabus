# Design Tokens

**Porcelain & Ocean** — a near-white cool field, one slate-tinted neutral ("ink")
for all text and structure, and a single ocean-blue accent. The palette follows
the OS light/dark setting. Every design value lives as a Tailwind CSS 4 `@theme`
token in `src/app.css` — that file is the source of truth for all hex colors,
font sizes, shadows, easings, and animations. Components use semantic utility
classes; they never hardcode `#`, px, or shadow values.

`design-tokens.test.ts` enforces this against `app.css`.

## Themes

- Light values live in `@theme`. Dark values override the **same** semantic
  tokens under `@media (prefers-color-scheme: dark)` (Tailwind emits its
  utilities as `var(--color-…)`, so redeclaring the variables cascades). Never
  add a `-dark` suffixed token — flip the value, keep the name.
- The course-tile and eval-arc colors are applied as inline styles (dynamic, so
  not CSS tokens). They carry a `light`/`dark` pair in `lib/colors.ts` /
  `lib/syllabus-icons.ts` and are picked at runtime via `lib/theme.svelte.ts`
  (`useTheme().isDark`, seeded synchronously to avoid a first-paint flash).

## Conventions

- **Text hierarchy is a solid token, not opacity.** Use `text-apple-text`
  (primary), `text-apple-text-secondary` (labels, headers), `text-apple-text-tertiary`
  (times, counts, placeholders). Solid slate ink keeps contrast on every surface
  and flips correctly in dark mode — an opacity-of-ink would vanish on a dark
  field. Both muted tokens clear WCAG AA (4.5:1) on every surface in both themes,
  locked by `design-tokens.test.ts`.
- **Overlays are slate ink at semantic opacities** (`--color-overlay-*`), used
  for borders, hovers, skeletons, and dividers rather than named greys or pure
  black — the tint keeps them from muddying the light field.
- **Palettes are derived, not hand-picked.** The 10 course tiles ("macaron") are
  10 hues 36° apart from rose (350°), each held at ONE chroma per theme (equal
  vividness) but the lightest lightness that still carries it — so hue is even and
  chroma uniform while lightness varies per hue (yellow light, blue deeper). Each
  tile's `text`/`mutedText`/`accentText` is its own hue-tinted ink clearing 4.5:1
  on its `bg`; text on a tile uses that ink, never the global slate greys. The 6
  eval colours reuse a subset of the ring (`lib/colors.ts`, `lib/syllabus-icons.ts`).
  Locked by `colors.test.ts` / `palette.test.ts`; regenerate from the derivation
  script, don't hand-edit.

## Primitives

One value per role, so the same thing always looks the same. The ★ rules are
enforced by `design-tokens.test.ts` (a design lint over the components); the
rest are conventions.

- **Type scale** ★ — only the `@theme` font tokens, never raw Tailwind sizes
  (`text-lg`/`text-xl`/…): `text-micro` (11) · `text-fine` (10) · `text-caption`
  (13) · `text-sub` (14) · `text-body` (15) · `text-cta` (17) · `text-headline`
  (18, bar titles) · `text-title` (20, dialog headings + eval glyph).
- **Font weight** ★ — three roles only: `font-normal` (body), `font-medium`
  (labels, meta, tabs), `font-semibold` (headings, course names, emphasis). No
  `font-bold`/`-light`/etc.
- **Radius** ★ — a scale by element size: `rounded-full` (pills, chips, badges,
  dots, circular buttons, the semester selector) · `rounded-2xl` (modal/sheet;
  `rounded-t-2xl` for the mobile sheet) · `rounded-xl` (form controls — inputs &
  selects — and the day-period group) · `rounded-lg` (small cards/cells, callouts,
  skeletons). No bare `rounded`, `rounded-sm/md/3xl`, or `rounded-[…]`.
- **Dividers** ★ — one hairline: `border-b border-overlay-subtle`. The
  `overlay-light/medium/strong` steps are *fills* (`bg-…`), never border colours.
  An accent underline (selected tab) is `border-b-2 border-apple-blue`.
- **Accent** — `apple-blue` marks *interaction and state*: links, primary
  buttons, the selected/active item, focus rings, and the one decision-critical
  callout (grading caveats). Never decorative. Text on an accent fill is
  `text-on-accent` (not `text-white`), so it stays legible when the accent
  brightens in dark mode.
- **Focus** — interactive elements show `focus:ring-2 focus:ring-apple-blue/40`
  (inputs also swap to `focus:bg-surface-primary`). One ring opacity everywhere.
- **Elevation** — separation comes from `shadow-card` / `shadow-modal` plus the
  hairline border, not from a darker page. Cards are `surface-primary` on the
  near-white `surface-page`.
- **No magic numbers** ★ — a recurring `[…]` value becomes a token or a named
  `@utility`: the 44px tap floor is `min-h-tap` (`--spacing-tap`); overlay heights
  `max-h-overlay` / `max-h-overlay-sm`; deferred cells `content-auto`. Raw
  size/spacing `[…]` fails the lint. (`scale-[1.02]`, the one hover micro-scale,
  is the lone sanctioned exception.)
- **Stacking** ★ — one named z-ladder in `app.css`, never a raw `z-<n>`:
  `z-sticky` (10) < `z-sticky-head` (20) < `z-sticky-corner` (30) < `z-nav` (50).
  The `sticky-*` rungs order the grid's sticky siblings; `nav` is the chrome.
- **Overlays** — modal, sheet, and the consent gate render in the native **top
  layer** via `<dialog>.showModal()` (`dialog.overlay` in `app.css`): above every
  stacking context with **no z-index**, plus a free focus trap and inert page.
  Esc arrives as the dialog's `cancel` event, which the sheet routes through its
  single history/back close-path (the consent gate swallows it).
- **Motion** — one system, three registers:
  - **State** (hover / focus / press): property-specific `transition-colors` /
    `-transform` at `duration-200`.
  - **Movement** (drag-follow sheets & the day pager): inline
    `transition: <prop> <dur> var(--ease-spring)` — the spring easing is the
    single movement curve; the physical follow-durations are the one sanctioned
    inline exception (like dynamic colours).
  - **Enter / loading**: the `@theme` keyframe tokens `animate-fade-in`,
    `animate-dialog-in`, `animate-spinner`, `animate-pulse`.

  Arbitrary `ease-[cubic-bezier(…)]` is lint-blocked — movement uses
  `ease-spring`, nothing else.

## Guidelines

1. Don't hardcode color / font-size / radius / weight / shadow — use a token or
   scale utility. The design lint in `design-tokens.test.ts` fails the build on
   raw values; run `bun run test`.
2. Error states (`red-50`, `red-600`, …) may use standard Tailwind red — it is
   not a brand color.
3. Add a new token to `@theme` (with its dark override) before using it.
