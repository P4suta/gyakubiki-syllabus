//! `fetch-details` subcommand — crawl the KULAS「シラバス参照」detail pages the
//! findPage API omits (授業計画・成績評価・オフィスアワー…) and write structured
//! `raw-details/{kogiCd}.json`.
//!
//! Politeness is the priority: strictly sequential, a jittered sleep between
//! courses, bounded retries, and a **circuit breaker** that aborts the whole run
//! after N consecutive server refusals (403/429/5xx) so a block never turns into
//! hammering. Incremental by default — only courses whose grid `lastUpdate`
//! changed since the previous crawl are refetched.

mod client;
mod ui;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Days, Utc};
use clap::Args;
use serde::{Deserialize, Serialize};
use syllabus_core::{CourseCode, RawCourse};

use crate::detail::{SanshoDetail, parse_sansho_html};
use crate::io;
use crate::net::{Politeness, backoff};
use client::{CourseRef, DetailError, DetailFetcher, SanshoClient};

const TOKEN_ENV: &str = "KULAS_API_TOKEN";

#[derive(Args)]
pub struct FetchDetailsArgs {
    /// Directory containing findPage raw JSON (source list of target courses).
    #[arg(long = "raw-dir", default_value = "raw")]
    raw_dir: PathBuf,
    /// Output directory for structured detail JSON.
    #[arg(long = "out-dir", default_value = "raw-details")]
    out_dir: PathBuf,
    /// Base sleep between requests (ms).
    #[arg(long = "sleep-ms", default_value_t = 3000)]
    sleep_ms: u64,
    /// Upper bound of random jitter added to the sleep (ms).
    #[arg(long = "jitter-ms", default_value_t = 2000)]
    jitter_ms: u64,
    /// Max courses fetched per run (0 = all). CI passes this as a daily cap so a
    /// large backlog is spread over days instead of one long session; also handy
    /// for smoke tests. Combined with incremental skipping, the window advances
    /// each run.
    #[arg(long, default_value_t = 0)]
    limit: usize,
    /// Fetch only these course codes (comma-separated).
    #[arg(long)]
    only: Option<String>,
    /// Refetch even when lastUpdate is unchanged (full crawl).
    #[arg(long)]
    force: bool,
    /// Abort the whole run after this many consecutive server refusals (403/429/5xx).
    #[arg(long = "max-consecutive-blocks", default_value_t = 3)]
    max_consecutive_blocks: u32,
    /// Retries per course on transient errors.
    #[arg(long, default_value_t = 3)]
    retries: u32,
    /// Days before an officially confirmed unavailable course is retried.
    /// Protocol, parse, HTTP, and transport errors are never cached this way.
    #[arg(long = "unavailable-ttl-days", default_value_t = 30)]
    unavailable_ttl_days: u64,
    /// Stop cleanly after this many seconds (for CI partial commits; 0 = unlimited).
    #[arg(long = "max-secs", default_value_t = 0)]
    max_secs: u64,
    /// Override the KULAS token (for verification; normally empty to extract from HTML).
    #[arg(long)]
    token: Option<String>,
}

/// Run the detail crawl end to end.
pub fn run(args: FetchDetailsArgs) -> Result<()> {
    let all = course_refs(&load_dir(&args.raw_dir)?)?;

    fs::create_dir_all(&args.out_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            args.out_dir.display()
        )
    })?;

    ensure!(
        args.unavailable_ttl_days > 0 && args.unavailable_ttl_days <= 365,
        "--unavailable-ttl-days must be in 1..=365"
    );
    let now = Utc::now();
    let unavailable = load_unavailable(&args.out_dir)?;
    let selected = select_courses(all, &args, &unavailable, now);
    if selected.is_empty() {
        eprintln!(
            "fetch-details: nothing to fetch (all up to date, temporarily unavailable, or filtered out)"
        );
        return Ok(());
    }
    ui::header(selected.len(), args.sleep_ms, args.jitter_ms, &args.out_dir);

    let token = resolve_token_override(args.token.as_deref());
    let client = SanshoClient::new(&selected[0], token.as_deref())
        .context("failed to establish the syllabus detail session")?;

    let out_dir = args.out_dir.clone();
    let opts = CrawlOpts {
        retries: args.retries,
        max_consecutive_blocks: args.max_consecutive_blocks,
        politeness: Politeness {
            base: Duration::from_millis(args.sleep_ms),
            jitter_ms: args.jitter_ms,
        },
        max_run: (args.max_secs > 0).then(|| Duration::from_secs(args.max_secs)),
    };
    let report = crawl(&selected, &client, &opts, &mut |detail| {
        write_detail(&out_dir, detail)
    });
    report.print();
    ui::step_summary(
        report.fetched,
        &report.skipped,
        report.elapsed,
        report.aborted,
    );

    // Only explicit official absence responses are cached, and only for a
    // bounded TTL. Fatal protocol/parse/save failures are never converted into
    // absence.
    let unavailable = update_unavailable(
        unavailable,
        &report,
        &selected,
        &args.out_dir,
        now,
        args.unavailable_ttl_days,
    )?;
    save_unavailable(&args.out_dir, &unavailable)?;

    let attempted = report.fetched + report.skipped.len();
    if !report.diagnostics.is_empty() || report.aborted {
        let path = write_diagnostics(&report)?;
        let (headline, hint) = diagnose(&report);
        ui::diagnosis(&headline, hint.as_deref(), &path);
    }

    if let Some((course, error)) = &report.fatal_error {
        bail!("fatal detail crawl failure for {course:?}: {error}");
    }
    if report.aborted {
        bail!(
            "circuit breaker tripped after {} consecutive server refusals — see the diagnosis above",
            args.max_consecutive_blocks
        );
    }
    // A crawl that attempted real work but saved nothing is a silent systemic
    // failure (bad endpoint, blocked, changed HTML). Make it loud, not a green 0.
    if report.fetched == 0 && attempted >= 3 {
        bail!(
            "fetched 0 of {attempted} attempted courses — systemic failure; see the diagnosis above and diagnostics/fetch-details.md"
        );
    }
    Ok(())
}

