# KULAS findPage API spec

Spec for the KULAS syllabus search API that `syllabus-cli fetch` calls.

## Endpoint

```http
POST https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Gkm.Com.KogiKensaku.App.KogiKensakuWebApi/findPage
```

## Session flow

KULAS requires a session cookie plus a token even for the GUEST user. Both come from **a single GET of the search page**.

1. **GET** `https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku`
   - Response: HTML (~120KB)
   - Read `Set-Cookie: CPSMART_PUBLIC_AUTH=...; GCLB=...`
   - In the inline script `cpSmartVueStartup('dash-app-main', '<ver>', true, '<base64>')`, base64-decode the 4th argument into JSON (the `entryContext`, containing the 64-char hex `token`).
2. **POST** findPage (above)
   - Cookies: `CPSMART_PUBLIC_AUTH`, `GCLB`
   - body: JSON (below)
   - Replace the body's `tempData.entryContext` with the extracted object.

### Token extraction (implementation: `crates/cli/src/fetch/token.rs`)

> Note: extract the **entire `entryContext`** (including session identifiers such as `cpClientPid` / `userId`), not just `entryContext.token`, and replace the findPage body's `tempData.entryContext` with it wholesale. findPage validates the token against the rest of the context, so an old `entryContext` with only the token swapped is rejected as "Token invalid (400)".

The regex captures the base64 4th argument of the `dash-app-main` startup call:

```text
cpSmartVueStartup\(\s*'dash-app-main'\s*,\s*'[^']+'\s*,\s*\w+\s*,\s*'([A-Za-z0-9+/=]+)'
```

Other components (`dash-header`, `dash-watcher`, etc.) carry different tokens, so the match is pinned to `dash-app-main`. The decoded JSON must have a non-empty `token` field or extraction fails.

## Request headers

```http
accept: */*
accept-language: ja
content-type: application/json
origin: https://kulas.kochi-u.ac.jp
referer: https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku
user-agent: <realistic Chrome-family UA>
```

## Request body

Large (~60KB of formatted JSON). Stored as a template at `crates/cli/assets/findpage_body.tmpl.json`; substitute `{{.PageNo}}` / `{{.KaikoNendo}}`, then replace `tempData.entryContext` wholesale at runtime:

| Placeholder | Location | Content |
|---|---|---|
| `{{.PageNo}}` | `methodParams.kensakuJoken.pageNo` and `methodParams.kensakuJoken.values.pageNo` | 1, 2, 3, ... |
| `{{.KaikoNendo}}` | `methodParams.kensakuJoken.values.kaikoNendo.values[0]` and `tempData.entryContext.shoriNendo` | e.g. `"2026"`. Academic year |
| `{{.Token}}` | `tempData.entryContext.token` | token extracted from the search-page HTML |

The remaining hundreds of fields are search-condition schema declarations (empty values); KULAS requires the full body, so they are kept as-is.

## Response

```json
{
  "pageNo": 1,
  "maxPageNo": 8,
  "total": 3850,
  "pageSize": 500,
  "selectKogiDtoList": [ ... 500 RawCourse entries ... ]
}
```

Request each `pageNo` from 1 to `maxPageNo`. Response field definitions: see `docs/kulas-api-fields.md`.

## Saved file names

| pageNo | File name |
|---|---|
| 1 | `raw/講義データ.json` |
| 2+ | `raw/講義データ-{pageNo:02d}.json` |

## Known constraints

- No login required (GUEST user).
- Whether the university system blocks GitHub Actions runner IPs is untested. Always confirm a 200 via a `dry-run` on the first `workflow_dispatch`.
- `kaikoNendo` needs a yearly update. It defaults to "current year (from April) or previous year (Jan–Mar)" and can be overridden with `--year`.

## Politeness / responsible access

We only access syllabus data the university publishes openly (no login, GUEST user),
and treat access as a courtesy. The crawl is engineered to stay well under any
reasonable load and to be identifiable rather than anonymous:

- **Single source, no evasion.** All traffic comes from one GitHub Actions runner.
  We do not rotate IPs or otherwise hide — a single, predictable, contactable source
  is the point.
- **Identified User-Agent.** `USER_AGENT` (`crates/cli/src/fetch/client.rs`) keeps a
  browser-compatible prefix but appends `gyakubiki-syllabus/1.0 (+<repo URL>)`, so an
  operator who notices the traffic can reach the project and contact us.
- **Detail crawl is gentle** (`fetch-details`): strictly sequential (no parallelism),
  3-5 s jittered sleep between courses, exponential backoff on retries (honoring the
  server's `Retry-After`), and a circuit breaker that aborts after a few consecutive
  server refusals so a block never turns into hammering.
- **Incremental + capped + off-peak.** Only courses whose grid `lastUpdate` changed
  are refetched; a per-run `--limit` spreads any large backlog over many days; runs
  are scheduled for JST early morning.
- **Grid fetch is seasonal** (`fetch-syllabus`): daily in Mar/Apr/Sep/Oct, weekly
  otherwise — frequent when freshness matters, quiet the rest of the year.

### robots.txt

`https://kulas.kochi-u.ac.jp/robots.txt` returns **404 (Not Found)** as of 2026-07-07
(checked manually; note robots.txt is per-host, so the crawl target `kulas.kochi-u.ac.jp`
is what matters, not `www.kochi-u.ac.jp`). Per RFC 9309 an absent robots.txt means no
crawl restrictions are declared, so there is no `Disallow` to honor. Re-check if the
host starts serving one.

## TLS chain

KULAS (IIS 10) sends **only the leaf certificate** during the TLS handshake and does not serve the intermediate CA. Chrome / Firefox complete the chain via AIA fetching, but the HTTP client does not, so it would fail with `certificate signed by unknown authority`.

To handle this, the intermediate CA (`NII Open Domain CA - G7 RSA`) is bundled at `crates/cli/assets/kulas_ca.pem` and added via reqwest's `add_root_certificate` in `crates/cli/src/fetch/client.rs`. The root CA (`Security Communication RootCA2`) is in the system bundle, so it needs no addition.

### Intermediate CA expiry and renewal

Current bundle expiry: **2029-05-29** (check: `openssl x509 -in crates/cli/assets/kulas_ca.pem -noout -dates`).

When it expires, or KULAS's chain changes and fetch starts failing with `unknown authority` again, re-fetch:

```sh
# 1. Find the AIA URL of the current leaf certificate
openssl s_client -connect kulas.kochi-u.ac.jp:443 \
  -servername kulas.kochi-u.ac.jp </dev/null 2>/dev/null \
  | openssl x509 -noout -text | grep -A1 'Authority Information Access'

# 2. Fetch the DER from the CA Issuers URL
curl -sS -o /tmp/intermediate.cer "<URL from above>"

# 3. Convert to PEM and overwrite
openssl x509 -inform DER -in /tmp/intermediate.cer -outform PEM \
  -out crates/cli/assets/kulas_ca.pem
```
