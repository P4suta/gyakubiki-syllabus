//! HTTP client for the KULAS「シラバス参照」detail flow.
//!
//! Two requests per course, reusing findPage's session mechanism:
//! 1. `POST …/SyllabusSanshoWebApi/initFindAndUpdate` → a per-request `guid`.
//! 2. `GET  …/webmvc/SyllabusSansho?TYPE=0&GUID=<guid>` → the syllabus HTML.
//!
//! The session cookie + `entryContext` come from one GET of the sansho dashboard
//! shell.

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::fetch::token::extract_entry_context;
use crate::fetch::{USER_AGENT, browser_entry_context, build_http_client};

const SANSHO_PAGE_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/simple/1900/3000280/wsl/SyllabusSansho";
// The endpoint method is `initFindAndUpdate` (not `initFind`); the server returns
// HTTP 400 "service method … not found" for the latter.
const INIT_FIND_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Wsl.SyllabusSansho.App.SyllabusSanshoWebApi/initFindAndUpdate";
const WEBMVC_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/webmvc/SyllabusSansho";

/// The identifiers that open one course's detail page.
#[derive(Debug, Clone)]
pub struct CourseRef {
    pub cd: String,
    pub kaiko_nendo: String,
    pub pattern_id: String,
    /// `lastUpdate` from the grid data, persisted so a later run can skip unchanged
    /// courses.
    pub last_update: String,
}

/// Re-export the shared classified error under the crawler's historical name,
/// and pull in the shared HTTP helpers.
pub use crate::net::FetchError as DetailError;
use crate::net::{parse_retry_after, snippet};

/// Abstracts the network so the orchestration loop is testable offline.
pub trait DetailFetcher {
    /// Fetch one course's raw detail HTML.
    fn fetch_html(&self, course: &CourseRef) -> Result<String, DetailError>;
}

/// A live KULAS sansho client holding the session cookie jar + `entryContext`.
pub struct SanshoClient {
    http: reqwest::blocking::Client,
    entry_context: Value,
}

impl SanshoClient {
    /// Establish a session from the sansho dashboard shell (any course's params
    /// render it; we only need the cookie + `entryContext`). `token_override`
    /// (verification only) replaces the extracted token when non-empty.
    pub fn new(seed: &CourseRef, token_override: Option<&str>) -> Result<Self> {
        let http = build_http_client()?;
        let shell_url = format!(
            "{SANSHO_PAGE_URL}?kogiCd={}&kaikoNendo={}&syllabusKomokuPatternId={}",
            seed.cd, seed.kaiko_nendo, seed.pattern_id
        );
        let response = http
            .get(&shell_url)
            .header("User-Agent", USER_AGENT)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "ja")
            .send()
            .context("syllabus detail page GET failed (check TLS and network connectivity)")?;
        let status = response.status();
        let html = response
            .text()
            .context("failed to read syllabus detail page HTML")?;
        anyhow::ensure!(
            status.is_success(),
            "syllabus detail page returned HTTP {}",
            status.as_u16()
        );

