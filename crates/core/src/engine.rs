//! The domain layer: an [`Engine`] that owns the parsed dataset and answers the
//! two questions the UI asks — *which courses match these filters* and *how do
//! they lay out on the timetable*.
//!
//! This is the single source of truth that replaces the TS `CourseIndex`
//! (`web/src/lib/course-index.ts`) plus the data wiring in `load-data.ts`.
//! Parsing, bitset decoding and index construction all happen in [`Engine::from_json`],
//! so the WASM layer only ever marshals **indices** across the boundary.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

use crate::bitset::BitSet;
use crate::grid::{build_grid, Grid};
use crate::index::{CampusIndex, CourseIndex, DepartmentIndex, SemesterIndex};
use crate::model::{Dictionaries, IndicesMap, ProcessedDataV2};
use crate::normalize;

/// The semester label whose courses appear under every *other* semester filter.
const TSUUNEN_LABEL: &str = "通年";

/// Errors that can arise while constructing an [`Engine`] from JSON.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The input is a raw KULAS API response, not a converted v2 dataset.
    #[error("KULAS の生レスポンスです。先に syllabus-cli convert --v2 で変換してください。")]
    RawKulasResponse,
    /// No usable `version` field — not a syllabus-cli v2 output.
    #[error("v2 フォーマットではありません（version フィールドが見つかりません）。")]
    NotV2Format,
    /// A `version` was present but unsupported.
    #[error("version {0} は対応していません。version 2 が必要です。")]
    UnsupportedVersion(u64),
    /// An `indices` map key was not a valid dictionary index.
    #[error("インデックスのキー {0:?} が不正です。")]
    InvalidIndexKey(String),
    /// The JSON itself could not be parsed / did not match the v2 schema.
    #[error("JSON の解析に失敗しました: {0}")]
    Parse(#[from] serde_json::Error),
    /// A base64 bitset could not be decoded.
    #[error("ビットセットのデコードに失敗しました: {0}")]
    Bitset(#[from] crate::bitset::DecodeError),
}

/// The query parameters for [`Engine::filter`].
///
/// Each dimension is `None` for "all", or the dictionary *value* to narrow by;
/// `query` is matched (case-insensitively, after [`normalize`]) as a substring
/// of each candidate's search haystack. Grouping them in one struct keeps
/// callers from transposing three same-typed `Option<&str>` arguments.
#[derive(Debug, Default, Clone, Copy)]
pub struct Filters<'a> {
    pub semester: Option<&'a str>,
    pub department: Option<&'a str>,
    pub campus: Option<&'a str>,
    pub query: &'a str,
}

/// The parsed dataset with its precomputed filter indices, ready to query.
#[derive(Debug)]
pub struct Engine {
    courses: Vec<crate::model::CourseV2>,
    dicts: Dictionaries,
    generated_at: String,
    semester_bitsets: HashMap<SemesterIndex, BitSet>,
    department_bitsets: HashMap<DepartmentIndex, BitSet>,
    campus_bitsets: HashMap<CampusIndex, BitSet>,
    /// `1` bits for every course index, the starting point of an AND filter.
    all_bits: BitSet,
    /// The 通年 semester, if the dataset has it.
    tsuunen_index: Option<SemesterIndex>,
    /// Whether any course meets on Saturday (drives the extra grid column).
    has_saturday: bool,
}

impl Engine {
    /// Parse a v2 `data.json` payload and build the queryable engine.
    ///
    /// # Errors
    /// Returns an [`EngineError`] if the text is a raw KULAS response, is not a
    /// supported v2 document, or fails schema/bitset decoding.
    pub fn from_json(json: &str) -> Result<Self, EngineError> {
        // A cheap structural pre-check gives friendly errors for the two common
        // "wrong file" cases before the full schema deserialization.
        let value: serde_json::Value = serde_json::from_str(json)?;
        if value.get("selectKogiDtoList").is_some() {
            return Err(EngineError::RawKulasResponse);
        }
        match value.get("version").and_then(serde_json::Value::as_u64) {
            Some(2) => {}
            Some(other) => return Err(EngineError::UnsupportedVersion(other)),
            None => return Err(EngineError::NotV2Format),
        }

        let data: ProcessedDataV2 = serde_json::from_value(value)?;
        Self::build(data)
    }

