//! KULAS HTTP client for findPage.
//!
//! One GET to the search page establishes the session cookie and yields the
//! **full `entryContext`** (token + session identifiers like `cpClientPid`,
//! `userId`, …). findPage validates the token against the rest of that context,
//! so each request must carry the *current* session's context, not a stale one
//! with only the token swapped. The embedded KULAS intermediate CA completes the
//! TLS chain the server omits.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::PageFetcher;
use super::token::extract_entry_context;

const SEARCH_PAGE_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku";
const FIND_PAGE_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Gkm.Com.KogiKensaku.App.KogiKensakuWebApi/findPage";
// Browser-compatible (keeps the Mozilla/Chrome prefix so UA-gating servers pass
// us) but honestly identified: the trailing product token + repo URL let an
// operator who notices the traffic reach the project (README disclaimer, issues)
// and contact us instead of treating it as an unknown crawler.
pub(crate) const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36 gyakubiki-syllabus/1.0 (+https://github.com/p4suta/gyakubiki-syllabus)";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// The findPage request body template (`{{.PageNo}}`/`{{.KaikoNendo}}` for the
/// search params; `tempData.entryContext` is replaced wholesale at runtime).
const BODY_TEMPLATE: &str = include_str!("../../assets/findpage_body.tmpl.json");
/// The KULAS intermediate CA (NII Open Domain CA - G7 RSA), embedded so the
/// chain completes even though the server does not send it.
const KULAS_CA_PEM: &[u8] = include_bytes!("../../assets/kulas_ca.pem");

/// Display-only entryContext keys the browser strips when building the findPage
/// body.
const ENTRY_CONTEXT_DROP: [&str; 10] = [
    "ResourceId",
    "contextName",
    "el",
    "isCustom",
    "isKikannai",
    "isNarikawariSeigen",
    "kinoNm",
    "kinoType",
    "scriptsVersion",
    "systemNm",
];

/// A live KULAS client: session cookie jar, current `entryContext`, academic year.
pub struct Client {
    http: reqwest::blocking::Client,
    entry_context: Value,
    kaiko_nendo: String,
}

impl Client {
    /// Build a client and establish a KULAS session (cookie + entryContext).
    ///
    /// `token_override` (from `--token` / `KULAS_API_TOKEN`) replaces the
    /// HTML-extracted token when non-empty — for verification only.
    pub fn new(kaiko_nendo: &str, token_override: Option<&str>) -> Result<Self> {
        let http = build_http_client()?;

        let mut client = Self {
            http,
            entry_context: Value::Null,
            kaiko_nendo: kaiko_nendo.to_owned(),
        };
        client.establish_session()?;
        if let Some(token) = token_override.filter(|t| !t.is_empty()) {
            client.entry_context["token"] = Value::String(token.to_owned());
        }
        Ok(client)
    }

    /// GET the search page: stores the session cookie and the full entryContext.
    fn establish_session(&mut self) -> Result<()> {
        let response = self
            .http
            .get(SEARCH_PAGE_URL)
            .header("User-Agent", USER_AGENT)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "ja")
            .send()
            .context("search page GET failed (check the TLS chain and network connectivity)")?;

        let status = response.status();
        let html = response.text().context("failed to read search page HTML")?;
        if !status.is_success() {
            bail!("search page returned HTTP {}", status.as_u16());
        }

        self.entry_context =
            extract_entry_context(&html).context("failed to extract entryContext")?;
        Ok(())
    }
}

impl PageFetcher for Client {
    /// Fetch one findPage page as raw JSON bytes (the exact response body).
    fn fetch_page(&self, page_no: i32) -> Result<Vec<u8>> {
        let body = build_body(
            BODY_TEMPLATE,
            page_no,
            &self.kaiko_nendo,
            &self.entry_context,
        )?;
        let response = self
            .http
            .post(FIND_PAGE_URL)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*")
            .header("Accept-Language", "ja")
            .header("Content-Type", "application/json")
            .header("Origin", "https://kulas.kochi-u.ac.jp")
            .header("Referer", SEARCH_PAGE_URL)
            .body(body)
            .send()
            .with_context(|| format!("findPage HTTP call failed (page {page_no})"))?;

        let status = response.status();
        let bytes = response
            .bytes()
            .with_context(|| format!("failed to read findPage response (page {page_no})"))?
            .to_vec();

        if !status.is_success() {
            let preview: String = String::from_utf8_lossy(&bytes).chars().take(500).collect();
            bail!(
                "findPage returned HTTP {} (page {page_no}): {preview}\n  \
                 note: the token/entryContext may be stale, or the body format may not match the API",
                status.as_u16()
            );
        }
        Ok(bytes)
    }
}

