//! Dictionary ordering for the v3 output.
//!
//! Semesters and campuses use a fixed domain order with unknown values sorted
//! last; departments and kubun are lexical with the `その他` catch-all pushed to
//! the end. A lexical tiebreak makes the ordering of equal-rank labels
//! deterministic.

use std::collections::BTreeSet;

/// The catch-all bucket label for empty dimension values, sorted last in the
/// otherwise-lexical dimensions.
const SONOTA_LABEL: &str = "その他";

/// Sort key for a semester label; unknown labels sort last.
fn semester_order(label: &str) -> u8 {
    match label {
        "1学期" => 0,
        "1学期前半" => 1,
        "1学期後半" => 2,
        "2学期" => 3,
        "2学期前半" => 4,
        "2学期後半" => 5,
        "通年" => 6,
        "前期" => 7,
        "後期" => 8,
        _ => 99,
    }
}

/// Sort key for a campus label; unknown labels sort last.
fn campus_order(label: &str) -> u8 {
    match label {
        "朝倉キャンパス" => 0,
        "物部キャンパス" => 1,
        "岡豊キャンパス" => 2,
        "その他" => 3,
        _ => 99,
    }
}

/// Semesters ordered by [`semester_order`], then lexically.
#[must_use]
pub fn sort_semesters(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, semester_order)
}

/// Kaikojiki share the semester ordering.
#[must_use]
pub fn sort_kaikojiki(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, semester_order)
}

/// Campuses ordered by [`campus_order`], then lexically.
#[must_use]
pub fn sort_campuses(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, campus_order)
}

/// Sort key that pushes the `その他` catch-all to the end of an otherwise
/// lexical dimension.
fn sonota_last(label: &str) -> u8 {
    u8::from(label == SONOTA_LABEL)
}

/// Departments in lexical (UTF-8 byte) order, with `その他` last.
#[must_use]
pub fn sort_departments(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, sonota_last)
}

/// Kubun in lexical order, with `その他` last.
#[must_use]
pub fn sort_kubun(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, sonota_last)
}

/// Collect a set into a vector ordered by `order` first, then lexically. Since
/// the source is a [`BTreeSet`] it is already lexical, so the lexical leg is the
/// stable tiebreak for equal-order (unknown) labels.
fn sorted_by_order(set: &BTreeSet<String>, order: fn(&str) -> u8) -> Vec<String> {
    let mut labels: Vec<String> = set.iter().cloned().collect();
    labels.sort_by(|a, b| order(a).cmp(&order(b)).then_with(|| a.cmp(b)));
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn semesters_follow_the_domain_order() {
        let got = sort_semesters(&set(&["通年", "2学期", "1学期", "1学期前半"]));
        assert_eq!(got, ["1学期", "1学期前半", "2学期", "通年"]);
    }

    #[test]
    fn unknown_semesters_sort_last_deterministically() {
        let got = sort_semesters(&set(&["集中講義", "1学期", "AAA"]));
        // 1学期 (order 0) first; the two unknowns (order 99) follow lexically.
        assert_eq!(got, ["1学期", "AAA", "集中講義"]);
    }

    #[test]
    fn campuses_follow_the_domain_order_then_lexical() {
        let got = sort_campuses(&set(&[
            "その他",
            "岡豊キャンパス",
            "朝倉キャンパス",
            "物部キャンパス",
        ]));
        assert_eq!(
            got,
            [
                "朝倉キャンパス",
                "物部キャンパス",
                "岡豊キャンパス",
                "その他"
            ]
        );
    }

    #[test]
    fn departments_are_lexical() {
        let got = sort_departments(&set(&["理工学部", "人文社会科学部", "医学部"]));
        let mut want = ["理工学部", "人文社会科学部", "医学部"];
        want.sort_unstable(); // UTF-8 byte order
        assert_eq!(got, want);
    }

    #[test]
    fn semester_domain_order_covers_every_label() {
        // Every domain label in a scrambled set must come back in domain order —
        // pins each arm of `semester_order`, and (via 通年 < 前期 < 後期, which is
        // the reverse of their lexical order) that the ordering is not lexical.
        let all = set(&[
            "後期",
            "前期",
            "通年",
            "2学期後半",
            "2学期前半",
            "2学期",
            "1学期後半",
            "1学期前半",
            "1学期",
        ]);
        assert_eq!(
            sort_semesters(&all),
            [
                "1学期",
                "1学期前半",
                "1学期後半",
                "2学期",
                "2学期前半",
                "2学期後半",
                "通年",
                "前期",
                "後期",
            ]
        );
    }

    #[test]
    fn the_last_known_semester_still_beats_an_unknown() {
        // 後期 has the highest domain order (8); dropping its arm would tie it with
        // the unknowns, so pin that a known label wins even against an unknown that
        // precedes it lexically (AAA < 後).
        let got = sort_semesters(&set(&["AAA", "後期"]));
        assert_eq!(got, ["後期", "AAA"]);
    }

    #[test]
    fn sonota_campus_sorts_before_a_lexically_earlier_unknown() {
        // その他 (order 3) sits between the known campuses and the unknowns, so it
        // must beat an unknown label even one that would precede it lexically.
        let got = sort_campuses(&set(&["あ大学", "その他"]));
        assert_eq!(got, ["その他", "あ大学"]);
    }

    #[test]
    fn sonota_catch_all_sorts_last_in_departments_and_kubun() {
        // その他 begins with a hiragana (U+305D) that would sort *before* the
        // kanji-led names lexically; the catch-all must still land last.
        let got = sort_departments(&set(&["その他", "理工学部", "医学部"]));
        assert_eq!(got.last().unwrap(), "その他");
        let got = sort_kubun(&set(&["その他", "講義", "演習"]));
        assert_eq!(got.last().unwrap(), "その他");
    }
}
