//! Dictionary ordering for the v2 output — a faithful port of the Go pipeline's
//! `sortSemesters` / `sortCampuses` / `sortDepartments` / `sortKubun` /
//! `sortKaikojiki` (`internal/transform/{transform,v2}.go`).
//!
//! Semesters and campuses use a fixed domain order with unknown values sorted
//! last; departments and kubun are plain lexical. Go's `sortSemesters` has no
//! tiebreak (its only ties are unknown values, which the dataset never has and
//! Go itself leaves non-deterministic), so we add a lexical tiebreak: identical
//! output for the known values the data contains, and *deterministic* for the
//! unknowns Go left to chance.

use std::collections::BTreeSet;

/// Sort key for a semester label; unknown labels sort last, as in Go's
/// `semesterOrder` map (`internal/transform/transform.go`).
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

/// Sort key for a campus label; unknown labels sort last, as in Go's
/// `campusOrder` map (`internal/transform/v2.go`).
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

/// Kaikojiki share the semester ordering (Go's `sortKaikojiki` delegates to
/// `sortSemesters`).
#[must_use]
pub fn sort_kaikojiki(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, semester_order)
}

/// Campuses ordered by [`campus_order`], then lexically.
#[must_use]
pub fn sort_campuses(set: &BTreeSet<String>) -> Vec<String> {
    sorted_by_order(set, campus_order)
}

/// Departments in plain lexical (UTF-8 byte) order — matches Go's `sort.Strings`.
#[must_use]
pub fn sort_departments(set: &BTreeSet<String>) -> Vec<String> {
    set.iter().cloned().collect()
}

/// Kubun in plain lexical order — matches Go's `sort.Strings`.
#[must_use]
pub fn sort_kubun(set: &BTreeSet<String>) -> Vec<String> {
    set.iter().cloned().collect()
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
        want.sort_unstable(); // UTF-8 byte order, same as Go sort.Strings
        assert_eq!(got, want);
    }
}
