//! `fetch` subcommand.
//!
//! Downloads every findPage page from KULAS, validates the pagination, and
//! writes each page's raw JSON to `raw/` verbatim (no re-serialization). The
//! HTTP layer is behind [`PageFetcher`] so the orchestration is unit-testable
//! offline; the live client lives in the private `client` module.

mod client;
pub(crate) mod token;

pub(crate) use client::{USER_AGENT, browser_entry_context, build_http_client};

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail, ensure};
use chrono::{DateTime, Datelike, FixedOffset, Utc};
use clap::Args;
use syllabus_core::CourseCode;

use crate::io;
use crate::net::{FetchError, Politeness, backoff};
use client::Client;

const TOKEN_ENV: &str = "KULAS_API_TOKEN";

/// Abstracts the HTTP layer so the fetch orchestration can run against a fake in tests.
/// Returns a classified [`FetchError`] so the orchestration can back off on
/// 429/5xx instead of bailing on the first hiccup (symmetric with the detail
/// crawler).
pub trait PageFetcher {
    fn fetch_page(&self, page_no: i32) -> Result<Vec<u8>, FetchError>;
}

#[derive(Args)]
pub struct FetchArgs {
    /// Output directory for raw JSON.
    #[arg(long = "out-dir", default_value = "raw")]
    out_dir: PathBuf,
    /// kaikoNendo (auto-computed from the current academic year when empty).
    #[arg(long)]
    year: Option<String>,
    /// Override the KULAS API token (extract from HTML when empty; KULAS_API_TOKEN env also works).
    #[arg(long)]
    token: Option<String>,
    /// Minimum guard for page 1's total count (fails below this).
    #[arg(long = "min-total", default_value_t = 1500)]
    min_total: i32,
    /// Polite base sleep between pages (ms). The grid is light (~8 pages), so this
    /// stays small — it just avoids a tight back-to-back burst.
    #[arg(long = "sleep-ms", default_value_t = 500)]
    sleep_ms: u64,
    /// Upper bound of random jitter added to the sleep (ms).
    #[arg(long = "jitter-ms", default_value_t = 500)]
    jitter_ms: u64,
    /// Bounded retries per page on a transient/5xx/429 error (with backoff).
    #[arg(long = "retries", default_value_t = 3)]
    retries: u32,
    /// Fetch and report counts only, without writing files.
    #[arg(long = "dry-run")]
    dry_run: bool,
}

/// Run the fetch. Establishes a session, downloads all pages, writes `raw/`.
pub fn run(args: FetchArgs) -> Result<()> {
    let kaiko_nendo = args
        .year
        .clone()
        .unwrap_or_else(|| current_kaiko_nendo().to_string());
    let token_override = resolve_token_override(args.token.as_deref());

    eprintln!(
        "fetch start: kaikoNendo={kaiko_nendo} out_dir={} min_total={} dry_run={}",
        args.out_dir.display(),
        args.min_total,
        args.dry_run,
    );

    let client = Client::new(&kaiko_nendo, token_override.as_deref())
        .context("failed to initialize client (TLS, session, or token)")?;

    let result = fetch_all(
        &Options {
            out_dir: args.out_dir,
            expected_year: kaiko_nendo,
            min_total: args.min_total,
            dry_run: args.dry_run,
            politeness: Politeness::from_ms(args.sleep_ms, args.jitter_ms),
            retries: args.retries,
        },
        &client,
    )?;
    report(&result, args.dry_run);
    Ok(())
}

struct Options {
    out_dir: PathBuf,
    expected_year: String,
    min_total: i32,
    dry_run: bool,
    /// Sleep between pages (jittered), so the burst never looks like an attack.
    politeness: Politeness,
    /// Bounded per-page retries on a retriable error.
    retries: u32,
}

/// One page in the final report.
struct PageResult {
    page_no: i32,
    list_len: i32,
    file_name: String,
    changed: bool,
}

