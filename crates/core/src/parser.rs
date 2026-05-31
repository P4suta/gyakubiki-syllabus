//! Parse a `jikanwari` (時間割) string into semester / day / period slots — a
//! hand-written port of Go's `ParseJikanwari` (`internal/parser/parser.go`).
//!
//! Hand-written rather than regex-based on purpose: it keeps the core crate
//! dependency-free (nothing for the WASM consumer to drag in), and it lets the
//! whitespace handling match Go's RE2 `\s` exactly — ASCII-only, which is what
//! [`char::is_ascii_whitespace`] already is.

/// A parsed time slot: semester label, weekday label, and the 1-based period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSlot {
    pub semester: String,
    pub day: String,
    pub period: i32,
}

/// The slots parsed from one `jikanwari`, with warnings for any part that could
/// not be parsed (collected rather than silently dropped, as in Go).
#[derive(Debug, Default)]
pub struct ParseResult {
    pub slots: Vec<ParsedSlot>,
    pub warnings: Vec<String>,
}

/// Weekday characters, recognized in `X曜日` form.
const DAYS: [char; 7] = ['月', '火', '水', '木', '金', '土', '日'];

/// Parse a comma-separated `jikanwari` into its slots.
#[must_use]
pub fn parse_jikanwari(jikanwari: &str) -> ParseResult {
    let mut result = ParseResult::default();
    if jikanwari.is_empty() {
        return result;
    }

    for raw_part in jikanwari.split(',') {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }

        let (semester, rest) = split_semester(part);
        match (find_day(rest), find_period(rest)) {
            (Some(day), Some(period)) => result.slots.push(ParsedSlot {
                semester: semester.to_owned(),
                day: day.to_string(),
                period,
            }),
            (None, None) => result
                .warnings
                .push(format!("曜日・時限が見つかりません: {part:?}")),
            (None, Some(_)) => result
                .warnings
                .push(format!("曜日が見つかりません: {part:?}")),
            (Some(_), None) => result
                .warnings
                .push(format!("時限が見つかりません: {part:?}")),
        }
    }

    result
}

/// Split `part` at its first ASCII colon into `(semester, rest)` — Go's
/// `^(.*?):\s*`. The semester is whitespace-trimmed; the rest has its leading
/// ASCII whitespace removed. With no colon: empty semester, rest is all of `part`.
fn split_semester(part: &str) -> (&str, &str) {
    match part.find(':') {
        Some(colon) => {
            let semester = part[..colon].trim();
            let rest = part[colon + 1..].trim_start_matches(|c: char| c.is_ascii_whitespace());
            (semester, rest)
        }
        None => ("", part),
    }
}

/// The leftmost weekday appearing in `X曜日` form — Go's `(月|…|日)曜日`.
fn find_day(rest: &str) -> Option<char> {
    rest.char_indices().find_map(|(i, c)| {
        (DAYS.contains(&c) && rest[i + c.len_utf8()..].starts_with("曜日")).then_some(c)
    })
}

/// The leftmost full-width digit in `N時限` form, as a 1-based period — Go's
/// `([１-８])時限` with `fullWidthToInt`.
fn find_period(rest: &str) -> Option<i32> {
    rest.char_indices().find_map(|(i, c)| {
        let period = full_width_digit(c)?;
        rest[i + c.len_utf8()..]
            .starts_with("時限")
            .then_some(period)
    })
}

/// Map a full-width digit `１`–`８` to `1`–`8`; any other char is `None`.
fn full_width_digit(c: char) -> Option<i32> {
    matches!(c, '１'..='８').then(|| c as i32 - '０' as i32)
}

#[cfg(test)]
mod tests {
    //! Ported from `internal/parser/parser_test.go`.
    use super::{parse_jikanwari, ParsedSlot};

    fn slot(semester: &str, day: &str, period: i32) -> ParsedSlot {
        ParsedSlot {
            semester: semester.to_owned(),
            day: day.to_owned(),
            period,
        }
    }

    /// (name, input, want_slots, want_warnings)
    fn cases() -> Vec<(&'static str, &'static str, Vec<ParsedSlot>, usize)> {
        vec![
            (
                "single slot",
                "1学期: 月曜日１時限",
                vec![slot("1学期", "月", 1)],
                0,
            ),
            (
                "multiple slots",
                "2学期: 月曜日２時限, 2学期: 木曜日１時限",
                vec![slot("2学期", "月", 2), slot("2学期", "木", 1)],
                0,
            ),
            (
                "tsuunen",
                "通年: 火曜日３時限",
                vec![slot("通年", "火", 3)],
                0,
            ),
            (
                "zenki",
                "前期: 水曜日４時限",
                vec![slot("前期", "水", 4)],
                0,
            ),
            ("koki", "後期: 金曜日５時限", vec![slot("後期", "金", 5)], 0),
            (
                "period 6 saturday",
                "1学期: 土曜日６時限",
                vec![slot("1学期", "土", 6)],
                0,
            ),
            (
                "period 7 and 8",
                "1学期: 月曜日７時限, 1学期: 月曜日８時限",
                vec![slot("1学期", "月", 7), slot("1学期", "月", 8)],
                0,
            ),
            (
                "sunday",
                "1学期: 日曜日１時限",
                vec![slot("1学期", "日", 1)],
                0,
            ),
            (
                "three slots",
                "1学期: 月曜日１時限, 1学期: 水曜日３時限, 1学期: 金曜日５時限",
                vec![
                    slot("1学期", "月", 1),
                    slot("1学期", "水", 3),
                    slot("1学期", "金", 5),
                ],
                0,
            ),
            ("empty string", "", vec![], 0),
            ("whitespace only", "   ", vec![], 0),
            ("commas only", ", , ,", vec![], 0),
            ("no day match", "1学期: ３時限", vec![], 1),
            ("no period match", "1学期: 月曜日", vec![], 1),
            ("no day and no period", "1学期: 集中講義", vec![], 1),
            (
                "partial parse - one good one bad",
                "1学期: 月曜日１時限, 1学期: 集中講義",
                vec![slot("1学期", "月", 1)],
                1,
            ),
            ("day without 曜日 suffix", "1学期: 月１時限", vec![], 1),
            (
                "extra whitespace",
                "  1学期:  月曜日１時限  ,  2学期:  火曜日２時限  ",
                vec![slot("1学期", "月", 1), slot("2学期", "火", 2)],
                0,
            ),
            (
                "no semester prefix",
                "月曜日３時限",
                vec![slot("", "月", 3)],
                0,
            ),
        ]
    }

    #[test]
    fn parses_all_cases() {
        for (name, input, want_slots, want_warnings) in cases() {
            let result = parse_jikanwari(input);
            assert_eq!(result.slots, want_slots, "slots for {name:?}");
            assert_eq!(
                result.warnings.len(),
                want_warnings,
                "warning count for {name:?}: {:?}",
                result.warnings
            );
        }
    }

    #[test]
    fn warning_includes_original_text() {
        let result = parse_jikanwari("1学期: ３時限");
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("1学期: ３時限"));
    }
}