/// Build the blocking HTTP client KULAS needs: a cookie jar (session) plus the
/// embedded intermediate CA so native-tls completes the chain the server omits.
/// Shared by the findPage [`Client`] and the sansho detail client.
pub(crate) fn build_http_client() -> Result<reqwest::blocking::Client> {
    let ca =
        reqwest::Certificate::from_pem(KULAS_CA_PEM).context("failed to load embedded KULAS CA")?;
    reqwest::blocking::Client::builder()
        .cookie_store(true)
        .add_root_certificate(ca)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("failed to build HTTP client")
}

/// Build the findPage body: substitute the search params, then inject the fresh
/// session `entryContext` (replacing the template's placeholder context).
fn build_body(
    template: &str,
    page_no: i32,
    kaiko_nendo: &str,
    entry_context: &Value,
) -> Result<String> {
    // The template's `{{.Token}}` lives inside the entryContext we are about to
    // replace, so a blank substitution just keeps the JSON valid before parsing.
    let rendered = template
        .replace("{{.PageNo}}", &page_no.to_string())
        .replace("{{.KaikoNendo}}", kaiko_nendo)
        .replace("{{.Token}}", "");
    let mut body: Value = serde_json::from_str(&rendered)
        .context("failed to parse findPage body template as JSON")?;
    body["tempData"]["entryContext"] = browser_entry_context(entry_context);
    serde_json::to_string(&body).context("failed to serialize findPage body")
}

/// Reshape the page's `entryContext` into what the browser sends to findPage:
/// keep every session field, drop the display-only keys, expose `ResourceId` as
/// `resourceId`.
///
/// The denylist (`ENTRY_CONTEXT_DROP`) is load-bearing: KULAS validates the
/// *whole* context, so every other field must survive. Do **not** rewrite as an
/// allowlist — a silently dropped key breaks the request.
pub(crate) fn browser_entry_context(entry_context: &Value) -> Value {
    let Some(object) = entry_context.as_object() else {
        return entry_context.clone();
    };
    let resource_id = object
        .get("ResourceId")
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()));
    let mut out: serde_json::Map<String, Value> = object
        .iter()
        .filter(|(key, _)| !ENTRY_CONTEXT_DROP.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    out.insert("resourceId".to_owned(), resource_id);
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::{BODY_TEMPLATE, browser_entry_context, build_body};
    use serde_json::{Value, json};

    fn sample_entry_context() -> Value {
        json!({
            "token": "FRESH_TOKEN_64",
            "cpClientPid": "pid-1",
            "userId": "u1",
            "ResourceId": "",
            "contextName": "",
            "el": "dash-app-main",
            "kinoNm": "シラバス検索",
            "scriptsVersion": "2025-03-26",
            "isProduction": false
        })
    }

    #[test]
    fn browser_entry_context_drops_display_keys_and_renames_resource_id() {
        let out = browser_entry_context(&sample_entry_context());
        let obj = out.as_object().unwrap();
        // display-only keys gone
        for k in [
            "ResourceId",
            "contextName",
            "el",
            "kinoNm",
            "scriptsVersion",
        ] {
            assert!(!obj.contains_key(k), "{k} should be dropped");
        }
        // lowercase resourceId present; session fields preserved
        assert!(obj.contains_key("resourceId"));
        assert_eq!(obj["token"], json!("FRESH_TOKEN_64"));
        assert_eq!(obj["cpClientPid"], json!("pid-1"));
    }

    #[test]
    fn build_body_injects_fresh_entry_context_and_params() {
        let body = build_body(BODY_TEMPLATE, 3, "2026", &sample_entry_context()).unwrap();
        let parsed: Value = serde_json::from_str(&body).unwrap();
        // entryContext came from our fresh session, not the stale template
        assert_eq!(
            parsed["tempData"]["entryContext"]["token"],
            json!("FRESH_TOKEN_64")
        );
        assert_eq!(
            parsed["tempData"]["entryContext"]["cpClientPid"],
            json!("pid-1")
        );
        // display key dropped even though present in the source context
        assert!(parsed["tempData"]["entryContext"].get("kinoNm").is_none());
        // methodParams (search config) is preserved from the template
        assert!(parsed["methodParams"].is_object());
    }
}
