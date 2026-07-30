//! Pure `convert` pipeline: raw KULAS courses → canonical `data.json` bytes.
//!
//! No I/O or clocks, so the byte-exact `golden_convert` test can pin it; the
//! binary supplies the timestamp and output sink.

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use syllabus_core::convert_v4;
use syllabus_core::{Course, DocFields, ProcessedData, RawCourse, SearchIndex};

use crate::detail::SanshoDetail;

/// Canonical data and search-index output.
pub struct Rendered {
    /// `data.json` bytes: compact-or-pretty JSON, HTML-escaped, with **no**
    /// trailing newline. The binary adds a newline only when writing to stdout.
    pub bytes: Vec<u8>,
    /// `search.idx` bytes: the compact binary full-text index, shipped alongside
    /// `data.json` and loaded lazily in the worker.
    pub index: Vec<u8>,
}

/// Convert raw courses into `data.json` bytes, stamping `generated_at` (RFC
/// 3339) and binding the dataset identity to `source_commit`.
///
/// # Errors
/// Returns an error if the source violates the v4 publication schema, detail
/// enrichment is inconsistent, or serialization fails.
pub fn render_data_json(
    raw: &[RawCourse],
    generated_at: String,
    source_commit: &str,
    compact: bool,
    details: &HashMap<String, SanshoDetail>,
) -> Result<Rendered> {
    let dataset_id = dataset_identity(raw, details, &generated_at, source_commit)?;
    let mut result =
        convert_v4(raw, generated_at, dataset_id.clone()).context("source validation failed")?;
    if !details.is_empty() {
        for course in &mut result.data.courses {
            if let Some(detail) = details.get(&course.cd) {
                enrich_course(course, detail);
            }
        }
    }
    let index = build_search_index(&result.data, details);
    let json = encode(&result.data, compact)?;
    Ok(Rendered {
        bytes: json.into_bytes(),
        index,
    })
}

#[derive(Default)]
struct OwnedSearchFields {
    department: String,
    summary: String,
    aims: String,
    goals: String,
    plan: String,
    textbooks: String,
    prerequisite: String,
    preparation: String,
    office_hour: String,
    keywords: String,
    teachers: String,
    numbering: String,
    sdgs: String,
    evaluation: String,
    delivery: String,
    extra: String,
}