struct FetchResult {
    total: i32,
    max_page_no: i32,
    pages: Vec<PageResult>,
    cleaned: Vec<String>,
}

impl FetchResult {
    /// Report a dry run: pages fetched but never written, so nothing changed.
    fn report_only(fetched: &FetchedPages) -> Self {
        let pages = fetched.pages.iter().map(|p| p.report(false)).collect();
        Self {
            total: fetched.total,
            max_page_no: fetched.max_page_no,
            pages,
            cleaned: Vec::new(),
        }
    }

    /// Report a real run, folding in each page's on-disk changed flag.
    fn written(fetched: &FetchedPages, writes: &[PageWrite], cleaned: Vec<String>) -> Self {
        let pages = fetched
            .pages
            .iter()
            .map(|p| p.report(writes.iter().any(|w| w.page_no == p.page_no && w.changed)))
            .collect();
        Self {
            total: fetched.total,
            max_page_no: fetched.max_page_no,
            pages,
            cleaned,
        }
    }
}

/// A fetched, validated page held in memory before any disk write.
struct RawPage {
    page_no: i32,
    list_len: i32,
    bytes: Vec<u8>,
}

impl RawPage {
    fn report(&self, changed: bool) -> PageResult {
        PageResult {
            page_no: self.page_no,
            list_len: self.list_len,
            file_name: raw_file_name(self.page_no),
            changed,
        }
    }
}

/// Every page fetched and validated, before writing — the filesystem-free
/// product of [`fetch_and_validate`].
struct FetchedPages {
    total: i32,
    max_page_no: i32,
    pages: Vec<RawPage>,
}

/// The outcome of writing one page: whether its bytes differed from disk.
struct PageWrite {
    page_no: i32,
    changed: bool,
}

/// Download every page, validate the pagination, and write raw JSON. Split into
/// fetch/validate, write, and cleanup so each step is testable on its own.
fn fetch_all(opts: &Options, fetcher: &impl PageFetcher) -> Result<FetchResult> {
    let fetched = fetch_and_validate(opts, fetcher)?;
    if opts.dry_run {
        eprintln!(
            "dry-run: skipping write (total={}, pages={})",
            fetched.total,
            fetched.pages.len()
        );
        return Ok(FetchResult::report_only(&fetched));
    }
    let (writes, cleaned) = write_pages_atomically(&opts.out_dir, &fetched)?;
    Ok(FetchResult::written(&fetched, &writes, cleaned))
}

/// Fetch page 1 and guard its totals, then fetch and validate pages 2..=max.
/// Touches no filesystem, so it is unit-testable with a fake fetcher.
fn fetch_and_validate(opts: &Options, fetcher: &impl PageFetcher) -> Result<FetchedPages> {
    let first_bytes = fetch_page_resilient(fetcher, 1, opts)?;
    let first = io::parse_page(&first_bytes).context("cannot parse page 1 response as JSON")?;
    validate_first_page(&first, opts)?;
    if first.total < opts.min_total {
        bail!(
            "page 1 total is below the threshold ({} < {}) — the API may be unhealthy",
            first.total,
            opts.min_total
        );
    }

    let mut seen_codes = HashSet::with_capacity(first.total as usize);
    validate_courses(&first, &opts.expected_year, &mut seen_codes)?;
    let first_list_len = first.courses.len() as i32;
    let mut pages = vec![RawPage {
        page_no: 1,
        list_len: first_list_len,
        bytes: first_bytes,
    }];
    for page_no in 2..=first.max_page_no {
        opts.politeness.wait(); // stay polite between pages
        eprintln!("fetching page {page_no} of {}", first.max_page_no);
        let bytes = fetch_page_resilient(fetcher, page_no, opts)?;
        let page = io::parse_page(&bytes)
            .with_context(|| format!("cannot parse page {page_no} response as JSON"))?;
        validate_page(&page, page_no, &first)?;
        validate_courses(&page, &opts.expected_year, &mut seen_codes)?;
        pages.push(RawPage {
            page_no,
            list_len: page.courses.len() as i32,
            bytes,
        });
    }
    let fetched_total: i32 = pages.iter().map(|page| page.list_len).sum();
    ensure!(
        fetched_total == first.total,
        "sum of page item counts ({fetched_total}) does not equal total ({})",
        first.total
    );
    ensure!(
        seen_codes.len() == first.total as usize,
        "unique course count ({}) does not equal total ({})",
        seen_codes.len(),
        first.total
    );

    Ok(FetchedPages {
        total: first.total,
        max_page_no: first.max_page_no,
        pages,
    })
}

