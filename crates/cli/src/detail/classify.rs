//! Keyword-based classification of free-text syllabus values into stable enum
//! strings (`"exam"`, `"hybrid"`, …). The icon/color for each is a display
//! concern owned by `web/src/lib/syllabus-fields`.

/// Classify a grade-weight item label into an assessment `type`.
/// First match wins (most-specific first); falls back to `"other"`.
#[must_use]
pub fn eval_type(item: &str) -> &'static str {
    const TABLE: &[(&str, &[&str])] = &[
        ("exam", &["試験", "テスト", "定期"]),
        ("report", &["レポート", "課題", "提出", "作品", "小論"]),
        (
            "attendance",
            &[
                "出席",
                "参加",
                "受講態度",
                "授業態度",
                "平常",
                "意欲",
                "取り組み",
            ],
        ),
        ("presentation", &["発表", "プレゼン", "報告", "口頭"]),
        ("quiz", &["小テスト", "小テ", "クイズ", "確認テスト"]),
    ];
    // 「小テスト」(quiz) also contains「テスト」(exam), so scan quiz first.
    for kind in ["quiz", "exam", "report", "attendance", "presentation"] {
        if let Some((_, kws)) = TABLE.iter().find(|(k, _)| *k == kind)
            && kws.iter().any(|kw| item.contains(kw))
        {
            return kind_static(kind);
        }
    }
    // Bare 期末/中間 (e.g.「期末評価」) reads as an exam — but only when nothing
    // more specific matched:「期末レポート」is a report, same as the plan-kind
    // classifier treats it.
    if item.contains("期末") || item.contains("中間") {
        return "exam";
    }
    "other"
}

/// Map the transient `&str` back to a `'static` label.
fn kind_static(kind: &str) -> &'static str {
    match kind {
        "exam" => "exam",
        "report" => "report",
        "attendance" => "attendance",
        "presentation" => "presentation",
        "quiz" => "quiz",
        _ => "other",
    }
}

/// Classify the「授業実施方法」text into a delivery `mode`.
/// `hybrid` when both onsite and online signals appear; otherwise the single
/// matching signal; `unknown` when nothing matches.
#[must_use]
pub fn delivery_mode(raw: &str) -> &'static str {
    // 「非対面」contains「対面」; treat it as an online signal, not onsite.
    let onsite = raw.contains("対面") && !raw.contains("非対面");
    let ondemand = raw.contains("オンデマンド") || raw.contains("録画") || raw.contains("配信");
    let online = raw.contains("オンライン")
        || raw.contains("遠隔")
        || raw.contains("双方向")
        || raw.contains("同時");

    match (onsite, online || ondemand) {
        (true, true) => "hybrid",
        (true, false) => "onsite",
        (false, true) => {
            if ondemand && !raw.contains("双方向") && !raw.contains("同時") {
                "ondemand"
            } else {
                "online"
            }
        }
        (false, false) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{delivery_mode, eval_type};

    #[test]
    fn eval_types() {
        assert_eq!(eval_type("期末試験"), "exam");
        assert_eq!(eval_type("レポート"), "report");
        assert_eq!(eval_type("学習意欲・授業参加度"), "attendance");
        assert_eq!(eval_type("最終発表"), "presentation");
        assert_eq!(eval_type("毎回の小テスト"), "quiz");
        assert_eq!(eval_type("その他"), "other");
    }

    #[test]
    fn interim_final_qualifiers_do_not_hijack_the_category() {
        // 中間レポート + 期末レポート is a report course, exactly as
        // 中間試験 + 期末試験 is an exam course — the qualifier must not win.
        assert_eq!(eval_type("中間レポート"), "report");
        assert_eq!(eval_type("期末レポート"), "report");
        assert_eq!(eval_type("中間発表"), "presentation");
        // Bare 期末/中間 with no category word still reads as an exam.
        assert_eq!(eval_type("期末評価"), "exam");
        assert_eq!(eval_type("中間"), "exam");
    }

    #[test]
    fn eval_type_precedence_is_intentional() {
        // Characterization: for labels that hit several keyword groups the scan
        // order [quiz, exam, report, attendance, presentation] decides the
        // winner. Pinned so the precedence is a conscious choice, not accidental.
        assert_eq!(eval_type("レポート試験"), "exam"); // exam scanned before report
        assert_eq!(eval_type("発表課題"), "report"); // report before presentation
        assert_eq!(eval_type("出席課題"), "report"); // report before attendance
    }

    #[test]
    fn delivery_modes() {
        assert_eq!(
            delivery_mode("主に対面（全開講回数の過半数）、一部オンライン"),
            "hybrid"
        );
        assert_eq!(delivery_mode("すべて対面"), "onsite");
        assert_eq!(delivery_mode("オンライン（同時双方向型）"), "online");
        assert_eq!(delivery_mode("オンデマンド配信のみ"), "ondemand");
        assert_eq!(delivery_mode("未定"), "unknown");
    }

    #[test]
    fn hitaimen_is_not_treated_as_onsite() {
        // 「非対面」contains「対面」as a substring; a fully-online class must not
        // be reported as onsite/hybrid.
        assert_eq!(delivery_mode("非対面（オンデマンド）"), "ondemand");
        assert_eq!(delivery_mode("すべて非対面で実施（オンライン）"), "online");
        assert_eq!(delivery_mode("非対面"), "unknown");
    }
}
