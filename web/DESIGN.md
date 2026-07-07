# Design Tokens

Apple-style light theme: a single accent (Apple Blue `#0071e3`), two surfaces
(`#ffffff` / `#f5f5f7`), Noto Sans JP body text. Every design value lives as a
Tailwind CSS 4 `@theme` token in `src/app.css` — that file is the source of
truth for all hex colors, font sizes, shadows, easings, and animations.
Components use semantic utility classes; they never hardcode `#`, px, or shadow
values.

`design-tokens.test.ts` enforces this against `app.css`.

## Conventions

- **Text hierarchy is opacity, not color.** Layer `text-apple-text` with an
  opacity modifier: `/70` body, `/60` medium (day headers, icons), `/50`
  secondary (professor names), `/40` tertiary (labels, counters), `/30`
  placeholder, `/20` inactive.
- **Overlays are black at semantic opacities** (`--color-overlay-*`), used for
  borders, hovers, and backdrops rather than named greys.

## Guidelines

1. Don't hardcode color / font-size / shadow — use a token utility. Grep `[#`
   to catch stray hardcoded values.
2. Error states (`red-50`, `red-600`, …) may use standard Tailwind red — it is
   not a brand color.
3. Standard Tailwind utilities (`shadow-sm`, `duration-200`, `rounded-xl`,
   `text-xl`) compose freely with tokens.
4. Add a new token to `@theme` before using it.