/// A short verdict + an actionable hint, inferred from the captured failures.
fn diagnose(report: &CrawlReport) -> (String, Option<String>) {
    let attempted = report.fetched + report.skipped.len();
    let headline = format!(
        "{} fetched · {} skipped of {attempted} attempted",
        report.fetched,
        report.skipped.len()
    );
    let blob = report
        .diagnostics
        .iter()
        .map(|(_, d)| d.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    let has = |needle: &str| report.skipped.iter().any(|(_, w)| w.contains(needle));
    let hint = if blob.contains("service method") || blob.contains("not found") {
        Some(
            "The sansho API path/method may have changed — compare the captured body with INIT_FIND_URL / WEBMVC_URL in fetch_details/client.rs.",
        )
    } else if has("HTTP 403") || has("HTTP 429") {
        Some(
            "The server is rate-limiting or blocking us — raise --sleep-ms and retry later; the circuit breaker already backed off.",
        )
    } else if blob.contains("no guid") {
        Some(
            "initFind returned no guid — the request params or entryContext are likely stale/wrong.",
        )
    } else if blob.contains("empty html") {
        Some("webmvc returned empty HTML — the guid or session may be invalid.")
    } else {
        None
    };
    (headline, hint.map(str::to_owned))
}

/// Persist captured failure context to `diagnostics/fetch-details.md` (uploaded as
/// a CI artifact) so a failure is root-causable without an expensive re-run.
fn write_diagnostics(report: &CrawlReport) -> Result<PathBuf> {
    let dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&dir).context("failed to create diagnostics directory")?;
    let path = dir.join("fetch-details.md");
    let attempted = report.fetched + report.skipped.len();
    let (headline, hint) = diagnose(report);

    let mut md = String::new();
    md.push_str("# fetch-details diagnostics\n\n");
    md.push_str(&format!(
        "- **fetched**: {}\n- **skipped**: {}\n- **attempted**: {attempted}\n- **aborted**: {}\n- **elapsed**: {:?}\n\n",
        report.fetched,
        report.skipped.len(),
        report.aborted,
        report.elapsed,
    ));
    md.push_str(&format!("## verdict\n\n{headline}\n\n"));
    if let Some(h) = hint {
        md.push_str(&format!("> **hint** — {h}\n\n"));
    }

    md.push_str("## captured failures (with response bodies)\n\n");
    for (cd, diag) in &report.diagnostics {
        md.push_str(&format!("### course `{cd}`\n\n```\n{diag}\n```\n\n"));
    }

    // Full skip tally, so nothing is hidden by the capture cap.
    md.push_str("## all skip reasons\n\n");
    let mut tally: HashMap<&str, usize> = HashMap::new();
    for (_, why) in &report.skipped {
        *tally.entry(why.as_str()).or_default() += 1;
    }
    let mut rows: Vec<(&str, usize)> = tally.into_iter().collect();
    rows.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (why, n) in rows {
        md.push_str(&format!("- {n}× {why}\n"));
    }

    fs::write(&path, md).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

/// Load and merge every `*.json` under `dir` (the raw findPage pages).
fn load_dir(dir: &std::path::Path) -> Result<Vec<syllabus_core::RawCourse>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("failed to read raw directory {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let loaded = io::load(&files)?;
    Ok(loaded.courses)
}

/// Validate the complete source list before making a request. Missing identifiers
/// and duplicate/unsafe course codes are source-integrity failures, not values to
/// skip or fabricate.
fn course_refs(raw: &[syllabus_core::RawCourse]) -> Result<Vec<CourseRef>> {
    ensure!(!raw.is_empty(), "raw course list is empty");
    let mut seen = HashSet::new();
    raw.iter()
        .enumerate()
        .map(|(index, course)| {
            let code = CourseCode::parse(&course.kogi_cd)
                .with_context(|| format!("invalid course code at record {}", index + 1))?;
            ensure!(
                seen.insert(code.as_str().to_owned()),
                "duplicate course code {:?}",
                code.as_str()
            );
            let kaiko_nendo = course
                .kaiko_nendo
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .context("course has no academic year")?;
            ensure!(
                kaiko_nendo.len() == 4 && kaiko_nendo.chars().all(|value| value.is_ascii_digit()),
                "course {:?} has invalid academic year {:?}",
                code.as_str(),
                kaiko_nendo
            );
            let pattern_id = course
                .syllabus_komoku_pattern_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .with_context(|| {
                    format!("course {:?} has no syllabus pattern ID", code.as_str())
                })?;
            ensure!(
                !pattern_id.chars().any(char::is_control),
                "course {:?} has a control character in its pattern ID",
                code.as_str()
            );
            Ok(CourseRef {
                cd: code.as_str().to_owned(),
                kaiko_nendo: kaiko_nendo.to_owned(),
                pattern_id: pattern_id.to_owned(),
                last_update: course.last_update.clone().unwrap_or_default(),
            })
        })
        .collect()
}

/// Apply `--only`, incremental skipping, temporary official-unavailable state,
/// and `--limit`.
fn select_courses(
    all: Vec<CourseRef>,
    args: &FetchDetailsArgs,
    unavailable: &UnavailableState,
    now: DateTime<Utc>,
) -> Vec<CourseRef> {
    let existing = existing_last_updates(&args.out_dir);
    filter_courses(
        all,
        args.only.as_deref(),
        &existing,
        args.force,
        args.limit,
        unavailable,
        &now,
    )
}

/// Pure selection: `--only` → skip already-fetched (incremental) and unexpired
/// official-unavailable records → `--limit`. `--force` keeps everything.
fn filter_courses(
    all: Vec<CourseRef>,
    only: Option<&str>,
    existing: &HashMap<String, String>,
    force: bool,
    limit: usize,
    unavailable: &UnavailableState,
    now: &DateTime<Utc>,
) -> Vec<CourseRef> {
    let only: Option<std::collections::HashSet<&str>> = only.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect()
    });

    let mut out: Vec<CourseRef> = all
        .into_iter()
        .filter(|c| only.as_ref().is_none_or(|set| set.contains(c.cd.as_str())))
        .filter(|c| {
            force
                || (existing
                    .get(&c.cd)
                    .is_none_or(|prev| prev != &c.last_update || c.last_update.is_empty())
                    && !is_temporarily_unavailable(unavailable, &c.cd, &c.last_update, now))
        })
        .collect();
    if limit > 0 {
        out.truncate(limit);
    }
    out
}