/// Fetch one page, retrying retriable errors (transient / 429 / 5xx) up to
/// `opts.retries` times with an exponential backoff that also honors the
/// server's `Retry-After`. A non-retriable error, or the last retry, surfaces as
/// a diagnostic-carrying `anyhow` error (the whole fetch is all-or-nothing).
fn fetch_page_resilient(
    fetcher: &impl PageFetcher,
    page_no: i32,
    opts: &Options,
) -> Result<Vec<u8>> {
    let mut last: Option<FetchError> = None;
    for tryno in 0..=opts.retries {
        match fetcher.fetch_page(page_no) {
            Ok(bytes) => return Ok(bytes),
            Err(e) if e.is_retriable() && tryno < opts.retries => {
                let wait = backoff(opts.politeness.base, tryno)
                    .max(e.retry_after().unwrap_or(Duration::ZERO));
                eprintln!(
                    "page {page_no}: {e} — retry {}/{} in {:.1}s",
                    tryno + 1,
                    opts.retries,
                    wait.as_secs_f32()
                );
                if !wait.is_zero() {
                    std::thread::sleep(wait);
                }
                last = Some(e);
            }
            Err(e) => return Err(anyhow!("page {page_no}: {}", e.diagnostic())),
        }
    }
    Err(anyhow!(
        "page {page_no} still failing after {} retries: {}",
        opts.retries,
        last.map_or_else(|| "unknown".to_owned(), |e| e.diagnostic())
    ))
}

/// Validate one non-first page's metadata against what was requested: the page
/// number echoes back, `maxPageNo` is stable, and every page before the last is
/// full (`listLen == pageSize`).
fn validate_first_page(page: &io::PageEnvelope, opts: &Options) -> Result<()> {
    ensure!(
        page.page_no == 1,
        "requested page 1 but got pageNo={}",
        page.page_no
    );
    ensure!(
        page.max_page_no >= 1,
        "page 1 has invalid maxPageNo={}",
        page.max_page_no
    );
    ensure!(
        page.page_size > 0,
        "page 1 has invalid pageSize={}",
        page.page_size
    );
    ensure!(page.total >= 0, "page 1 has invalid total={}", page.total);
    ensure!(
        page.courses.len() <= page.page_size as usize,
        "page 1 item count exceeds pageSize"
    );
    let expected_max = if page.total == 0 {
        1
    } else {
        (page.total + page.page_size - 1) / page.page_size
    };
    ensure!(
        page.max_page_no == expected_max,
        "maxPageNo={} is inconsistent with total={} and pageSize={}",
        page.max_page_no,
        page.total,
        page.page_size
    );
    validate_page_length(page, 1, page.max_page_no, page.total, page.page_size)?;
    ensure!(
        opts.expected_year.len() == 4
            && opts
                .expected_year
                .chars()
                .all(|value| value.is_ascii_digit()),
        "requested academic year is invalid"
    );
    Ok(())
}