        let mut entry_context =
            extract_entry_context(&html).context("failed to extract entryContext")?;
        if let Some(token) = token_override.filter(|t| !t.is_empty()) {
            entry_context["token"] = Value::String(token.to_owned());
        }
        Ok(Self {
            http,
            entry_context,
        })
    }

    /// Replace the session token in captured text with a placeholder, so response
    /// bodies are safe to surface as diagnostics / CI artifacts.
    fn redact(&self, s: &str) -> String {
        match self.entry_context.get("token").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => s.replace(t, "[TOKEN]"),
            _ => s.to_owned(),
        }
    }

    /// `initFind` → `guid` for the given course.
    fn init_find(&self, course: &CourseRef) -> Result<String, DetailError> {
        let body = json!({
            "methodParams": {
                "langId": 0,
                "kogiCd": course.cd,
                "kaikoNendo": course.kaiko_nendo,
                "syllabusKomokuPatternId": course.pattern_id,
                "userKubunCd": "",
                "isOutputSuppress": "",
            },
            "tempData": { "entryContext": browser_entry_context(&self.entry_context) },
        });
        let response = self
            .http
            .post(INIT_FIND_URL)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*")
            .header("Accept-Language", "ja")
            .header("Content-Type", "application/json")
            .header("Origin", "https://kulas.kochi-u.ac.jp")
            .header("Referer", SANSHO_PAGE_URL)
            .body(body.to_string())
            .send()
            .map_err(|e| DetailError::Transient(e.into()))?;
        let status = response.status();
        let retry_after = parse_retry_after(response.headers());
        // Always read the body (redacted) — on a 2xx it carries the guid, on a
        // non-2xx it carries the server's error message we want in diagnostics.
        let text = self.redact(&response.text().unwrap_or_default());
        classify_init_find(status.as_u16(), &text, retry_after)
    }

    /// `webmvc` GET → the syllabus HTML for a `guid`.
    fn get_html(&self, guid: &str) -> Result<String, DetailError> {
        let url = format!("{WEBMVC_URL}?TYPE=0&GUID={guid}");
        let response = self
            .http
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*")
            .header("Accept-Language", "ja")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Referer", SANSHO_PAGE_URL)
            .send()
            .map_err(|e| DetailError::Transient(e.into()))?;
        let status = response.status();
        let retry_after = parse_retry_after(response.headers());
        let html = self.redact(&response.text().unwrap_or_default());
        classify_html(status.as_u16(), &html, retry_after)
    }
}

/// Classify an `initFind` response into its `guid` or a [`DetailError`]. Pure —
/// the network is done; this is the decision the circuit breaker relies on.
/// Non-2xx → [`DetailError::Http`] (so 403/429/5xx trip the breaker); a 2xx with
/// an `errorMsg`, missing/empty `guid`, or non-JSON body → [`DetailError::Fatal`]
/// (a bad course to skip, not a block).
fn classify_init_find(
    status: u16,
    body: &str,
    retry_after: Option<Duration>,
) -> Result<String, DetailError> {
    if !(200..300).contains(&status) {
        return Err(DetailError::Http {
            status,
            retry_after,
            body: snippet(body),
        });
    }
    let value: Value = serde_json::from_str(body).map_err(|e| DetailError::Fatal(e.into()))?;
    if let Some(msg) = value.get("errorMsg").and_then(Value::as_str) {
        return Err(DetailError::Fatal(anyhow::anyhow!(
            "initFind errorMsg: {msg}"
        )));
    }
    match value.get("guid").and_then(Value::as_str) {
        Some(guid) if !guid.is_empty() => Ok(guid.to_owned()),
        _ => Err(DetailError::Fatal(anyhow::anyhow!(
            "initFind returned no guid"
        ))),
    }
}

/// Classify a `webmvc` response into its HTML or a [`DetailError`]. Non-2xx →
/// [`DetailError::Http`]; a 2xx with an empty body → [`DetailError::Fatal`].
fn classify_html(
    status: u16,
    html: &str,
    retry_after: Option<Duration>,
) -> Result<String, DetailError> {
    if !(200..300).contains(&status) {
        return Err(DetailError::Http {
            status,
            retry_after,
            body: snippet(html),
        });
    }
    if html.trim().is_empty() {
        return Err(DetailError::Fatal(anyhow::anyhow!("empty HTML returned")));
    }
    Ok(html.to_owned())
}

impl DetailFetcher for SanshoClient {
    fn fetch_html(&self, course: &CourseRef) -> Result<String, DetailError> {
        let guid = self.init_find(course)?;
        self.get_html(&guid)
    }
}

#[cfg(test)]
mod tests {
    use super::{DetailError, classify_html, classify_init_find, parse_retry_after};
    use std::time::Duration;

    /// Test helper: an `Http` error with no `Retry-After` and no body.
    fn http(status: u16) -> DetailError {
        DetailError::Http {
            status,
            retry_after: None,
            body: String::new(),
        }
    }

    #[test]
    fn is_blocking_flags_server_refusals_only() {
        assert!(http(403).is_blocking());
        assert!(http(429).is_blocking());
        assert!(http(500).is_blocking());
        assert!(http(599).is_blocking());
        // A bad course (404) or a non-HTTP failure is not "the server is blocking us".
        assert!(!http(404).is_blocking());
        assert!(!http(400).is_blocking());
        assert!(!DetailError::Transient(anyhow::anyhow!("x")).is_blocking());
        assert!(!DetailError::Fatal(anyhow::anyhow!("x")).is_blocking());
    }

