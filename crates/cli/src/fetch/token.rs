//! Extract the `entryContext` (and its `token`) from the search-page HTML.
//!
//! The whole `entryContext` matters, not just the token: findPage validates the
//! token against the rest of the session context (`cpClientPid`, `userId`, …),
//! so we return the entire object for the client to inject into the request.

use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;

/// Matches the inline `cpSmartVueStartup('dash-app-main', '<ver>', <bool>,
/// '<base64-json>')` script and captures the base64 `entryContext` (4th arg).
static CP_SMART_VUE_STARTUP: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"cpSmartVueStartup\(\s*'dash-app-main'\s*,\s*'[^']+'\s*,\s*\w+\s*,\s*'([A-Za-z0-9+/=]+)'",
    )
    .expect("valid regex")
});

/// Extract the full `entryContext` object from the search-page HTML.
///
/// Locate the `dash-app-main` startup call, base64-decode its 4th argument,
/// parse the JSON, and return it. Errors if the script is missing, the base64 or
/// JSON is malformed, or the `token` field is empty.
pub fn extract_entry_context(html: &str) -> Result<Value> {
    let captures = CP_SMART_VUE_STARTUP.captures(html).context(
        "cpSmartVueStartup('dash-app-main', ...) inline script not found (the HTML structure may have changed)",
    )?;
    let decoded = STANDARD
        .decode(&captures[1])
        .context("failed to base64-decode entryContext")?;
    let context: Value =
        serde_json::from_slice(&decoded).context("failed to parse entryContext as JSON")?;
    match context.get("token").and_then(Value::as_str) {
        Some(token) if !token.is_empty() => Ok(context),
        _ => bail!("entryContext.token is empty or missing"),
    }
}

#[cfg(test)]
mod tests {
    use super::extract_entry_context;

    fn token_of(html: &str) -> Option<String> {
        extract_entry_context(html)
            .ok()
            .and_then(|c| c.get("token").and_then(|t| t.as_str()).map(str::to_owned))
    }

    #[test]
    fn extracts_token_from_valid_html() {
        // base64 of {"token":"abc123"}
        let html = "<html><body><script>\n\
            cpSmartVueStartup('dash-app-main', '2025-03-26-13-31-19-072', true, 'eyJ0b2tlbiI6ImFiYzEyMyJ9')\n\
            </script></body></html>";
        assert_eq!(token_of(html).as_deref(), Some("abc123"));
    }

    #[test]
    fn errors_when_no_startup_script() {
        let err = extract_entry_context("<html><body>no startup script</body></html>").unwrap_err();
        assert!(err.to_string().contains("cpSmartVueStartup"));
    }

    #[test]
    fn errors_on_bad_base64() {
        assert!(
            extract_entry_context("cpSmartVueStartup('dash-app-main', 'v', true, 'not!base64')")
                .is_err()
        );
    }

    #[test]
    fn errors_on_empty_token() {
        // base64 of {"token":""}
        assert!(
            extract_entry_context(
                "cpSmartVueStartup('dash-app-main', 'v', true, 'eyJ0b2tlbiI6IiJ9')"
            )
            .is_err()
        );
    }

    #[test]
    fn ignores_non_dash_app_main_component() {
        // base64 of {"token":"other"} under dash-header — must not match.
        assert!(
            extract_entry_context(
                "cpSmartVueStartup('dash-header', 'v', true, 'eyJ0b2tlbiI6Im90aGVyIn0=')"
            )
            .is_err()
        );
    }
}
