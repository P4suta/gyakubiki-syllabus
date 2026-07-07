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

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;

use crate::detail::{parse_sansho_html, SanshoDetail};
use crate::io;
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
    /// Stop cleanly after this many seconds (for CI partial commits; 0 = unlimited).
    #[arg(long = "max-secs", default_value_t = 0)]
    max_secs: u64,
    /// Override the KULAS token (for verification; normally empty to extract from HTML).
    #[arg(long)]
    token: Option<String>,
}

/// Run the detail crawl end to end.
pub fn run(args: FetchDetailsArgs) -> Result<()> {
    let all = course_refs(&load_dir(&args.raw_dir)?);

    fs::create_dir_all(&args.out_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            args.out_dir.display()
        )
    })?;

    let selected = select_courses(all, &args);
    eprintln!(
        "fetch-details: {} courses selected (out={})",
        selected.len(),
        args.out_dir.display()
    );
    if selected.is_empty() {
        eprintln!("nothing to fetch (all up to date, or 0 after filtering)");
        return Ok(());
    }

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
    if report.aborted {
        anyhow::bail!(
            "circuit breaker tripped: aborted after {} consecutive server refusals",
            args.max_consecutive_blocks
        );
    }
    Ok(())
}

/// Load and merge every `*.json` under `dir` (the raw findPage pages).
fn load_dir(dir: &std::path::Path) -> Result<Vec<syllabus_core::model::RawCourse>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("failed to read raw directory {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let loaded = io::load(&files)?;
    for warning in &loaded.warnings {
        eprintln!("{warning}");
    }
    Ok(loaded.courses)
}

/// De-duplicate raw courses by `kogiCd` (first wins) into fetch targets.
fn course_refs(raw: &[syllabus_core::model::RawCourse]) -> Vec<CourseRef> {
    let mut seen = std::collections::HashSet::new();
    raw.iter()
        .filter_map(|r| {
            let cd = r.kogi_cd.trim();
            if cd.is_empty() || !seen.insert(cd.to_owned()) {
                return None;
            }
            Some(CourseRef {
                cd: cd.to_owned(),
                kaiko_nendo: r.kaiko_nendo.clone().unwrap_or_default(),
                pattern_id: r
                    .syllabus_komoku_pattern_id
                    .clone()
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| "4".to_owned()),
                last_update: r.last_update.clone().unwrap_or_default(),
            })
        })
        .collect()
}

/// Apply `--only`, incremental (`lastUpdate` unchanged) skipping, and `--limit`.
fn select_courses(all: Vec<CourseRef>, args: &FetchDetailsArgs) -> Vec<CourseRef> {
    let existing = existing_last_updates(&args.out_dir);
    filter_courses(all, args.only.as_deref(), &existing, args.force, args.limit)
}

/// Pure selection: `--only` → incremental skip (drop courses whose `lastUpdate`
/// matches what we already saved) → `--limit`. Split from the filesystem read so
/// the day-to-day window advance (last run's fetches are skipped, so `--limit`
/// takes the next batch) is unit-testable.
fn filter_courses(
    all: Vec<CourseRef>,
    only: Option<&str>,
    existing: &HashMap<String, String>,
    force: bool,
    limit: usize,
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
                || existing
                    .get(&c.cd)
                    .is_none_or(|prev| prev != &c.last_update || c.last_update.is_empty())
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

/// Sleep policy between courses.
struct Politeness {
    base: Duration,
    jitter_ms: u64,
}

impl Politeness {
    /// The next delay: `base` plus `[0, jitter_ms)` of random jitter. Pure, so the
    /// bounds are testable without sleeping.
    fn delay(&self) -> Duration {
        let extra = if self.jitter_ms == 0 {
            0
        } else {
            rand::random_range(0..self.jitter_ms)
        };
        self.base + Duration::from_millis(extra)
    }

    fn wait(&self) {
        let d = self.delay();
        if !d.is_zero() {
            std::thread::sleep(d);
        }
    }
}

/// Backoff before a retry: `base * 2^tryno`, capped, so we back off *faster* when
/// a server is struggling instead of hammering it. Pure, so the schedule is
/// testable; saturating math keeps large `tryno` from overflowing.
fn backoff(base: Duration, tryno: u32) -> Duration {
    const CAP: Duration = Duration::from_secs(60);
    base.saturating_mul(2u32.saturating_pow(tryno)).min(CAP)
}

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
    aborted: bool,
}

