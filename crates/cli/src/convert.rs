//! Pure `convert` pipeline: raw KULAS courses → canonical `data.json` bytes.
//!
//! No I/O or clocks, so the byte-exact `golden_convert` test can pin it; the
//! binary supplies the timestamp and output sink.

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{Context, Result};
use syllabus_core::convert_v2;
use syllabus_core::model::{Course, ProcessedData, RawCourse};
use syllabus_core::normalize;

use crate::detail::SanshoDetail;

/// Canonical output plus any warnings raised while converting.
pub struct Rendered {
    /// `data.json` bytes: compact-or-pretty JSON, HTML-escaped, with **no**
    /// trailing newline. The binary adds a newline only when writing to stdout.
    pub bytes: Vec<u8>,
    pub warnings: Vec<String>,
}

/// Convert raw courses into `data.json` bytes, stamping `generated_at` (RFC 3339).
///
/// # Errors
/// Returns an error if serialization fails.
pub fn render_data_json(
    raw: &[RawCourse],
    generated_at: String,
    compact: bool,
    details: &HashMap<String, SanshoDetail>,
) -> Result<Rendered> {
    let mut result = convert_v2(raw, generated_at);
    if !details.is_empty() {
        for course in &mut result.data.courses {
            if let Some(detail) = details.get(&course.cd) {
                enrich_course(course, detail);
            }
        }
    }
    let json = encode(&result.data, compact)?;
    Ok(Rendered {
        bytes: json.into_bytes(),
        warnings: result.warnings,
    })
}

/// Fold a course's syllabus detail into its grid record: card fields
/// (`unit`/`dm`/`ev`) plus キーワード appended to the search haystack `st`. The
/// full detail (概要・到達目標 …) stays in `details/{cd}.json`.
fn enrich_course(course: &mut Course, detail: &SanshoDetail) {
    course.unit = detail.unit.clone();
    course.dm = detail
        .delivery
        .as_ref()
        .map(|d| d.mode.clone())
        .filter(|m| m != "unknown");
    // `kind:weight`, or just `kind` when the weight is unknown — never a
    // fabricated `:0`, which would read as a real 0-point item. The frontend's
    // `e.split(':')` tolerates the missing weight.
    course.ev = detail.eval.as_ref().map(|e| {
        e.rows
            .iter()
            .map(|r| match r.weight {
                Some(w) => format!("{}:{}", r.kind, w),
                None => r.kind.clone(),
            })
            .collect()
    });

    let extra = detail_search_text(detail);
    if !extra.is_empty() {
        course.st = format!("{} {}", course.st, normalize(&extra));
    }
}

/// Detail text folded into the search haystack. Only キーワード — the short,
/// high-signal tags — are carried; the long 概要・目的・到達目標 prose is left out
/// so `data.json` stays small (it dominated the payload and gated LCP). That
/// prose is still readable in `details/{cd}.json`, just not full-text searchable.
fn detail_search_text(detail: &SanshoDetail) -> String {
    detail.keywords.join(" ").replace(['\n', '\r'], " ")
}

/// Serialize to JSON, then HTML-escape inside string values.
fn encode(data: &ProcessedData, compact: bool) -> Result<String> {
    let json = if compact {
        serde_json::to_string(data)
    } else {
        serde_json::to_string_pretty(data)
    }
    .context("failed to generate JSON output")?;
    Ok(escape_html(&json).into_owned())
}

