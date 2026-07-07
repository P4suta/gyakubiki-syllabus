//! Build the optimized `data.json` — dictionaries plus per-dimension bitsets —
//! from raw KULAS courses.
//!
//! `generated_at` is injected by the caller, keeping conversion pure and
//! deterministic for a given input.

use std::collections::{BTreeSet, HashMap};

use crate::bitset::BitSet;
use crate::dict;
use crate::model::{Course, Dictionaries, IndicesMap, ProcessedData, RawCourse, Slot};
use crate::parser::{self, ParsedSlot};
use crate::text::search_text;

/// The v3 payload plus any warnings raised while converting (empty course codes,
/// unparsable jikanwari, …). Warnings are surfaced by the CLI, never in `data.json`.
pub struct ConvertResult {
    pub data: ProcessedData,
    pub warnings: Vec<String>,
}

/// The 通年 (year-long) semester label, whose courses appear under every other
/// semester's filter.
const TSUUNEN_LABEL: &str = "通年";

/// Convert raw KULAS courses into the v3 output, stamping `generated_at`
/// (an RFC 3339 string).
///
/// Three phases: [`first_pass`] dedups courses and gathers the dictionary
/// values, [`build_dictionaries`] sorts them into the wire dictionaries, and
/// [`second_pass`] resolves each course's indices and builds the filter bitsets.
#[must_use]
pub fn convert_v2(raw: &[RawCourse], generated_at: String) -> ConvertResult {
    let first = first_pass(raw);
    let dicts = build_dictionaries(&first.dict_sets);
    let dict_index = DictIndex::from(&dicts);
    let (courses, indices) = second_pass(
        raw,
        first.courses,
        &first.slots_per_course,
        &dict_index,
        &dicts,
    );

    ConvertResult {
        data: ProcessedData {
            version: 3,
            generated_at,
            year: dataset_year(raw),
            total_raw: raw.len() as u32,
            dicts,
            indices,
            courses,
        },
        warnings: first.warnings,
    }
}

/// The dictionary value sets gathered in the first pass (before dedup chooses a
/// canonical course), one [`BTreeSet`] per dimension for stable ordering.
#[derive(Default)]
struct DictSets {
    semester: BTreeSet<String>,
    department: BTreeSet<String>,
    campus: BTreeSet<String>,
    kubun: BTreeSet<String>,
    kaikojiki: BTreeSet<String>,
}

/// The first pass output: deduped courses (index fields still 0, slots still
/// empty), their parsed slots (parallel to `courses`), the gathered dictionary
/// value sets, and any warnings.
struct FirstPass {
    courses: Vec<Course>,
    slots_per_course: Vec<Vec<ParsedSlot>>,
    dict_sets: DictSets,
    warnings: Vec<String>,
}

