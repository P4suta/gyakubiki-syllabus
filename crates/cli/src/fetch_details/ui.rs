//! Pretty, information-dense progress output for the detail crawl (stderr).
//!
//! A long crawl is otherwise a black box until it finishes, so we stream a
//! colored header, a periodic progress line (bar + rate + ETA), and a summary
//! box. Plain text with ANSI + Unicode — GitHub Actions renders both. Honors
//! `NO_COLOR`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

const RULE: &str = "──────────────────────────────────────────────";
const BAR_WIDTH: usize = 22;

/// ANSI styling, disabled when `NO_COLOR` is set.
pub struct Style {
    on: bool,
}

impl Style {
    pub fn detect() -> Self {
        Self {
            on: std::env::var_os("NO_COLOR").is_none(),
        }
    }
    fn paint(&self, code: &str, s: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_owned()
        }
    }
    pub fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }
    pub fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }
    pub fn green(&self, s: &str) -> String {
        self.paint("32", s)
    }
    pub fn yellow(&self, s: &str) -> String {
        self.paint("33", s)
    }
    pub fn cyan(&self, s: &str) -> String {
        self.paint("1;36", s)
    }
    pub fn red(&self, s: &str) -> String {
        self.paint("1;31", s)
    }
}

/// `19m48s`, `2h05m`, `48s`.
pub fn dur(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}h{m:02}m")
    } else if m > 0 {
        format!("{m}m{sec:02}s")
    } else {
        format!("{sec}s")
    }
}