    /// Construct an engine from an already-deserialized payload (decoding the
    /// base64 bitsets and deriving the cached lookups).
    fn build(data: ProcessedDataV2) -> Result<Self, EngineError> {
        let ProcessedDataV2 {
            dicts,
            indices,
            courses,
            generated_at,
            ..
        } = data;

        let IndicesMap {
            semester,
            department,
            campus,
        } = indices;

        let all_bits = BitSet::all_ones(courses.len());
        let tsuunen_index = dicts
            .semesters
            .iter()
            .position(|s| s == TSUUNEN_LABEL)
            .map(SemesterIndex::from);
        let has_saturday = courses.iter().any(|c| c.slots.iter().any(|s| s.d == 5));

        Ok(Self {
            courses,
            dicts,
            generated_at,
            semester_bitsets: decode_bitsets(&semester)?,
            department_bitsets: decode_bitsets(&department)?,
            campus_bitsets: decode_bitsets(&campus)?,
            all_bits,
            tsuunen_index,
            has_saturday,
        })
    }

    /// Return the indices of courses matching the [`Filters`], in ascending order.
    #[must_use]
    pub fn filter(&self, filters: &Filters) -> Vec<CourseIndex> {
        if self.courses.is_empty() {
            return Vec::new();
        }

        let mut bits = self.all_bits.clone();
        bits = narrow(
            bits,
            &self.dicts.semesters,
            &self.semester_bitsets,
            filters.semester,
        );
        bits = narrow(
            bits,
            &self.dicts.departments,
            &self.department_bitsets,
            filters.department,
        );
        bits = narrow(
            bits,
            &self.dicts.campuses,
            &self.campus_bitsets,
            filters.campus,
        );

        let candidates = bits.iter_ones().map(CourseIndex::new);
        if filters.query.is_empty() {
            candidates.collect()
        } else {
            let needle = normalize(filters.query);
            candidates
                .filter(|&i| self.courses[i.get()].st.contains(&needle))
                .collect()
        }
    }

    /// Lay the given (already-filtered) course indices onto the timetable.
    #[must_use]
    pub fn grid(&self, course_indices: &[CourseIndex], semester: Option<&str>) -> Grid {
        let semester_index = semester
            .and_then(|value| self.dicts.semesters.iter().position(|s| s == value))
            .map(SemesterIndex::from);
        build_grid(
            course_indices.iter().map(|&i| (i, &self.courses[i.get()])),
            semester_index,
            self.tsuunen_index,
            self.has_saturday,
        )
    }

    /// The full course list, in index order (the WASM layer hands this to the UI
    /// once as a read-only view cache).
    #[must_use]
    pub fn courses(&self) -> &[crate::model::CourseV2] {
        &self.courses
    }

    /// The dictionaries (semesters / departments / campuses / kubun / kaikojiki).
    #[must_use]
    pub fn dicts(&self) -> &Dictionaries {
        &self.dicts
    }

    /// When the dataset was generated (RFC 3339 string from the pipeline).
    #[must_use]
    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    /// Whether the timetable needs a Saturday column.
    #[must_use]
    pub fn has_saturday(&self) -> bool {
        self.has_saturday
    }
}

/// AND a running bitset with one filter dimension.
///
/// `None` (i.e. "all") leaves it untouched; a value that is absent from the
/// dictionary, or whose bitset is missing, yields the empty set — mirroring the
/// TS `semBits ? bits.and(semBits) : BitSet.allOnes(0)`. Generic over the
/// dimension's index type so a campus value can't query the semester map.
fn narrow<K>(
    bits: BitSet,
    dict: &[String],
    bitsets: &HashMap<K, BitSet>,
    selector: Option<&str>,
) -> BitSet
where
    K: From<usize> + Eq + Hash,
{
    match selector {
        None => bits,
        Some(value) => match dict
            .iter()
            .position(|v| v == value)
            .map(K::from)
            .and_then(|index| bitsets.get(&index))
        {
            Some(dimension) => bits.and(dimension),
            None => BitSet::empty(),
        },
    }
}

