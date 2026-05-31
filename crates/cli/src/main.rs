//! Native CLI that builds `data.json` from raw KULAS JSON — the Rust port of
//! Go's `syllabus-cli convert`. The pipeline lives in the `syllabus_cli` library
//! (and `syllabus_core`); this binary is only argument parsing, timestamping and
//! output wiring.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::SecondsFormat;
use clap::{Args, Parser, Subcommand};

use syllabus_cli::convert::render_data_json;
use syllabus_cli::{fetch, io};

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
    /// 生 KULAS JSON を v3 `data.json` に変換する。
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
    let rendered = render_data_json(&raw, generated_at, args.compact)?;

    for warning in &rendered.warnings {
        eprintln!("{warning}");
    }
    emit(&rendered.bytes, args.output.as_deref())
}

/// Write the canonical bytes. A file gets them verbatim (no trailing newline,
/// like Go's `os.WriteFile`); stdout gets a trailing newline (like `fmt.Println`).
fn emit(bytes: &[u8], output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => fs::write(path, bytes)
            .with_context(|| format!("出力ファイルの書き込みに失敗しました: {}", path.display())),
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(bytes)?;
            stdout.write_all(b"\n")?;
            Ok(())
        }
    }
}