/// Build the positional full-text index. Each public syllabus field has its own
/// stable field discriminant so ranking, match labels, and offsets never collapse
/// into an opaque "body" bucket.
fn build_search_index(data: &ProcessedData, details: &HashMap<String, SanshoDetail>) -> Vec<u8> {
    let search_fields: Vec<OwnedSearchFields> = data
        .courses
        .iter()
        .map(|course| {
            let dept = data
                .dicts
                .departments
                .get(course.dept as usize)
                .map_or("", String::as_str);
            let mut fields = OwnedSearchFields {
                department: join_search_values([
                    dept,
                    course.gaku.as_deref().unwrap_or_default(),
                    course.gakka.as_deref().unwrap_or_default(),
                    course.bunya.as_deref().unwrap_or_default(),
                    course.bunrui.as_deref().unwrap_or_default(),
                ]),
                ..OwnedSearchFields::default()
            };
            if let Some(detail) = details.get(&course.cd) {
                fields.summary = join_search_values(detail.summary.iter().map(String::as_str));
                fields.aims = join_search_values(detail.aims.iter().map(String::as_str));
                fields.goals = join_search_values(detail.goals.iter().map(String::as_str));
                fields.plan =
                    join_search_values(detail.plan.iter().map(|value| value.text.as_str()));
                fields.textbooks = join_search_values(detail.textbooks.iter().map(String::as_str));
                fields.prerequisite = join_search_values(detail.prereq.iter().map(String::as_str));
                fields.preparation = join_search_values(detail.prep.iter().map(String::as_str));
                fields.office_hour =
                    join_search_values(detail.office_hour.iter().flat_map(|value| {
                        [
                            value.name.as_str(),
                            value.day.as_str(),
                            value.time.as_str(),
                            value.place.as_str(),
                        ]
                    }));
                fields.keywords = join_search_values(detail.keywords.iter().map(String::as_str));
                fields.teachers = join_search_values(detail.teachers.iter().map(String::as_str));
                fields.numbering = join_search_values(detail.numbering.iter().map(String::as_str));
                fields.sdgs = join_search_values(detail.sdgs.iter().map(String::as_str));
                fields.evaluation = join_search_values(detail.eval.iter().flat_map(|value| {
                    value
                        .rows
                        .iter()
                        .map(|row| row.item.as_str())
                        .chain(value.note.iter().map(String::as_str))
                }));
                fields.delivery = join_search_values(
                    detail
                        .delivery
                        .iter()
                        .flat_map(|value| [value.mode.as_str(), value.raw.as_str()]),
                );
                fields.extra = join_search_values(
                    detail
                        .extra
                        .iter()
                        .flat_map(|value| [value.label.as_str(), value.text.as_str()]),
                );
            }
            fields
        })
        .collect();

    let docs = data
        .courses
        .iter()
        .zip(&search_fields)
        .map(|(course, fields)| DocFields {
            name: &course.nm,
            subtitle: course.sub.as_deref(),
            instructor: &course.prof,
            code: &course.cd,
            department: &fields.department,
            summary: &fields.summary,
            aims: &fields.aims,
            goals: &fields.goals,
            plan: &fields.plan,
            textbooks: &fields.textbooks,
            prerequisite: &fields.prerequisite,
            preparation: &fields.preparation,
            office_hour: &fields.office_hour,
            keywords: &fields.keywords,
            teachers: &fields.teachers,
            numbering: &fields.numbering,
            sdgs: &fields.sdgs,
            evaluation: &fields.evaluation,
            delivery: &fields.delivery,
            extra: &fields.extra,
        });
    SearchIndex::build_for_dataset(&data.dataset_id, docs).encode()
}

