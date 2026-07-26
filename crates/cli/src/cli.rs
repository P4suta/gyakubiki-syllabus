//! `syllabus-cli` binary. The pipeline lives in the `syllabus_cli` library (and
//! `syllabus_core`); this binary is only argument parsing, timestamping, and
//! output wiring.

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::Result;
use chrono::SecondsFormat;
use clap::{Args, Parser, Subcommand};

use crate::{banner, commit, dataset, fetch, fetch_details, fields, gen_sample, palette, term};

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
    /// Build and atomically publish a complete manifest-addressed v4 dataset.
    BuildDataset(BuildDatasetArgs),
    /// Fetch syllabus pages from KULAS monthly and update `raw/`.
    Fetch(fetch::FetchArgs),
    /// Fetch KULAS syllabus detail pages and update `raw-details/`.
    FetchDetails(fetch_details::FetchDetailsArgs),
    /// Commit changed files to the current branch (signed, via the GitHub API; CI only).
    Commit(commit::CommitArgs),
    /// Generate the display-spec doc / TS from FIELD_SPEC (--check verifies only).
    GenFieldDocs(GenFieldDocsArgs),
    /// Verify the committed palette matches the OKLCH derivation (--check for CI).
    GenPalette(GenPaletteArgs),
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
struct GenPaletteArgs {
    /// Repository root (base for the colour files).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Verify the committed colours match the derivation (for CI).
    #[arg(long)]
    check: bool,
}

#[derive(Args)]
struct BuildDatasetArgs {
    /// Input files. Multiple page files are merged in argument order.
    #[arg(required = true)]
    files: Vec<PathBuf>,
    /// Directory containing crawled syllabus details.
    #[arg(long = "details-dir")]
    details_dir: Option<PathBuf>,
    /// Public output root. Only manifest.json has a stable URL.
    #[arg(short, long, default_value = "web/public")]
    output: PathBuf,
    /// Source commit recorded in the manifest (defaults to GITHUB_SHA; required otherwise).
    #[arg(long)]
    source_commit: Option<String>,
    /// Override generatedAt. Defaults to the current Asia/Tokyo time.
    #[arg(long = "generated-at")]
    generated_at: Option<String>,
    /// Compact data.json output.
    #[arg(long, default_value_t = true)]
    compact: bool,
    /// Allow missing details only for synthetic development/E2E datasets.
    #[arg(long, hide = true)]
    allow_incomplete_details: bool,
}

pub(crate) fn run() -> std::process::ExitCode {
    if std::io::stderr().is_terminal() {
        banner::print();
    }
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
        Command::BuildDataset(args) => build_dataset(args),
        Command::Fetch(args) => fetch::run(args),
        Command::FetchDetails(args) => fetch_details::run(args),
        Command::Commit(args) => commit::run(args),
        Command::GenFieldDocs(args) => fields::generate(&args.root, args.check),
        Command::GenPalette(args) => gen_palette(&args),
        Command::GenSample(args) => gen_sample::run(args),
    }
}

fn build_dataset(args: BuildDatasetArgs) -> Result<()> {
    let generated_at = args.generated_at.unwrap_or_else(|| {
        let jst = chrono::FixedOffset::east_opt(9 * 60 * 60).expect("valid JST offset");
        chrono::Utc::now()
            .with_timezone(&jst)
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    });
    let source_commit = args
        .source_commit
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("--source-commit or the GITHUB_SHA environment variable is required")
        })?;
    let manifest = dataset::build(&dataset::BuildDatasetOptions {
        files: args.files,
        details_dir: args.details_dir,
        output_dir: args.output,
        generated_at,
        source_commit,
        compact: args.compact,
        allow_incomplete_details: args.allow_incomplete_details,
    })?;
    serde_json::to_writer(std::io::stdout().lock(), &manifest)?;
    println!();
    Ok(())
}

/// Verify (or print) the derived Macaron palette. `--check` asserts the
/// committed colour files still match; otherwise stdout is one JSON document.
fn gen_palette(args: &GenPaletteArgs) -> Result<()> {
    if args.check {
        palette::check(&args.root)?;
        term::ok("palette matches the derivation");
        return Ok(());
    }
    let p = palette::derive();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &p)?;
    println!();
    Ok(())
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
    fn build_dataset_flags_parse() {
        let cli = Cli::try_parse_from([
            "syllabus-cli",
            "build-dataset",
            "a.json",
            "b.json",
            "-o",
            "public",
        ])
        .expect("valid build-dataset invocation");
        match cli.command {
            Command::BuildDataset(args) => {
                assert_eq!(args.files.len(), 2);
                assert!(args.compact);
                assert_eq!(args.output, std::path::Path::new("public"));
            }
            _ => panic!("expected BuildDataset"),
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