/// First pass: dedup by trimmed `kogiCd` (first wins, later occurrences merge
/// their slots), gather the five dictionary value sets, and collect warnings.
/// Index fields stay 0 and slots empty until [`second_pass`] fills them.
fn first_pass(raw: &[RawCourse]) -> FirstPass {
    let mut warnings = Vec::new();
    let mut dict_sets = DictSets::default();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut courses: Vec<Course> = Vec::new();
    let mut slots_per_course: Vec<Vec<ParsedSlot>> = Vec::new();

    for (i, r) in raw.iter().enumerate() {
        let cd = r.kogi_cd.trim();
        if cd.is_empty() {
            warnings.push(format!(
                "  [item {}] course code (kogiCd) is empty; skipping",
                i + 1
            ));
            continue;
        }
        let nm = r.kogi_nm.trim();
        if nm.is_empty() {
            warnings.push(format!("  [{}] course name (kogiNm) is empty", r.kogi_cd));
        }

        let parsed = parser::parse_jikanwari(&r.jikanwari);
        for w in &parsed.warnings {
            warnings.push(format!("  [{}] {}: {}", r.kogi_cd, r.kogi_nm, w));
        }
        if !r.jikanwari.is_empty() && parsed.slots.is_empty() {
            warnings.push(format!(
                "  [{}] {}: has jikanwari but it could not be parsed: {:?}",
                r.kogi_cd, r.kogi_nm, r.jikanwari
            ));
        }

        // Slots merge across duplicates, so every occurrence's semesters count.
        for s in &parsed.slots {
            if !s.semester.is_empty() {
                dict_sets.semester.insert(s.semester.clone());
            }
        }

        // Duplicate code: merge its slots into the first occurrence. Its other
        // fields are discarded, so they must NOT be gathered into the
        // dictionaries — otherwise a dropped record's department leaves a filter
        // entry that selects no course.
        if let Some(&idx) = seen.get(cd) {
            merge_slots(&mut slots_per_course[idx], &parsed.slots);
            continue;
        }

        // First occurrence is canonical: gather its dimension values.
        let dept = r.sekinin_busho_nm.trim();
        dict_sets.department.insert(or_sonota(dept).to_owned());
        dict_sets.campus.insert(campus_of(r).to_owned());
        dict_sets
            .kubun
            .insert(or_sonota(r.kogi_kubun_nm.trim()).to_owned());
        dict_sets
            .kaikojiki
            .insert(or_sonota(r.kogi_kaikojiki_nm.trim()).to_owned());

        seen.insert(cd.to_owned(), courses.len());
        let prof = r.tanto_kyoin.trim();
        let sub = trim_opt(&r.fukudai);
        let gakusoku = r.gakusoku_kamoku_nm.trim();
        let gaku = (!gakusoku.is_empty() && gakusoku != nm).then(|| gakusoku.to_owned());
        courses.push(Course {
            cd: cd.to_owned(),
            nm: nm.to_owned(),
            st: search_text(nm, sub.as_deref(), prof, cd, dept),
            sub,
            prof: prof.to_owned(),
            raw: r.jikanwari.clone(),
            slots: Vec::new(), // filled in the second pass
            ki: 0,
            kbn: 0,
            dept: 0,
            campus: 0,
            gaku,
            gakka: trim_opt(&r.taisho_gakka),
            nen: trim_opt(&r.taisho_nenji),
            bunrui: trim_opt(&r.kamoku_bunrui),
            bunya: trim_opt(&r.kamoku_bunya),
            pat: trim_opt(&r.syllabus_komoku_pattern_id),
            unit: None, // detail-derived; filled by the CLI's enrichment pass
            dm: None,
            ev: None,
        });
        slots_per_course.push(parsed.slots);
    }

    FirstPass {
        courses,
        slots_per_course,
        dict_sets,
        warnings,
    }
}

/// Sort the gathered value sets into the wire dictionaries (each dimension has
/// its own ordering rule — see [`crate::dict`]).
fn build_dictionaries(sets: &DictSets) -> Dictionaries {
    Dictionaries {
        semesters: dict::sort_semesters(&sets.semester),
        departments: dict::sort_departments(&sets.department),
        campuses: dict::sort_campuses(&sets.campus),
        kubun: dict::sort_kubun(&sets.kubun),
        kaikojiki: dict::sort_kaikojiki(&sets.kaikojiki),
    }
}

/// Label → index lookups for every dictionary, built once so the second pass can
/// resolve each course's dimensions without re-scanning the dictionaries.
struct DictIndex {
    semester: HashMap<String, usize>,
    department: HashMap<String, usize>,
    campus: HashMap<String, usize>,
    kubun: HashMap<String, usize>,
    kaikojiki: HashMap<String, usize>,
}

impl From<&Dictionaries> for DictIndex {
    fn from(dicts: &Dictionaries) -> Self {
        Self {
            semester: index_map(&dicts.semesters),
            department: index_map(&dicts.departments),
            campus: index_map(&dicts.campuses),
            kubun: index_map(&dicts.kubun),
            kaikojiki: index_map(&dicts.kaikojiki),
        }
    }
}

