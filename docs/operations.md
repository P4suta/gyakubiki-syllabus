# Production operations

## Deployment and smoke monitoring

`CI` builds the GitHub Pages artifact once, validates it, attests the packed
bytes, uploads the Pages artifact, and deploys that same artifact. The deploy job
uses `always()` only to evaluate upstream results; it deploys exclusively when
both `Required gate` and `Production artifact` succeeded on a push to `main`.
It never rebuilds the site.

`Production smoke` runs after a successful `CI` run on `main`, every day at
07:00 JST, and on manual dispatch. Set the repository variable `PRODUCTION_URL`
to the canonical deployment URL. If it is unset, the workflow uses
`https://p4suta.github.io/gyakubiki-syllabus/`.

The post-deploy run validates every manifest-addressed detail asset. Daily runs
validate the manifest, root assets, dataset counts and scheduled/unscheduled
union, a representative detail asset, search, intensive and TBA courses, plan
registration, the data-status dialog, and an offline reload from already cached
assets.

## Data automation

Data workflows are intentionally inert until the repository variable
`DATA_AUTOMATION_ENABLED` is exactly `true`. While disabled, scheduled and manual
runs report why they are disabled and exit successfully. Enabling the variable
without every required secret fails before a token is minted, a branch is
created, KULAS is contacted, or repository content is changed.

Create and install a dedicated GitHub App with access only to this repository:

- Repository contents: read and write
- Pull requests: read and write
- Metadata: read (implicit)

Configure these repository-level Actions secrets:

- `DATA_BOT_CLIENT_ID`: GitHub App client ID
- `DATA_BOT_PRIVATE_KEY`: complete PEM private key
- `KULAS_API_TOKEN`: token used only by the KULAS fetch steps

After the secrets are present, set `DATA_AUTOMATION_ENABLED=true`, manually run
both data workflows, and confirm that they create or update the dedicated data
PRs. Do not mark the automation rollout complete until a normal scheduled run
also succeeds.

## Release automation

Release Please is intentionally inert until the repository variable
`RELEASE_AUTOMATION_ENABLED` is exactly `true`. Enabling it without both secrets
fails before a token is minted or a release PR is changed.

Create and install a separate GitHub App with access only to this repository:

- Repository contents: read and write
- Pull requests: read and write
- Metadata: read (implicit)

Configure these repository-level Actions secrets so the approval-free policy job can fail closed before the release environment is entered:

- `RELEASE_PLEASE_CLIENT_ID`: GitHub App client ID
- `RELEASE_PLEASE_PRIVATE_KEY`: complete PEM private key

Keep the existing `release-please` approval environment. After the secrets are
present, set `RELEASE_AUTOMATION_ENABLED=true`, manually run the workflow, and
confirm that it creates or updates the expected release PR.

## Content Security Policy on GitHub Pages

The production build injects a Content Security Policy meta element before any
page content. Scripts, workers, WASM, network data, fonts, and images are limited
to the deployed origin; the structured-data script is allowed by its generated
SHA-256 hash. Inline styles remain allowed because the app inlines its stylesheet
and uses data-derived card colors.

GitHub Pages does not provide a repository setting for arbitrary HTTP response
headers. A meta policy cannot enforce header-only directives such as
`frame-ancestors`, cannot protect content that precedes the meta element, and
cannot emit CSP reports. The build therefore places the policy first and also
uses `object-src 'none'` and `frame-src 'none'`, but clickjacking protection would
require a custom host or edge proxy that can send an HTTP CSP header. This is a
known hosting limitation, not an untracked application defect.

## Release evidence

Copy [the release report template](release-report.md), fill every placeholder,
and attach the completed report to the tracked Linear project. Evidence must
identify the source commit, dataset and assets, CI run, deployment, production
smoke, measured budgets, and any known issue. A green application deployment
may be recorded separately from the still-disabled data/release automation.
