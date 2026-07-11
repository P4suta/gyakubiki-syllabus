//! Convert-time enrichment: derive structured, display-ready fields from an
//! already-parsed [`SanshoDetail`] (no re-crawl). Faithful only — every
//! derivation reorganises or annotates the existing text; it never rewrites,
//! summarises, or drops it, and falls back to nothing when unsure.

use std::sync::LazyLock;

use super::model::{PrepInfo, SanshoDetail, TextbookInfo, TextbookSection};

/// Annotate a `SanshoDetail` in place with derived fields, before it is written
/// to `web/public/details/{cd}.json`. Applied per course during `convert`.
pub fn enrich(detail: &mut SanshoDetail) {
    // Normalise the display text first so the derived views inherit the folding.
    purify_in_place(detail);
    for item in &mut detail.plan {
        item.kind = classify_plan_kind(&item.text, item.n);
    }
    detail.textbook_info = detail.textbooks.as_deref().map(split_textbooks);
    detail.prep_info = detail
        .prep
        .as_deref()
        .map(parse_prep)
        .filter(|p| p.hours.is_some() || p.yoshu.is_some() || p.fukushu.is_some());
    detail.keywords = repair_keywords(std::mem::take(&mut detail.keywords));
}

/// Fold full-width ASCII letters/digits to half-width across the display text
/// (the「表記ゆれ」cleanup). Full-width punctuation/parens and kana are left as-is
/// — they are legitimate Japanese typography — and the parser has already run, so
/// 第１回/：markers are unaffected. Only the web-detail copy is touched; the
/// crawled raw-details stay verbatim.
fn purify_in_place(d: &mut SanshoDetail) {
    purify_opt(&mut d.summary);
    purify_opt(&mut d.aims);
    purify_opt(&mut d.prereq);
    purify_opt(&mut d.prep);
    purify_opt(&mut d.textbooks);
    for g in &mut d.goals {
        *g = purify_text(g);
    }
    for p in &mut d.plan {
        p.text = purify_text(&p.text);
    }
    for k in &mut d.keywords {
        *k = purify_text(k);
    }
    for e in &mut d.extra {
        e.text = purify_text(&e.text);
    }
}

fn purify_opt(o: &mut Option<String>) {
    if let Some(s) = o {
        *s = purify_text(s);
    }
}

fn purify_text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'Ａ'..='Ｚ' => char::from(b'A' + (c as u32 - 'Ａ' as u32) as u8),
            'ａ'..='ｚ' => char::from(b'a' + (c as u32 - 'ａ' as u32) as u8),
            '０'..='９' => char::from(b'0' + (c as u32 - '０' as u32) as u8),
            other => other,
        })
        .collect()
}

