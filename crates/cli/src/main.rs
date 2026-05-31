//! Native CLI that builds `data.json` from raw KULAS JSON — the Rust port of
//! Go's `syllabus-cli convert`. The domain logic lives in `syllabus_core`; this
//! binary is only argument parsing, I/O, timestamping and output encoding.

mod fetch;
mod io;

use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use chrono::SecondsFormat;
use clap::{Args, Parser, Subcommand};
use syllabus_core::{convert_v2, model::ProcessedDataV2};

#[derive(Parser)]
#[command(
    name = "syllabus-cli",
    about = "高知大学シラバスの変換 CLI (Rust)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 生 KULAS JSON を v2 `data.json` に変換する。
    Convert(ConvertArgs),
    /// KULAS から月次でシラバスを取得し raw/ を更新する。
    Fetch(fetch::FetchArgs),
}

#[derive(Args)]
struct ConvertArgs {
    /// 入力ファイル (省略時は stdin)。複数指定すると順にマージする。
    files: Vec<PathBuf>,
    /// 出力先ファイル (省略時は stdout)。
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// 圧縮出力 (インデントなし)。
    #[arg(long)]
    compact: bool,
    /// `generatedAt` を上書き (RFC 3339)。省略時は現在時刻 (UTC)。
    #[arg(long = "generated-at")]
    generated_at: Option<String>,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Convert(args) => convert(args),
        Command::Fetch(args) => fetch::run(args),
    }
}

fn convert(args: ConvertArgs) -> Result<()> {
    let raw = io::load(&args.files)?;
    if raw.is_empty() {
        bail!("講義データが0件です。入力ファイルの内容を確認してください");
    }

    let generated_at = args
        .generated_at
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
    let result = convert_v2(&raw, generated_at);

    for warning in &result.warnings {
        eprintln!("{warning}");
    }

    let json = encode(&result.data, args.compact)?;
    match &args.output {
        // File output is byte-exact (no trailing newline), matching Go's os.WriteFile.
        Some(path) => fs::write(path, json.as_bytes())
            .with_context(|| format!("出力ファイルの書き込みに失敗しました: {}", path.display()))?,
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(json.as_bytes())?;
            stdout.write_all(b"\n")?; // Go's fmt.Println adds a trailing newline on stdout.
        }
    }
    Ok(())
}

/// Serialize to JSON, then HTML-escape inside string values to match Go's
/// `encoding/json` (which defaults to `SetEscapeHTML(true)`).
fn encode(data: &ProcessedDataV2, compact: bool) -> Result<String> {
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
