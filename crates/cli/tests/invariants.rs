//! Model-based invariants over the full `convert_v3` → serialize → `Engine`
//! chain. Each invariant is checked two ways where possible — e.g. the semester
//! filter bitset is re-derived from the courses' own slots by an independent,
//! naive walk and compared to the precomputed index — so a divergence between
//! the two representations the consumer relies on (filter vs. grid) is caught.
//!
//! Driven both by the committed `sample_raw.json` (a deterministic baseline) and
//! by proptest-generated raw input (breadth).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use syllabus_core::bitset::BitSet;
use syllabus_core::model::{ProcessedData, RawCourse};
use syllabus_core::{Engine, Filters, convert_v3};

const PINNED_GENERATED_AT: &str = "2026-01-01T00:00:00Z";
const TSUUNEN_LABEL: &str = "通年";
const SONOTA_LABEL: &str = "その他";

fn build(raw: &[RawCourse]) -> ProcessedData {
    convert_v3(raw, PINNED_GENERATED_AT.to_owned()).data
}

fn decode(encoded: &[String], i: usize) -> BitSet {
    BitSet::from_base64(&encoded[i]).expect("valid base64 bitset")
}

/// The set of course indices a decoded bitset selects.
fn members(encoded: &[String], i: usize) -> BTreeSet<usize> {
    decode(encoded, i).iter_ones().collect()
}

/// All the input-agnostic invariants that must hold for any converted dataset.
fn check_invariants(data: &ProcessedData) {
    let n = data.courses.len();

    // (a) Every dictionary index is in range — no course points past its dict.
    for c in &data.courses {
        assert!((c.dept as usize) < data.dicts.departments.len(), "dept oob");
        assert!(
            (c.campus as usize) < data.dicts.campuses.len(),
            "campus oob"
        );
        assert!((c.kbn as usize) < data.dicts.kubun.len(), "kbn oob");
        assert!((c.ki as usize) < data.dicts.kaikojiki.len(), "ki oob");
        // (i) Codes are non-empty and already trimmed.
        assert!(!c.cd.is_empty());
        assert_eq!(c.cd.trim(), c.cd.as_str());
        // (f) Slots are real timetable cells.
        for s in &c.slots {
            assert!((0..=6).contains(&s.d), "day oob");
            assert!((1..=8).contains(&s.p), "period oob");
            assert!((s.s as usize) < data.dicts.semesters.len(), "semester oob");
        }
    }

    // Dictionaries are duplicate-free, and the「その他」catch-all sorts last in the
    // lexical dimensions (departments, kubun). (k)
    for dict in [
        &data.dicts.semesters,
        &data.dicts.departments,
        &data.dicts.campuses,
        &data.dicts.kubun,
        &data.dicts.kaikojiki,
    ] {
        let unique: BTreeSet<&String> = dict.iter().collect();
        assert_eq!(unique.len(), dict.len(), "dictionary has duplicates");
    }
    for dict in [&data.dicts.departments, &data.dicts.kubun] {
        if let Some(pos) = dict.iter().position(|d| d == SONOTA_LABEL) {
            assert_eq!(pos, dict.len() - 1, "その他 must sort last");
        }
    }

    // (j) Filter dictionaries are dense: every entry selects at least one course.
    for (dim, dict) in [
        (&data.indices.semester, &data.dicts.semesters),
        (&data.indices.department, &data.dicts.departments),
        (&data.indices.campus, &data.dicts.campuses),
    ] {
        assert_eq!(dim.len(), dict.len(), "index vector length != dict length");
        for i in 0..dict.len() {
            assert!(
                decode(dim, i).count_ones() >= 1,
                "dictionary entry {i} has an empty bitset"
            );
        }
    }

    // (b)/(c) Semester bitset membership, re-derived independently from each
    // course's own slots plus the 通年 propagation rule.
    let tsuunen_idx = data.dicts.semesters.iter().position(|s| s == TSUUNEN_LABEL);
    let tsuunen_courses: BTreeSet<usize> = (0..n)
        .filter(|&i| {
            tsuunen_idx.is_some_and(|t| data.courses[i].slots.iter().any(|s| s.s as usize == t))
        })
        .collect();
    for si in 0..data.dicts.semesters.len() {
        let mut expected: BTreeSet<usize> = (0..n)
            .filter(|&i| data.courses[i].slots.iter().any(|s| s.s as usize == si))
            .collect();
        if Some(si) != tsuunen_idx {
            expected.extend(&tsuunen_courses); // 通年 shows under every other semester
        }
        assert_eq!(
            members(&data.indices.semester, si),
            expected,
            "semester bitset {si} disagrees with slot-derived membership"
        );
    }

    // Department/campus bitsets follow directly from the resolved index fields.
    for di in 0..data.dicts.departments.len() {
        let expected: BTreeSet<usize> = (0..n)
            .filter(|&i| data.courses[i].dept as usize == di)
            .collect();
        assert_eq!(
            members(&data.indices.department, di),
            expected,
            "dept bitset {di}"
        );
    }
    for ci in 0..data.dicts.campuses.len() {
        let expected: BTreeSet<usize> = (0..n)
            .filter(|&i| data.courses[i].campus as usize == ci)
            .collect();
        assert_eq!(
            members(&data.indices.campus, ci),
            expected,
            "campus bitset {ci}"
        );
    }

    // (l) Producer→consumer round-trip: the serialized payload always rebuilds,
    // and the default (unfiltered) query returns every course.
    let json = serde_json::to_string(data).expect("serialize v3");
    let engine = Engine::from_json(&json).expect("engine rebuilds from converted data");
    assert_eq!(engine.filter(&Filters::default()).len(), n);
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn sample_fixture_satisfies_all_invariants() {
    let raw_json =
        fs::read_to_string(fixtures_dir().join("sample_raw.json")).expect("read fixture");
    let raw: Vec<RawCourse> = serde_json::from_str(&raw_json).expect("parse fixture");
    check_invariants(&build(&raw));
}

// --- Generators for breadth ---

fn label() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "",
        "理工学部",
        "共通教育",
        "農林海洋科学部",
        "1学期",
        "2学期",
        "通年",
        "朝倉キャンパス",
        "物部キャンパス",
        "講義",
        "演習",
    ])
    .prop_map(str::to_owned)
}