impl CrawlReport {
    fn print(&self) {
        eprintln!(
            "fetch-details summary: fetched={} skipped={} aborted={}",
            self.fetched,
            self.skipped.len(),
            self.aborted
        );
        for (cd, why) in &self.skipped {
            eprintln!("  skip {cd}: {why}");
        }
    }
}

/// Crawl `courses` sequentially through `fetcher`, persisting each via `sink`.
///
/// Retries transient/5xx errors with backoff; skips a course on a fatal error;
/// and **aborts** once `max_consecutive_blocks` server refusals pile up in a row
/// — the block never escalates into hammering.
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
    let mut consecutive_blocks = 0u32;

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
                if let Err(e) = sink(&detail) {
                    skipped.push((course.cd.clone(), format!("save failed: {e}")));
                } else {
                    fetched += 1;
                }
            }
            Err(err) => {
                let blocking = err.is_blocking();
                skipped.push((course.cd.clone(), err.to_string()));
                if blocking {
                    consecutive_blocks += 1;
                    if consecutive_blocks >= opts.max_consecutive_blocks {
                        return CrawlReport {
                            fetched,
                            skipped,
                            aborted: true,
                        };
                    }
                } else {
                    consecutive_blocks = 0;
                }
            }
        }
    }

    CrawlReport {
        fetched,
        skipped,
        aborted: false,
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
        // Day 1: nothing saved yet → the first `limit` courses are selected.
        let day1 = filter_courses(all(), None, &HashMap::new(), false, 3);
        assert_eq!(cds(&day1), ["001", "002", "003"]);

        // Day 2: day 1's fetches are now saved at the same lastUpdate → skipped,
        // so the window advances to the next batch (this is the daily amortization).
        let saved: HashMap<String, String> = day1
            .iter()
            .map(|c| (c.cd.clone(), c.last_update.clone()))
            .collect();
        let day2 = filter_courses(all(), None, &saved, false, 3);
        assert_eq!(cds(&day2), ["004", "005"]);

        // --force ignores saved state and re-selects from the top.
        let forced = filter_courses(all(), None, &saved, true, 3);
        assert_eq!(cds(&forced), ["001", "002", "003"]);

        // --only narrows to specific codes before the limit applies.
        let only = filter_courses(all(), Some("002,004"), &HashMap::new(), false, 0);
        assert_eq!(cds(&only), ["002", "004"]);
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
    fn fatal_error_skips_course_without_tripping_breaker() {
        let courses = vec![course("001"), course("002")];
        let fake = Fake::new(vec![
            Err(DetailError::Fatal(anyhow::anyhow!("bad guid"))),
            Ok(html_table()),
        ]);
        let report = crawl(&courses, &fake, &opts(0, 2), &mut |_| Ok(()));
        assert!(!report.aborted);
        assert_eq!(report.fetched, 1);
        assert_eq!(report.skipped.len(), 1);
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

    // --- Deterministic timing / rate-limit invariants (no real sleeping) ---

    #[test]
    fn politeness_delay_respects_base_and_jitter_bounds() {
        // jitter=0 is fully deterministic; base=0 never sleeps.
        let fixed = Politeness {
            base: Duration::from_millis(2000),
            jitter_ms: 0,
        };
        assert_eq!(fixed.delay(), Duration::from_millis(2000));
        assert!(Politeness {
            base: Duration::ZERO,
            jitter_ms: 0,
        }
        .delay()
        .is_zero());

        // With jitter, every draw stays in [base, base + jitter_ms).
        let jittered = Politeness {
            base: Duration::from_millis(2000),
            jitter_ms: 1000,
        };
        for _ in 0..1000 {
            let d = jittered.delay();
            assert!(d >= Duration::from_millis(2000));
            assert!(d < Duration::from_millis(3000));
        }
    }

    #[test]
    fn backoff_grows_exponentially_and_caps() {
        let base = Duration::from_millis(3000);
        assert_eq!(backoff(base, 0), Duration::from_millis(3000));
        assert_eq!(backoff(base, 1), Duration::from_millis(6000));
        assert_eq!(backoff(base, 2), Duration::from_millis(12000));
        // 3s * 2^5 = 96s would exceed the 60s cap.
        assert_eq!(backoff(base, 5), Duration::from_secs(60));
        // Saturating math: a huge exponent caps rather than overflowing.
        assert_eq!(backoff(base, 99), Duration::from_secs(60));
        assert!(backoff(Duration::ZERO, 5).is_zero());
    }

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