fn validate_page(
    page: &io::PageEnvelope,
    expected_page: i32,
    first: &io::PageEnvelope,
) -> Result<()> {
    if page.page_no != expected_page {
        bail!(
            "requested page {expected_page} but got pageNo={}",
            page.page_no
        );
    }
    if page.max_page_no != first.max_page_no {
        bail!(
            "maxPageNo changed on page {expected_page} ({} → {})",
            first.max_page_no,
            page.max_page_no
        );
    }
    ensure!(
        page.page_size == first.page_size,
        "pageSize changed on page {expected_page} ({} → {})",
        first.page_size,
        page.page_size
    );
    ensure!(
        page.total == first.total,
        "total changed on page {expected_page} ({} → {})",
        first.total,
        page.total
    );
    validate_page_length(
        page,
        expected_page,
        first.max_page_no,
        first.total,
        first.page_size,
    )
}

fn validate_page_length(
    page: &io::PageEnvelope,
    page_no: i32,
    max_page_no: i32,
    total: i32,
    page_size: i32,
) -> Result<()> {
    let expected = if page_no < max_page_no {
        page_size
    } else {
        total - page_size * (max_page_no - 1)
    };
    ensure!(
        page.courses.len() as i32 == expected,
        "page {page_no} has {} items; expected {expected}",
        page.courses.len()
    );
    Ok(())
}

fn validate_courses(
    page: &io::PageEnvelope,
    expected_year: &str,
    seen_codes: &mut HashSet<String>,
) -> Result<()> {
    for (position, course) in page.courses.iter().enumerate() {
        let code = CourseCode::parse(&course.kogi_cd).with_context(|| {
            format!(
                "page {} item {} has an invalid course code",
                page.page_no,
                position + 1
            )
        })?;
        ensure!(
            seen_codes.insert(code.as_str().to_owned()),
            "duplicate course code {:?} across fetched pages",
            code.as_str()
        );
        let year = course.kaiko_nendo.as_deref().unwrap_or("").trim();
        ensure!(
            year == expected_year,
            "course {:?} belongs to academic year {:?}, expected {:?}",
            code.as_str(),
            year,
            expected_year
        );
    }
    Ok(())
}

