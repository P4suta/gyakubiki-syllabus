<!--
Thanks for contributing! A few reminders (see CONTRIBUTING.md for the full loop):
- Commits follow Conventional Commits (feat: / fix: / perf: / docs: / …); PRs are squash-merged.
- Do not hand-edit generated artifacts (the FIELD_SPEC output: docs/syllabus-fields.md and its TS) — regenerate them.
-->

## What & why

Describe the change and the motivation. Link any related issue (`Closes #123`).

## Linear

Closes DEV-___
<!-- Links the Linear issue on merge; requires the Linear GitHub integration. -->

## Checklist

- [ ] `just lint` passes (fmt, clippy `-D warnings`, typos, actionlint, markdownlint)
- [ ] `just test` passes (Rust, WASM boundary, web)
- [ ] New or changed product code is covered by a test that pins its behavior
      (the `mutants` CI gate mutation-tests the diff)
- [ ] Generated artifacts were regenerated with `just gen-field-docs`, not hand-edited
- [ ] Docs / CHANGELOG updated if user-facing
