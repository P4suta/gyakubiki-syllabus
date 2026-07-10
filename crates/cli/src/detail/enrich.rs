//! Convert-time enrichment: derive structured, display-ready fields from an
//! already-parsed [`SanshoDetail`] (no re-crawl). Faithful only — every
//! derivation reorganises or annotates the existing text; it never rewrites,
//! summarises, or drops it, and falls back to nothing when unsure.

use std::sync::LazyLock;

use super::model::{PrepInfo, SanshoDetail, TextbookInfo, TextbookSection};

/// Annotate a `SanshoDetail` in place with derived fields, before it is written
/// to `web/public/details/{cd}.json`. Applied per course during `convert`.
pub fn enrich(detail: &mut SanshoDetail) {
    for item in &mut detail.plan {
        item.kind = classify_plan_kind(&item.text, item.n);
    }
    detail.textbook_info = detail.textbooks.as_deref().map(split_textbooks);
    detail.prep_info = detail
        .prep
        .as_deref()
        .map(parse_prep)
        .filter(|p| p.hours.is_some() || p.yoshu.is_some() || p.fukushu.is_some());
}

static HOUR_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(\d+(?:\.\d+)?)\s*時間").expect("re"));
static MIN_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(\d+)\s*分").expect("re"));

/// Split 教科書・参考書 free text into labelled sections, preserving every source
/// line verbatim (book-title linkifying happens at render). Marks `is_none` only
/// for a short, unambiguous "not specified" statement, so a real reading list is
/// never hidden.
fn split_textbooks(s: &str) -> TextbookInfo {
    let trimmed = s.trim();
    let has_title = trimmed.contains('『') || trimmed.contains('「');
    let null_kw = ["特になし", "なし", "無し", "使用しない", "指定しない", "指定なし"];
    let is_none = trimmed.chars().count() <= 20
        && !has_title
        && null_kw.iter().any(|k| trimmed.contains(k));

    let labels = ["教科書", "参考書", "参考資料", "参考文献", "テキスト"];
    let is_label = |line: &str| {
        labels.iter().any(|l| {
            line.strip_prefix(l).is_some_and(|rest| {
                rest.is_empty()
                    || rest.starts_with(['：', ':', '【', '［', '[', ' ', '\u{3000}'])
            })
        })
    };

    let mut sections: Vec<TextbookSection> = Vec::new();
    for line in trimmed.split('\n').map(str::trim).filter(|l| !l.is_empty()) {
        if is_label(line) {
            sections.push(TextbookSection {
                label: Some(line.to_owned()),
                lines: Vec::new(),
            });
        } else if let Some(last) = sections.last_mut() {
            last.lines.push(line.to_owned());
        } else {
            sections.push(TextbookSection {
                label: None,
                lines: vec![line.to_owned()],
            });
        }
    }
    if sections.is_empty() {
        sections.push(TextbookSection {
            label: None,
            lines: vec![trimmed.to_owned()],
        });
    }
    TextbookInfo { is_none, sections }
}

/// Extract a per-session study time and 予習/復習 split from 授業時間外の学習.
/// Conservative: `hours` is set only for a plausible total (0.25–12h); the full
/// prep text is always shown regardless, so a miss just omits the badge.
fn parse_prep(s: &str) -> PrepInfo {
    let ascii = to_ascii_digits(s);
    let mut total = 0.0f64;
    for cap in HOUR_RE.captures_iter(&ascii) {
        total += cap[1].parse::<f64>().unwrap_or(0.0);
    }
    for cap in MIN_RE.captures_iter(&ascii) {
        total += cap[1].parse::<f64>().unwrap_or(0.0) / 60.0;
    }
    let hours = (0.25..=12.0)
        .contains(&total)
        .then(|| (total * 10.0).round() / 10.0);
    PrepInfo {
        hours,
        yoshu: label_segment(s, "予習"),
        fukushu: label_segment(s, "復習"),
    }
}

/// Text after a `予習[:：]` / `復習[:：]` label (optionally `…内容：`), verbatim.
fn label_segment(s: &str, label: &str) -> Option<String> {
    for line in s.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix(label) {
            let rest = rest.strip_prefix("内容").unwrap_or(rest);
            if let Some(after) = rest.trim_start().strip_prefix(['：', ':']) {
                let val = after.trim();
                if !val.is_empty() {
                    return Some(val.to_owned());
                }
            }
        }
    }
    None
}

/// Full-width digits (`０`-`９`) → ASCII so time extraction works either way.
fn to_ascii_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '０'..='９' => char::from(b'0' + (c as u32 - '０' as u32) as u8),
            other => other,
        })
        .collect()
}