/// Read the `lastUpdate` already saved for each course (for incremental skipping).
fn existing_last_updates(out_dir: &std::path::Path) -> HashMap<String, String> {
    let Ok(entries) = fs::read_dir(out_dir) else {
        return HashMap::new();
    };
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let text = fs::read_to_string(&p).ok()?;
            let detail: SanshoDetail = serde_json::from_str(&text).ok()?;
            Some((detail.cd, detail.last_update))
        })
        .collect()
}

// --- bounded official-unavailable state ---

const UNAVAILABLE_FILE: &str = "_unavailable.json";
const LEGACY_NO_DETAIL_FILE: &str = "_no-detail.tsv";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UnavailableState {
    version: u32,
    entries: BTreeMap<String, UnavailableRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UnavailableRecord {
    last_update: String,
    confirmed_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    reason: String,
}

fn load_unavailable(out_dir: &Path) -> Result<UnavailableState> {
    let path = out_dir.join(UNAVAILABLE_FILE);
    if !path.exists() {
        return Ok(UnavailableState {
            version: 1,
            entries: BTreeMap::new(),
        });
    }
    let state: UnavailableState = serde_json::from_slice(&fs::read(&path)?)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    ensure!(state.version == 1, "unsupported unavailable-state version");
    for (code, record) in &state.entries {
        CourseCode::parse(code).context("unavailable state contains an invalid course code")?;
        ensure!(
            record.expires_at > record.confirmed_at,
            "unavailable state for {code:?} has an invalid expiry"
        );
        ensure!(
            !record.reason.trim().is_empty(),
            "unavailable state for {code:?} has no official reason"
        );
    }
    Ok(state)
}

fn save_unavailable(out_dir: &Path, state: &UnavailableState) -> Result<()> {
    let path = out_dir.join(UNAVAILABLE_FILE);
    let legacy = out_dir.join(LEGACY_NO_DETAIL_FILE);
    if legacy.exists() {
        fs::remove_file(&legacy)
            .with_context(|| format!("failed to remove legacy {}", legacy.display()))?;
    }
    if state.entries.is_empty() {
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove empty {}", path.display()))?;
        }
        return Ok(());
    }
    let bytes =
        serde_json::to_vec_pretty(state).context("failed to serialize unavailable state")?;
    let next = out_dir.join("_unavailable.next.json");
    fs::write(&next, bytes)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to replace {}", path.display()))?;
    }
    fs::rename(&next, &path).with_context(|| format!("failed to promote {}", path.display()))
}

