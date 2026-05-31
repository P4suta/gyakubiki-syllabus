//! Extract the findPage `token` from the search-page HTML — a port of Go's
//! `extractTokenFromHTML` (`internal/fetch/html_token.go`).

use std::sync::LazyLock;

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;

/// Matches the inline `cpSmartVueStartup('dash-app-main', '<ver>', <bool>,
/// '<base64-json>')` script and captures the base64 `entryContext` (4th arg).
static CP_SMART_VUE_STARTUP: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"cpSmartVueStartup\(\s*'dash-app-main'\s*,\s*'[^']+'\s*,\s*\w+\s*,\s*'([A-Za-z0-9+/=]+)'",
    )
    .expect("valid regex")
});

/// The subset of `entryContext` we need; other fields are ignored.
#[derive(Deserialize)]
struct EntryContext {
    #[serde(default)]
    token: String,
}

/// Extract `entryContext.token` (64-char hex) from the search-page HTML.
///
/// Mirrors Go: locate the `dash-app-main` startup call, base64-decode its 4th
/// argument, parse the JSON, return `.token`. Any missing step is an error.
pub fn extract_token(html: &str) -> Result<String> {
    let captures = CP_SMART_VUE_STARTUP.captures(html).context(
        "cpSmartVueStartup('dash-app-main', ...) inline script が見つかりません (HTML 構造変更の可能性)",
    )?;
    let decoded = STANDARD
        .decode(&captures[1])
        .context("entryContext の base64 デコードに失敗")?;
    let ctx: EntryContext =
        serde_json::from_slice(&decoded).context("entryContext の JSON パースに失敗")?;
    if ctx.token.is_empty() {
        bail!("entryContext.token が空");
    }
    Ok(ctx.token)
}

#[cfg(test)]
mod tests {
    //! Ported from `internal/fetch/fetch_test.go` (TestExtractTokenFromHTML*).
    use super::extract_token;

    #[test]
    fn extracts_token_from_valid_html() {
        // base64 of {"token":"abc123"}
        let html = "<html><body><script>\n\
            cpSmartVueStartup('dash-app-main', '2025-03-26-13-31-19-072', true, 'eyJ0b2tlbiI6ImFiYzEyMyJ9')\n\
            </script></body></html>";
        assert_eq!(extract_token(html).unwrap(), "abc123");
    }

    #[test]
    fn errors_when_no_startup_script() {
        let err = extract_token("<html><body>no startup script</body></html>").unwrap_err();
        assert!(err.to_string().contains("cpSmartVueStartup"));
    }

    #[test]
    fn errors_on_bad_base64() {
        assert!(
            extract_token("cpSmartVueStartup('dash-app-main', 'v', true, 'not!base64')").is_err()
        );
    }

    #[test]
    fn errors_on_empty_token() {
        // base64 of {"token":""}
        assert!(
            extract_token("cpSmartVueStartup('dash-app-main', 'v', true, 'eyJ0b2tlbiI6IiJ9')")
                .is_err()
        );
    }

    #[test]
    fn ignores_non_dash_app_main_component() {
        // base64 of {"token":"other"} under dash-header — must not match.
        assert!(extract_token(
            "cpSmartVueStartup('dash-header', 'v', true, 'eyJ0b2tlbiI6Im90aGVyIn0=')"
        )
        .is_err());
    }
}
