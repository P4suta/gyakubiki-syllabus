//! Port of Go's `ConvertV2` (`internal/transform/v2.go`): raw KULAS courses →
//! the optimized v2 `data.json` (dictionaries + per-dimension bitsets).
//!
//! The output is byte-identical to the Go pipeline (enforced by the `parity` CI
//! gate), so this is a drop-in replacement for `syllabus-cli convert --v2`.
//! `generated_at` is injected by the caller, keeping this function pure and its
//! output deterministic for a given input.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::bitset::BitSet;
use crate::dict;
use crate::model::{CourseV2, Dictionaries, IndicesMap, ProcessedDataV2, RawCourse, SlotV2};
use crate::parser::{self, Slot};
use crate::text::search_text;

/// The v2 payload plus any warnings raised while converting (empty course codes,
/// unparsable jikanwari, …). Warnings are surfaced by the CLI; they never reach
/// `data.json`.
pub struct ConvertResult {
    pub data: ProcessedDataV2,
    pub warnings: Vec<String>,
}

/// Convert raw KULAS courses into the v2 output, stamping `generated_at`
/// (an RFC 3339 string) into the payload.
#[must_use]
pub fn convert_v2(raw: &[RawCourse], generated_at: String) -> ConvertResult {
    let mut warnings = Vec::new();

    // Dictionary value sets, collected across every raw record (before dedup),
    // exactly as the Go first pass does.
    let mut semester_set = BTreeSet::new();
    let mut dept_set = BTreeSet::new();
    let mut campus_set = BTreeSet::new();
    let mut kubun_set = BTreeSet::new();
    let mut kaikojiki_set = BTreeSet::new();

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut courses: Vec<CourseV2> = Vec::new();
    let mut slots_per_course: Vec<Vec<Slot>> = Vec::new();

    // --- First pass: dedup courses by code, collect dictionary values ---
    for (i, r) in raw.iter().enumerate() {
        let cd = r.kogi_cd.trim();
        if cd.is_empty() {
            warnings.push(format!(
                "  [{}件目] 授業コード(kogiCd)が空です。スキップします",
                i + 1
            ));
            continue;
        }
        let nm = r.kogi_nm.trim();
        if nm.is_empty() {
            warnings.push(format!("  [{}] 科目名(kogiNm)が空です", r.kogi_cd));
        }

        let parsed = parser::parse_jikanwari(&r.jikanwari);
        for w in &parsed.warnings {
            warnings.push(format!("  [{}] {}: {}", r.kogi_cd, r.kogi_nm, w));
        }
        if !r.jikanwari.is_empty() && parsed.slots.is_empty() {
            warnings.push(format!(
                "  [{}] {}: 時間割情報がありますがパースできませんでした: {:?}",
                r.kogi_cd, r.kogi_nm, r.jikanwari
            ));
        }

        for s in &parsed.slots {
            if !s.semester.is_empty() {
                semester_set.insert(s.semester.clone());
            }
        }
        let dept = r.sekinin_busho_nm.trim();
        if !dept.is_empty() {
            dept_set.insert(dept.to_owned());
        }
        campus_set.insert(campus_of(r).to_owned());
        let kubun = r.kogi_kubun_nm.trim();
        if !kubun.is_empty() {
            kubun_set.insert(kubun.to_owned());
        }
        let kaikojiki = r.kogi_kaikojiki_nm.trim();
        if !kaikojiki.is_empty() {
            kaikojiki_set.insert(kaikojiki.to_owned());
        }

        // Duplicate code → merge its slots into the first occurrence and move on.
        if let Some(&idx) = seen.get(cd) {
            merge_slots(&mut slots_per_course[idx], &parsed.slots);
            continue;
        }

        seen.insert(cd.to_owned(), courses.len());
        let prof = r.tanto_kyoin.trim();
        let sub = trim_opt(&r.fukudai);
        let gakusoku = r.gakusoku_kamoku_nm.trim();
        let gaku = (!gakusoku.is_empty() && gakusoku != nm).then(|| gakusoku.to_owned());
        courses.push(CourseV2 {
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
        });
        slots_per_course.push(parsed.slots);
    }

    // --- Dictionaries ---
    let dicts = Dictionaries {
        semesters: dict::sort_semesters(&semester_set),
        departments: dict::sort_departments(&dept_set),
        campuses: dict::sort_campuses(&campus_set),
        kubun: dict::sort_kubun(&kubun_set),
        kaikojiki: dict::sort_kaikojiki(&kaikojiki_set),
    };
    let semester_idx = index_map(&dicts.semesters);
    let dept_idx = index_map(&dicts.departments);
    let campus_idx = index_map(&dicts.campuses);
    let kubun_idx = index_map(&dicts.kubun);
    let kaikojiki_idx = index_map(&dicts.kaikojiki);

    // --- Second pass: resolve dictionary indices and build bitsets ---
    let num_words = courses.len().div_ceil(64);
    let mut sem_bits: BTreeMap<usize, BitSet> = BTreeMap::new();
    let mut dept_bits: BTreeMap<usize, BitSet> = BTreeMap::new();
    let mut campus_bits: BTreeMap<usize, BitSet> = BTreeMap::new();
    let mut tsuunen_courses: Vec<usize> = Vec::new();

    // kogiCd → first raw index, so each course re-reads its canonical record.
    let mut raw_first: HashMap<&str, usize> = HashMap::new();
    for (i, r) in raw.iter().enumerate() {
        raw_first.entry(r.kogi_cd.trim()).or_insert(i);
    }

    for i in 0..courses.len() {
        let r = &raw[raw_first[courses[i].cd.as_str()]];
        courses[i].dept = lookup(&dept_idx, r.sekinin_busho_nm.trim());
        courses[i].campus = lookup(&campus_idx, campus_of(r));
        courses[i].kbn = lookup(&kubun_idx, r.kogi_kubun_nm.trim());
        courses[i].ki = lookup(&kaikojiki_idx, r.kogi_kaikojiki_nm.trim());

        let mut slots = Vec::new();
        let mut has_tsuunen = false;
        for s in &slots_per_course[i] {
            let (Some(&si), Some(di)) = (semester_idx.get(&s.semester), day_index(&s.day)) else {
                continue;
            };
            slots.push(SlotV2 {
                s: si as u32,
                d: di,
                p: s.period,
            });
            set_bit(&mut sem_bits, si, i, num_words);
            if s.semester == "通年" {
                has_tsuunen = true;
            }
        }
        courses[i].slots = slots;
        if has_tsuunen {
            tsuunen_courses.push(i);
        }
        set_bit(&mut dept_bits, courses[i].dept as usize, i, num_words);
        set_bit(&mut campus_bits, courses[i].campus as usize, i, num_words);
    }

    // 通年 courses appear under every *other* semester's filter.
    if let Some(&tsuunen_idx) = semester_idx.get("通年") {
        for sem in sem_bits.keys().copied().collect::<Vec<_>>() {
            if sem == tsuunen_idx {
                continue;
            }
            for &course in &tsuunen_courses {
                set_bit(&mut sem_bits, sem, course, num_words);
            }
        }
    }

    ConvertResult {
        data: ProcessedDataV2 {
            version: 2,
            generated_at,
            total_raw: raw.len() as u32,
            dicts,
            indices: IndicesMap {
                semester: encode(&sem_bits),
                department: encode(&dept_bits),
                campus: encode(&campus_bits),
            },
            courses,
        },
        warnings,
    }
}

