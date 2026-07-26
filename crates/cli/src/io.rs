//! Strict input handling for dataset generation and fetch validation.
//!
//! Each document is deserialized exactly once into either a typed KULAS page or
//! a typed bare course array. Unknown fields and malformed records are fatal:
//! publishing a smaller dataset is never an acceptable recovery strategy.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use syllabus_core::RawCourse;

/// One typed findPage response.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageEnvelope {
    #[serde(rename = "pageNo")]
    pub page_no: i32,
    #[serde(rename = "maxPageNo")]
    pub max_page_no: i32,
    pub total: i32,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(rename = "selectKogiDtoList")]
    pub courses: Vec<RawCourse>,
}

/// Parsed raw courses.
pub struct Loaded {
    pub courses: Vec<RawCourse>,
}

/// Load raw courses from `files` (merged in argument order), or from stdin when
/// no files are given.
pub fn load(files: &[PathBuf]) -> Result<Loaded> {
    if files.is_empty() {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("failed to read from stdin")?;
        return parse(&text);
    }

    let mut courses = Vec::new();
    for file in files {
        let text = fs::read_to_string(file)
            .with_context(|| format!("cannot read file: {}", file.display()))?;
        let mut loaded = parse(&text)?;
        courses.append(&mut loaded.courses);
    }
    Ok(Loaded { courses })
}

/// Parse raw JSON as either the exact KULAS envelope or a bare typed array.
fn parse(text: &str) -> Result<Loaded> {
    let text = text.trim();
    if text.is_empty() {
        bail!("Input is empty. Provide a file containing JSON data");
    }

    let courses = match text.as_bytes().first() {
        Some(b'[') => serde_json::from_str::<Vec<RawCourse>>(text)
            .context("cannot parse bare course array")?,
        Some(b'{') => {
            serde_json::from_str::<PageEnvelope>(text)
                .context("cannot parse typed findPage response")?
                .courses
        }
        _ => bail!(
            "Cannot recognize course data as JSON (expected a selectKogiDtoList wrapper or a bare array)"
        ),
    };
    Ok(Loaded { courses })
}

/// Parse one fetched response without losing its pagination metadata.
pub(crate) fn parse_page(bytes: &[u8]) -> Result<PageEnvelope> {
    serde_json::from_slice(bytes).context("cannot parse typed findPage response")
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_kulas_envelope() {
        let loaded = parse(
            r#"{"pageNo":1,"maxPageNo":1,"total":1,"pageSize":500,"selectKogiDtoList":[{"kogiCd":"001","kogiNm":"A"}]}"#,
        )
        .unwrap();
        assert_eq!(loaded.courses.len(), 1);
        assert_eq!(loaded.courses[0].kogi_cd, "001");
        assert_eq!(loaded.courses[0].kogi_nm, "A");
    }

    #[test]
    fn parses_bare_array() {
        let loaded = parse(r#"[{"kogiCd":"001"},{"kogiCd":"002"}]"#).unwrap();
        assert_eq!(loaded.courses.len(), 2);
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(parse("   ").is_err());
    }

    #[test]
    fn non_course_json_is_an_error() {
        assert!(parse(r#"{"unexpected": true}"#).is_err());
    }

    #[test]
    fn envelope_with_null_list_falls_back_and_errors() {
        // `selectKogiDtoList: null` is not the envelope shape (it's `None`, not
        // `Some`), so the bare-array branch is tried next and also fails — an
        // error, never a panic.
        assert!(parse(r#"{"selectKogiDtoList": null}"#).is_err());
    }

    #[test]
    fn numeric_fields_no_longer_kill_the_record() {
        // KULAS occasionally sends numeric values for string-ish fields; lenient
        // deserialization keeps the record instead of aborting.
        let loaded = parse(r#"[{"kogiCd":"1","taishoNenji":1,"kaikoNendo":2026}]"#).unwrap();
        assert_eq!(loaded.courses.len(), 1);
    }

    #[test]
    fn empty_array_is_zero_courses_not_an_error() {
        let loaded = parse("[]").unwrap();
        assert!(loaded.courses.is_empty());
    }

    #[test]
    fn one_bad_record_aborts_the_whole_document() {
        assert!(parse(r#"[{"kogiCd":"ok"}, 42, {"kogiCd":"also-ok"}]"#).is_err());
    }

    #[test]
    fn unknown_envelope_field_is_rejected() {
        assert!(
            parse(
                r#"{"pageNo":1,"maxPageNo":1,"total":0,"pageSize":500,"selectKogiDtoList":[],"newMeta":true}"#
            )
            .is_err()
        );
    }
}