/// Write every validated page to a sibling staging directory, then swap the
/// complete directory into place. A write/promote failure restores the previous
/// directory, so readers can observe an old or new crawl but never a mixture.
fn write_pages_atomically(
    out_dir: &Path,
    fetched: &FetchedPages,
) -> Result<(Vec<PageWrite>, Vec<String>)> {
    let parent = out_dir.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = out_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("raw");
    let stage = parent.join(format!(".{name}.stage-{}", std::process::id()));
    let backup = parent.join(format!(".{name}.backup-{}", std::process::id()));
    ensure!(!stage.exists(), "fetch staging directory already exists");
    ensure!(!backup.exists(), "fetch backup directory already exists");
    fs::create_dir(&stage)?;

    let result = (|| -> Result<(Vec<PageWrite>, Vec<String>)> {
        let writes = fetched
            .pages
            .iter()
            .map(|page| {
                let file_name = raw_file_name(page.page_no);
                let changed = file_changed(&out_dir.join(&file_name), &page.bytes);
                fs::write(stage.join(&file_name), &page.bytes)
                    .with_context(|| format!("failed to stage {file_name}"))?;
                Ok(PageWrite {
                    page_no: page.page_no,
                    changed,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if std::env::var("SYLLABUS_FETCH_FAIL_AT").ok().as_deref() == Some("stage") {
            bail!("injected fetch failure after staging");
        }
        let cleaned = stale_page_names(out_dir, fetched.max_page_no)?;
        let had_previous = out_dir.exists();
        if had_previous {
            fs::rename(out_dir, &backup).context("failed to move previous raw directory aside")?;
        }
        if let Err(error) = fs::rename(&stage, out_dir) {
            if had_previous {
                let _ = fs::rename(&backup, out_dir);
            }
            return Err(error).context("failed to promote complete raw directory");
        }
        if backup.exists()
            && let Err(error) = fs::remove_dir_all(&backup)
        {
            eprintln!(
                "warning: promoted raw data but could not remove backup {}: {error}",
                backup.display()
            );
        }
        Ok((writes, cleaned))
    })();

    if stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}

/// On-disk file name for a page: page 1 is unsuffixed; pages 2+ get a
/// zero-padded suffix.
fn raw_file_name(page_no: i32) -> String {
    if page_no == 1 {
        "講義データ.json".to_owned()
    } else {
        format!("講義データ-{page_no:02}.json")
    }
}

fn file_changed(path: &Path, new_content: &[u8]) -> bool {
    match fs::read(path) {
        Ok(existing) => existing != new_content,
        Err(_) => true,
    }
}

/// Report page files that the directory swap will remove.
fn stale_page_names(out_dir: &Path, max_page_no: i32) -> Result<Vec<String>> {
    if !out_dir.exists() {
        return Ok(Vec::new());
    }
    let pattern = regex::Regex::new(r"^講義データ-(\d{2})\.json$").expect("valid regex");
    let mut cleaned = Vec::new();
    for entry in fs::read_dir(out_dir).context("failed to read output directory")? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(caps) = pattern.captures(&name) else {
            continue;
        };
        let page_no: i32 = caps[1].parse().unwrap_or(0);
        if page_no > max_page_no {
            cleaned.push(name);
        }
    }
    cleaned.sort();
    Ok(cleaned)
}

/// Academic year for now in Asia/Tokyo. Japan's academic year starts in April,
/// so Jan–Mar belongs to the previous calendar year.
fn current_kaiko_nendo() -> i32 {
    let jst = FixedOffset::east_opt(9 * 60 * 60).expect("valid JST offset");
    academic_year_at(Utc::now().with_timezone(&jst))
}

fn academic_year_at(now: DateTime<FixedOffset>) -> i32 {
    if now.month() < 4 {
        now.year() - 1
    } else {
        now.year()
    }
}

/// `--token` wins, then `KULAS_API_TOKEN`, else `None` (use HTML extraction).
fn resolve_token_override(flag: Option<&str>) -> Option<String> {
    if let Some(token) = flag.filter(|t| !t.is_empty()) {
        return Some(token.to_owned());
    }
    std::env::var(TOKEN_ENV).ok().filter(|v| !v.is_empty())
}

fn report(result: &FetchResult, dry_run: bool) {
    let changed = result
        .pages
        .iter()
        .filter(|p| !dry_run && p.changed)
        .count();
    eprintln!(
        "fetch summary: total={} max_page={} pages={} dry_run={dry_run}",
        result.total,
        result.max_page_no,
        result.pages.len(),
    );
    for p in &result.pages {
        eprintln!(
            "  page {} items={} file={} changed={}",
            p.page_no,
            p.list_len,
            p.file_name,
            !dry_run && p.changed
        );
    }
    if !result.cleaned.is_empty() {
        eprintln!("stale files removed: {:?}", result.cleaned);
    }
    if !dry_run {
        eprintln!("files changed: {changed}");
    }
    write_step_summary(result, dry_run, changed);
}

/// Append a markdown summary to `$GITHUB_STEP_SUMMARY` when set (Actions only).
fn write_step_summary(result: &FetchResult, dry_run: bool, changed: usize) {
    let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") else {
        return;
    };
    let mode = if dry_run { "dry-run" } else { "normal run" };
    let mut md = format!(
        "## Fetch syllabus result ({mode})\n\n- Fetched: **{}** across {} pages\n",
        result.total, result.max_page_no
    );
    if !dry_run {
        md.push_str(&format!("- Changed files: {changed}\n"));
    }
    md.push_str("\n| page | items | file | changed |\n|---|---|---|---|\n");
    for p in &result.pages {
        let mark = if !dry_run && p.changed { "✓" } else { "—" };
        md.push_str(&format!(
            "| {} | {} | `{}` | {} |\n",
            p.page_no, p.list_len, p.file_name, mark
        ));
    }
    if !result.cleaned.is_empty() {
        md.push_str(&format!("\nStale files removed: {:?}\n", result.cleaned));
    }
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().append(true).create(true).open(path) {
        let _ = f.write_all(md.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    //! Orchestration tests: no sockets — a fake fetcher returns canned page bytes.
    use super::*;
    use std::collections::HashMap;

    struct FakeFetcher {
        pages: HashMap<i32, Vec<u8>>,
    }

    impl PageFetcher for FakeFetcher {
        fn fetch_page(&self, page_no: i32) -> Result<Vec<u8>, FetchError> {
            self.pages.get(&page_no).cloned().ok_or_else(|| {
                FetchError::Fatal(anyhow!("fake: no canned response for page {page_no}"))
            })
        }
    }

    /// A realistic-shaped findPage response with `list_len` courses.
    fn page(page_no: i32, max_page_no: i32, total: i32, list_len: i32) -> Vec<u8> {
        let courses: Vec<serde_json::Value> = (0..list_len)
            .map(|i| {
                serde_json::json!({
                    "kogiCd": format!("{:05}", page_no * 1000 + i),
                    "kogiNm": "テスト講義",
                    "kaikoNendo": "2026"
                })
            })
            .collect();
        serde_json::to_vec(&serde_json::json!({
            "pageNo": page_no,
            "maxPageNo": max_page_no,
            "total": total,
            "pageSize": 500,
            "selectKogiDtoList": courses,
        }))
        .unwrap()
    }

    fn fake(pages: Vec<(i32, Vec<u8>)>) -> FakeFetcher {
        FakeFetcher {
            pages: pages.into_iter().collect(),
        }
    }

    fn opts(dir: &Path, dry_run: bool) -> Options {
        Options {
            out_dir: dir.to_path_buf(),
            expected_year: "2026".to_owned(),
            min_total: 100,
            dry_run,
            politeness: Politeness::from_ms(0, 0), // never sleep in tests
            retries: 0,
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("syllabus-fetch-test-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn happy_path_writes_all_pages_verbatim() {
        let dir = temp_dir("happy");
        let p1 = page(1, 3, 1200, 500);
        let fetcher = fake(vec![
            (1, p1.clone()),
            (2, page(2, 3, 1200, 500)),
            (3, page(3, 3, 1200, 200)),
        ]);
        let result = fetch_all(&opts(&dir, false), &fetcher).unwrap();
        assert_eq!(
            (result.total, result.max_page_no, result.pages.len()),
            (1200, 3, 3)
        );
        // Page 1 is written byte-for-byte as received.
        assert_eq!(fs::read(dir.join("講義データ.json")).unwrap(), p1);
        assert!(dir.join("講義データ-02.json").exists());
        assert!(dir.join("講義データ-03.json").exists());
    }

    #[test]
    fn errors_below_min_total() {
        let dir = temp_dir("min");
        let fetcher = fake(vec![(1, page(1, 1, 50, 50))]);
        assert!(fetch_all(&opts(&dir, false), &fetcher).is_err());
    }

    #[test]
    fn errors_on_partial_mid_page() {
        let dir = temp_dir("midpage");
        let fetcher = fake(vec![(1, page(1, 3, 1300, 500)), (2, page(2, 3, 1300, 300))]);
        assert!(fetch_all(&opts(&dir, false), &fetcher).is_err());
    }

    #[test]
    fn errors_on_page_no_mismatch() {
        let dir = temp_dir("mismatch");
        // page 2 slot returns a body that claims pageNo=9.
        let fetcher = fake(vec![(1, page(1, 2, 1000, 500)), (2, page(9, 2, 1000, 500))]);
        assert!(fetch_all(&opts(&dir, false), &fetcher).is_err());
    }

    #[test]
    fn dry_run_writes_nothing() {
        let dir = temp_dir("dry");
        let fetcher = fake(vec![(1, page(1, 1, 300, 300))]);
        fetch_all(&opts(&dir, true), &fetcher).unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[test]
    fn cleans_up_stale_pages() {
        let dir = temp_dir("cleanup");
        for stale in ["講義データ-04.json", "講義データ-05.json"] {
            fs::write(dir.join(stale), b"stale").unwrap();
        }
        let fetcher = fake(vec![
            (1, page(1, 3, 1200, 500)),
            (2, page(2, 3, 1200, 500)),
            (3, page(3, 3, 1200, 200)),
        ]);
        let result = fetch_all(&opts(&dir, false), &fetcher).unwrap();
        assert_eq!(result.cleaned, ["講義データ-04.json", "講義データ-05.json"]);
        assert!(!dir.join("講義データ-04.json").exists());
        assert!(!dir.join("講義データ-05.json").exists());
    }

    #[test]
    fn raw_file_name_padding() {
        assert_eq!(raw_file_name(1), "講義データ.json");
        assert_eq!(raw_file_name(2), "講義データ-02.json");
        assert_eq!(raw_file_name(8), "講義データ-08.json");
        assert_eq!(raw_file_name(10), "講義データ-10.json");
    }

    /// A typed page with `list_len` placeholder courses, for validation tests.
    fn meta(
        page_no: i32,
        max_page_no: i32,
        total: i32,
        page_size: i32,
        list_len: i32,
    ) -> io::PageEnvelope {
        io::PageEnvelope {
            page_no,
            max_page_no,
            total,
            page_size,
            courses: (0..list_len)
                .map(|index| syllabus_core::RawCourse {
                    kogi_cd: format!("T{page_no}-{index}"),
                    kaiko_nendo: Some("2026".into()),
                    ..Default::default()
                })
                .collect(),
        }
    }

    #[test]
    fn validate_page_accepts_a_full_middle_page() {
        let first = meta(1, 3, 1200, 500, 500);
        assert!(validate_page(&meta(2, 3, 1200, 500, 500), 2, &first).is_ok());
    }

    #[test]
    fn validate_page_accepts_a_short_last_page() {
        let first = meta(1, 3, 1200, 500, 500);
        assert!(validate_page(&meta(3, 3, 1200, 500, 200), 3, &first).is_ok());
    }

    #[test]
    fn validate_page_rejects_wrong_page_no() {
        let first = meta(1, 3, 1200, 500, 500);
        assert!(validate_page(&meta(9, 3, 1200, 500, 500), 2, &first).is_err());
    }

    #[test]
    fn validate_page_rejects_changed_max_page_no() {
        let first = meta(1, 3, 1200, 500, 500);
        assert!(validate_page(&meta(2, 5, 1200, 500, 500), 2, &first).is_err());
    }

    #[test]
    fn validate_page_rejects_short_middle_page() {
        let first = meta(1, 3, 1200, 500, 500);
        assert!(validate_page(&meta(2, 3, 1200, 500, 300), 2, &first).is_err());
    }

    #[test]
    fn validate_page_rejects_changed_total_and_page_size() {
        let first = meta(1, 3, 1200, 500, 500);
        assert!(validate_page(&meta(2, 3, 1199, 500, 500), 2, &first).is_err());
        assert!(validate_page(&meta(2, 3, 1200, 400, 400), 2, &first).is_err());
    }

    #[test]
    fn academic_year_switches_at_april_first_in_tokyo() {
        use chrono::TimeZone;
        let jst = chrono::FixedOffset::east_opt(9 * 60 * 60).unwrap();
        assert_eq!(
            academic_year_at(jst.with_ymd_and_hms(2026, 3, 31, 23, 59, 59).unwrap()),
            2025
        );
        assert_eq!(
            academic_year_at(jst.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap()),
            2026
        );
    }
}