/// Second pass: resolve each course's `dept`/`campus`/`kbn`/`ki` and its slots,
/// then build the per-dimension filter bitsets. 通年 courses are propagated into
/// every other semester's bitset, the way the filter UI expects.
fn second_pass(
    raw: &[RawCourse],
    mut courses: Vec<Course>,
    slots_per_course: &[Vec<ParsedSlot>],
    dict_index: &DictIndex,
    dicts: &Dictionaries,
) -> (Vec<Course>, IndicesMap) {
    let num_words = courses.len().div_ceil(64);

    // kogiCd → first raw index, so each course re-reads its canonical record.
    let mut raw_first: HashMap<&str, usize> = HashMap::new();
    for (i, r) in raw.iter().enumerate() {
        raw_first.entry(r.kogi_cd.trim()).or_insert(i);
    }

    // One bitset per semester dictionary index (positional, dense).
    let mut sem_bits = vec![BitSet::with_words(num_words); dicts.semesters.len()];
    let mut tsuunen_courses: Vec<usize> = Vec::new();

    for i in 0..courses.len() {
        let r = &raw[raw_first[courses[i].cd.as_str()]];
        courses[i].dept = lookup(&dict_index.department, or_sonota(r.sekinin_busho_nm.trim()));
        courses[i].campus = lookup(&dict_index.campus, campus_of(r));
        courses[i].kbn = lookup(&dict_index.kubun, or_sonota(r.kogi_kubun_nm.trim()));
        courses[i].ki = lookup(&dict_index.kaikojiki, or_sonota(r.kogi_kaikojiki_nm.trim()));

        let mut slots = Vec::new();
        let mut has_tsuunen = false;
        for s in &slots_per_course[i] {
            let (Some(&si), Some(di)) = (dict_index.semester.get(&s.semester), day_index(&s.day))
            else {
                continue;
            };
            slots.push(Slot {
                s: si as u32,
                d: di,
                p: s.period,
            });
            sem_bits[si].set(i);
            if s.semester == TSUUNEN_LABEL {
                has_tsuunen = true;
            }
        }
        courses[i].slots = slots;
        if has_tsuunen {
            tsuunen_courses.push(i);
        }
    }

    // 通年 courses appear under every *other* semester's filter.
    if let Some(&tsuunen_idx) = dict_index.semester.get(TSUUNEN_LABEL) {
        for (sem, bits) in sem_bits.iter_mut().enumerate() {
            if sem == tsuunen_idx {
                continue;
            }
            for &course in &tsuunen_courses {
                bits.set(course);
            }
        }
    }

    // dept/campus bitsets follow directly from the resolved index fields.
    let dept_bits = dimension_bitsets(&courses, dicts.departments.len(), num_words, |c| {
        c.dept as usize
    });
    let campus_bits = dimension_bitsets(&courses, dicts.campuses.len(), num_words, |c| {
        c.campus as usize
    });

    let indices = IndicesMap {
        semester: encode(&sem_bits),
        department: encode(&dept_bits),
        campus: encode(&campus_bits),
    };
    (courses, indices)
}

/// The dataset's academic year: the first non-empty `kaikoNendo` across the raw
/// records (the whole fetch is one year, so this is uniform). Empty when absent.
fn dataset_year(raw: &[RawCourse]) -> String {
    raw.iter()
        .find_map(|r| {
            r.kaiko_nendo
                .as_deref()
                .map(str::trim)
                .filter(|y| !y.is_empty())
        })
        .unwrap_or_default()
        .to_owned()
}

/// The catch-all bucket label for empty dimension values.
const SONOTA_LABEL: &str = "その他";

/// The label to file a dimension value under, mapping empty values to `その他` so
/// a course with no department/kubun/kaikojiki/campus gets its own dictionary
/// entry instead of silently landing on index 0 — a real, unrelated value.
fn or_sonota(label: &str) -> &str {
    if label.is_empty() {
        SONOTA_LABEL
    } else {
        label
    }
}

/// The campus label, defaulting empty values to `その他`.
fn campus_of(r: &RawCourse) -> &str {
    or_sonota(r.kochi_nm.trim())
}