/// Stable identity for all public assets in a build. Hash-map iteration is
/// sorted explicitly so identical inputs produce the same ID on every platform.
fn dataset_identity(
    raw: &[RawCourse],
    details: &HashMap<String, SanshoDetail>,
    generated_at: &str,
    source_commit: &str,
) -> Result<String> {
    let mut hasher = Sha256::new();
    // Bind the deterministic transform to the source truth. A wire/index codec
    // change must never reuse an immutable directory from an older app build.
    hasher.update(
        b"gyakubiki-dataset-v4-search-syx4-nfkc-positional-br2-decoded-sha-source-commit\0",
    );
    hasher.update(source_commit.as_bytes());
    hasher.update(b"\0");
    // `generated_at` is embedded in data.json, so it is part of the immutable
    // asset identity even when the source records themselves are unchanged.
    hasher.update(generated_at.as_bytes());
    hasher.update(b"\0");
    hasher.update(serde_json::to_vec(raw).context("failed to fingerprint courses")?);
    let mut keys: Vec<&String> = details.keys().collect();
    keys.sort_unstable();
    for key in keys {
        hasher.update(key.as_bytes());
        hasher.update(serde_json::to_vec(&details[key]).context("failed to fingerprint details")?);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn join_search_values<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.replace(['\n', '\r'], " "))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fold a course's syllabus detail into its grid record: the card fields
/// (`unit`/`dm`/`ev`). The syllabus キーワード go into the search index (see
/// [`build_search_index`]), and the full detail (概要・到達目標 …) stays in
/// the content-addressed detail asset selected by the dataset manifest.
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
    use syllabus_core::RawCourse;

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

        let rendered = render_data_json(&raw, "t".into(), "test-source", true, &details).unwrap();
        let json = String::from_utf8(rendered.bytes).unwrap();
        assert!(json.contains(r#""unit":"2.0""#));
        assert!(json.contains(r#""dm":"hybrid""#));
        assert!(json.contains(r#""ev":["report:40","exam:60"]"#));
        // The keyword is searchable via the index, not carried in data.json — see
        // `builds_a_searchable_index_covering_name_and_keywords`.
    }

    #[test]
    fn builds_a_searchable_index_covering_name_and_keywords() {
        use syllabus_core::{CourseIndex, SearchIndex};

        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "情報科学".into(),
            tanto_kyoin: "山田 太郎".into(),
            sekinin_busho_nm: "固有部署検索語".into(),
            ..Default::default()
        }];
        let detail = SanshoDetail {
            cd: "001".into(),
            keywords: vec!["アルゴリズム".into()],
            ..Default::default()
        };
        let details: HashMap<String, SanshoDetail> =
            [("001".to_owned(), detail)].into_iter().collect();

        let rendered = render_data_json(&raw, "t".into(), "test-source", true, &details).unwrap();
        let index = SearchIndex::decode(&rendered.index).expect("index decodes");
        let candidates = [CourseIndex::new(0)];
        // Name, instructor, and the detail keyword are all reachable.
        assert_eq!(index.search("科学", candidates).len(), 1);
        assert_eq!(index.search("山田", candidates).len(), 1);
        assert_eq!(index.search("固有部署検索語", candidates).len(), 1);
        assert_eq!(index.search("アルゴリズム", candidates).len(), 1);
        assert!(index.search("存在しない語", candidates).is_empty());
    }

    #[test]
    fn detail_prose_is_searchable_via_the_index_but_not_data_json() {
        use syllabus_core::{CourseIndex, SearchIndex};
        // Detail prose belongs in the lazy full-text index, while data.json stays
        // compact and keeps the canonical prose in its detail asset.
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "情報科学".into(),
            ..Default::default()
        }];
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

        let rendered = render_data_json(&raw, "t".into(), "test-source", true, &details).unwrap();
        let json = String::from_utf8(rendered.bytes).unwrap();
        let index = SearchIndex::decode(&rendered.index).expect("index decodes");
        let one = [CourseIndex::new(0)];

        // Keyword and prose are searchable; prose is not duplicated into data.json.
        assert_eq!(index.search("検索可能キーワード語", one).len(), 1);
        for prose in ["除外概要", "除外目的", "除外到達目標"] {
            assert_eq!(index.search(prose, one).len(), 1, "{prose} must be indexed");
            assert!(
                !json.contains(prose),
                "prose {prose} must not enter data.json"
            );
        }
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
            render_data_json(&raw, "t".into(), "test-source", true, &details)
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
            render_data_json(&raw, "t".into(), "test-source", true, &details)
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

    #[test]
    fn generated_at_changes_the_dataset_identity() {
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "X".into(),
            ..Default::default()
        }];
        let details = HashMap::new();
        let first = render_data_json(
            &raw,
            "2026-01-01T00:00:00Z".into(),
            "test-source",
            true,
            &details,
        )
        .unwrap()
        .bytes;
        let second = render_data_json(
            &raw,
            "2026-01-02T00:00:00Z".into(),
            "test-source",
            true,
            &details,
        )
        .unwrap()
        .bytes;
        let first: serde_json::Value = serde_json::from_slice(&first).unwrap();
        let second: serde_json::Value = serde_json::from_slice(&second).unwrap();
        assert_ne!(first["datasetId"], second["datasetId"]);
    }

    #[test]
    fn source_commit_changes_the_dataset_identity() {
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: "X".into(),
            ..Default::default()
        }];
        let details = HashMap::new();
        let first = render_data_json(&raw, "t".into(), "commit-a", true, &details)
            .unwrap()
            .bytes;
        let second = render_data_json(&raw, "t".into(), "commit-b", true, &details)
            .unwrap()
            .bytes;
        let first: serde_json::Value = serde_json::from_slice(&first).unwrap();
        let second: serde_json::Value = serde_json::from_slice(&second).unwrap();
        assert_ne!(first["datasetId"], second["datasetId"]);
    }
}
