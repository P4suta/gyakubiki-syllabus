//! `fetch` subcommand.
//!
//! Downloads every findPage page from KULAS, validates the pagination, and
//! writes each page's raw JSON to `raw/` verbatim (no re-serialization). The
//! HTTP layer is behind [`PageFetcher`] so the orchestration is unit-testable
//! offline; the live client lives in [`client`].

mod client;
pub(crate) mod token;

pub(crate) use client::{browser_entry_context, build_http_client, USER_AGENT};

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Datelike;
use clap::Args;
use serde::Deserialize;

use client::Client;

const TOKEN_ENV: &str = "KULAS_API_TOKEN";

/// Abstracts the HTTP layer so [`fetch_all`] can run against a fake in tests.
pub trait PageFetcher {
    fn fetch_page(&self, page_no: i32) -> Result<Vec<u8>>;
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
            min_total: args.min_total,
            dry_run: args.dry_run,
        },
        &client,
    )?;
    report(&result, args.dry_run);
    Ok(())
}

struct Options {
    out_dir: PathBuf,
    min_total: i32,
    dry_run: bool,
}

/// Per-page metadata; only used for validation — the bytes written are the raw
/// response, never a re-serialization of this.
#[derive(Deserialize)]
struct PageMeta {
    #[serde(rename = "pageNo", default)]
    page_no: i32,
    #[serde(rename = "maxPageNo", default)]
    max_page_no: i32,
    #[serde(default)]
    total: i32,
    #[serde(rename = "pageSize", default)]
    page_size: i32,
    #[serde(rename = "selectKogiDtoList", default)]
    select_kogi_dto_list: Vec<serde_json::Value>,
}

impl PageMeta {
    fn list_len(&self) -> i32 {
        self.select_kogi_dto_list.len() as i32
    }
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
    let writes = write_pages(&opts.out_dir, &fetched)?;
    let cleaned = cleanup_stale_pages(&opts.out_dir, fetched.max_page_no)?;
    Ok(FetchResult::written(&fetched, &writes, cleaned))
}

/// Fetch page 1 and guard its totals, then fetch and validate pages 2..=max.
/// Touches no filesystem, so it is unit-testable with a fake fetcher.
fn fetch_and_validate(opts: &Options, fetcher: &impl PageFetcher) -> Result<FetchedPages> {
    let first_bytes = fetcher.fetch_page(1)?;
    let first = parse_meta(&first_bytes).context("cannot parse page 1 response as JSON")?;
    if first.max_page_no < 1 {
        bail!("page 1 has an invalid maxPageNo (= {})", first.max_page_no);
    }
    if first.total < opts.min_total {
        bail!(
            "page 1 total is below the threshold ({} < {}) — the API may be unhealthy",
            first.total,
            opts.min_total
        );
    }

    let mut pages = vec![RawPage {
        page_no: 1,
        list_len: first.list_len(),
        bytes: first_bytes,
    }];
    for page_no in 2..=first.max_page_no {
        eprintln!("fetching page {page_no} of {}", first.max_page_no);
        let bytes = fetcher.fetch_page(page_no)?;
        let meta = parse_meta(&bytes)
            .with_context(|| format!("cannot parse page {page_no} response as JSON"))?;
        validate_page(&meta, page_no, first.max_page_no)?;
        pages.push(RawPage {
            page_no,
            list_len: meta.list_len(),
            bytes,
        });
    }

    Ok(FetchedPages {
        total: first.total,
        max_page_no: first.max_page_no,
        pages,
    })
}

/// Validate one non-first page's metadata against what was requested: the page
/// number echoes back, `maxPageNo` is stable, and every page before the last is
/// full (`listLen == pageSize`).
fn validate_page(meta: &PageMeta, expected_page: i32, first_max: i32) -> Result<()> {
    if meta.page_no != expected_page {
        bail!(
            "requested page {expected_page} but got pageNo={}",
            meta.page_no
        );
    }
    if meta.max_page_no != first_max {
        bail!(
            "maxPageNo changed on page {expected_page} ({} → {})",
            first_max,
            meta.max_page_no
        );
    }
    let list_len = meta.list_len();
    if expected_page < first_max && list_len != meta.page_size {
        bail!(
            "middle page {expected_page} is short (listLen={list_len}, pageSize={})",
            meta.page_size
        );
    }
    Ok(())
}

/// Write each validated page to `out_dir` verbatim, reporting which differed
/// from what was already on disk.
fn write_pages(out_dir: &Path, fetched: &FetchedPages) -> Result<Vec<PageWrite>> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create output directory {}", out_dir.display()))?;
    fetched
        .pages
        .iter()
        .map(|p| {
            let path = out_dir.join(raw_file_name(p.page_no));
            let changed = file_changed(&path, &p.bytes);
            fs::write(&path, &p.bytes)
                .with_context(|| format!("failed to write file {}", path.display()))?;
            Ok(PageWrite {
                page_no: p.page_no,
                changed,
            })
        })
        .collect()
}

fn parse_meta(bytes: &[u8]) -> Result<PageMeta> {
    serde_json::from_slice(bytes).map_err(anyhow::Error::from)
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

/// Remove `講義データ-NN.json` files whose page number exceeds `max_page_no`
/// (left over from a longer previous run).
fn cleanup_stale_pages(out_dir: &Path, max_page_no: i32) -> Result<Vec<String>> {
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
            fs::remove_file(entry.path())?;
            cleaned.push(name);
        }
    }
    cleaned.sort();
    Ok(cleaned)
}

/// Academic year for now (UTC): Japan's year starts in April, so Jan–Mar belongs
/// to the previous year.
fn current_kaiko_nendo() -> i32 {
    let now = chrono::Utc::now();
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
        fn fetch_page(&self, page_no: i32) -> Result<Vec<u8>> {
            self.pages
                .get(&page_no)
                .cloned()
                .with_context(|| format!("fake: no canned response for page {page_no}"))
        }
    }

    /// A realistic-shaped findPage response with `list_len` courses.
    fn page(page_no: i32, max_page_no: i32, total: i32, list_len: i32) -> Vec<u8> {
        let courses: Vec<serde_json::Value> = (0..list_len)
            .map(|i| serde_json::json!({"kogiCd": format!("{:05}", page_no * 1000 + i), "kogiNm": "テスト講義"}))
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
            min_total: 100,
            dry_run,
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

    /// A `PageMeta` with `list_len` placeholder courses, for `validate_page`.
    fn meta(page_no: i32, max_page_no: i32, page_size: i32, list_len: i32) -> PageMeta {
        PageMeta {
            page_no,
            max_page_no,
            total: 0,
            page_size,
            select_kogi_dto_list: vec![serde_json::Value::Null; list_len as usize],
        }
    }

    #[test]
    fn validate_page_accepts_a_full_middle_page() {
        assert!(validate_page(&meta(2, 3, 500, 500), 2, 3).is_ok());
    }

    #[test]
    fn validate_page_accepts_a_short_last_page() {
        // The final page may be partial.
        assert!(validate_page(&meta(3, 3, 500, 200), 3, 3).is_ok());
    }

    #[test]
    fn validate_page_rejects_wrong_page_no() {
        assert!(validate_page(&meta(9, 3, 500, 500), 2, 3).is_err());
    }

    #[test]
    fn validate_page_rejects_changed_max_page_no() {
        assert!(validate_page(&meta(2, 5, 500, 500), 2, 3).is_err());
    }

    #[test]
    fn validate_page_rejects_short_middle_page() {
        assert!(validate_page(&meta(2, 3, 500, 300), 2, 3).is_err());
    }
}
