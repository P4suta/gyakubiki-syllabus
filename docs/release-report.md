# Release readiness report template

Copy this file for each production candidate. Replace every `<required>` value;
do not delete a row or claim `GO` while a required value is unknown.

## Decision

- Application: `<GO | NO-GO>`
- Data automation: `<READY | WAITING FOR CREDENTIALS | NO-GO>`
- Release automation: `<READY | WAITING FOR CREDENTIALS | NO-GO>`
- Decision recorded at (UTC): `<required>`
- Recorded by: `<required>`

A deployable application may be `GO` while either automation line remains
`WAITING FOR CREDENTIALS`. Keep the automation issues and parent project open
until credentials are installed and a normal automatic run succeeds.

## Candidate identity

| Field | Evidence |
| --- | --- |
| Version | `<Cargo.toml and web/package.json value>` |
| Source commit | `<full main SHA>` |
| Dataset source commit | `<manifest sourceCommit>` |
| Dataset ID | `<manifest datasetId>` |
| Academic year | `<manifest year>` |
| Generated at | `<manifest generatedAt>` |
| Working tree | `<clean>` |

## Dataset and asset identity

| Asset | Bytes | SHA-256 | Verification |
| --- | ---: | --- | --- |
| Data | `<required>` | `<required>` | `<pass/fail>` |
| Search index (transport) | `<required>` | `<required>` | `<pass/fail>` |
| Search index (decoded) | `<required>` | `<required>` | `<pass/fail>` |
| Detail index | `<required>` | `<required>` | `<pass/fail>` |
| Every detail asset | `<count>` | `manifest-bound` | `<pass/fail>` |

- Courses: `<required>`
- Details and coverage: `<required>`
- Courses with scheduled offerings: `<required>`
- Courses with intensive/TBA offerings: `<required>`
- Distinct scheduled/unscheduled union: `<required; must equal courses>`
- Published range: `<required>`

Scheduled and unscheduled counts may overlap when one course has both offering
types. Record both counts and the distinct union.

## Reproducible verification

Run from a clean checkout of the candidate SHA:

```text
just check
just release-check
cargo mutants --timeout 60 -j 2
cd web && bun run mutation
cd ../crates/cli/fuzz && cargo +nightly fuzz run fuzz_parse_jikanwari -- -max_total_time=120
cargo +nightly fuzz run fuzz_parse_sansho_html -- -max_total_time=120
```

| Gate | Result | Evidence URL or artifact |
| --- | --- | --- |
| `just check` | `<pass/fail>` | `<required>` |
| `just release-check` | `<pass/fail>` | `<required>` |
| Rust full mutation | `<pass/fail>` | `<required>` |
| Stryker full mutation | `<pass/fail>` | `<required>` |
| Timetable fuzz | `<pass/fail>` | `<required>` |
| Detail HTML fuzz | `<pass/fail>` | `<required>` |
| Required gate | `<pass/fail>` | `<required>` |
| Linux visual regression | `<pass/fail>` | `<required>` |
| Lighthouse 3-run median | `<P/A/BP/SEO>` | `<required>` |
| Security, audit, CodeQL | `<pass/fail>` | `<required>` |

## Deployment and production evidence

| Field | Evidence |
| --- | --- |
| Merged `main` commit | `<full SHA>` |
| CI run | `<URL>` |
| Pages deploy job | `<URL and success>` |
| Deployment commit | `<full SHA; must equal merged main>` |
| Deployment URL | `<URL>` |
| Immediate production smoke | `<URL and success>` |
| Full detail-asset verification | `<URL and count>` |
| Latest scheduled smoke | `<URL and success>` |

Confirm directly:

- `manifest.json` returned HTTP 200.
- Manifest-selected root assets matched declared byte sizes and SHA-256 values.
- Every detail asset matched its declared byte size and SHA-256 value.
- UI search, intensive/TBA, detail, plan, data status, and cached offline reload
  passed against `PRODUCTION_URL`.

## Performance evidence

| Metric | Result | Budget |
| --- | ---: | ---: |
| Lighthouse performance median | `<required>` | `>= 90` |
| Lighthouse accessibility median | `<required>` | `100` |
| Lighthouse best practices median | `<required>` | `100` |
| Lighthouse SEO median | `<required>` | `100` |
| Initial interactive payload, gzip | `<required>` | `<= 307,200 B` |
| App JavaScript, gzip | `<required>` | `<= 56,320 B` |
| Core WASM, gzip | `<required>` | `<= 117,760 B` |
| Search index transport | `<required>` | `<= 3,145,728 B` |
| Search query p95 | `<required>` | `<= 50 ms` |
| Worker heap | `<required>` | `<= 67,108,864 B` |

## Known issues and follow-up

- Application issues: `<none | list with owner and issue URL>`
- Credential/automation blockers: `<none | list>`
- GitHub Pages CSP header limitation acknowledged: `<yes/no>`
- Linear evidence document: `<URL>`
- Issues closed with this evidence: `<IDs>`
- Issues intentionally left In Progress: `<IDs and reason>`
