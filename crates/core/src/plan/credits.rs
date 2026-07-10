//! Credit and category tallies over a plan's registered courses.
//!
//! Three axes are aggregated because the data spreads "kind" across three
//! fields: `kbn` → 授業形態 (講義/演習…, via `Dictionaries::kubun`), `bunrui` →
//! 科目分類 (free text), and `nen` → the 必修/選択 marker. The UI leads with the
//! total and the 必修/選択 (`by_nen`) split; the others are there for whoever
//! wants them.

use std::collections::BTreeMap;

use crate::model::Course;

/// One category's rolled-up credits and course count.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryTally {
    pub key: String,
    pub credits: f32,
    pub count: u32,
}

/// A plan's credit summary across the three category axes.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditSummary {
    pub total_credits: f32,
    pub total_courses: u32,
    /// Courses whose `unit` could not be parsed as a number (shown as a caveat).
    pub uncredited: u32,
    pub by_kubun: Vec<CategoryTally>,
    pub by_bunrui: Vec<CategoryTally>,
    pub by_nen: Vec<CategoryTally>,
}

/// Parse a course's `unit` field into a credit count. Tolerant of the shapes
/// KULAS uses: `"2"`, `"2.0"`, `"2単位"` → `Some(2.0)`; empty/None/non-numeric →
/// `None`.
pub fn parse_unit(unit: Option<&str>) -> Option<f32> {
    let s = unit?.trim();
    let end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let head = &s[..end];
    if head.is_empty() {
        return None;
    }
    head.parse::<f32>().ok()
}

/// Summarize registered `courses` into per-axis tallies. `kubun_dict` resolves
/// each course's `kbn` index to its label.
pub fn summarize_credits<'a>(
    courses: impl IntoIterator<Item = &'a Course>,
    kubun_dict: &[String],
) -> CreditSummary {
    let mut total_credits = 0.0f32;
    let mut total_courses = 0u32;
    let mut uncredited = 0u32;
    let mut kubun: BTreeMap<String, (f32, u32)> = BTreeMap::new();
    let mut bunrui: BTreeMap<String, (f32, u32)> = BTreeMap::new();
    let mut nen: BTreeMap<String, (f32, u32)> = BTreeMap::new();

    for course in courses {
        total_courses += 1;
        let credit = parse_unit(course.unit.as_deref());
        match credit {
            Some(c) => total_credits += c,
            None => uncredited += 1,
        }
        let add = |map: &mut BTreeMap<String, (f32, u32)>, key: String| {
            let e = map.entry(key).or_insert((0.0, 0));
            e.0 += credit.unwrap_or(0.0);
            e.1 += 1;
        };
        if let Some(label) = kubun_dict.get(course.kbn as usize) {
            add(&mut kubun, label.clone());
        }
        if let Some(b) = course.bunrui.as_deref().filter(|s| !s.is_empty()) {
            add(&mut bunrui, b.to_owned());
        }
        if let Some(n) = course.nen.as_deref().filter(|s| !s.is_empty()) {
            add(&mut nen, n.to_owned());
        }
    }

    let tallies = |map: BTreeMap<String, (f32, u32)>| {
        map.into_iter()
            .map(|(key, (credits, count))| CategoryTally {
                key,
                credits,
                count,
            })
            .collect()
    };

    CreditSummary {
        total_credits,
        total_courses,
        uncredited,
        by_kubun: tallies(kubun),
        by_bunrui: tallies(bunrui),
        by_nen: tallies(nen),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_unit, summarize_credits};
    use crate::model::Course;

    fn course(unit: Option<&str>, kbn: u32, bunrui: Option<&str>, nen: Option<&str>) -> Course {
        Course {
            cd: "x".into(),
            nm: "x".into(),
            sub: None,
            prof: String::new(),
            raw: String::new(),
            slots: Vec::new(),
            ki: 0,
            kbn,
            dept: 0,
            campus: 0,
            gaku: None,
            gakka: None,
            nen: nen.map(str::to_owned),
            bunrui: bunrui.map(str::to_owned),
            bunya: None,
            pat: None,
            unit: unit.map(str::to_owned),
            dm: None,
            ev: None,
            st: String::new(),
        }
    }

    #[test]
    fn parse_unit_is_lenient() {
        assert_eq!(parse_unit(Some("2")), Some(2.0));
        assert_eq!(parse_unit(Some("2.0")), Some(2.0));
        assert_eq!(parse_unit(Some("2単位")), Some(2.0));
        assert_eq!(parse_unit(Some("1.5")), Some(1.5));
        assert_eq!(parse_unit(Some("")), None);
        assert_eq!(parse_unit(Some("なし")), None);
        assert_eq!(parse_unit(None), None);
    }

    #[test]
    fn totals_are_the_sum_of_parts_and_count_uncredited() {
        let kubun = ["講義".to_owned(), "演習".to_owned()];
        let courses = [
            course(Some("2"), 0, Some("専門"), Some("必修")),
            course(Some("1.5"), 1, Some("専門"), Some("選択")),
            course(None, 0, None, None), // uncredited
        ];
        let s = summarize_credits(courses.iter(), &kubun);
        assert_eq!(s.total_courses, 3);
        assert_eq!(s.uncredited, 1);
        assert!((s.total_credits - 3.5).abs() < 1e-6);
        // by_kubun: 講義 = 2.0 (course 0; course 2 has no credit) / 2 courses.
        let lecture = s.by_kubun.iter().find(|t| t.key == "講義").unwrap();
        assert_eq!(lecture.count, 2);
        assert!((lecture.credits - 2.0).abs() < 1e-6);
        // by_nen carries the 必修/選択 split.
        assert_eq!(s.by_nen.len(), 2);
        // The sum of each axis's credits equals the total (minus uncredited zeros).
        let sum_nen: f32 = s.by_nen.iter().map(|t| t.credits).sum();
        assert!((sum_nen - 3.5).abs() < 1e-6);
    }

    #[test]
    fn empty_plan_is_all_zero() {
        let s = summarize_credits(std::iter::empty(), &[]);
        assert_eq!(s.total_courses, 0);
        assert_eq!(s.total_credits, 0.0);
        assert!(s.by_kubun.is_empty() && s.by_bunrui.is_empty() && s.by_nen.is_empty());
    }
}
