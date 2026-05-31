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
