# Release readiness report

## Decision

**NO-GO for production deployment.** The implementation and locally runnable
release gates pass, but this working tree is not committed and the required
Linux CI, protected-environment, deployment, and production-smoke evidence does
not exist yet. Re-evaluate only after every item in
[External release gates](#external-release-gates) passes.

## Candidate identity

- Version: `0.1.0` (workspace package version)
- Source commit recorded by the dataset:
  `4146d3fa0cfdef4f128e3269a7ffebf82ecee903`
- Working tree: uncommitted release-hardening changes
- Dataset ID:
  `3879a9790e28eedc3094b02eb85f6d25a84d762d920c327bad1702372249053b`
- Dataset generated: `2026-07-26T09:54:04+09:00`
- Academic year: `2026`

## Dataset evidence

- Courses: 3,928
- Details: 3,928 (100% coverage)
- Courses with scheduled offerings: 2,155
- Courses with intensive/TBA offerings: 1,775
- Unscheduled-only courses: 1,773
- Period-7 courses: 19
- Published range: Monday–Friday, periods 1–7
- Published generations: current and previous only
- Staging or legacy stable-name assets: none

Scheduled and unscheduled classifications may overlap when a course has both
offering types. Their distinct union is all 3,928 courses.

## Asset identity

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `data.2e688fe79087f3f5.json` | 1,193,421 | `2e688fe79087f3f527f8834872c68ba578625f203d533e20ccf27b5500d7b926` |
| `search.8d5c711b08f5cb20.idx.br` | 2,932,890 | `8d5c711b08f5cb20d74ee938382555b19090db3fe67b3b9fd62f55d5a9ad513f` |
| Decoded search index | 20,814,675 | `bf2eff3521da806722730617dc6d1dd34a7f791bf132766b507292fc8e75f7b1` |
| `details.1d2002f6d5d008d5.json` | 609,123 | `1d2002f6d5d008d5ec0e7dfc3bc6251ba9ff2ce5a5cfd6a69fd5226ecb191f54` |

Every entry in the 3,928-item detail index has its own verified size and
SHA-256 in the generated index.

## Local verification

| Gate | Result |
| --- | --- |
| Rust fmt and clippy `-D warnings` | Pass |
| Rust workspace/coverage tests | 330 passed; line coverage 87.51% |
| WASM Node boundary | 12 passed |
| Svelte and TypeScript check | 0 errors, 0 warnings |
| Biome | Pass |
| Web unit/component tests | 244 passed |
| Web coverage | Statements 95.73%, branches 89.44%, functions 92.85%, lines 97.09% |
| Browser E2E | 52 passed across Chromium, Firefox, and WebKit |
| Windows visual tests | 10 intentionally skipped; Linux CI owns the baselines |
| Dataset atomic failure injection | Pass |
| Generated-file drift | Pass |
| Actionlint, typos, markdownlint | Pass |
| Cargo audit/deny and Bun audit | Pass; no known vulnerability |

## Performance evidence

| Metric | Result | Budget |
| --- | ---: | ---: |
| Initial interactive payload, gzip | 176,684 B | 307,200 B |
| App JavaScript, gzip | 49,508 B | 56,320 B |
| Core WASM, gzip | 117,459 B | 117,760 B |
| Search index transport | 2,933,669 B | 3,145,728 B |
| Search query p95 | 25.09 ms | 50 ms |
| Worker heap | 58,982,400 B | 67,108,864 B |

The stable manifest itself is included in the initial-payload measurement.
The content-addressed detail index is lazy and is verified before use.

## External release gates

- [ ] Commit these changes, open a PR, and record the exact
  [required-check URL](https://github.com/P4suta/gyakubiki-syllabus/actions).
- [ ] Pass Linux visual regression and the required three-run mobile Lighthouse
  median: Performance 90+, Accessibility 100, Best Practices 100, SEO 100.
  Windows Chrome/Edge launchers returned `NO_FCP`, so no local score is claimed.
- [ ] Add the single required CI gate to the `main` ruleset, configure the
  code-release approval, and install the data-update GitHub App credentials.
  The Pages environment is already restricted to `main`; the App secrets are
  currently absent, and the fetch/release workflows fail closed as designed.
- [ ] Deploy the attested Pages artifact and pass production smoke at
  <https://p4suta.github.io/gyakubiki-syllabus/>.

Tracking and acceptance criteria are recorded in the
[Linear project](https://linear.app/yasunobu/project/gyakubiki-syllabus-next-shippable-milestone-d312af1defb8).
Known release-blocking external gates: **4**. The final report may state zero
known issues only after all four are closed.
