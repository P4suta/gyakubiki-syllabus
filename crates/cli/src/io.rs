//! Input handling for `convert`: read one or more raw KULAS JSON files (or
//! stdin) and parse each as either the `selectKogiDtoList` envelope or a bare
//! course array.
//!
//! Records are deserialized one at a time: a single malformed course is skipped
//! with a warning rather than aborting the whole batch, so one bad row can never
//! zero out an entire monthly fetch.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use syllabus_core::model::RawCourse;

/// Parsed raw courses plus any per-record warnings raised while parsing.
pub struct Loaded {
    pub courses: Vec<RawCourse>,
    pub warnings: Vec<String>,
}

/// Load raw courses from `files` (merged in argument order), or from stdin when
/// no files are given. Warnings from every file are concatenated.
pub fn load(files: &[PathBuf]) -> Result<Loaded> {
    if files.is_empty() {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("failed to read from stdin")?;
        return parse(&text);
    }

    let mut courses = Vec::new();
    let mut warnings = Vec::new();
    for file in files {
        let text = fs::read_to_string(file)
            .with_context(|| format!("cannot read file: {}", file.display()))?;
        let mut loaded = parse(&text)?;
        courses.append(&mut loaded.courses);
        warnings.append(&mut loaded.warnings);
    }
    Ok(Loaded { courses, warnings })
}

/// Parse raw JSON as the KULAS envelope, falling back to a bare array, then
/// deserialize each course individually so a bad record is skipped, not fatal.
fn parse(text: &str) -> Result<Loaded> {
    let text = text.trim();
    if text.is_empty() {
        bail!("Input is empty. Provide a file containing JSON data");
    }

    let values = extract_course_values(text)?;
    let mut courses = Vec::with_capacity(values.len());
    let mut warnings = Vec::new();
    for (i, value) in values.into_iter().enumerate() {
        match serde_json::from_value::<RawCourse>(value) {
            Ok(course) => courses.push(course),
            Err(e) => warnings.push(format!(
                "  [item {}] skipped: cannot parse course record: {e}",
                i + 1
            )),
        }
    }
    Ok(Loaded { courses, warnings })
}

/// Pull the array of course objects out of either the `selectKogiDtoList`
/// envelope (which wins when present and non-null) or a bare top-level array.
/// Elements stay as untyped [`Value`] so [`parse`] can validate them one by one.
fn extract_course_values(text: &str) -> Result<Vec<Value>> {
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(rename = "selectKogiDtoList")]
        select_kogi_dto_list: Option<Vec<Value>>,
    }

    if let Ok(env) = serde_json::from_str::<Envelope>(text)
        && let Some(list) = env.select_kogi_dto_list
    {
        return Ok(list);
    }
    if let Ok(arr) = serde_json::from_str::<Vec<Value>>(text) {
        return Ok(arr);
    }
    bail!(
        "Cannot recognize course data as JSON (expected a selectKogiDtoList wrapper or a bare array)"
    )
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_kulas_envelope() {
        let loaded = parse(r#"{"selectKogiDtoList":[{"kogiCd":"001","kogiNm":"A"}]}"#).unwrap();
        assert_eq!(loaded.courses.len(), 1);
        assert_eq!(loaded.courses[0].kogi_cd, "001");
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn parses_bare_array() {
        let loaded = parse(r#"[{"kogiCd":"001"},{"kogiCd":"002"}]"#).unwrap();
        assert_eq!(loaded.courses.len(), 2);
        assert!(loaded.warnings.is_empty());
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
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn empty_array_is_zero_courses_not_an_error() {
        let loaded = parse("[]").unwrap();
        assert!(loaded.courses.is_empty());
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn one_bad_record_is_skipped_with_a_warning() {
        // A structurally wrong element (a bare scalar where an object is
        // expected) must not drop the good records alongside it.
        let loaded = parse(r#"[{"kogiCd":"ok"}, 42, {"kogiCd":"also-ok"}]"#).unwrap();
        assert_eq!(loaded.courses.len(), 2);
        assert_eq!(loaded.courses[0].kogi_cd, "ok");
        assert_eq!(loaded.courses[1].kogi_cd, "also-ok");
        assert_eq!(loaded.warnings.len(), 1);
        assert!(loaded.warnings[0].contains("[item 2]"));
    }
}
