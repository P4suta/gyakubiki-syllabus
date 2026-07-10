//! `syllabus-cli` binary. The pipeline lives in the `syllabus_cli` library (and
//! `syllabus_core`); this binary is only argument parsing, timestamping, and
//! output wiring.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::SecondsFormat;
use clap::{Args, Parser, Subcommand};

use syllabus_cli::convert::render_data_json;
use syllabus_cli::detail::SanshoDetail;
use syllabus_cli::{banner, commit, fetch, fetch_details, fields, gen_sample, io, term};

#[derive(Parser)]
#[command(
    name = "syllabus-cli",
    about = "Kochi University syllabus conversion CLI (Rust)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert raw KULAS JSON into v3 `data.json`.
    Convert(ConvertArgs),
    /// Fetch syllabus pages from KULAS monthly and update `raw/`.
    Fetch(fetch::FetchArgs),
    /// Fetch KULAS syllabus detail pages and update `raw-details/`.
    FetchDetails(fetch_details::FetchDetailsArgs),
    /// Commit changed files to the current branch (signed, via the GitHub API; CI only).
    Commit(commit::CommitArgs),
    /// Generate the display-spec doc / TS from FIELD_SPEC (--check verifies only).
    GenFieldDocs(GenFieldDocsArgs),
    /// Synthesize a dummy dataset (raw + details) for local UI development.
    GenSample(gen_sample::GenSampleArgs),
}

#[derive(Args)]
struct GenFieldDocsArgs {
    /// Repository root (base for generated output).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Verify existing files are up to date instead of generating (for CI).
    #[arg(long)]
    check: bool,
}

#[derive(Args)]
struct ConvertArgs {
    /// Input files (stdin when omitted). Multiple files are merged in order.
    files: Vec<PathBuf>,
    /// Output file (stdout when omitted).
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Compact output (no indentation).
    #[arg(long)]
    compact: bool,
    /// Override `generatedAt` (RFC 3339). Defaults to the current time (UTC).
    #[arg(long = "generated-at")]
    generated_at: Option<String>,
    /// Directory of syllabus detail JSON (if present, enrich data.json and copy details).
    #[arg(long = "details-dir")]
    details_dir: Option<PathBuf>,
    /// Destination directory for detail JSON copies (lazy-loaded by the frontend).
    #[arg(long = "details-out", default_value = "web/public/details")]
    details_out: PathBuf,
}

fn main() -> std::process::ExitCode {
    banner::print();
    let started = std::time::Instant::now();
    let result = dispatch();
    match &result {
        Ok(()) => term::footer_ok(started.elapsed()),
        Err(e) => term::footer_err(e, started.elapsed()),
    }
    if result.is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

fn dispatch() -> Result<()> {
    match Cli::parse().command {
        Command::Convert(args) => convert(args),
        Command::Fetch(args) => fetch::run(args),
        Command::FetchDetails(args) => fetch_details::run(args),
        Command::Commit(args) => commit::run(args),
        Command::GenFieldDocs(args) => fields::generate(&args.root, args.check),
        Command::GenSample(args) => gen_sample::run(args),
    }
}

fn convert(args: ConvertArgs) -> Result<()> {
    let loaded = io::load(&args.files)?;
    for warning in &loaded.warnings {
        term::warn(warning);
    }
    let raw = loaded.courses;
    if raw.is_empty() {
        bail!("No course data found. Check the input file contents");
    }

    let details = match &args.details_dir {
        Some(dir) if dir.is_dir() => load_details(dir)?,
        _ => std::collections::HashMap::new(),
    };
    if !details.is_empty() {
        write_details_out(&details, &args.details_out)?;
    }

    let generated_at = args
        .generated_at
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
    let rendered = render_data_json(&raw, generated_at, args.compact, &details)?;

    for warning in &rendered.warnings {
        term::warn(warning);
    }
    if let Some(path) = args.output.as_deref() {
        term::ok(&format!("wrote {} ({} courses)", path.display(), raw.len()));
        // The binary search index ships beside data.json (loaded lazily in the
        // worker). Only written in file mode — stdout carries data.json alone.
        let index_path = path.with_file_name("search.idx");
        fs::write(&index_path, &rendered.index)
            .with_context(|| format!("failed to write search index: {}", index_path.display()))?;
        term::ok(&format!(
            "wrote {} ({} bytes)",
            index_path.display(),
            rendered.index.len()
        ));
    }
    emit(&rendered.bytes, args.output.as_deref())
}

/// Load every `raw-details/*.json` into a `cd → SanshoDetail` map for enrichment.
fn load_details(dir: &Path) -> Result<std::collections::HashMap<String, SanshoDetail>> {
    let mut map = std::collections::HashMap::new();
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read details directory: {}", dir.display()))?
    {
        let path = entry?.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("cannot read detail JSON: {}", path.display()))?;
        let detail: SanshoDetail = serde_json::from_str(&text)
            .with_context(|| format!("cannot parse detail JSON: {}", path.display()))?;
        map.insert(detail.cd.clone(), detail);
    }
    term::ok(&format!("loaded {} syllabus details", map.len()));
    Ok(map)
}

/// Copy each detail to `out/{cd}.json` (compact) for the frontend to lazy-load.
fn write_details_out(
    details: &std::collections::HashMap<String, SanshoDetail>,
    out: &Path,
) -> Result<()> {
    fs::create_dir_all(out).with_context(|| {
        format!(
            "failed to create details output directory: {}",
            out.display()
        )
    })?;
    for detail in details.values() {
        let path = out.join(format!("{}.json", detail.cd));
        let bytes = serde_json::to_vec(detail).context("failed to serialize detail JSON")?;
        fs::write(&path, bytes)
            .with_context(|| format!("failed to write detail JSON: {}", path.display()))?;
    }
    term::ok(&format!(
        "copied {} details → {}",
        details.len(),
        out.display()
    ));
    Ok(())
}

/// Write the canonical bytes. A file gets them verbatim (no trailing newline);
/// stdout gets a trailing newline.
fn emit(bytes: &[u8], output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => fs::write(path, bytes)
            .with_context(|| format!("failed to write output file: {}", path.display())),
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(bytes)?;
            stdout.write_all(b"\n")?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_definition_is_valid() {
        // Catches clap wiring mistakes (duplicate args, bad defaults) at test time.
        Cli::command().debug_assert();
    }

    #[test]
    fn convert_flags_parse() {
        let cli = Cli::try_parse_from([
            "syllabus-cli",
            "convert",
            "a.json",
            "b.json",
            "--compact",
            "-o",
            "out.json",
        ])
        .expect("valid convert invocation");
        match cli.command {
            Command::Convert(args) => {
                assert_eq!(args.files.len(), 2);
                assert!(args.compact);
                assert_eq!(
                    args.output.as_deref(),
                    Some(std::path::Path::new("out.json"))
                );
            }
            _ => panic!("expected Convert"),
        }
    }

    #[test]
    fn fetch_and_fetch_details_parse() {
        assert!(matches!(
            Cli::try_parse_from(["syllabus-cli", "fetch-details"])
                .unwrap()
                .command,
            Command::FetchDetails(_)
        ));
        assert!(matches!(
            Cli::try_parse_from(["syllabus-cli", "fetch"])
                .unwrap()
                .command,
            Command::Fetch(_)
        ));
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["syllabus-cli", "bogus"]).is_err());
    }

    #[test]
    fn missing_subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["syllabus-cli"]).is_err());
    }
}
