//! `fetch` subcommand — port of Go's `internal/fetch` + `cmd/syllabus-cli/fetch.go`.
//!
//! Downloads every findPage page from KULAS, validates the pagination, and
//! writes each page's raw JSON to `raw/` verbatim (no re-serialization). The
//! HTTP layer is behind [`PageFetcher`] so the orchestration is unit-testable
//! offline; the live client lives in [`client`].

mod client;
mod token;

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
    /// raw JSON の出力ディレクトリ。
    #[arg(long = "out-dir", default_value = "raw")]
    out_dir: PathBuf,
    /// kaikoNendo (空なら現在年度を自動計算)。
    #[arg(long)]
    year: Option<String>,
    /// KULAS API token を上書き (空なら HTML 抽出、KULAS_API_TOKEN env でも可)。
    #[arg(long)]
    token: Option<String>,
    /// page 1 の total 件数の最小ガード (これを下回ると fail)。
    #[arg(long = "min-total", default_value_t = 1500)]
    min_total: i32,
    /// 取得して件数のみ報告、ファイル書き込みなし。
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
        .context("client 初期化に失敗 (TLS/session/token のいずれか)")?;

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

/// Download every page, validate counts, and write raw JSON — port of Go's `All`.
fn fetch_all(opts: &Options, fetcher: &impl PageFetcher) -> Result<FetchResult> {
    let first_bytes = fetcher.fetch_page(1)?;
    let first =
        parse_meta(&first_bytes).context("page 1 のレスポンスを JSON として解析できません")?;
    if first.max_page_no < 1 {
        bail!("page 1 の maxPageNo が無効です (= {})", first.max_page_no);
    }
    if first.total < opts.min_total {
        bail!(
            "page 1 の total が閾値を下回っています ({} < {}) — API 不調の可能性",
            first.total,
            opts.min_total
        );
    }

    let mut pages: Vec<(i32, Vec<u8>)> = vec![(1, first_bytes)];
    let mut result = FetchResult {
        total: first.total,
        max_page_no: first.max_page_no,
        pages: vec![PageResult {
            page_no: 1,
            list_len: first.list_len(),
            file_name: raw_file_name(1),
            changed: false,
        }],
        cleaned: Vec::new(),
    };

    for page_no in 2..=first.max_page_no {
        eprintln!("fetching page {page_no} of {}", first.max_page_no);
        let bytes = fetcher.fetch_page(page_no)?;
        let meta = parse_meta(&bytes)
            .with_context(|| format!("page {page_no} のレスポンスを JSON として解析できません"))?;
        if meta.page_no != page_no {
            bail!(
                "page {page_no} を要求したが pageNo={} が返ってきました",
                meta.page_no
            );
        }
        if meta.max_page_no != first.max_page_no {
            bail!(
                "page {page_no} で maxPageNo が変化 ({} → {})",
                first.max_page_no,
                meta.max_page_no
            );
        }
        let list_len = meta.list_len();
        if page_no < first.max_page_no && list_len != meta.page_size {
            bail!(
                "中間 page {page_no} の件数が不足 (listLen={list_len}, pageSize={})",
                meta.page_size
            );
        }
        pages.push((page_no, bytes));
        result.pages.push(PageResult {
            page_no,
            list_len,
            file_name: raw_file_name(page_no),
            changed: false,
        });
    }

    if opts.dry_run {
        eprintln!(
            "dry-run: skipping write (total={}, pages={})",
            result.total,
            result.pages.len()
        );
        result.pages.sort_by_key(|p| p.page_no);
        return Ok(result);
    }

    fs::create_dir_all(&opts.out_dir)
        .with_context(|| format!("出力ディレクトリの作成に失敗 {}", opts.out_dir.display()))?;
    for (page_no, bytes) in &pages {
        let path = opts.out_dir.join(raw_file_name(*page_no));
        let changed = file_changed(&path, bytes);
        fs::write(&path, bytes)
            .with_context(|| format!("ファイル書き込みに失敗 {}", path.display()))?;
        if let Some(p) = result.pages.iter_mut().find(|p| p.page_no == *page_no) {
            p.changed = changed;
        }
    }
    result.pages.sort_by_key(|p| p.page_no);
    result.cleaned = cleanup_stale_pages(&opts.out_dir, first.max_page_no)?;
    Ok(result)
}

fn parse_meta(bytes: &[u8]) -> Result<PageMeta> {
    serde_json::from_slice(bytes).map_err(anyhow::Error::from)
}

/// On-disk file name for a page: page 1 keeps the legacy unsuffixed name; pages
/// 2+ get a zero-padded suffix (Go's `RawFileName`).
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
/// (left over from a longer previous run) — Go's `cleanupStalePages`.
fn cleanup_stale_pages(out_dir: &Path, max_page_no: i32) -> Result<Vec<String>> {
    let pattern = regex::Regex::new(r"^講義データ-(\d{2})\.json$").expect("valid regex");
    let mut cleaned = Vec::new();
    for entry in fs::read_dir(out_dir).context("出力ディレクトリの読み取りに失敗")?
    {
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
/// to the previous year. Matches Go's `currentKaikoNendo`.
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
    let mode = if dry_run { "dry-run" } else { "通常実行" };
    let mut md = format!(
        "## Fetch syllabus result ({mode})\n\n- 取得件数: **{}** / 全 {} ページ\n",
        result.total, result.max_page_no
    );
    if !dry_run {
        md.push_str(&format!("- 変更ファイル: {changed}\n"));
    }
    md.push_str("\n| page | 件数 | ファイル | 変更 |\n|---|---|---|---|\n");
    for p in &result.pages {
        let mark = if !dry_run && p.changed { "✓" } else { "—" };
        md.push_str(&format!(
            "| {} | {} | `{}` | {} |\n",
            p.page_no, p.list_len, p.file_name, mark
        ));
    }
    if !result.cleaned.is_empty() {
        md.push_str(&format!("\n古いファイルを削除: {:?}\n", result.cleaned));
    }
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().append(true).create(true).open(path) {
        let _ = f.write_all(md.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    //! Orchestration parity, ported from `internal/fetch/fetch_test.go`. No
    //! sockets — a fake fetcher returns canned page bytes.
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
}
