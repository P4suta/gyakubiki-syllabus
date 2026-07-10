//! Convert-time enrichment: derive structured, display-ready fields from an
//! already-parsed [`SanshoDetail`] (no re-crawl). Faithful only — every
//! derivation reorganises or annotates the existing text; it never rewrites,
//! summarises, or drops it, and falls back to nothing when unsure.

use super::model::SanshoDetail;

/// Annotate a `SanshoDetail` in place with derived fields, before it is written
/// to `web/public/details/{cd}.json`. Applied per course during `convert`.
pub fn enrich(detail: &mut SanshoDetail) {
    for item in &mut detail.plan {
        item.kind = classify_plan_kind(&item.text, item.n);
    }
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
}
