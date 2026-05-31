//! The `convert` pipeline as a pure function: raw KULAS courses → the canonical
//! `data.json` bytes.
//!
//! Kept free of I/O and clocks so the byte-exact `golden_convert` test can pin
//! it; the binary supplies the timestamp and chooses the output sink (file or
//! stdout).

use std::borrow::Cow;

use anyhow::{Context, Result};
use syllabus_core::convert_v2;
use syllabus_core::model::{ProcessedData, RawCourse};

/// The canonical output plus any warnings raised while converting (empty course
/// codes, unparsable jikanwari, …).
pub struct Rendered {
    /// Bytes identical to `data.json`: compact-or-pretty JSON with Go-style HTML
    /// escaping and **no** trailing newline. The binary adds a newline only when
    /// writing to stdout.
    pub bytes: Vec<u8>,
    pub warnings: Vec<String>,
}

/// Convert raw courses into the canonical `data.json` bytes, stamping
/// `generated_at` (an RFC 3339 string). Pure: no filesystem, no clock.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn render_data_json(
    raw: &[RawCourse],
    generated_at: String,
    compact: bool,
) -> Result<Rendered> {
    let result = convert_v2(raw, generated_at);
    let json = encode(&result.data, compact)?;
    Ok(Rendered {
        bytes: json.into_bytes(),
        warnings: result.warnings,
    })
}

/// Serialize to JSON, then HTML-escape inside string values to match Go's
/// `encoding/json` (which defaults to `SetEscapeHTML(true)`).
fn encode(data: &ProcessedData, compact: bool) -> Result<String> {
    let json = if compact {
        serde_json::to_string(data)
    } else {
        serde_json::to_string_pretty(data)
    }
    .context("JSON 出力の生成に失敗しました")?;
    Ok(escape_html(&json).into_owned())
}

/// Escape the characters Go's JSON encoder escapes by default — `<`, `>`, `&`,
/// U+2028, U+2029. They appear only inside string values (never in JSON
/// structure), so one output-wide pass is correct. Returns the input untouched
/// when there is nothing to escape — the common case for this dataset.
fn escape_html(json: &str) -> Cow<'_, str> {
    let needs_escape = json.bytes().any(|b| matches!(b, b'<' | b'>' | b'&'))
        || json.contains(['\u{2028}', '\u{2029}']);
    if !needs_escape {
        return Cow::Borrowed(json);
    }

    let mut out = String::with_capacity(json.len());
    for ch in json.chars() {
        match ch {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other => out.push(other),
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::escape_html;
    use std::borrow::Cow;

    #[test]
    fn escapes_html_chars_inside_strings() {
        let escaped = escape_html("x<y>&z");
        assert!(
            !escaped.contains(['<', '>', '&']),
            "raw HTML chars remain: {escaped}"
        );
        // The `\uXXXX` escapes (checked without the backslash to keep the literal simple).
        assert!(
            escaped.contains("u003c") && escaped.contains("u003e") && escaped.contains("u0026")
        );
    }

    #[test]
    fn leaves_clean_json_borrowed() {
        assert!(matches!(
            escape_html(r#"{"nm":"日本語 abc 123"}"#),
            Cow::Borrowed(_)
        ));
    }
}