/// Decode a `{ "dictIndex": base64 }` map into `{ K: BitSet }`, where `K` is the
/// dimension's typed index (inferred from the destination field).
fn decode_bitsets<K>(encoded: &BTreeMap<String, String>) -> Result<HashMap<K, BitSet>, EngineError>
where
    K: From<usize> + Eq + Hash,
{
    encoded
        .iter()
        .map(|(key, value)| {
            let index = key
                .parse::<usize>()
                .map_err(|_| EngineError::InvalidIndexKey(key.clone()))?;
            Ok((K::from(index), BitSet::from_base64(value)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    //! Ported from `web/src/lib/course-index.test.ts`. The fixture indices are
    //! built here the way the Go pipeline does — little-endian `u64` words,
    //! base64-encoded — so `from_base64` round-trips them bit-for-bit.

    use super::{Engine, Filters};
    use crate::index::CourseIndex;
    use crate::model::{CourseV2, Dictionaries, IndicesMap, ProcessedDataV2, SlotV2};
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use std::collections::BTreeMap;

    fn dicts() -> Dictionaries {
        Dictionaries {
            semesters: vec!["1学期".into(), "2学期".into(), "通年".into()],
            departments: vec!["人文社会科学部".into(), "理工学部".into()],
            campuses: vec!["朝倉キャンパス".into(), "物部キャンパス".into()],
            kubun: vec!["講義".into(), "演習".into()],
            kaikojiki: vec!["1学期".into(), "2学期".into(), "通年".into()],
        }
    }

    /// Minimal course builder mirroring the TS `makeCourse`.
    fn course(cd: &str, slots: &[(u32, i32, i32)], dept: u32, campus: u32, st: &str) -> CourseV2 {
        CourseV2 {
            cd: cd.into(),
            nm: "テスト講義".into(),
            sub: None,
            prof: "教員 太郎".into(),
            raw: String::new(),
            slots: slots.iter().map(|&(s, d, p)| SlotV2 { s, d, p }).collect(),
            ki: 0,
            kbn: 0,
            dept,
            campus,
            gaku: None,
            gakka: None,
            nen: None,
            bunrui: None,
            bunya: None,
            st: st.into(),
        }
    }

    fn encode(words: &[u64]) -> String {
        let mut bytes = Vec::with_capacity(words.len() * 8);
        for w in words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        STANDARD.encode(bytes)
    }

    /// Port of the TS `buildTestIndices`: one `u64` word array per dictionary
    /// value, with 通年 courses propagated into every other semester bitset.
    fn build_test_indices(courses: &[CourseV2], dicts: &Dictionaries) -> IndicesMap {
        let n = courses.len();
        let num_words = n.div_ceil(64).max(1);
        let set = |words: &mut [u64], ci: usize| words[ci / 64] |= 1u64 << (ci % 64);
        let tsuunen = dicts.semesters.iter().position(|s| s == "通年");

        let tsuunen_courses: Vec<usize> = courses
            .iter()
            .enumerate()
            .filter(|(_, c)| c.slots.iter().any(|sl| Some(sl.s as usize) == tsuunen))
            .map(|(ci, _)| ci)
            .collect();

        let mut semester = BTreeMap::new();
        for si in 0..dicts.semesters.len() {
            let mut words = vec![0u64; num_words];
            for (ci, c) in courses.iter().enumerate() {
                if c.slots.iter().any(|sl| sl.s as usize == si) {
                    set(&mut words, ci);
                }
            }
            if Some(si) != tsuunen {
                for &ci in &tsuunen_courses {
                    set(&mut words, ci);
                }
            }
            semester.insert(si.to_string(), encode(&words));
        }

        let dimension = |selector: &dyn Fn(&CourseV2) -> u32, len: usize| {
            let mut map = BTreeMap::new();
            for di in 0..len {
                let mut words = vec![0u64; num_words];
                for (ci, c) in courses.iter().enumerate() {
                    if selector(c) as usize == di {
                        set(&mut words, ci);
                    }
                }
                map.insert(di.to_string(), encode(&words));
            }
            map
        };

        IndicesMap {
            semester,
            department: dimension(&|c| c.dept, dicts.departments.len()),
            campus: dimension(&|c| c.campus, dicts.campuses.len()),
        }
    }

    fn engine_of(courses: Vec<CourseV2>) -> Engine {
        let d = dicts();
        let indices = build_test_indices(&courses, &d);
        Engine::build(ProcessedDataV2 {
            version: 2,
            generated_at: "2026-05-31T00:00:00Z".into(),
            total_raw: courses.len() as u32,
            dicts: d,
            indices,
            courses,
        })
        .expect("engine builds")
    }

    /// The three-course fixture used by most filter cases.
    fn sample() -> Vec<CourseV2> {
        vec![
            course(
                "001",
                &[(0, 0, 1)],
                1,
                0,
                "微分積分学 山田 太郎 001 理工学部",
            ),
            course(
                "002",
                &[(1, 1, 2)],
                0,
                1,
                "政治学概論 小川 寛貴 002 人文社会科学部",
            ),
            course(
                "003",
                &[(2, 4, 5)],
                0,
                0,
                "哲学概論 佐藤 哲也 003 人文社会科学部",
            ),
        ]
    }

    /// Resolve filter output back to `cd`s, in result order.
    fn cds(engine: &Engine, indices: &[CourseIndex]) -> Vec<String> {
        indices
            .iter()
            .map(|&i| engine.courses()[i.get()].cd.clone())
            .collect()
    }

    #[test]
    fn returns_all_when_no_filters() {
        let e = engine_of(sample());
        assert_eq!(e.filter(&Filters::default()).len(), 3);
    }

    #[test]
    fn filters_by_semester_first_term() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    semester: Some("1学期"),
                    ..Default::default()
                })
            ),
            ["001", "003"]
        );
    }

    #[test]
    fn filters_by_semester_second_term() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    semester: Some("2学期"),
                    ..Default::default()
                })
            ),
            ["002", "003"]
        );
    }

    #[test]
    fn tsuunen_appears_in_every_semester_filter() {
        let e = engine_of(sample());
        let first = cds(
            &e,
            &e.filter(&Filters {
                semester: Some("1学期"),
                ..Default::default()
            }),
        );
        let second = cds(
            &e,
            &e.filter(&Filters {
                semester: Some("2学期"),
                ..Default::default()
            }),
        );
        assert!(first.contains(&"003".to_string()));
        assert!(second.contains(&"003".to_string()));
    }

    #[test]
    fn filters_by_department() {
        let e = engine_of(sample());
        let r = e.filter(&Filters {
            department: Some("理工学部"),
            ..Default::default()
        });
        assert_eq!(cds(&e, &r), ["001"]);
    }

    #[test]
    fn empty_for_nonexistent_department() {
        let e = engine_of(sample());
        assert!(e
            .filter(&Filters {
                department: Some("医学部"),
                ..Default::default()
            })
            .is_empty());
    }

    #[test]
    fn filters_by_campus() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    campus: Some("朝倉キャンパス"),
                    ..Default::default()
                })
            ),
            ["001", "003"]
        );
    }

    #[test]
    fn filters_by_campus_monobe() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    campus: Some("物部キャンパス"),
                    ..Default::default()
                })
            ),
            ["002"]
        );
    }

    #[test]
    fn empty_for_nonexistent_campus() {
        let e = engine_of(sample());
        assert!(e
            .filter(&Filters {
                campus: Some("岡豊キャンパス"),
                ..Default::default()
            })
            .is_empty());
    }

    #[test]
    fn searches_by_course_name() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    query: "微分",
                    ..Default::default()
                })
            ),
            ["001"]
        );
    }

    #[test]
    fn searches_by_instructor() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    query: "小川",
                    ..Default::default()
                })
            ),
            ["002"]
        );
    }

    #[test]
    fn searches_by_kogi_cd() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    query: "003",
                    ..Default::default()
                })
            ),
            ["003"]
        );
    }

    #[test]
    fn search_is_case_insensitive() {
        let e = engine_of(vec![course(
            "004",
            &[(0, 0, 1)],
            1,
            0,
            "english communication smith, john 004 理工学部",
        )]);
        assert_eq!(
            e.filter(&Filters {
                query: "english",
                ..Default::default()
            })
            .len(),
            1
        );
        assert_eq!(
            e.filter(&Filters {
                query: "SMITH",
                ..Default::default()
            })
            .len(),
            1
        );
    }

    #[test]
    fn combines_semester_and_department() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    semester: Some("2学期"),
                    department: Some("人文社会科学部"),
                    ..Default::default()
                })
            ),
            ["002", "003"]
        );
    }

    #[test]
    fn combines_all_four_filters() {
        let e = engine_of(sample());
        let r = e.filter(&Filters {
            semester: Some("2学期"),
            department: Some("人文社会科学部"),
            campus: Some("物部キャンパス"),
            query: "",
        });
        assert_eq!(cds(&e, &r), ["002"]);
    }

    #[test]
    fn combines_campus_with_semester() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    semester: Some("1学期"),
                    campus: Some("朝倉キャンパス"),
                    ..Default::default()
                })
            ),
            ["001", "003"]
        );
    }

    #[test]
    fn combines_campus_department_and_search() {
        let e = engine_of(sample());
        let r = e.filter(&Filters {
            department: Some("人文社会科学部"),
            campus: Some("朝倉キャンパス"),
            query: "哲学",
            ..Default::default()
        });
        assert_eq!(cds(&e, &r), ["003"]);
    }

    #[test]
    fn handles_empty_courses() {
        let e = engine_of(vec![]);
        assert!(e.filter(&Filters::default()).is_empty());
    }

    #[test]
    fn handles_course_with_empty_slots() {
        let e = engine_of(vec![course(
            "001",
            &[],
            1,
            0,
            "テスト講義 教員 太郎 001 理工学部",
        )]);
        assert_eq!(e.filter(&Filters::default()).len(), 1);
        // A specific semester has no matching slot → filtered out via the bitset.
        assert!(e
            .filter(&Filters {
                semester: Some("1学期"),
                ..Default::default()
            })
            .is_empty());
    }

    #[test]
    fn does_not_treat_query_as_regex() {
        let e = engine_of(sample());
        assert!(e
            .filter(&Filters {
                query: "[.*+?]",
                ..Default::default()
            })
            .is_empty());
    }

    #[test]
    fn department_with_semester_narrows_to_empty() {
        let e = engine_of(sample());
        assert!(e
            .filter(&Filters {
                semester: Some("2学期"),
                department: Some("理工学部"),
                ..Default::default()
            })
            .is_empty());
    }

    #[test]
    fn semester_all_with_department() {
        let e = engine_of(sample());
        assert_eq!(
            cds(
                &e,
                &e.filter(&Filters {
                    department: Some("人文社会科学部"),
                    ..Default::default()
                })
            ),
            ["002", "003"]
        );
    }

    #[test]
    fn detects_saturday_from_data() {
        let weekday = engine_of(sample());
        assert!(!weekday.has_saturday());
        let saturday = engine_of(vec![course("010", &[(0, 5, 1)], 1, 0, "土曜講義 010")]);
        assert!(saturday.has_saturday());
    }

    #[test]
    fn rejects_raw_kulas_response() {
        let err = Engine::from_json(r#"{"selectKogiDtoList": []}"#).unwrap_err();
        assert!(matches!(err, super::EngineError::RawKulasResponse));
    }

    #[test]
    fn rejects_unsupported_version() {
        let err = Engine::from_json(r#"{"version": 1}"#).unwrap_err();
        assert!(matches!(err, super::EngineError::UnsupportedVersion(1)));
    }

    #[test]
    fn rejects_missing_version() {
        let err = Engine::from_json(r#"{"courses": []}"#).unwrap_err();
        assert!(matches!(err, super::EngineError::NotV2Format));
    }
}