/// Trim an optional string, collapsing `None`/empty to `None`.
fn trim_opt(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

/// Append `new` slots not already present, deduplicating by value.
fn merge_slots(existing: &mut Vec<ParsedSlot>, new: &[ParsedSlot]) {
    for slot in new {
        if !existing.contains(slot) {
            existing.push(slot.clone());
        }
    }
}

/// Map each dictionary label to its index.
fn index_map(labels: &[String]) -> HashMap<String, usize> {
    labels
        .iter()
        .enumerate()
        .map(|(i, label)| (label.clone(), i))
        .collect()
}

/// Resolve a label to its dictionary index. Callers normalize empty values to
/// `その他` (see [`or_sonota`]) before calling, so every real value is present;
/// the `0` fallback is only a defensive net for a genuinely unknown key (e.g. an
/// empty dictionary).
fn lookup(idx: &HashMap<String, usize>, key: &str) -> u32 {
    idx.get(key).copied().unwrap_or(0) as u32
}

/// Day label → column index (0=月 … 6=日).
fn day_index(day: &str) -> Option<i32> {
    match day {
        "月" => Some(0),
        "火" => Some(1),
        "水" => Some(2),
        "木" => Some(3),
        "金" => Some(4),
        "土" => Some(5),
        "日" => Some(6),
        _ => None,
    }
}

/// Build a dimension's positional bitsets from already-resolved courses: each
/// course sets its bit in the bucket its `project`ed dictionary index names.
/// An index outside `dict_len` — reachable only when the dictionary is empty and
/// `lookup` fell back to 0 — is skipped rather than panicking.
fn dimension_bitsets(
    courses: &[Course],
    dict_len: usize,
    num_words: usize,
    project: impl Fn(&Course) -> usize,
) -> Vec<BitSet> {
    let mut bitsets = vec![BitSet::with_words(num_words); dict_len];
    for (course, c) in courses.iter().enumerate() {
        if let Some(bucket) = bitsets.get_mut(project(c)) {
            bucket.set(course);
        }
    }
    bitsets
}

/// Base64-encode a dimension's positional bitsets (vector index = dictionary
/// index).
fn encode(bitsets: &[BitSet]) -> Vec<String> {
    bitsets.iter().map(BitSet::to_base64).collect()
}

#[cfg(test)]
mod tests {
    use super::convert_v2;
    use crate::bitset::BitSet;
    use crate::model::{ProcessedData, RawCourse};

    /// A raw course with the given code/name; other fields default to empty.
    fn raw(cd: &str, nm: &str) -> RawCourse {
        RawCourse {
            kogi_cd: cd.to_owned(),
            kogi_nm: nm.to_owned(),
            ..Default::default()
        }
    }

    fn convert(raw: &[RawCourse]) -> ProcessedData {
        convert_v2(raw, "2026-05-31T00:00:00Z".to_owned()).data
    }

    /// Position of `label` in a dictionary.
    fn pos(dict: &[String], label: &str) -> usize {
        dict.iter().position(|s| s == label).expect("label present")
    }

    /// The bitset for dictionary index `idx` in a positional `[base64]` vector.
    fn bitset(encoded: &[String], idx: usize) -> BitSet {
        BitSet::from_base64(&encoded[idx]).expect("valid base64")
    }

    #[test]
    fn basic_shape() {
        let data = convert(&[
            RawCourse {
                jikanwari: "1学期: 月曜日１時限".into(),
                ..raw("001", "基礎数学")
            },
            RawCourse {
                jikanwari: "2学期: 火曜日２時限".into(),
                ..raw("002", "政治学概論")
            },
        ]);
        assert_eq!(data.version, 3);
        assert_eq!(data.total_raw, 2);
        assert_eq!(data.courses.len(), 2);
    }

    #[test]
    fn dictionaries_are_ordered() {
        let data = convert(&[
            RawCourse {
                jikanwari: "1学期: 月曜日１時限".into(),
                kogi_kaikojiki_nm: "1学期".into(),
                kogi_kubun_nm: "講義".into(),
                sekinin_busho_nm: "理工学部".into(),
                kochi_nm: "朝倉キャンパス".into(),
                ..raw("001", "A")
            },
            RawCourse {
                jikanwari: "2学期: 火曜日２時限".into(),
                kogi_kaikojiki_nm: "2学期".into(),
                kogi_kubun_nm: "演習".into(),
                sekinin_busho_nm: "人文社会科学部".into(),
                kochi_nm: "物部キャンパス".into(),
                ..raw("002", "B")
            },
        ]);
        assert_eq!(data.dicts.semesters, ["1学期", "2学期"]);
        assert_eq!(data.dicts.campuses, ["朝倉キャンパス", "物部キャンパス"]);
        assert_eq!(data.dicts.kaikojiki, ["1学期", "2学期"]);
    }

    #[test]
    fn campus_sort_order() {
        let data = convert(&[
            RawCourse {
                kochi_nm: "その他".into(),
                ..raw("001", "A")
            },
            RawCourse {
                kochi_nm: "岡豊キャンパス".into(),
                ..raw("002", "B")
            },
            RawCourse {
                kochi_nm: "朝倉キャンパス".into(),
                ..raw("003", "C")
            },
            RawCourse {
                kochi_nm: "物部キャンパス".into(),
                ..raw("004", "D")
            },
        ]);
        assert_eq!(
            data.dicts.campuses,
            [
                "朝倉キャンパス",
                "物部キャンパス",
                "岡豊キャンパス",
                "その他"
            ]
        );
    }

    #[test]
    fn course_indices_resolve() {
        let data = convert(&[RawCourse {
            jikanwari: "1学期: 月曜日１時限".into(),
            kogi_kaikojiki_nm: "1学期".into(),
            kogi_kubun_nm: "講義".into(),
            sekinin_busho_nm: "理工学部".into(),
            kochi_nm: "朝倉キャンパス".into(),
            ..raw("001", "基礎数学")
        }]);
        let c = &data.courses[0];
        assert_eq!(data.dicts.kaikojiki[c.ki as usize], "1学期");
        assert_eq!(data.dicts.kubun[c.kbn as usize], "講義");
        assert_eq!(data.dicts.departments[c.dept as usize], "理工学部");
        assert_eq!(data.dicts.campuses[c.campus as usize], "朝倉キャンパス");
    }

    #[test]
    fn slot_indices() {
        let data = convert(&[RawCourse {
            jikanwari: "1学期: 月曜日１時限, 2学期: 火曜日２時限".into(),
            ..raw("001", "A")
        }]);
        let c = &data.courses[0];
        assert_eq!(c.slots.len(), 2);
        assert_eq!(data.dicts.semesters[c.slots[0].s as usize], "1学期");
        assert_eq!((c.slots[0].d, c.slots[0].p), (0, 1)); // 月, 1限
        assert_eq!(data.dicts.semesters[c.slots[1].s as usize], "2学期");
        assert_eq!((c.slots[1].d, c.slots[1].p), (1, 2)); // 火, 2限
    }

    #[test]
    fn gakusoku_kept_only_when_different() {
        let data = convert(&[
            RawCourse {
                gakusoku_kamoku_nm: "基礎数学".into(),
                ..raw("001", "基礎数学")
            },
            RawCourse {
                gakusoku_kamoku_nm: "基礎数学".into(),
                ..raw("002", "基礎数学A")
            },
        ]);
        assert_eq!(data.courses[0].gaku, None);
        assert_eq!(data.courses[1].gaku.as_deref(), Some("基礎数学"));
    }

    #[test]
    fn search_text_is_normalized() {
        let data = convert(&[RawCourse {
            fukudai: Some("副題テスト".into()),
            tanto_kyoin: "Smith\u{3000}John".into(),
            sekinin_busho_nm: "共通教育".into(),
            ..raw("ABC", "English Communication")
        }]);
        let st = &data.courses[0].st;
        assert!(!st.contains("English"));
        assert!(st.contains("english communication"));
        assert!(!st.contains('\u{3000}'));
        assert!(st.contains("副題テスト"));
        assert!(st.contains("smith john"));
    }

    #[test]
    fn dedups_by_code_and_merges_slots() {
        let data = convert(&[
            RawCourse {
                jikanwari: "1学期: 月曜日１時限".into(),
                ..raw("001", "A")
            },
            RawCourse {
                jikanwari: "1学期: 月曜日１時限, 1学期: 水曜日３時限".into(),
                ..raw("001", "A")
            },
        ]);
        assert_eq!(data.courses.len(), 1);
        assert_eq!(data.courses[0].slots.len(), 2);
    }

    #[test]
    fn dedup_does_not_leave_dead_dictionary_entries() {
        // Two records share a code; the second (discarded) has a different
        // department. That department must NOT become a filter entry, since no
        // surviving course belongs to it.
        let data = convert(&[
            RawCourse {
                sekinin_busho_nm: "理工学部".into(),
                ..raw("001", "A")
            },
            RawCourse {
                sekinin_busho_nm: "共通教育".into(),
                ..raw("001", "A")
            },
        ]);
        assert_eq!(data.courses.len(), 1);
        assert_eq!(data.dicts.departments, ["理工学部"]);
        assert!(!data.dicts.departments.contains(&"共通教育".to_owned()));
    }

    #[test]
    fn skips_empty_code_with_warning() {
        let result = convert_v2(&[raw("", "空コード"), raw("001", "正常")], "t".to_owned());
        assert_eq!(result.data.courses.len(), 1);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn empty_input_is_valid_v3() {
        let data = convert(&[]);
        assert_eq!(data.courses.len(), 0);
        assert_eq!(data.version, 3);
    }

    #[test]
    fn semester_bitset_marks_the_right_courses() {
        let data = convert(&[
            RawCourse {
                jikanwari: "1学期: 月曜日１時限".into(),
                kochi_nm: "朝倉キャンパス".into(),
                ..raw("001", "A")
            },
            RawCourse {
                jikanwari: "2学期: 火曜日２時限".into(),
                kochi_nm: "物部キャンパス".into(),
                ..raw("002", "B")
            },
            RawCourse {
                jikanwari: "1学期: 水曜日３時限".into(),
                kochi_nm: "朝倉キャンパス".into(),
                ..raw("003", "C")
            },
        ]);
        let sem = bitset(&data.indices.semester, pos(&data.dicts.semesters, "1学期"));
        assert!(sem.has(0) && !sem.has(1) && sem.has(2));
        let camp = bitset(
            &data.indices.campus,
            pos(&data.dicts.campuses, "朝倉キャンパス"),
        );
        assert!(camp.has(0) && !camp.has(1) && camp.has(2));
    }

    #[test]
    fn tsuunen_propagates_into_every_semester() {
        let data = convert(&[
            RawCourse {
                jikanwari: "1学期: 月曜日１時限".into(),
                ..raw("001", "A")
            },
            RawCourse {
                jikanwari: "通年: 火曜日２時限".into(),
                ..raw("002", "B")
            },
            RawCourse {
                jikanwari: "2学期: 水曜日３時限".into(),
                ..raw("003", "C")
            },
        ]);
        let sem = bitset(&data.indices.semester, pos(&data.dicts.semesters, "1学期"));
        // course 0 (1学期) and course 1 (通年) appear; course 2 (2学期) does not.
        assert!(sem.has(0) && sem.has(1) && !sem.has(2));
    }

    #[test]
    fn unknown_campus_sorts_after_known_lexically() {
        let data = convert(&[
            RawCourse {
                kochi_nm: "未知のキャンパスB".into(),
                ..raw("001", "A")
            },
            RawCourse {
                kochi_nm: "未知のキャンパスA".into(),
                ..raw("002", "B")
            },
            RawCourse {
                kochi_nm: "朝倉キャンパス".into(),
                ..raw("003", "C")
            },
        ]);
        assert_eq!(
            data.dicts.campuses,
            ["朝倉キャンパス", "未知のキャンパスA", "未知のキャンパスB"]
        );
    }

    #[test]
    fn empty_campus_maps_to_sonota() {
        let data = convert(&[
            RawCourse {
                kochi_nm: String::new(),
                ..raw("001", "A")
            },
            RawCourse {
                kochi_nm: "朝倉キャンパス".into(),
                ..raw("002", "B")
            },
        ]);
        assert_eq!(
            data.dicts.campuses[data.courses[0].campus as usize],
            "その他"
        );
    }

    #[test]
    fn empty_department_maps_to_sonota_not_index_zero() {
        // Regression: an empty 学部/区分/開講時期 used to fall back to dictionary
        // index 0 — a real, unrelated value — and pollute that value's filter
        // bitset. It must instead resolve to its own "その他" entry.
        let data = convert(&[
            RawCourse {
                sekinin_busho_nm: "理工学部".into(),
                kogi_kubun_nm: "講義".into(),
                kogi_kaikojiki_nm: "1学期".into(),
                ..raw("001", "A")
            },
            RawCourse {
                sekinin_busho_nm: String::new(),
                kogi_kubun_nm: String::new(),
                kogi_kaikojiki_nm: String::new(),
                ..raw("002", "B")
            },
        ]);

        // The empty-valued course resolves to "その他" in all three dimensions.
        assert_eq!(
            data.dicts.departments[data.courses[1].dept as usize],
            "その他"
        );
        assert_eq!(data.dicts.kubun[data.courses[1].kbn as usize], "その他");
        assert_eq!(data.dicts.kaikojiki[data.courses[1].ki as usize], "その他");

        // It is NOT attributed to the real "理工学部" at whatever index that holds,
        // and the 理工学部 department bitset does not include the empty course.
        let rigaku = pos(&data.dicts.departments, "理工学部");
        assert_ne!(data.courses[1].dept as usize, rigaku);
        let rigaku_bits = bitset(&data.indices.department, rigaku);
        assert!(rigaku_bits.has(0)); // course 0 (理工学部) is present
        assert!(!rigaku_bits.has(1)); // course 1 (empty) must not be
    }

    #[test]
    fn preserves_raw_jikanwari_and_optional_fields() {
        let data = convert(&[RawCourse {
            jikanwari: "1学期: 集中講義".into(),
            taisho_gakka: Some("理工学部".into()),
            taisho_nenji: Some("1年".into()),
            kamoku_bunrui: Some("専門".into()),
            kamoku_bunya: Some("数学".into()),
            ..raw("001", "集中講義")
        }]);
        let c = &data.courses[0];
        assert_eq!(c.raw, "1学期: 集中講義");
        assert_eq!(c.gakka.as_deref(), Some("理工学部"));
        assert_eq!(c.nen.as_deref(), Some("1年"));
        assert_eq!(c.bunrui.as_deref(), Some("専門"));
        assert_eq!(c.bunya.as_deref(), Some("数学"));
    }

    #[test]
    fn carries_pattern_id_and_dataset_year() {
        let data = convert(&[
            RawCourse {
                syllabus_komoku_pattern_id: Some("4".into()),
                kaiko_nendo: Some("2026".into()),
                ..raw("001", "A")
            },
            RawCourse {
                syllabus_komoku_pattern_id: Some("5".into()),
                kaiko_nendo: Some("2026".into()),
                ..raw("002", "B")
            },
        ]);
        assert_eq!(data.year, "2026");
        assert_eq!(data.courses[0].pat.as_deref(), Some("4"));
        assert_eq!(data.courses[1].pat.as_deref(), Some("5"));
    }

    #[test]
    fn warns_on_unparsable_jikanwari_but_keeps_course() {
        let result = convert_v2(
            &[RawCourse {
                jikanwari: "集中".into(),
                ..raw("001", "集中講義")
            }],
            "t".to_owned(),
        );
        assert_eq!(result.data.courses.len(), 1);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn empty_jikanwari_yields_no_slots_no_warning() {
        let result = convert_v2(&[raw("001", "A")], "t".to_owned());
        assert_eq!(result.data.courses[0].slots.len(), 0);
        assert!(result.warnings.is_empty());
    }

    use proptest::prelude::*;

    fn label() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "",
            "理工学部",
            "共通教育",
            "1学期",
            "通年",
            "朝倉キャンパス",
            "講義",
        ])
        .prop_map(str::to_owned)
    }

    fn any_raw_course() -> impl Strategy<Value = RawCourse> {
        (
            "[a-z0-9]{0,4}", // cd (may be empty → skipped with a warning)
            "[\\p{Han}a-z]{0,8}",
            label(),
            label(),
            label(),
            label(),
            prop::sample::select(vec![
                "1学期: 月曜日１時限",
                "通年: 火曜日３時限",
                "1学期: 集中講義",
                "1学期: 月曜日１時限, 2学期: 金曜日５時限",
                "",
                "garbage",
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
        /// Whatever the raw input, the v3 output is structurally sound: counts
        /// hold, every dictionary index is in range, every slot is a real cell,
        /// and codes/search text are canonical. This is the core safety net for
        /// the whole conversion.
        #[test]
        fn convert_output_is_structurally_sound(
            raw in prop::collection::vec(any_raw_course(), 0..12)
        ) {
            let full_width_space = '\u{3000}';
            let data = convert(&raw);

            prop_assert_eq!(data.total_raw as usize, raw.len());
            prop_assert!(data.courses.len() <= raw.len());

            for c in &data.courses {
                prop_assert!(!c.cd.is_empty());
                prop_assert_eq!(c.cd.trim(), c.cd.as_str()); // trimmed
                prop_assert!((c.dept as usize) < data.dicts.departments.len());
                prop_assert!((c.campus as usize) < data.dicts.campuses.len());
                prop_assert!((c.kbn as usize) < data.dicts.kubun.len());
                prop_assert!((c.ki as usize) < data.dicts.kaikojiki.len());
                for s in &c.slots {
                    prop_assert!((0..=6).contains(&s.d));
                    prop_assert!((1..=8).contains(&s.p));
                    prop_assert!((s.s as usize) < data.dicts.semesters.len());
                }
                // Search haystack is normalized (no full-width space, no upper ASCII).
                prop_assert!(!c.st.contains(full_width_space));
                prop_assert!(!c.st.bytes().any(|b| b.is_ascii_uppercase()));
            }
        }
    }
}