/// The campus label, defaulting empty values to `その他` (Go's behavior).
fn campus_of(r: &RawCourse) -> &str {
    let campus = r.kochi_nm.trim();
    if campus.is_empty() {
        "その他"
    } else {
        campus
    }
}

/// Trim an optional string, collapsing `None`/empty to `None` (Go's `trimPtr`).
fn trim_opt(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

/// Append `new` slots not already present, deduplicating by value (Go's slot merge).
fn merge_slots(existing: &mut Vec<Slot>, new: &[Slot]) {
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

/// Resolve a label to its dictionary index, falling back to `0` for unknown or
/// empty keys — faithfully reproducing Go's `lookupIndex` quirk (an empty
/// department/kubun/kaikojiki is attributed to index 0).
fn lookup(idx: &HashMap<String, usize>, key: &str) -> u32 {
    idx.get(key).copied().unwrap_or(0) as u32
}

/// Day label → column index (0=月 … 6=日), matching Go's `dayIndex`.
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

/// Set course `course`'s bit in dimension `dict_idx`, creating the fixed-width
/// bitset on first use (Go's `setBit`).
fn set_bit(
    bitsets: &mut BTreeMap<usize, BitSet>,
    dict_idx: usize,
    course: usize,
    num_words: usize,
) {
    bitsets
        .entry(dict_idx)
        .or_insert_with(|| BitSet::with_words(num_words))
        .set(course);
}

/// Base64-encode each dimension's bitset, keyed by dictionary index as a string
/// (Go's `encodeBitsets`).
fn encode(bitsets: &BTreeMap<usize, BitSet>) -> BTreeMap<String, String> {
    bitsets
        .iter()
        .map(|(idx, bits)| (idx.to_string(), bits.to_base64()))
        .collect()
}

#[cfg(test)]
mod tests {
    //! Ported from `internal/transform/v2_test.go`.
    use super::convert_v2;
    use crate::bitset::BitSet;
    use crate::model::{ProcessedDataV2, RawCourse};

    /// A raw course with the given code/name; other fields default to empty.
    fn raw(cd: &str, nm: &str) -> RawCourse {
        RawCourse {
            kogi_cd: cd.to_owned(),
            kogi_nm: nm.to_owned(),
            ..Default::default()
        }
    }

    fn convert(raw: &[RawCourse]) -> ProcessedDataV2 {
        convert_v2(raw, "2026-05-31T00:00:00Z".to_owned()).data
    }

    /// Position of `label` in a dictionary.
    fn pos(dict: &[String], label: &str) -> usize {
        dict.iter().position(|s| s == label).expect("label present")
    }

    /// The bitset for dictionary index `idx` in a `{ "idx": base64 }` map.
    fn bitset(map: &std::collections::BTreeMap<String, String>, idx: usize) -> BitSet {
        BitSet::from_base64(&map[&idx.to_string()]).expect("valid base64")
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
        assert_eq!(data.version, 2);
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
    fn skips_empty_code_with_warning() {
        let result = convert_v2(&[raw("", "空コード"), raw("001", "正常")], "t".to_owned());
        assert_eq!(result.data.courses.len(), 1);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn empty_input_is_valid_v2() {
        let data = convert(&[]);
        assert_eq!(data.courses.len(), 0);
        assert_eq!(data.version, 2);
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
}