/// A 授業計画 session's highlight hint: `exam` | `milestone` | `start`, or `None`
/// for an ordinary session. The session `text` is never modified — this only
/// tints the timeline node — so keywords are kept high-precision to avoid a
/// misleading「試験」badge (e.g.「期末レポート」/「中間発表」must not read as an exam).
fn classify_plan_kind(text: &str, n: i64) -> Option<String> {
    let has = |kw: &str| text.contains(kw);

    // Exam: 「試験」covers 期末試験・中間試験・定期試験; テスト too, but 小テスト is a
    // quiz, not a graded exam, so it must not force the exam highlight. Bare
    // 期末/中間 are ambiguous (期末レポート・中間発表) and deliberately excluded.
    if has("試験") || (has("テスト") && !has("小テスト")) {
        return Some("exam".into());
    }
    // Milestone: a deliverable / turning point.
    if has("まとめ") || has("発表") || has("プレゼン") || has("報告会") {
        return Some("milestone".into());
    }
    // Start: only the first session, and only when it names an intro.
    if n == 1 && (has("オリエンテーション") || has("ガイダンス") || has("イントロダクション")) {
        return Some("start".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::classify_plan_kind;

    #[test]
    fn exam_matches_shiken_and_test_but_not_small_quiz() {
        assert_eq!(classify_plan_kind("期末試験", 15).as_deref(), Some("exam"));
        assert_eq!(classify_plan_kind("定期試験を行う", 16).as_deref(), Some("exam"));
        assert_eq!(classify_plan_kind("到達度テスト", 15).as_deref(), Some("exam"));
        // 小テスト is a quiz — not a graded exam highlight.
        assert_eq!(classify_plan_kind("毎回の小テスト", 5), None);
    }

    #[test]
    fn ambiguous_kimatsu_chukan_do_not_force_exam() {
        // 期末レポート is a report, not an exam — bare 期末 must not trip exam.
        assert_eq!(classify_plan_kind("期末レポートの作成", 14), None);
        // 中間発表 is a presentation → milestone (via 発表), never exam (no 試験).
        assert_eq!(classify_plan_kind("中間発表", 8).as_deref(), Some("milestone"));
    }

    #[test]
    fn milestone_matches_deliverables() {
        assert_eq!(classify_plan_kind("成果発表会", 14).as_deref(), Some("milestone"));
        assert_eq!(classify_plan_kind("全体のまとめ", 16).as_deref(), Some("milestone"));
        assert_eq!(
            classify_plan_kind("グループプレゼンテーション", 13).as_deref(),
            Some("milestone")
        );
    }

    #[test]
    fn start_only_on_first_session() {
        assert_eq!(
            classify_plan_kind("オリエンテーション", 1).as_deref(),
            Some("start")
        );
        // Same word later in the term is not a "start" node.
        assert_eq!(classify_plan_kind("ガイダンス", 8), None);
    }

    #[test]
    fn ordinary_session_is_unmarked() {
        assert_eq!(classify_plan_kind("地域社会の課題を調べる", 3), None);
    }

    #[test]
    fn textbooks_split_by_label_keeps_every_line() {
        let info = super::split_textbooks(
            "教科書\n夫馬賢治 『データでわかる2030年地球のすがた』日経BP 2020年\n参考書\n夫馬賢治 『ESG思考』講談社α新書 2020年",
        );
        assert!(!info.is_none);
        assert_eq!(info.sections.len(), 2);
        assert_eq!(info.sections[0].label.as_deref(), Some("教科書"));
        assert_eq!(info.sections[0].lines.len(), 1);
        assert_eq!(info.sections[1].label.as_deref(), Some("参考書"));
        assert!(info.sections[1].lines[0].contains("ESG思考"));
    }

    #[test]
    fn textbooks_none_flagged_only_when_confident() {
        assert!(super::split_textbooks("特になし").is_none);
        assert!(super::split_textbooks("なし").is_none);
        // A real title is never hidden, even alongside a "なし".
        let info = super::split_textbooks("教科書はなし。参考書『統計学入門』を適宜参照。");
        assert!(!info.is_none);
    }

    #[test]
    fn textbooks_unlabelled_stays_one_section_verbatim() {
        let info = super::split_textbooks("進化の教科書 第1－3巻 カール・ジンマー著");
        assert!(!info.is_none);
        assert_eq!(info.sections.len(), 1);
        assert_eq!(info.sections[0].label, None);
        assert_eq!(info.sections[0].lines[0], "進化の教科書 第1－3巻 カール・ジンマー著");
    }

    #[test]
    fn prep_extracts_plausible_hours() {
        assert_eq!(super::parse_prep("毎回およそ2時間の予習・復習を行うこと。").hours, Some(2.0));
        // 予習45分 + 復習45分 = 1.5h.
        let p = super::parse_prep("予習に45分、復習に45分をあてること。");
        assert_eq!(p.hours, Some(1.5));
        // No figure → no badge, but the text is still (elsewhere) shown.
        assert_eq!(super::parse_prep("相当な授業時間外の学習が必要です。").hours, None);
    }

    #[test]
    fn prep_splits_yoshu_fukushu_verbatim() {
        let p = super::parse_prep("予習内容：参考文献の熟読\n復習内容：配布資料の整理");
        assert_eq!(p.yoshu.as_deref(), Some("参考文献の熟読"));
        assert_eq!(p.fukushu.as_deref(), Some("配布資料の整理"));
    }
}