/// `▕▓▓▓▓▓░░░░░░░▏` for a fraction, filled part colored.
fn bar(done: usize, total: usize, st: &Style) -> String {
    let frac = if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    };
    let filled = ((frac * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
    let fill = "█".repeat(filled);
    let rest = "░".repeat(BAR_WIDTH - filled);
    format!("▕{}{}▏", st.cyan(&fill), st.dim(&rest))
}

/// Opening banner: what we are about to crawl and how gently.
pub fn header(count: usize, sleep_ms: u64, jitter_ms: u64, out_dir: &Path) {
    let st = Style::detect();
    let lo = sleep_ms as f64 / 1000.0;
    let hi = (sleep_ms + jitter_ms) as f64 / 1000.0;
    let row = |k: &str, v: String| eprintln!("  {}  {}", st.dim(&format!("{k:<7}")), v);
    eprintln!();
    eprintln!(
        "  🐟  {}",
        st.bold("逆引きシラバス · syllabus detail crawl")
    );
    eprintln!("  {}", st.dim(RULE));
    row(
        "target",
        format!("{} {}", st.cyan(&count.to_string()), st.dim("courses")),
    );
    row(
        "pace",
        st.dim(&format!("{lo:.1}–{hi:.1}s / course (jittered, sequential)")),
    );
    row("output", st.dim(&out_dir.display().to_string()));
    eprintln!("  {}", st.dim(RULE));
    eprintln!();
}

/// One streamed progress line: `⟳ 520/1360 ▕▓▓▓░░▏ 38% ✓517 ⚠3 · rate · ETA`.
pub fn progress(done: usize, total: usize, fetched: usize, skipped: usize, elapsed: Duration) {
    let st = Style::detect();
    let secs = elapsed.as_secs().max(1);
    let per = secs as f64 / done.max(1) as f64;
    let per_min = done as f64 / (secs as f64 / 60.0);
    let remaining = total.saturating_sub(done);
    let eta = Duration::from_secs((per * remaining as f64) as u64);
    let pct = done * 100 / total.max(1);
    eprintln!(
        "  {} {:>5}/{:<5} {} {:>3}%   {}  {}   {}",
        st.cyan("⟳"),
        done,
        total,
        bar(done, total, &st),
        pct,
        st.green(&format!("✓ {fetched}")),
        st.yellow(&format!("⚠ {skipped}")),
        st.dim(&format!(
            "· {per:.1}s/ea · {per_min:.0}/min · {} elapsed · ETA ~{}",
            dur(elapsed),
            dur(eta)
        )),
    );
}

/// Closing box: totals, top skip reasons, success rate.
pub fn summary(fetched: usize, skipped: &[(String, String)], elapsed: Duration, aborted: bool) {
    let st = Style::detect();
    let total = fetched + skipped.len();
    let row = |k: &str, v: String| eprintln!("  {}  {}", st.dim(&format!("{k:<7}")), v);
    eprintln!();
    if aborted {
        eprintln!("  ✗  {}", st.red("crawl aborted — circuit breaker tripped"));
    } else {
        eprintln!("  ✓  {}", st.bold("crawl complete"));
    }
    eprintln!("  {}", st.dim(RULE));
    row("fetched", st.green(&fetched.to_string()));

    let skip_note = if skipped.is_empty() {
        st.dim("none")
    } else {
        format!(
            "{}   {}",
            st.yellow(&skipped.len().to_string()),
            st.dim(&top_reasons(skipped))
        )
    };
    row("skipped", skip_note);

    let per = if total > 0 {
        elapsed.as_secs_f64() / total as f64
    } else {
        0.0
    };
    row(
        "elapsed",
        st.dim(&format!("{}  ·  {per:.1}s / course", dur(elapsed))),
    );
    if total > 0 {
        let rate = fetched as f64 * 100.0 / total as f64;
        let colored = format!("{rate:.1}%");
        row(
            "success",
            if rate >= 95.0 {
                st.green(&colored)
            } else {
                st.yellow(&colored)
            },
        );
    }
    eprintln!("  {}", st.dim(RULE));
    eprintln!();
}

/// A framed, front-and-center box for the first failure — its cause is the first
/// thing you see when you open a failed run.
pub fn spotlight(cd: &str, diagnostic: &str) {
    let st = Style::detect();
    eprintln!();
    eprintln!(
        "  {} {}",
        st.red("┏━ first failure"),
        st.dim(&format!("· course {cd}")),
    );
    for line in diagnostic.lines() {
        eprintln!("  {} {}", st.red("┃"), st.dim(line));
    }
    eprintln!("  {}", st.red("┗━"));
    eprintln!();
}

/// A one-line verdict + an actionable hint + where the full context was saved.
pub fn diagnosis(headline: &str, hint: Option<&str>, artifact: &Path) {
    let st = Style::detect();
    eprintln!("  {}  {}", st.red("⚑ diagnosis"), st.bold(headline));
    if let Some(h) = hint {
        eprintln!("  {} {}", st.yellow("→"), st.yellow(h));
    }
    eprintln!(
        "  {} {}",
        st.dim("full context →"),
        st.dim(&artifact.display().to_string()),
    );
    eprintln!();
}

/// Skip reasons bucketed and ranked most-common first.
fn reason_counts(skipped: &[(String, String)]) -> Vec<(&str, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, why) in skipped {
        *counts.entry(condense(why)).or_default() += 1;
    }
    let mut pairs: Vec<(&str, usize)> = counts.into_iter().collect();
    pairs.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    pairs
}

/// `(3× HTTP 400, 1× no guid)` — the most common skip reasons, condensed.
fn top_reasons(skipped: &[(String, String)]) -> String {
    let pairs = reason_counts(skipped);
    let shown: Vec<String> = pairs
        .iter()
        .take(3)
        .map(|(reason, n)| format!("{n}× {reason}"))
        .collect();
    let extra = pairs.len().saturating_sub(3);
    let mut out = format!("({})", shown.join(", "));
    if extra > 0 {
        out.push_str(&format!(" +{extra} more"));
    }
    out
}