fn is_temporarily_unavailable(
    state: &UnavailableState,
    cd: &str,
    last_update: &str,
    now: &DateTime<Utc>,
) -> bool {
    state
        .entries
        .get(cd)
        .is_some_and(|record| record.last_update == last_update && record.expires_at > *now)
}

/// Active courses whose explicit official-unavailable record is still valid for
/// the same upstream `lastUpdate`. Used by the publisher to distinguish a
/// complete crawl with genuine absences from a partial crawl.
pub(crate) fn confirmed_unavailable_codes(
    out_dir: &Path,
    courses: &[RawCourse],
    now: DateTime<Utc>,
) -> Result<HashSet<String>> {
    let state = load_unavailable(out_dir)?;
    Ok(courses
        .iter()
        .filter(|course| {
            is_temporarily_unavailable(
                &state,
                course.kogi_cd.trim(),
                course.last_update.as_deref().unwrap_or(""),
                &now,
            )
        })
        .map(|course| course.kogi_cd.trim().to_owned())
        .collect())
}

fn update_unavailable(
    mut state: UnavailableState,
    report: &CrawlReport,
    selected: &[CourseRef],
    out_dir: &Path,
    now: DateTime<Utc>,
    ttl_days: u64,
) -> Result<UnavailableState> {
    let expires_at = now
        .checked_add_days(Days::new(ttl_days))
        .context("unavailable TTL overflow")?;
    let last_updates: HashMap<&str, &str> = selected
        .iter()
        .map(|course| (course.cd.as_str(), course.last_update.as_str()))
        .collect();
    for (code, reason) in &report.unavailable {
        let last_update = last_updates
            .get(code.as_str())
            .copied()
            .context("unavailable report contains an unknown course")?;
        state.entries.insert(
            code.clone(),
            UnavailableRecord {
                last_update: last_update.to_owned(),
                confirmed_at: now,
                expires_at,
                reason: reason.clone(),
            },
        );
    }
    state.entries.retain(|code, record| {
        !out_dir.join(format!("{code}.json")).exists() && record.expires_at > now
    });
    Ok(state)
}

/// Write one course's detail as compact JSON to `out_dir/{cd}.json`.
fn write_detail(out_dir: &std::path::Path, detail: &SanshoDetail) -> Result<()> {
    let path = out_dir.join(format!("{}.json", detail.cd));
    let bytes = serde_json::to_vec(detail).context("failed to serialize detail JSON")?;
    fs::write(&path, bytes)
        .with_context(|| format!("failed to write detail JSON {}", path.display()))
}

fn resolve_token_override(flag: Option<&str>) -> Option<String> {
    flag.filter(|t| !t.is_empty())
        .map(str::to_owned)
        .or_else(|| std::env::var(TOKEN_ENV).ok().filter(|v| !v.is_empty()))
}

// --- Orchestration (network-agnostic; unit-tested with a fake fetcher) ---

struct CrawlOpts {
    retries: u32,
    max_consecutive_blocks: u32,
    politeness: Politeness,
    /// Stop cleanly once elapsed exceeds this, so CI can commit partial progress
    /// before a job timeout (a later run resumes). `None` = no limit.
    max_run: Option<Duration>,
}

/// The outcome of a crawl.
struct CrawlReport {
    fetched: usize,
    skipped: Vec<(String, String)>,
    /// Rich, self-explaining context for the first few failures (status + response
    /// body), captured so a failure is diagnosable without an expensive re-run.
    diagnostics: Vec<(String, String)>,
    /// Courses the official service explicitly confirmed absent this run.
    unavailable: Vec<(String, String)>,
    /// A protocol, parse, or persistence failure. Such a failure aborts the
    /// complete crawl and can never be converted to unavailable.
    fatal_error: Option<(String, String)>,
    aborted: bool,
    elapsed: Duration,
}

/// How many failures to capture in full (body + context) before just counting.
const MAX_DIAGNOSTICS: usize = 8;

impl CrawlReport {
    fn print(&self) {
        ui::summary(self.fetched, &self.skipped, self.elapsed, self.aborted);
    }
}

/// Crawl `courses` sequentially through `fetcher`, persisting each via `sink`.
///
/// Retries transient/5xx errors with backoff; records only explicit official
/// absence; aborts immediately on protocol/parse/save failures; and aborts once
/// `max_consecutive_blocks` server refusals pile up in a row.
fn crawl(
    courses: &[CourseRef],
    fetcher: &impl DetailFetcher,
    opts: &CrawlOpts,
    sink: &mut dyn FnMut(&SanshoDetail) -> Result<()>,
) -> CrawlReport {
    let start = std::time::Instant::now();
    crawl_with_clock(courses, fetcher, opts, sink, || start.elapsed())
}

