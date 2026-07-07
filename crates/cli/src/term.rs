//! Shared terminal flourishes for the CLI (stderr): iconified status lines, a
//! timed completion footer, and a framed error. Honors `NO_COLOR`. Kept simple —
//! GitHub Actions logs are append-only (no spinners / cursor tricks).

use std::time::Duration;

fn colored() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

fn paint(code: &str, s: &str) -> String {
    if colored() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_owned()
    }
}

/// `1.2s`, `1m03s`.
fn dur(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 10_000 {
        format!("{:.1}s", d.as_secs_f64())
    } else {
        let s = d.as_secs();
        let (m, sec) = (s / 60, s % 60);
        if m > 0 {
            format!("{m}m{sec:02}s")
        } else {
            format!("{sec}s")
        }
    }
}

/// `✓ …` (green).
pub fn ok(msg: &str) {
    eprintln!("  {} {msg}", paint("1;32", "✓"));
}
/// `ℹ …` (cyan).
pub fn info(msg: &str) {
    eprintln!("  {} {msg}", paint("1;36", "ℹ"));
}
/// `⚠ …` (yellow).
pub fn warn(msg: &str) {
    eprintln!("  {} {}", paint("1;33", "⚠"), paint("33", msg));
}

/// Timed completion footer for a successful run.
pub fn footer_ok(elapsed: Duration) {
    eprintln!();
    eprintln!(
        "  {}  {}",
        paint("1;32", "✔ done"),
        paint("2", &format!("in {}", dur(elapsed))),
    );
    eprintln!();
}

/// Framed failure footer: status, elapsed, then the full error cause chain.
pub fn footer_err(e: &anyhow::Error, elapsed: Duration) {
    eprintln!();
    eprintln!(
        "  {}  {}",
        paint("1;31", "✘ failed"),
        paint("2", &format!("after {}", dur(elapsed))),
    );
    for (i, cause) in e.chain().enumerate() {
        let bullet = if i == 0 { "✗" } else { "↳" };
        eprintln!(
            "  {} {}",
            paint("31", bullet),
            paint("2", &cause.to_string())
        );
    }
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dur_switches_units() {
        assert_eq!(dur(Duration::from_millis(1234)), "1.2s");
        assert_eq!(dur(Duration::from_secs(9)), "9.0s");
        assert_eq!(dur(Duration::from_secs(75)), "1m15s");
    }
}
