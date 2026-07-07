//! HTTP client for the KULAS「シラバス参照」detail flow.
//!
//! Two requests per course, reusing findPage's session mechanism:
//! 1. `POST …/SyllabusSanshoWebApi/initFind` → a per-request `guid`.
//! 2. `GET  …/webmvc/SyllabusSansho?TYPE=0&GUID=<guid>` → the syllabus HTML.
//!
//! The session cookie + `entryContext` come from one GET of the sansho dashboard
//! shell.

use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::fetch::token::extract_entry_context;
use crate::fetch::{browser_entry_context, build_http_client, USER_AGENT};

const SANSHO_PAGE_URL: &str =
    "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/simple/1900/3000280/wsl/SyllabusSansho";
const INIT_FIND_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Wsl.SyllabusSansho.App.SyllabusSanshoWebApi/initFind";
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

/// A fetch failure, classified so the orchestrator can tell "this course is bad,
/// skip it" from "the server is refusing us, stop before we get banned".
#[derive(Debug)]
pub enum DetailError {
    /// Non-2xx HTTP — carries the status so 403/429/5xx trip the circuit breaker,
    /// plus any `Retry-After` the server asked us to wait (honored on retry).
    Http {
        status: u16,
        retry_after: Option<Duration>,
    },
    /// Network/transport error — worth a bounded retry.
    Transient(anyhow::Error),
    /// The response was reached but is unusable for this course (bad guid, empty
    /// body). Skip the course; not a sign of blocking.
    Fatal(anyhow::Error),
}

impl std::fmt::Display for DetailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetailError::Http { status, .. } => write!(f, "HTTP {status}"),
            DetailError::Transient(e) => write!(f, "transient: {e}"),
            DetailError::Fatal(e) => write!(f, "fatal: {e}"),
        }
    }
}

impl DetailError {
    /// Whether this looks like the server refusing/limiting us (vs. a bad course).
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            DetailError::Http {
                status: 403 | 429 | 500..=599,
                ..
            }
        )
    }
    /// Whether a bounded retry is worthwhile.
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            DetailError::Transient(_)
                | DetailError::Http {
                    status: 429 | 500..=599,
                    ..
                }
        )
    }
    /// The server-requested wait (`Retry-After`), if any, to honor before retrying.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            DetailError::Http { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

/// Parse a `Retry-After` header. Only the delta-seconds form is honored (the
/// common case for 429/503); the HTTP-date form falls back to `None` and lets the
/// exponential backoff take over.
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let secs: u64 = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(secs))
}

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
        // Only a 2xx carries a body worth parsing; read it, then classify.
        let text = if status.is_success() {
            response.text().map_err(|e| DetailError::Fatal(e.into()))?
        } else {
            String::new()
        };
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
        let html = if status.is_success() {
            response.text().map_err(|e| DetailError::Fatal(e.into()))?
        } else {
            String::new()
        };
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
    use super::{classify_html, classify_init_find, parse_retry_after, DetailError};
    use std::time::Duration;

    /// Test helper: an `Http` error with no `Retry-After`.
    fn http(status: u16) -> DetailError {
        DetailError::Http {
            status,
            retry_after: None,
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
}