/// The crawl loop with an injectable `elapsed` clock (seconds since start), so
/// the time-budget boundary can be tested with virtual time.
fn crawl_with_clock(
    courses: &[CourseRef],
    fetcher: &impl DetailFetcher,
    opts: &CrawlOpts,
    sink: &mut dyn FnMut(&SanshoDetail) -> Result<()>,
    elapsed: impl Fn() -> Duration,
) -> CrawlReport {
    let mut fetched = 0usize;
    let mut skipped = Vec::new();
    let mut diagnostics: Vec<(String, String)> = Vec::new();
    let mut unavailable: Vec<(String, String)> = Vec::new();
    let mut consecutive_blocks = 0u32;
    // Emit a progress line every N courses so a long run is observable live in the
    // Actions log (step logs only finalize on completion otherwise).
    const PROGRESS_EVERY: usize = 25;

    for (i, course) in courses.iter().enumerate() {
        if opts.max_run.is_some_and(|max| elapsed() >= max) {
            eprintln!("fetch-details: time budget reached, stopping early ({fetched} fetched)");
            break;
        }
        if i > 0 {
            opts.politeness.wait();
        }
        match attempt(fetcher, course, opts.retries, &opts.politeness) {
            Ok(html) => {
                consecutive_blocks = 0;
                let mut detail = parse_sansho_html(&course.cd, &html);
                detail.last_update = course.last_update.clone();
                if !detail.has_public_content() {
                    let error = "parsed HTML contained no recognized syllabus content".to_owned();
                    diagnostics.push((course.cd.clone(), error.clone()));
                    skipped.push((course.cd.clone(), format!("fatal: {error}")));
                    return CrawlReport {
                        fetched,
                        skipped,
                        diagnostics,
                        unavailable,
                        fatal_error: Some((course.cd.clone(), error)),
                        aborted: true,
                        elapsed: elapsed(),
                    };
                }
                if let Err(error) = sink(&detail) {
                    let error = format!("failed to persist detail: {error:#}");
                    diagnostics.push((course.cd.clone(), error.clone()));
                    skipped.push((course.cd.clone(), format!("fatal: {error}")));
                    return CrawlReport {
                        fetched,
                        skipped,
                        diagnostics,
                        unavailable,
                        fatal_error: Some((course.cd.clone(), error)),
                        aborted: true,
                        elapsed: elapsed(),
                    };
                }
                fetched += 1;
            }
            Err(err) => {
                let blocking = err.is_blocking();
                // Capture rich context for the first few failures; spotlight the
                // very first so the cause is right there when the log is opened.
                if diagnostics.len() < MAX_DIAGNOSTICS {
                    if diagnostics.is_empty() {
                        ui::spotlight(&course.cd, &err.diagnostic());
                    }
                    diagnostics.push((course.cd.clone(), err.diagnostic()));
                }
                if let DetailError::Unavailable { reason } = &err {
                    unavailable.push((course.cd.clone(), reason.clone()));
                }
                if let DetailError::Fatal(error) = &err {
                    let error = format!("{error:#}");
                    skipped.push((course.cd.clone(), format!("fatal: {error}")));
                    return CrawlReport {
                        fetched,
                        skipped,
                        diagnostics,
                        unavailable,
                        fatal_error: Some((course.cd.clone(), error)),
                        aborted: true,
                        elapsed: elapsed(),
                    };
                }
                skipped.push((course.cd.clone(), err.to_string()));
                if blocking {
                    consecutive_blocks += 1;
                    if consecutive_blocks >= opts.max_consecutive_blocks {
                        return CrawlReport {
                            fetched,
                            skipped,
                            diagnostics,
                            unavailable,
                            fatal_error: None,
                            aborted: true,
                            elapsed: elapsed(),
                        };
                    }
                } else {
                    consecutive_blocks = 0;
                }
            }
        }

        let done = i + 1;
        if done % PROGRESS_EVERY == 0 || done == courses.len() {
            ui::progress(done, courses.len(), fetched, skipped.len(), elapsed());
        }
    }

    CrawlReport {
        fetched,
        skipped,
        diagnostics,
        unavailable,
        fatal_error: None,
        aborted: false,
        elapsed: elapsed(),
    }
}

