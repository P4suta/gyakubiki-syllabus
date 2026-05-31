//! KULAS HTTP client — port of Go's `internal/fetch/client.go`.
//!
//! One GET to the search page establishes the session cookie and yields the
//! token (extracted from the inline `cpSmartVueStartup` script); subsequent
//! POSTs to findPage reuse both. native-tls (OpenSSL) plus the embedded KULAS
//! intermediate CA completes the TLS chain exactly as the Go pipeline did.

use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::token::extract_token;
use super::PageFetcher;

const SEARCH_PAGE_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku";
const FIND_PAGE_URL: &str = "https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Gkm.Com.KogiKensaku.App.KogiKensakuWebApi/findPage";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// The findPage request body template (3 logicless placeholders).
const BODY_TEMPLATE: &str = include_str!("../../assets/findpage_body.tmpl.json");
/// The KULAS intermediate CA (NII Open Domain CA - G7 RSA), embedded so the
/// chain completes even though the server does not send it.
const KULAS_CA_PEM: &[u8] = include_bytes!("../../assets/kulas_ca.pem");

/// A live KULAS client holding the session cookie jar, token and academic year.
pub struct Client {
    http: reqwest::blocking::Client,
    token: String,
    kaiko_nendo: String,
}

impl Client {
    /// Build a client and establish a KULAS session (cookie + token).
    ///
    /// `token_override` (from `--token` / `KULAS_API_TOKEN`) replaces the
    /// HTML-extracted token when non-empty — for verification only.
    pub fn new(kaiko_nendo: &str, token_override: Option<&str>) -> Result<Self> {
        let ca = reqwest::Certificate::from_pem(KULAS_CA_PEM)
            .context("埋め込み KULAS CA の読み込みに失敗")?;
        let http = reqwest::blocking::Client::builder()
            .cookie_store(true)
            .add_root_certificate(ca)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("HTTP クライアントの構築に失敗")?;

        let mut client = Self {
            http,
            token: String::new(),
            kaiko_nendo: kaiko_nendo.to_owned(),
        };
        client.establish_session()?;
        if let Some(token) = token_override.filter(|t| !t.is_empty()) {
            client.token = token.to_owned();
        }
        Ok(client)
    }

    /// GET the search page: stores the session cookie and extracts the token.
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
            .context("検索ページ GET に失敗 (TLS chain やネットワーク疎通を確認)")?;

        let status = response.status();
        let html = response
            .text()
            .context("検索ページ HTML の読み込みに失敗")?;
        if !status.is_success() {
            bail!("検索ページが HTTP {} を返しました", status.as_u16());
        }

        self.token = extract_token(&html).context("token 抽出に失敗")?;
        Ok(())
    }
}

impl PageFetcher for Client {
    /// Fetch one findPage page as raw JSON bytes (the exact response body).
    fn fetch_page(&self, page_no: i32) -> Result<Vec<u8>> {
        let body = render_body(BODY_TEMPLATE, page_no, &self.kaiko_nendo, &self.token);
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
            .with_context(|| format!("findPage HTTP 呼び出しに失敗 (page {page_no})"))?;

        let status = response.status();
        let bytes = response
            .bytes()
            .with_context(|| format!("findPage レスポンス読み込みに失敗 (page {page_no})"))?
            .to_vec();

        if !status.is_success() {
            let preview: String = String::from_utf8_lossy(&bytes).chars().take(500).collect();
            bail!(
                "findPage が HTTP {} を返しました (page {page_no}): {preview}\n  \
                 ※ token が古い / body フォーマットが API と不整合の可能性があります",
                status.as_u16()
            );
        }
        Ok(bytes)
    }
}

/// Render the findPage body by substituting the three template placeholders —
/// byte-identical to Go's `text/template` for these logicless value insertions.
pub(super) fn render_body(template: &str, page_no: i32, kaiko_nendo: &str, token: &str) -> String {
    template
        .replace("{{.PageNo}}", &page_no.to_string())
        .replace("{{.KaikoNendo}}", kaiko_nendo)
        .replace("{{.Token}}", token)
}

#[cfg(test)]
mod tests {
    use super::{render_body, BODY_TEMPLATE};

    /// The body Rust renders must match the body Go's `text/template` renders for
    /// the same inputs — the golden was produced by the Go template engine
    /// (`crates/cli/assets/testdata/findpage_body.golden.json`). This is the
    /// load-bearing parity check: it determines whether Rust would get the same
    /// data from the real server. Template drift breaks whichever side lagged.
    #[test]
    fn render_matches_go_golden() {
        let golden = include_str!("../../assets/testdata/findpage_body.golden.json");
        let rendered = render_body(BODY_TEMPLATE, 1, "2026", "GOLDEN_TOKEN_0123456789");
        assert_eq!(rendered, golden);
    }
}
