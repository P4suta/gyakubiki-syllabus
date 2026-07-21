# Contributing

Contributions to 逆引きシラバス (gyakubiki-syllabus) are welcome. It is a Rust
core (`crates/core`) compiled to WebAssembly (`crates/wasm`) behind a Svelte
frontend (`web/`), plus a CLI (`crates/cli`) that fetches and converts KULAS
syllabus data. The app is deployed to GitHub Pages.

## Setup

The toolchain is pinned via [mise](https://mise.jdx.dev/) (`.mise.toml`); tasks
run through [just](https://github.com/casey/just). The Rust toolchain itself is
owned by `rust-toolchain.toml`.

```sh
just install-tools   # mise install — provisions the pinned tools
just install-hooks   # lefthook install — enables the git hooks
```

Declare tools in `.mise.toml` and install via `mise install`; do not add them ad
hoc.

## Dev loop

```sh
just dev      # web dev server (builds WASM first)
just lint     # fmt-check + clippy -D warnings + typos + actionlint + markdownlint
just test     # Rust (nextest) + WASM boundary tests + web (svelte-check + vitest)
just check    # CI-equivalent: lint + test
```

## Generated artifacts

Do not hand-edit generated files — regenerate them:

- `docs/syllabus-fields.md` and the frontend TS are derived from `FIELD_SPEC`
  (`crates/cli/src/fields.rs`); regenerate with `just gen-field-docs`.
- `web/public/data.json` + `web/public/details/` are built from `raw/` and
  `raw-details/` with `just convert`.
- `web/src/wasm/` is the `wasm-pack` output; regenerate with `just wasm-build`.

## Commit / PR rules

- [Conventional Commits](https://www.conventionalcommits.org/) (`feat:` / `fix:` /
  `perf:` / `docs:` / `refactor:` / `test:` / `chore:` / `ci:` / `build:` /
  `deps:` / `style:` / `revert:`). Enforced locally by
  [committed](https://github.com/crate-ci/committed) (`committed.toml`) in the
  lefthook `commit-msg` hook.
- **Squash-merge only.**
- Releases are cut by
  [release-please](https://github.com/googleapis/release-please): it opens a
  release PR that bumps the version + CHANGELOG from conventional commits, then
  tags on merge (`release-please-config.json` + `.release-please-manifest.json`).

## Before pushing

- `just lint` and `just test` green (the `lefthook` pre-push hook runs both).
- **Do not hand-edit generated artifacts** (see above); regenerate them.
- Do not bypass hooks with `--no-verify`. If a hook fails, fix the cause.

## License

Contributions are accepted under the project's
[GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0-or-later).