/// Fetch one course, retrying retriable errors up to `retries` times with a
/// growing backoff.
fn attempt(
    fetcher: &impl DetailFetcher,
    course: &CourseRef,
    retries: u32,
    politeness: &Politeness,
) -> Result<String, DetailError> {
    let mut last = DetailError::Fatal(anyhow::anyhow!("no attempt"));
    for tryno in 0..=retries {
        match fetcher.fetch_html(course) {
            Ok(html) => return Ok(html),
            Err(e) if e.is_retriable() && tryno < retries => {
                // Honor the server's Retry-After when it asks for longer than our
                // own exponential backoff would wait.
                let wait =
                    backoff(politeness.base, tryno).max(e.retry_after().unwrap_or(Duration::ZERO));
                if !wait.is_zero() {
                    std::thread::sleep(wait);
                }
                last = e;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn course(cd: &str) -> CourseRef {
        CourseRef {
            cd: cd.into(),
            kaiko_nendo: "2026".into(),
            pattern_id: "4".into(),
            last_update: "t".into(),
        }
    }

    /// An `Http` error with no `Retry-After` (the common shape in these tests).
    fn http(status: u16) -> DetailError {
        DetailError::Http {
            status,
            retry_after: None,
            body: String::new(),
        }
    }

    fn opts(retries: u32, breaker: u32) -> CrawlOpts {
        CrawlOpts {
            retries,
            max_consecutive_blocks: breaker,
            politeness: Politeness {
                base: Duration::ZERO,
                jitter_ms: 0,
            },
            max_run: None,
        }
    }

    /// A fetcher returning canned per-call results in order.
    struct Fake {
        results: RefCell<Vec<Result<String, DetailError>>>,
        calls: RefCell<usize>,
    }
    impl Fake {
        fn new(results: Vec<Result<String, DetailError>>) -> Self {
            Self {
                results: RefCell::new(results),
                calls: RefCell::new(0),
            }
        }
    }
    impl DetailFetcher for Fake {
        fn fetch_html(&self, _c: &CourseRef) -> Result<String, DetailError> {
            *self.calls.borrow_mut() += 1;
            let mut r = self.results.borrow_mut();
            if r.is_empty() {
                Err(DetailError::Fatal(anyhow::anyhow!("exhausted")))
            } else {
                r.remove(0)
            }
        }
    }

    fn html_table() -> String {
        "<table class=\"tbl_status\"><tr><th>単位数</th><td>2.0</td></tr></table>".to_owned()
    }

    fn fixed_now() -> DateTime<Utc> {
        "2026-07-26T00:00:00Z".parse().unwrap()
    }

    fn no_unavailable() -> UnavailableState {
        UnavailableState {
            version: 1,
            entries: BTreeMap::new(),
        }
    }

    #[test]
    fn limit_and_incremental_advance_the_window() {
        let cds = |v: &[CourseRef]| v.iter().map(|c| c.cd.clone()).collect::<Vec<_>>();
        let all = || {
            vec![
                course("001"),
                course("002"),
                course("003"),
                course("004"),
                course("005"),
            ]
        };
        let none = no_unavailable();
        let now = fixed_now();
        // Day 1: nothing saved yet → the first `limit` courses are selected.
        let day1 = filter_courses(all(), None, &HashMap::new(), false, 3, &none, &now);
        assert_eq!(cds(&day1), ["001", "002", "003"]);

        // Day 2: day 1's fetches are now saved at the same lastUpdate → skipped,
        // so the window advances to the next batch (this is the daily amortization).
        let saved: HashMap<String, String> = day1
            .iter()
            .map(|c| (c.cd.clone(), c.last_update.clone()))
            .collect();
        let day2 = filter_courses(all(), None, &saved, false, 3, &none, &now);
        assert_eq!(cds(&day2), ["004", "005"]);

        // --force ignores saved state and re-selects from the top.
        let forced = filter_courses(all(), None, &saved, true, 3, &none, &now);
        assert_eq!(cds(&forced), ["001", "002", "003"]);

        // --only narrows to specific codes before the limit applies.
        let only = filter_courses(
            all(),
            Some("002,004"),
            &HashMap::new(),
            false,
            0,
            &none,
            &now,
        );
        assert_eq!(cds(&only), ["002", "004"]);
    }

    #[test]
    fn unavailable_courses_are_skipped_until_expired_forced_or_changed() {
        let cds = |v: &[CourseRef]| v.iter().map(|c| c.cd.clone()).collect::<Vec<_>>();
        let all = || vec![course("001"), course("002"), course("003")];
        let none = HashMap::new();
        let now = fixed_now();

        let mut unavailable = no_unavailable();
        unavailable.entries.insert(
            "001".to_owned(),
            UnavailableRecord {
                last_update: "t".into(),
                confirmed_at: now,
                expires_at: now.checked_add_days(Days::new(30)).unwrap(),
                reason: "該当するシラバスがありません".into(),
            },
        );
        let sel = filter_courses(all(), None, &none, false, 0, &unavailable, &now);
        assert_eq!(cds(&sel), ["002", "003"]);

        // Expiry re-opens the course.
        let expired = now.checked_add_days(Days::new(31)).unwrap();
        assert_eq!(
            cds(&filter_courses(
                all(),
                None,
                &none,
                false,
                0,
                &unavailable,
                &expired,
            )),
            ["001", "002", "003"]
        );

        // --force retries even an unexpired unavailable course.
        assert_eq!(
            cds(&filter_courses(
                all(),
                None,
                &none,
                true,
                0,
                &unavailable,
                &now,
            )),
            ["001", "002", "003"]
        );

        // A changed grid lastUpdate re-opens it.
        let changed = vec![CourseRef {
            cd: "001".into(),
            kaiko_nendo: "2026".into(),
            pattern_id: "4".into(),
            last_update: "u".into(),
        }];
        assert_eq!(
            cds(&filter_courses(
                changed,
                None,
                &none,
                false,
                0,
                &unavailable,
                &now,
            )),
            ["001"]
        );
    }

    #[test]
    fn publisher_sees_only_current_official_unavailable_records() {
        let directory = std::env::temp_dir().join(format!(
            "gyakubiki-confirmed-unavailable-{}",
            std::process::id()
        ));
        if directory.exists() {
            fs::remove_dir_all(&directory).unwrap();
        }
        fs::create_dir_all(&directory).unwrap();
        let now = fixed_now();
        let mut state = no_unavailable();
        state.entries.insert(
            "001".into(),
            UnavailableRecord {
                last_update: "current".into(),
                confirmed_at: now,
                expires_at: now.checked_add_days(Days::new(30)).unwrap(),
                reason: "公式応答で不存在".into(),
            },
        );
        save_unavailable(&directory, &state).unwrap();
        let courses = vec![
            RawCourse {
                kogi_cd: "001".into(),
                last_update: Some("current".into()),
                ..RawCourse::default()
            },
            RawCourse {
                kogi_cd: "002".into(),
                last_update: Some("changed".into()),
                ..RawCourse::default()
            },
        ];

        let confirmed = confirmed_unavailable_codes(&directory, &courses, now).unwrap();
        assert_eq!(confirmed, HashSet::from(["001".to_owned()]));
        let after_expiry =
            confirmed_unavailable_codes(&directory, &courses, now + Days::new(31)).unwrap();
        assert!(after_expiry.is_empty());
        fs::remove_dir_all(directory).unwrap();
    }

    fn report_with(
        fetched: usize,
        skipped: Vec<(String, String)>,
        diagnostics: Vec<(String, String)>,
        aborted: bool,
    ) -> CrawlReport {
        CrawlReport {
            fetched,
            skipped,
            diagnostics,
            unavailable: Vec::new(),
            fatal_error: None,
            aborted,
            elapsed: Duration::ZERO,
        }
    }

    #[test]
    fn diagnose_flags_a_changed_endpoint() {
        let report = report_with(
            0,
            vec![("001".into(), "HTTP 400".into())],
            vec![(
                "001".into(),
                "HTTP 400 — response body:\nservice method not found".into(),
            )],
            false,
        );
        let (headline, hint) = diagnose(&report);
        assert!(headline.contains("0 fetched"), "{headline}");
        assert!(hint.unwrap().contains("API path/method may have changed"));
    }

    #[test]
    fn diagnose_flags_rate_limiting() {
        let report = report_with(
            0,
            vec![("001".into(), "HTTP 429".into())],
            vec![("001".into(), "HTTP 429 (empty body)".into())],
            true,
        );
        assert!(diagnose(&report).1.unwrap().contains("rate-limiting"));
    }

    #[test]
    fn fetches_and_persists_all() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![Ok(html_table()), Ok(html_table())]);
        let saved = RefCell::new(Vec::new());
        let report = crawl(&courses, &fake, &opts(2, 5), &mut |d| {
            saved.borrow_mut().push(d.cd.clone());
            Ok(())
        });
        assert_eq!(report.fetched, 2);
        assert!(!report.aborted);
        assert_eq!(*saved.borrow(), ["001", "002"]);
    }

    #[test]
    fn retries_transient_then_succeeds() {
        let courses = vec![course("001")];
        let fake = Fake::new(vec![Err(http(503)), Ok(html_table())]);
        let report = crawl(&courses, &fake, &opts(2, 5), &mut |_| Ok(()));
        assert_eq!(report.fetched, 1);
        assert_eq!(*fake.calls.borrow(), 2);
    }

    #[test]
    fn fatal_protocol_error_aborts_the_whole_crawl() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![
            Err(DetailError::Fatal(anyhow::anyhow!("bad guid"))),
            Ok(html_table()),
        ]);
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |_| Ok(()));
        assert!(report.aborted);
        assert_eq!(report.fetched, 0);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(*fake.calls.borrow(), 1);
        assert!(report.fatal_error.is_some());
    }

    #[test]
    fn official_unavailable_is_recorded_but_does_not_abort() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![
            Err(DetailError::Unavailable {
                reason: "該当するシラバスがありません".into(),
            }),
            Ok(html_table()),
        ]);
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |_| Ok(()));
        assert!(!report.aborted);
        assert_eq!(report.fetched, 1);
        assert_eq!(
            report.unavailable,
            [("001".into(), "該当するシラバスがありません".into())]
        );
    }

    #[test]
    fn changed_empty_html_parse_aborts_instead_of_saving_empty_detail() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![Ok("<html><body>changed</body></html>".into())]);
        let saved = RefCell::new(Vec::new());
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |detail| {
            saved.borrow_mut().push(detail.cd.clone());
            Ok(())
        });
        assert!(report.aborted);
        assert!(report.fatal_error.is_some());
        assert!(saved.borrow().is_empty());
        assert_eq!(*fake.calls.borrow(), 1);
    }

    #[test]
    fn persistence_failure_aborts_instead_of_skipping() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![Ok(html_table()), Ok(html_table())]);
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |_| bail!("disk full"));
        assert!(report.aborted);
        assert!(report.fatal_error.is_some());
        assert_eq!(*fake.calls.borrow(), 1);
    }

    #[test]
    fn circuit_breaker_aborts_on_consecutive_blocks() {
        let courses = vec![course("001"), course("002"), course("003"), course("004")];
        // Every fetch is a 403 (no retries) → 2 consecutive blocks aborts.
        let fake = Fake::new(vec![
            Err(http(403)),
            Err(http(403)),
            Err(http(403)),
            Err(http(403)),
        ]);
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |_| Ok(()));
        assert!(report.aborted);
        assert_eq!(report.fetched, 0);
        // Stopped after the 2nd block, not all 4.
        assert_eq!(*fake.calls.borrow(), 2);
    }

    #[test]
    fn stops_early_when_time_budget_is_zero_duration() {
        // max_run = 0 → the very first iteration is already over budget: stop
        // cleanly (partial, not aborted) so CI commits and a later run resumes.
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![Ok(html_table()), Ok(html_table())]);
        let mut o = opts(0, 5);
        o.max_run = Some(Duration::ZERO);
        let report = crawl(&courses, &fake, &o, &mut |_| Ok(()));
        assert!(!report.aborted);
        assert_eq!(report.fetched, 0);
        assert_eq!(*fake.calls.borrow(), 0);
    }

    #[test]
    fn breaker_resets_after_a_success() {
        let courses = vec![course("001"), course("002"), course("003")];
        let fake = Fake::new(vec![Err(http(403)), Ok(html_table()), Err(http(403))]);
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |_| Ok(()));
        // block, reset by success, block → never 2 in a row → no abort.
        assert!(!report.aborted);
        assert_eq!(report.fetched, 1);
    }

    // Politeness delay + backoff are unit-tested in `crate::net`; here we cover
    // the crawler behaviours that use them (breaker, retry, bounded
    // official-unavailable state).

    #[test]
    fn breaker_of_one_aborts_on_the_first_block() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![Err(http(403)), Ok(html_table())]);
        let report = crawl(&courses, &fake, &opts(0, 1), &mut |_| Ok(()));
        assert!(report.aborted);
        assert_eq!(report.fetched, 0);
        assert_eq!(*fake.calls.borrow(), 1); // stopped immediately
    }

    #[test]
    fn exhausted_retriable_counts_as_a_single_block() {
        // A 503 retried to exhaustion is one course = one block increment, not one
        // per network call — so the breaker measures courses, not requests.
        let courses = vec![course("001")];
        let fake = Fake::new(vec![Err(http(503)), Err(http(503))]);
        let report = crawl(&courses, &fake, &opts(1, 5), &mut |_| Ok(()));
        assert_eq!(*fake.calls.borrow(), 2); // retries + 1
        assert_eq!(report.skipped.len(), 1);
        assert!(!report.aborted); // one block < breaker of 5
        assert_eq!(report.fetched, 0);
    }

    #[test]
    fn time_budget_stops_before_the_course_that_would_exceed_it() {
        use std::cell::Cell;
        // Virtual clock: elapsed is 0s at courses 1 & 2, then 10s at course 3.
        let elapsed_seq = [
            Duration::from_secs(0),
            Duration::from_secs(0),
            Duration::from_secs(10),
        ];
        let idx = Cell::new(0);
        let clock = || {
            let i = idx.get();
            idx.set(i + 1);
            elapsed_seq[i.min(elapsed_seq.len() - 1)]
        };

        let courses = vec![course("001"), course("002"), course("003")];
        let fake = Fake::new(vec![Ok(html_table()), Ok(html_table()), Ok(html_table())]);
        let mut o = opts(0, 5);
        o.max_run = Some(Duration::from_secs(5));
        let report = crawl_with_clock(&courses, &fake, &o, &mut |_| Ok(()), clock);

        // Two fetched before the budget was hit; a partial stop, not an abort.
        assert_eq!(report.fetched, 2);
        assert!(!report.aborted);
        assert_eq!(*fake.calls.borrow(), 2);
    }
}