fn any_raw_course() -> impl Strategy<Value = RawCourse> {
    (
        "[a-z0-9]{0,4}",
        "[\\p{Han}a-z]{0,8}",
        label(),
        label(),
        label(),
        label(),
        prop::sample::select(vec![
            "1学期: 月曜日１時限",
            "通年: 火曜日３時限",
            "2学期: 金曜日５時限",
            "1学期: 月曜日１時限, 通年: 木曜日４時限",
            "1学期: 集中講義",
            "",
        ]),
    )
        .prop_map(|(cd, nm, dept, campus, kubun, kaiko, jik)| RawCourse {
            kogi_cd: cd,
            kogi_nm: nm,
            sekinin_busho_nm: dept,
            kochi_nm: campus,
            kogi_kubun_nm: kubun,
            kogi_kaikojiki_nm: kaiko,
            jikanwari: jik.to_owned(),
            ..Default::default()
        })
}

proptest! {
    #[test]
    fn generated_datasets_satisfy_all_invariants(
        raw in prop::collection::vec(any_raw_course(), 0..20)
    ) {
        check_invariants(&build(&raw));
    }
}

// --- (e) XSS: no raw HTML metacharacter survives into data.json bytes ---

proptest! {
    /// Whatever course text the raw data carries, the rendered `data.json` bytes
    /// contain no raw `<`, `>` or `&` (nor U+2028/U+2029) — the frontend injects
    /// these strings into the DOM, so a raw metacharacter would be an XSS vector.
    #[test]
    fn rendered_json_escapes_all_html_metacharacters(
        name in r#"[\p{Han}a-zA-Z<>&/"'0-9\u{2028}\u{2029} ]{0,30}"#,
        prof in r#"[\p{Han}a-zA-Z<>&/"' ]{0,20}"#,
    ) {
        let raw = vec![RawCourse {
            kogi_cd: "001".into(),
            kogi_nm: name,
            tanto_kyoin: prof,
            ..Default::default()
        }];
        let bytes = syllabus_cli::convert::render_data_json(
            &raw,
            "t".into(),
            true,
            &std::collections::HashMap::new(),
        )
        .expect("render")
        .bytes;
        let text = String::from_utf8(bytes).expect("utf8");
        let (ls, ps) = ('\u{2028}', '\u{2029}');
        prop_assert!(!text.contains('<'), "raw < in output");
        prop_assert!(!text.contains('>'), "raw > in output");
        prop_assert!(!text.contains('&'), "raw & in output");
        prop_assert!(!text.contains(ls));
        prop_assert!(!text.contains(ps));
    }
}