/// Repair keywords that an older space-split fragmented mid-parenthesis: merge
/// consecutive entries while an opening bracket is still unclosed, so a term like
/// 「方程式(Euler」「Lagrange's」「equation)」rejoins into one. Conservative — only
/// the unclosed-open signal drives a merge (capped), balanced entries are kept.
fn repair_keywords(kw: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut merges = 0;
    for k in kw {
        if buf.is_empty() {
            buf = k;
        } else {
            buf.push(' ');
            buf.push_str(&k);
            merges += 1;
        }
        if open_paren_debt(&buf) <= 0 || merges >= 6 {
            out.push(std::mem::take(&mut buf));
            merges = 0;
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// Count of unclosed opening brackets (half- and full-width); stray closes are
/// ignored so a lone「equation)」isn't treated as needing a merge.
fn open_paren_debt(s: &str) -> i32 {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' | '（' => depth += 1,
            ')' | '）' if depth > 0 => depth -= 1,
            _ => {}
        }
    }
    depth
}

static HOUR_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(\d+(?:\.\d+)?)\s*時間(半)?").expect("re"));

/// Split 教科書・参考書 free text into labelled sections, preserving every source
/// line verbatim (book-title linkifying happens at render). Marks `is_none` only
/// for a short, unambiguous "not specified" statement, so a real reading list is
/// never hidden.
fn split_textbooks(s: &str) -> TextbookInfo {
    let trimmed = s.trim();
    let has_title = trimmed.contains('『') || trimmed.contains('「');
    let null_kw = [
        "特になし",
        "なし",
        "無し",
        "使用しない",
        "指定しない",
        "指定なし",
    ];
    let is_none =
        trimmed.chars().count() <= 20 && !has_title && null_kw.iter().any(|k| trimmed.contains(k));

    let labels = ["教科書", "参考書", "参考資料", "参考文献", "テキスト"];
    let is_label = |line: &str| {
        labels.iter().any(|l| {
            line.strip_prefix(l).is_some_and(|rest| {
                rest.is_empty() || rest.starts_with(['：', ':', '【', '［', '[', ' ', '\u{3000}'])
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
/// The full prep text is always shown regardless, so a miss just omits the badge.
fn parse_prep(s: &str) -> PrepInfo {
    let ascii = to_ascii_digits(s);
    PrepInfo {
        hours: study_hours(&ascii),
        yoshu: label_segment(s, "予習"),
        fukushu: label_segment(s, "復習"),
    }
}

/// A single, unambiguous per-session study time in hours. Conservative on
/// purpose: ranges (2時間～3時間) and multi-figure texts mix scopes (1時間/日 +
/// 5時間/学期), so summing would fabricate a wrong number — those return `None`
/// and the badge is simply omitted. 時間半 counts as +0.5h.
fn study_hours(ascii: &str) -> Option<f64> {
    if ascii.contains('～') || ascii.contains('〜') || ascii.contains('~') {
        return None;
    }
    let caps: Vec<_> = HOUR_RE.captures_iter(ascii).collect();
    if caps.len() != 1 {
        return None;
    }
    let mut h: f64 = caps[0][1].parse().ok()?;
    if caps[0].get(2).is_some() {
        h += 0.5; // 「N時間半」
    }
    (0.25..=12.0)
        .contains(&h)
        .then_some((h * 10.0).round() / 10.0)
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
/// tints the timeline node — so exam detection is HIGH-PRECISION: a lecture topic
/// that merely mentions 試験/テスト (臨床試験・ソフトウェアテスト・試験対策・…コンテスト・
/// 小テスト) must not read as an exam. We require a compound exam term.
const EXAM_TERMS: &[&str] = &[
    "期末試験",
    "定期試験",
    "中間試験",
    "最終試験",
    "筆記試験",
    "実技試験",
    "口述試験",
    "口頭試験",
    "期末テスト",
    "中間テスト",
    "定期テスト",
];

fn classify_plan_kind(text: &str, n: i64) -> Option<String> {
    let has = |kw: &str| text.contains(kw);

    if EXAM_TERMS.iter().any(|t| has(t)) {
        return Some("exam".into());
    }
    // Milestone: a deliverable / turning point.
    if has("まとめ") || has("発表") || has("プレゼン") || has("報告会") {
        return Some("milestone".into());
    }
    // Start: only the first session, and only when it names an intro.
    if n == 1 && (has("オリエンテーション") || has("ガイダンス") || has("イントロダクション"))
    {
        return Some("start".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::classify_plan_kind;

    #[test]
    fn exam_needs_a_compound_exam_term() {
        assert_eq!(classify_plan_kind("期末試験", 15).as_deref(), Some("exam"));
        assert_eq!(
            classify_plan_kind("定期試験を行う", 16).as_deref(),
            Some("exam")
        );
        assert_eq!(classify_plan_kind("中間テスト", 8).as_deref(), Some("exam"));
    }

    #[test]
    fn lecture_topics_that_merely_mention_shiken_are_not_exams() {
        // Real false-positives caught by the fidelity review — none is an exam.
        assert_eq!(classify_plan_kind("臨床試験とEBM", 3), None);
        assert_eq!(classify_plan_kind("ソフトウェアテスト1", 11), None);
        assert_eq!(
            classify_plan_kind("基本情報技術者試験 科目B試験対策", 5),
            None
        );
        assert_eq!(classify_plan_kind("試験管に植菌する", 6), None);
        assert_eq!(classify_plan_kind("中国語音読コンテスト", 12), None);
        assert_eq!(classify_plan_kind("毎回の小テスト", 5), None);
    }

    #[test]
    fn ambiguous_kimatsu_chukan_do_not_force_exam() {
        // 期末レポート is a report, not an exam — bare 期末 must not trip exam.
        assert_eq!(classify_plan_kind("期末レポートの作成", 14), None);
        // 中間発表 is a presentation → milestone (via 発表), never exam.
        assert_eq!(
            classify_plan_kind("中間発表", 8).as_deref(),
            Some("milestone")
        );
    }

    #[test]
    fn milestone_matches_deliverables() {
        assert_eq!(
            classify_plan_kind("成果発表会", 14).as_deref(),
            Some("milestone")
        );
        assert_eq!(
            classify_plan_kind("全体のまとめ", 16).as_deref(),
            Some("milestone")
        );
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
        assert_eq!(
            info.sections[0].lines[0],
            "進化の教科書 第1－3巻 カール・ジンマー著"
        );
    }

    #[test]
    fn prep_extracts_a_single_unambiguous_hour_figure() {
        assert_eq!(
            super::parse_prep("毎回およそ2時間の予習・復習を行うこと。").hours,
            Some(2.0)
        );
        // 「N時間半」→ +0.5h.
        assert_eq!(
            super::parse_prep("毎回1時間半程度の学習が必要。").hours,
            Some(1.5)
        );
        // No 時間 figure → no badge (the full text is still shown).
        assert_eq!(
            super::parse_prep("予習に45分、復習に45分をあてること。").hours,
            None
        );
        assert_eq!(
            super::parse_prep("相当な授業時間外の学習が必要です。").hours,
            None
        );
    }

    #[test]
    fn prep_skips_ranges_and_multi_figure_to_avoid_a_wrong_sum() {
        // Range double-count and scope-mixing produced fabricated totals — skip.
        assert_eq!(
            super::parse_prep("復習2時間～3時間、予習1時間～2時間。").hours,
            None
        );
        assert_eq!(
            super::parse_prep("授業期間中に1日1時間、終了後に5時間程度の復習。").hours,
            None
        );
    }

    #[test]
    fn prep_splits_yoshu_fukushu_verbatim() {
        let p = super::parse_prep("予習内容：参考文献の熟読\n復習内容：配布資料の整理");
        assert_eq!(p.yoshu.as_deref(), Some("参考文献の熟読"));
        assert_eq!(p.fukushu.as_deref(), Some("配布資料の整理"));
    }

    #[test]
    fn repair_keywords_rejoins_broken_parentheses() {
        // The real 71609 breakage: 括弧内の英語句がスペースで寸断されていた。
        let got = super::repair_keywords(vec![
            "Euler-Lagrange".into(),
            "方程式(Euler".into(),
            "Lagrange's".into(),
            "equation)".into(),
            "Hamilton方程式(Hamilton's".into(),
            "equation)".into(),
        ]);
        assert_eq!(
            got,
            vec![
                "Euler-Lagrange",
                "方程式(Euler Lagrange's equation)",
                "Hamilton方程式(Hamilton's equation)",
            ]
        );
    }

    #[test]
    fn purify_folds_full_width_ascii_but_keeps_typography() {
        // Full-width latin/digits → half-width.
        assert_eq!(
            super::purify_text("ＴＯＥＩＣ ８００点、Ｗで始まる"),
            "TOEIC 800点、Wで始まる"
        );
        // Full-width punctuation/parens and half-width kana are preserved.
        assert_eq!(super::purify_text("（重要）：ﾃｽﾄ"), "（重要）：ﾃｽﾄ");
    }

    #[test]
    fn repair_keywords_leaves_balanced_terms_untouched() {
        let got = super::repair_keywords(vec![
            "機械学習".into(),
            "データサイエンス".into(),
            "正準変換(canonical transformation)".into(),
        ]);
        assert_eq!(
            got,
            vec![
                "機械学習",
                "データサイエンス",
                "正準変換(canonical transformation)"
            ]
        );
    }
}