/// Escape `<`, `>`, `&`, U+2028, U+2029. These appear only inside string values
/// (never in JSON structure), so one output-wide pass is correct. Returns the
/// input untouched when nothing needs escaping — the common case here.
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
    use super::{escape_html, render_data_json};
    use crate::detail::{Delivery, Eval, EvalRow, SanshoDetail};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use syllabus_core::model::RawCourse;

    #[test]
    fn enriches_course_with_detail_card_fields_and_search() {
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "情報科学".into(),
            ..Default::default()
        }];
        let detail = SanshoDetail {
            cd: "001".into(),
            unit: Some("2.0".into()),
            delivery: Some(Delivery {
                mode: "hybrid".into(),
                ..Default::default()
            }),
            eval: Some(Eval {
                rows: vec![
                    EvalRow {
                        item: "レポート".into(),
                        weight: Some(40),
                        kind: "report".into(),
                    },
                    EvalRow {
                        item: "期末試験".into(),
                        weight: Some(60),
                        kind: "exam".into(),
                    },
                ],
                note: None,
            }),
            keywords: vec!["アルゴリズム".into()],
            ..Default::default()
        };
        let details: HashMap<String, SanshoDetail> =
            [("001".to_owned(), detail)].into_iter().collect();

        let rendered = render_data_json(&raw, "t".into(), true, &details).unwrap();
        let json = String::from_utf8(rendered.bytes).unwrap();
        assert!(json.contains(r#""unit":"2.0""#));
        assert!(json.contains(r#""dm":"hybrid""#));
        assert!(json.contains(r#""ev":["report:40","exam:60"]"#));
        // Keyword folded into the search haystack `st`.
        assert!(json.contains("アルゴリズム"));
    }

    #[test]
    fn detail_prose_stays_out_of_the_search_haystack() {
        // Only キーワード ride into `st`; the long 概要/目的/到達目標 prose does not,
        // so it can't bloat data.json (it stays in details/{cd}.json). Guarding the
        // *negative* here — the earlier positive test alone wouldn't catch a
        // regression that re-adds the prose.
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "情報科学".into(),
            ..Default::default()
        }];
        // Japanese-only tokens: `normalize` lowercases ASCII, so distinct kana/kanji
        // avoid a false match from casing.
        let detail = SanshoDetail {
            cd: "001".into(),
            summary: Some("除外概要プロース本文".into()),
            aims: Some("除外目的プロース本文".into()),
            goals: vec!["除外到達目標プロース本文".into()],
            keywords: vec!["検索可能キーワード語".into()],
            ..Default::default()
        };
        let details: HashMap<String, SanshoDetail> =
            [("001".to_owned(), detail)].into_iter().collect();

        let json = String::from_utf8(
            render_data_json(&raw, "t".into(), true, &details)
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert!(
            json.contains("検索可能キーワード語"),
            "keyword should be searchable"
        );
        assert!(
            !json.contains("除外概要"),
            "summary prose must not enter st"
        );
        assert!(!json.contains("除外目的"), "aims prose must not enter st");
        assert!(
            !json.contains("除外到達目標"),
            "goals prose must not enter st"
        );
    }

    #[test]
    fn unparsable_weight_renders_kind_without_fabricated_zero() {
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "X".into(),
            ..Default::default()
        }];
        let detail = SanshoDetail {
            cd: "001".into(),
            eval: Some(Eval {
                rows: vec![
                    EvalRow {
                        item: "レポート".into(),
                        weight: None, // e.g. an unparsable / overflowing weight
                        kind: "report".into(),
                    },
                    EvalRow {
                        item: "期末試験".into(),
                        weight: Some(60),
                        kind: "exam".into(),
                    },
                ],
                note: None,
            }),
            ..Default::default()
        };
        let details: HashMap<String, SanshoDetail> =
            [("001".to_owned(), detail)].into_iter().collect();
        let json = String::from_utf8(
            render_data_json(&raw, "t".into(), true, &details)
                .unwrap()
                .bytes,
        )
        .unwrap();
        // No fabricated ":0"; the weightless item is just its kind.
        assert!(json.contains(r#""ev":["report","exam:60"]"#));
        assert!(!json.contains("report:0"));
    }

    #[test]
    fn unknown_delivery_mode_is_dropped_from_card() {
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "X".into(),
            ..Default::default()
        }];
        let detail = SanshoDetail {
            cd: "001".into(),
            delivery: Some(Delivery {
                mode: "unknown".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let details: HashMap<String, SanshoDetail> =
            [("001".to_owned(), detail)].into_iter().collect();
        let json = String::from_utf8(
            render_data_json(&raw, "t".into(), true, &details)
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert!(!json.contains(r#""dm""#));
    }

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