/// Append a rendered-markdown recap to `$GITHUB_STEP_SUMMARY` (a no-op off CI) so
/// the Actions *run page* shows a clean dashboard, not just the raw log — the one
/// view that never depends on the log renderer's ANSI support.
pub fn step_summary(
    fetched: usize,
    skipped: &[(String, String)],
    elapsed: Duration,
    aborted: bool,
) {
    use std::io::Write;
    let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") else {
        return;
    };
    let total = fetched + skipped.len();
    let success = if total > 0 {
        fetched as f64 * 100.0 / total as f64
    } else {
        100.0
    };
    let per = if total > 0 {
        elapsed.as_secs_f64() / total as f64
    } else {
        0.0
    };
    let status = if aborted {
        "✗ aborted (circuit breaker)"
    } else {
        "✓ complete"
    };

    let mut md = String::new();
    md.push_str(&format!("### 🐟 syllabus detail crawl — {status}\n\n"));
    md.push_str("| metric | value |\n|---|--:|\n");
    md.push_str(&format!("| ✓ fetched | **{fetched}** |\n"));
    md.push_str(&format!("| ⚠ skipped | {} |\n", skipped.len()));
    md.push_str(&format!(
        "| ⏱ elapsed | {} · {per:.1}s/course |\n",
        dur(elapsed)
    ));
    md.push_str(&format!("| 📈 success | {success:.1}% |\n"));
    if !skipped.is_empty() {
        md.push_str("\n<details><summary>skip reasons</summary>\n\n");
        for (reason, n) in reason_counts(skipped) {
            md.push_str(&format!("- {n}× {reason}\n"));
        }
        md.push_str("\n</details>\n");
    }
    md.push('\n');

    if let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
    {
        let _ = f.write_all(md.as_bytes());
    }
}

/// Collapse a raw error string to a short bucket label for the summary.
fn condense(why: &str) -> &str {
    let why = why.trim();
    if let Some(rest) = why.strip_prefix("fatal: ") {
        return rest.split(':').next().unwrap_or(rest).trim();
    }
    if why.starts_with("save failed") {
        return "save failed";
    }
    why
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_sections_without_panicking() {
        // Exercises header/progress/summary across empty, partial, and full states
        // so a formatting or width mistake fails here rather than mid-crawl.
        header(1360, 1000, 1000, std::path::Path::new("raw-details/"));
        progress(25, 1360, 25, 0, Duration::from_secs(58));
        progress(520, 1360, 517, 3, Duration::from_secs(19 * 60 + 48));
        progress(1360, 1360, 1351, 9, Duration::from_secs(52 * 60 + 10));
        summary(
            1351,
            &[
                ("1".into(), "HTTP 400".into()),
                ("2".into(), "HTTP 400".into()),
                ("3".into(), "fatal: initFind returned no guid".into()),
                ("4".into(), "fatal: empty HTML returned".into()),
            ],
            Duration::from_secs(52 * 60 + 10),
            false,
        );
        spotlight(
            "09005",
            "HTTP 400 — response body:\n{\"errorMessages\":[{\"message\":\"service method 'SyllabusSanshoWebApi initFind' not found\"}]}",
        );
        diagnosis(
            "0 fetched · 10 skipped of 10 attempted",
            Some("The sansho API path/method may have changed — compare the captured body with INIT_FIND_URL."),
            std::path::Path::new("diagnostics/fetch-details.md"),
        );
    }

    #[test]
    fn dur_formats_by_magnitude() {
        assert_eq!(dur(Duration::from_secs(48)), "48s");
        assert_eq!(dur(Duration::from_secs(19 * 60 + 48)), "19m48s");
        assert_eq!(dur(Duration::from_secs(2 * 3600 + 5 * 60)), "2h05m");
    }

    #[test]
    fn top_reasons_ranks_and_condenses() {
        let skipped = vec![
            ("001".into(), "HTTP 400".into()),
            ("002".into(), "HTTP 400".into()),
            ("003".into(), "fatal: initFind returned no guid".into()),
        ];
        let s = top_reasons(&skipped);
        assert!(s.contains("2× HTTP 400"), "{s}");
        assert!(s.contains("1× initFind returned no guid"), "{s}");
    }

    #[test]
    fn no_color_disables_ansi() {
        // With styling off, painted text is returned verbatim (no escape codes).
        let plain = Style { on: false };
        assert_eq!(plain.green("x"), "x");
        let colored = Style { on: true };
        assert!(colored.green("x").contains('\x1b'));
    }
}