    #[test]
    fn is_retriable_covers_transient_and_transient_http() {
        assert!(DetailError::Transient(anyhow::anyhow!("x")).is_retriable());
        assert!(http(429).is_retriable());
        assert!(http(503).is_retriable());
        // 403 blocks but must NOT be retried (retrying hammers a refusing server).
        assert!(!http(403).is_retriable());
        assert!(!http(404).is_retriable());
        assert!(!DetailError::Fatal(anyhow::anyhow!("x")).is_retriable());
    }

    #[test]
    fn retry_after_seconds_is_parsed_and_carried() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "120".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(120)));

        // An HTTP-date form is not honored (falls back to backoff).
        let mut date = reqwest::header::HeaderMap::new();
        date.insert(
            reqwest::header::RETRY_AFTER,
            "Wed, 21 Oct 2026 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(parse_retry_after(&date), None);
        assert_eq!(parse_retry_after(&reqwest::header::HeaderMap::new()), None);

        // The parsed value rides on the classified error for the retry loop.
        match classify_init_find(503, "", Some(Duration::from_secs(30))) {
            Err(e) => assert_eq!(e.retry_after(), Some(Duration::from_secs(30))),
            _ => panic!("expected Http error"),
        }
    }

    #[test]
    fn classify_init_find_extracts_guid() {
        assert_eq!(
            classify_init_find(200, r#"{"guid":"G123"}"#, None).unwrap(),
            "G123"
        );
    }

    #[test]
    fn classify_init_find_accepts_real_success_shape() {
        // The actual KULAS initFind success body: errorMsg is JSON null (not an
        // empty string), alongside the guid and extra fields we ignore. Captured
        // from a live SyllabusSansho request.
        let body = r#"{"errorMsg":null,"guid":"d072539e161946fa99491124123f8845","kogiNm":"大学基礎論","sanshoUrl":null,"isShowPDF":true}"#;
        assert_eq!(
            classify_init_find(200, body, None).unwrap(),
            "d072539e161946fa99491124123f8845"
        );
    }

    #[test]
    fn classify_init_find_non_2xx_is_http() {
        assert!(matches!(
            classify_init_find(403, "", None),
            Err(DetailError::Http { status: 403, .. })
        ));
        assert!(matches!(
            classify_init_find(503, "whatever", None),
            Err(DetailError::Http { status: 503, .. })
        ));
    }

    #[test]
    fn classify_init_find_bad_body_is_fatal() {
        // errorMsg, empty guid, missing guid, and non-JSON are all "skip this
        // course", never a block.
        assert!(matches!(
            classify_init_find(200, r#"{"errorMsg":"該当なし"}"#, None),
            Err(DetailError::Fatal(_))
        ));
        assert!(matches!(
            classify_init_find(200, r#"{"guid":""}"#, None),
            Err(DetailError::Fatal(_))
        ));
        assert!(matches!(
            classify_init_find(200, r#"{"other":1}"#, None),
            Err(DetailError::Fatal(_))
        ));
        assert!(matches!(
            classify_init_find(200, "not json", None),
            Err(DetailError::Fatal(_))
        ));
    }

    #[test]
    fn classify_html_variants() {
        assert_eq!(
            classify_html(200, "<html>ok</html>", None).unwrap(),
            "<html>ok</html>"
        );
        assert!(matches!(
            classify_html(200, "   \n\t ", None),
            Err(DetailError::Fatal(_))
        ));
        assert!(matches!(
            classify_html(429, "<html>ok</html>", None),
            Err(DetailError::Http { status: 429, .. })
        ));
    }

    #[test]
    fn http_error_captures_body_for_diagnosis() {
        // The whole point of failure observability: the server's message survives
        // in the error so a failure is legible without a re-run.
        let e = classify_init_find(400, "service method not found", None).unwrap_err();
        assert!(matches!(e, DetailError::Http { status: 400, .. }));
        assert!(e.diagnostic().contains("service method not found"));
    }
}
