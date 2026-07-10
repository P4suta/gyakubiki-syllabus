//! Shared, transport-agnostic HTTP resilience primitives used by both KULAS
//! crawlers: a classified [`FetchError`], a jittered [`Politeness`] delay, and an
//! exponential [`backoff`]. Extracted so the grid fetch and the detail crawl are
//! symmetric — both back off on 429/5xx and stay polite — instead of the grid
//! side bailing on the first hiccup.

use std::time::Duration;

/// A fetch failure, classified so an orchestrator can tell "this item is bad,
/// skip it" from "the server is refusing us, back off before we get banned".
#[derive(Debug)]
pub enum FetchError {
    /// Non-2xx HTTP — carries the status so 403/429/5xx trip the circuit breaker,
    /// any `Retry-After` the server asked us to wait, and the response **body**
    /// (redacted, truncated) so a failure is diagnosable without a re-run.
    Http {
        status: u16,
        retry_after: Option<Duration>,
        body: String,
    },
    /// Network/transport error — worth a bounded retry.
    Transient(anyhow::Error),
    /// The response was reached but is unusable for this item (bad guid, empty
    /// body). Skip it; not a sign of blocking.
    Fatal(anyhow::Error),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Http { status, .. } => write!(f, "HTTP {status}"),
            FetchError::Transient(e) => write!(f, "transient: {e}"),
            FetchError::Fatal(e) => write!(f, "fatal: {e}"),
        }
    }
}

impl FetchError {
    /// Whether this looks like the server refusing/limiting us (vs. a bad item).
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            FetchError::Http {
                status: 403 | 429 | 500..=599,
                ..
            }
        )
    }

    /// Whether a bounded retry is worthwhile.
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            FetchError::Transient(_)
                | FetchError::Http {
                    status: 429 | 500..=599,
                    ..
                }
        )
    }

    /// Whether this is an item-specific "no usable response" — the server
    /// responded but there is nothing usable (bad guid, empty body). Such an item
    /// can be tombstoned after repeated failures, unlike an HTTP/network error.
    pub fn is_no_detail(&self) -> bool {
        matches!(self, FetchError::Fatal(_))
    }

    /// The server-requested wait (`Retry-After`), if any, to honor before retrying.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            FetchError::Http { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// A rich, self-explaining description for diagnostics: HTTP errors carry the
    /// actual server response body, so the failure is legible without a re-run.
    pub fn diagnostic(&self) -> String {
        match self {
            FetchError::Http { status, body, .. } if !body.trim().is_empty() => {
                format!("HTTP {status} — response body:\n{body}")
            }
            FetchError::Http { status, .. } => format!("HTTP {status} (empty body)"),
            FetchError::Transient(e) => format!("network/transport error: {e:#}"),
            FetchError::Fatal(e) => format!("unusable response: {e:#}"),
        }
    }
}

/// A jittered sleep policy between requests, so a burst never looks like an
/// attack. `base` plus `[0, jitter_ms)` of random jitter.
pub struct Politeness {
    pub(crate) base: Duration,
    pub(crate) jitter_ms: u64,
}

impl Politeness {
    /// Construct from a base delay and an upper jitter bound (both in ms).
    pub fn from_ms(base_ms: u64, jitter_ms: u64) -> Self {
        Self {
            base: Duration::from_millis(base_ms),
            jitter_ms,
        }
    }

    /// The next delay: `base` plus `[0, jitter_ms)` of random jitter. Pure, so the
    /// bounds are testable without sleeping.
    pub fn delay(&self) -> Duration {
        let extra = if self.jitter_ms == 0 {
            0
        } else {
            rand::random_range(0..self.jitter_ms)
        };
        self.base + Duration::from_millis(extra)
    }

    /// Sleep for [`Politeness::delay`], unless it is zero.
    pub fn wait(&self) {
        let d = self.delay();
        if !d.is_zero() {
            std::thread::sleep(d);
        }
    }
}

/// Backoff before a retry: `base * 2^tryno`, capped, so we back off *faster* when
/// a server is struggling instead of hammering it. Pure and saturating, so the
/// schedule is testable and large `tryno` never overflows.
pub fn backoff(base: Duration, tryno: u32) -> Duration {
    const CAP: Duration = Duration::from_secs(60);
    base.saturating_mul(2u32.saturating_pow(tryno)).min(CAP)
}

/// Truncate a captured body to a diagnostics-friendly size (keep the head; that's
/// where framework error messages live).
pub fn snippet(body: &str) -> String {
    const MAX: usize = 1200;
    let body = body.trim();
    if body.len() <= MAX {
        body.to_owned()
    } else {
        let mut end = MAX;
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}… ({} bytes total)", &body[..end], body.len())
    }
}

/// Parse a `Retry-After` header. Only the delta-seconds form is honored (the
/// common case for 429/503); the HTTP-date form falls back to `None` and lets the
/// exponential backoff take over.
pub fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let secs: u64 = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn politeness_delay_respects_base_and_jitter_bounds() {
        let p = Politeness::from_ms(100, 0);
        assert_eq!(p.delay(), Duration::from_millis(100)); // jitter=0 is deterministic
        let p = Politeness::from_ms(0, 50);
        for _ in 0..64 {
            assert!(p.delay() < Duration::from_millis(50));
        }
    }

    #[test]
    fn backoff_doubles_and_caps() {
        let base = Duration::from_secs(1);
        assert_eq!(backoff(base, 0), Duration::from_secs(1));
        assert_eq!(backoff(base, 1), Duration::from_secs(2));
        assert_eq!(backoff(base, 3), Duration::from_secs(8));
        assert_eq!(backoff(base, 30), Duration::from_secs(60)); // capped, no overflow
    }

    #[test]
    fn classification_matches_status() {
        let http = |status| FetchError::Http {
            status,
            retry_after: None,
            body: String::new(),
        };
        assert!(http(429).is_blocking() && http(429).is_retriable());
        assert!(http(503).is_retriable());
        assert!(!http(404).is_no_detail() && !http(404).is_retriable());
        assert!(FetchError::Fatal(anyhow::anyhow!("x")).is_no_detail());
        assert!(FetchError::Transient(anyhow::anyhow!("x")).is_retriable());
    }
}
