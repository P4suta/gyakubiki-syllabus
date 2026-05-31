//! Input handling for `convert`: read one or more raw KULAS JSON files (or
//! stdin) and parse each as either the `selectKogiDtoList` envelope or a bare
//! course array — a port of Go's `readInput` / `parseInput` / `loadAndMerge`.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use syllabus_core::model::{RawCourse, RawResponse};

/// Load raw courses from `files` (merged in argument order), or from stdin when
/// no files are given.
pub fn load(files: &[PathBuf]) -> Result<Vec<RawCourse>> {
    if files.is_empty() {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("標準入力の読み込みに失敗しました")?;
        return parse(&text);
    }

    let mut all = Vec::new();
    for file in files {
        let text = fs::read_to_string(file)
            .with_context(|| format!("ファイルを読み込めません: {}", file.display()))?;
        all.extend(parse(&text)?);
    }
    Ok(all)
}

/// Parse raw JSON as the KULAS envelope, falling back to a bare array.
fn parse(text: &str) -> Result<Vec<RawCourse>> {
    let text = text.trim();
    if text.is_empty() {
        bail!("入力が空です。JSON データを含むファイルを指定してください");
    }

    // The envelope wins when its key is present (Go's `resp.SelectKogiDtoList != nil`).
    if let Ok(resp) = serde_json::from_str::<RawResponse>(text) {
        if let Some(courses) = resp.select_kogi_dto_list {
            return Ok(courses);
        }
    }
    if let Ok(courses) = serde_json::from_str::<Vec<RawCourse>>(text) {
        return Ok(courses);
    }
    bail!("JSON として講義データを認識できません (selectKogiDtoList ラッパー、または生配列が必要です)")
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_kulas_envelope() {
        let raw = parse(r#"{"selectKogiDtoList":[{"kogiCd":"001","kogiNm":"A"}]}"#).unwrap();
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].kogi_cd, "001");
    }

    #[test]
    fn parses_bare_array() {
        let raw = parse(r#"[{"kogiCd":"001"},{"kogiCd":"002"}]"#).unwrap();
        assert_eq!(raw.len(), 2);
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
}
