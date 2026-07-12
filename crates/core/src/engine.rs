//! The domain layer: an [`Engine`] that owns the parsed dataset and answers the
//! two questions the UI asks — *which courses match these filters* and *how do
//! they lay out on the timetable*.
//!
//! Parsing, bitset decoding and index construction all happen in
//! [`Engine::from_json`], so the WASM layer only ever marshals **indices**
//! across the boundary.

use std::collections::{BTreeSet, HashMap};

use crate::bitset::BitSet;
use crate::grid::{Grid, GridSlot, build_grid};
use crate::index::{CourseIndex, SemesterIndex};
use crate::model::{Dictionaries, IndicesMap, ProcessedData};
use crate::plan::{PlanSummary, conflicts_in_grid, summarize_credits};
use crate::search::{IndexError, SearchHit, SearchIndex};
use crate::text::{normalize, search_text};

/// The semester label whose courses appear under every *other* semester filter.
const TSUUNEN_LABEL: &str = "通年";

/// Errors that can arise while constructing an [`Engine`] from JSON.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The input is a raw KULAS API response, not a converted v3 dataset.
    #[error("This is a raw KULAS response; run `syllabus-cli convert` on it first.")]
    RawKulasResponse,
    /// No usable `version` field — not a syllabus-cli v3 output.
    #[error("Not a v3 dataset: no `version` field found.")]
    NotV3Format,
    /// A `version` was present but unsupported.
    #[error("Unsupported version {0}; version 3 is required.")]
    UnsupportedVersion(u64),
    /// The JSON itself could not be parsed / did not match the v3 schema.
    #[error("Failed to parse JSON: {0}")]
    Parse(#[from] serde_json::Error),
    /// A base64 bitset could not be decoded.
    #[error("Failed to decode bitset: {0}")]
    Bitset(#[from] crate::bitset::DecodeError),
}

/// The query parameters for [`Engine::filter`].
///
/// Each dimension is `None` for "all", or the dictionary *value* to narrow by;
/// `query` is matched (case-insensitively, after [`normalize`]) as a substring
/// of each candidate's search haystack.
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
    /// The wire/view-model courses, handed to the UI as-is.
    courses: Vec<crate::model::Course>,
    /// Each course's validated timetable (parallel to `courses`) — the domain
    /// form the grid consumes, range-checked once here instead of per `grid` call.
    timetables: Vec<Vec<GridSlot>>,
    dicts: Dictionaries,
    generated_at: String,
    /// Academic year shared by the dataset (for the official syllabus deep link).
    year: String,
    semester_bitsets: Vec<BitSet>,
    department_bitsets: Vec<BitSet>,
    campus_bitsets: Vec<BitSet>,
    /// `1` bits for every course index, the starting point of an AND filter.
    all_bits: BitSet,
    /// The 通年 semester, if the dataset has it.
    tsuunen_index: Option<SemesterIndex>,
    /// Whether any course meets on Saturday (drives the extra grid column).
    has_saturday: bool,
    /// A normalized per-course search haystack (name/subtitle/instructor/code/
    /// department/taxonomy), built here at load — the wire format no longer
    /// carries it. Used by [`Engine::filter`] and as [`Engine::search`]'s
    /// pre-index fallback, so search works even if `search.idx` never arrives.
    haystack: Vec<String>,
    /// The full-text index, loaded from the companion `search.idx` after the
    /// engine is built (it ships separately from `data.json`). `None` until then;
    /// text queries fall back to `haystack` meanwhile.
    search_index: Option<SearchIndex>,
    /// `cd` → course index, for resolving a shared plan (a list of stable course
    /// codes) back to indices. Built once here so `resolve_cds` is O(n).
    cd_to_index: HashMap<String, CourseIndex>,
}

impl Engine {
    /// Parse a v3 `data.json` payload and build the queryable engine.
    ///
    /// # Errors
    /// Returns an [`EngineError`] if the text is a raw KULAS response, is not a
    /// supported v3 document, or fails schema/bitset decoding.
    pub fn from_json(json: &str) -> Result<Self, EngineError> {
        // Cheap structural pre-check: friendly errors for the two common "wrong
        // file" cases before full schema deserialization.
        let value: serde_json::Value = serde_json::from_str(json)?;
        if value.get("selectKogiDtoList").is_some() {
            return Err(EngineError::RawKulasResponse);
        }
        match value.get("version").and_then(serde_json::Value::as_u64) {
            Some(3) => {}
            Some(other) => return Err(EngineError::UnsupportedVersion(other)),
            None => return Err(EngineError::NotV3Format),
        }

        let data: ProcessedData = serde_json::from_value(value)?;
        Self::build(data)
    }

    /// Construct an engine from an already-deserialized payload (decoding the
    /// base64 bitsets and deriving the cached lookups).
    fn build(data: ProcessedData) -> Result<Self, EngineError> {
        let ProcessedData {
            dicts,
            indices,
            courses,
            generated_at,
            year,
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

        // Validate each course's wire slots into grid slots once, here.
        let timetables: Vec<Vec<GridSlot>> = courses
            .iter()
            .map(|c| c.slots.iter().filter_map(GridSlot::from_wire).collect())
            .collect();
        let has_saturday = timetables.iter().flatten().any(|s| s.is_saturday());
        let cd_to_index = courses
            .iter()
            .enumerate()
            .map(|(i, c)| (c.cd.clone(), CourseIndex::new(i)))
            .collect();
        let haystack = courses
            .iter()
            .map(|c| {
                let dept = dicts
                    .departments
                    .get(c.dept as usize)
                    .map_or("", String::as_str);
                search_text(
                    &c.nm,
                    c.sub.as_deref(),
                    &c.prof,
                    &c.cd,
                    dept,
                    &[
                        c.bunya.as_deref().unwrap_or_default(),
                        c.bunrui.as_deref().unwrap_or_default(),
                    ],
                )
            })
            .collect();

        Ok(Self {
            courses,
            timetables,
            dicts,
            generated_at,
            year,
            semester_bitsets: decode_dimension(&semester)?,
            department_bitsets: decode_dimension(&department)?,
            campus_bitsets: decode_dimension(&campus)?,
            all_bits,
            tsuunen_index,
            has_saturday,
            haystack,
            search_index: None,
            cd_to_index,
        })
    }

    /// Resolve a plan's stable course codes to indices, dropping any `cd` not in
    /// this dataset (a shared link may predate a data refresh), ascending and
    /// de-duplicated.
    #[must_use]
    pub fn resolve_cds(&self, cds: &[String]) -> Vec<CourseIndex> {
        let mut out: Vec<CourseIndex> = cds
            .iter()
            .filter_map(|cd| self.cd_to_index.get(cd).copied())
            .collect();
        out.sort_unstable_by_key(|i| i.get());
        out.dedup();
        out
    }

    /// Summarize a plan (registered course indices): every timetable collision
    /// and the credit tallies.
    ///
    /// Conflicts are found per semester (so 1学期 月1 and 2学期 月1 never collide),
    /// reusing [`Engine::grid`] so 通年 propagation is handled the same way as the
    /// display grid; a 通年×通年 pair that appears under every term is counted once.
    #[must_use]
    pub fn plan_summary(&self, indices: &[CourseIndex]) -> PlanSummary {
        let mut seen: BTreeSet<(u8, u8, Vec<usize>)> = BTreeSet::new();
        let mut conflicts = Vec::new();
        for (si, name) in self.dicts.semesters.iter().enumerate() {
            if self.tsuunen_index.is_some_and(|t| t.get() == si) {
                continue; // 通年 is surfaced under the real terms, not on its own
            }
            let grid = self.grid(indices, Some(name));
            for c in conflicts_in_grid(&grid) {
                let key = (
                    c.day.get(),
                    c.period.get(),
                    c.courses.iter().map(|i| i.get()).collect(),
                );
                if seen.insert(key) {
                    conflicts.push(c);
                }
            }
        }
        let courses = indices.iter().filter_map(|i| self.courses.get(i.get()));
        let credits = summarize_credits(courses, &self.dicts.kubun);
        PlanSummary { conflicts, credits }
    }

    /// Load the companion `search.idx` (fetched separately from `data.json`),
    /// enabling ranked search with match spans. Until this is called, text
    /// queries fall back to an unranked `st` substring scan.
    ///
    /// # Errors
    /// Returns an [`IndexError`] if the blob is not a valid `search.idx`.
    pub fn load_search_index(&mut self, bytes: &[u8]) -> Result<(), IndexError> {
        self.search_index = Some(SearchIndex::decode(bytes)?);
        Ok(())
    }

    /// AND the running set with each filter dimension (semester/department/
    /// campus), ignoring the text query. Each dictionary is paired with its own
    /// bitsets so a campus value can't query the semester vector.
    fn candidate_bits(&self, filters: &Filters) -> BitSet {
        let dimensions: [(&[String], &[BitSet], Option<&str>); 3] = [
            (
                &self.dicts.semesters,
                &self.semester_bitsets,
                filters.semester,
            ),
            (
                &self.dicts.departments,
                &self.department_bitsets,
                filters.department,
            ),
            (&self.dicts.campuses, &self.campus_bitsets, filters.campus),
        ];
        let mut bits = self.all_bits.clone();
        for (dict, bitsets, selector) in dimensions {
            bits = narrow(bits, dict, bitsets, selector);
        }
        bits
    }

    /// Return the indices of courses matching the [`Filters`], in ascending order.
    ///
    /// This is the dimension-and-substring path the WASM boundary still exposes;
    /// [`Engine::search`] is the ranked, span-carrying successor.
    #[must_use]
    pub fn filter(&self, filters: &Filters) -> Vec<CourseIndex> {
        if self.courses.is_empty() {
            return Vec::new();
        }
        let bits = self.candidate_bits(filters);
        let candidates = bits.iter_ones().map(CourseIndex::new);
        if filters.query.is_empty() {
            candidates.collect()
        } else {
            let needle = normalize(filters.query);
            candidates
                .filter(|&i| self.haystack[i.get()].contains(&needle))
                .collect()
        }
    }

    /// Search the dataset: dimension-filter, then rank the text query with match
    /// spans, best first (ties broken by ascending course index).
    ///
    /// An empty query returns every candidate unranked (score 0, no spans), in
    /// ascending index order — the browse view. With a query, ranking uses the
    /// loaded `search.idx`; before it loads, it falls back to an unranked `st`
    /// substring scan so search still works during the brief index fetch.
    #[must_use]
    pub fn search(&self, filters: &Filters) -> Vec<SearchHit> {
        if self.courses.is_empty() {
            return Vec::new();
        }
        let bits = self.candidate_bits(filters);
        let candidates = bits.iter_ones().map(CourseIndex::new);

        if filters.query.is_empty() {
            return candidates.map(SearchHit::unranked).collect();
        }
        match &self.search_index {
            Some(index) => index.search(filters.query, candidates),
            None => {
                let needle = normalize(filters.query);
                candidates
                    .filter(|&i| self.haystack[i.get()].contains(&needle))
                    .map(SearchHit::unranked)
                    .collect()
            }
        }
    }

    /// Lay ranked [`SearchHit`]s onto the timetable. Feeding [`build_grid`] the
    /// hits in score order makes each cell come out best-first (it appends in
    /// iteration order and de-duplicates), so no separate per-cell sort is needed.
    #[must_use]
    pub fn search_grid(&self, hits: &[SearchHit], semester: Option<&str>) -> Grid {
        let semester_index = semester
            .and_then(|value| self.dicts.semesters.iter().position(|s| s == value))
            .map(SemesterIndex::from);
        build_grid(
            hits.iter()
                .map(|h| (h.course, self.timetables[h.course.get()].as_slice())),
            semester_index,
            self.tsuunen_index,
            self.has_saturday,
        )
    }

    /// Lay the given (already-filtered) course indices onto the timetable.
    #[must_use]
    pub fn grid(&self, course_indices: &[CourseIndex], semester: Option<&str>) -> Grid {
        let semester_index = semester
            .and_then(|value| self.dicts.semesters.iter().position(|s| s == value))
            .map(SemesterIndex::from);
        build_grid(
            course_indices
                .iter()
                .map(|&i| (i, self.timetables[i.get()].as_slice())),
            semester_index,
            self.tsuunen_index,
            self.has_saturday,
        )
    }

    /// The full course list, in index order (the WASM layer hands this to the UI
    /// once as a read-only view cache).
    #[must_use]
    pub fn courses(&self) -> &[crate::model::Course] {
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

    /// The dataset's academic year (`kaikoNendo`), for the official deep link.
    #[must_use]
    pub fn year(&self) -> &str {
        &self.year
    }

    /// Whether the timetable needs a Saturday column.
    #[must_use]
    pub fn has_saturday(&self) -> bool {
        self.has_saturday
    }
}

/// AND a running bitset with one filter dimension.
///
/// `None` (i.e. "all") leaves it untouched; a value absent from the dictionary
/// (or whose positional bitset is missing) yields the empty set.
fn narrow(bits: BitSet, dict: &[String], bitsets: &[BitSet], selector: Option<&str>) -> BitSet {
    match selector {
        None => bits,
        Some(value) => match dict
            .iter()
            .position(|v| v == value)
            .and_then(|index| bitsets.get(index))
        {
            Some(dimension) => bits.and(dimension),
            None => BitSet::empty(),
        },
    }
}

/// Decode a dimension's positional base64 bitsets (vector index = dictionary
/// index).
fn decode_dimension(encoded: &[String]) -> Result<Vec<BitSet>, EngineError> {
    encoded
        .iter()
        .map(|value| BitSet::from_base64(value).map_err(EngineError::from))
        .collect()
}

#[cfg(test)]
mod tests {
    //! Fixture indices are built here as little-endian `u64` words, base64-encoded,
    //! so `from_base64` round-trips them bit-for-bit.

    use super::{Engine, Filters};
    use crate::index::CourseIndex;
    use crate::model::{Course, Dictionaries, IndicesMap, ProcessedData, Slot};
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    fn dicts() -> Dictionaries {
        Dictionaries {
            semesters: vec!["1学期".into(), "2学期".into(), "通年".into()],
            departments: vec!["人文社会科学部".into(), "理工学部".into()],
            campuses: vec!["朝倉キャンパス".into(), "物部キャンパス".into()],
            kubun: vec!["講義".into(), "演習".into()],
            kaikojiki: vec!["1学期".into(), "2学期".into(), "通年".into()],
        }
    }

    /// Minimal course builder for tests. `text` is the searchable content
    /// (name / instructor / code …); the engine builds its haystack from these
    /// fields, so putting it in `nm` keeps the filter/search tests self-contained.
    fn course(cd: &str, slots: &[(u32, i32, i32)], dept: u32, campus: u32, text: &str) -> Course {
        Course {
            cd: cd.into(),
            nm: text.into(),
            sub: None,
            prof: "教員 太郎".into(),
            raw: String::new(),
            slots: slots.iter().map(|&(s, d, p)| Slot { s, d, p }).collect(),
            ki: 0,
            kbn: 0,
            dept,
            campus,
            gaku: None,
            gakka: None,
            nen: None,
            bunrui: None,
            bunya: None,
            pat: None,
            unit: None,
            dm: None,
            ev: None,
        }
    }

    fn encode(words: &[u64]) -> String {
        let mut bytes = Vec::with_capacity(words.len() * 8);
        for w in words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        STANDARD.encode(bytes)
    }

    /// Build one positional `u64` word array per dictionary value, with 通年
    /// courses propagated into every other semester bitset.
    fn build_test_indices(courses: &[Course], dicts: &Dictionaries) -> IndicesMap {
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

        let mut semester = Vec::new();
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
            semester.push(encode(&words));
        }

        let dimension = |selector: &dyn Fn(&Course) -> u32, len: usize| {
            let mut bitsets = Vec::new();
            for di in 0..len {
                let mut words = vec![0u64; num_words];
                for (ci, c) in courses.iter().enumerate() {
                    if selector(c) as usize == di {
                        set(&mut words, ci);
                    }
                }
                bitsets.push(encode(&words));
            }
            bitsets
        };

        IndicesMap {
            semester,
            department: dimension(&|c| c.dept, dicts.departments.len()),
            campus: dimension(&|c| c.campus, dicts.campuses.len()),
        }
    }

    fn engine_of(courses: Vec<Course>) -> Engine {
        let d = dicts();
        let indices = build_test_indices(&courses, &d);
        Engine::build(ProcessedData {
            version: 3,
            generated_at: "2026-05-31T00:00:00Z".into(),
            year: "2026".into(),
            total_raw: courses.len() as u32,
            dicts: d,
            indices,
            courses,
        })
        .expect("engine builds")
    }

    /// The three-course fixture used by most filter cases.
    fn sample() -> Vec<Course> {
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
    fn exposes_the_dataset_metadata() {
        let e = engine_of(sample());
        assert_eq!(e.year(), "2026");
        assert_eq!(e.generated_at(), "2026-05-31T00:00:00Z");
    }

    #[test]
    fn grid_places_only_the_named_semester() {
        let e = engine_of(sample());
        let all = e.filter(&Filters::default());
        let cells: Vec<(u8, u8, Vec<usize>)> = e
            .grid(&all, Some("2学期"))
            .cells()
            .map(|(d, p, idx)| (d.get(), p.get(), idx.iter().map(|i| i.get()).collect()))
            .collect();
        // 002 (2学期, 火2 = day1/period2) is placed; 001 (1学期, 月1) is filtered out.
        assert!(
            cells.contains(&(1, 2, vec![1])),
            "002 belongs in the 2学期 grid: {cells:?}"
        );
        assert!(
            !cells.iter().any(|(d, p, _)| (*d, *p) == (0, 1)),
            "001 (1学期) must not appear: {cells:?}"
        );
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
        assert!(
            e.filter(&Filters {
                department: Some("医学部"),
                ..Default::default()
            })
            .is_empty()
        );
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
        assert!(
            e.filter(&Filters {
                campus: Some("岡豊キャンパス"),
                ..Default::default()
            })
            .is_empty()
        );
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
        assert!(
            e.filter(&Filters {
                semester: Some("1学期"),
                ..Default::default()
            })
            .is_empty()
        );
    }

    #[test]
    fn does_not_treat_query_as_regex() {
        let e = engine_of(sample());
        assert!(
            e.filter(&Filters {
                query: "[.*+?]",
                ..Default::default()
            })
            .is_empty()
        );
    }

    #[test]
    fn department_with_semester_narrows_to_empty() {
        let e = engine_of(sample());
        assert!(
            e.filter(&Filters {
                semester: Some("2学期"),
                department: Some("理工学部"),
                ..Default::default()
            })
            .is_empty()
        );
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

    // === search (ranked) ===

    /// Build + load an index over the engine's courses (name/instructor/code),
    /// so `search` takes the ranked path rather than the `st` fallback.
    fn load_index(engine: &mut Engine) {
        use crate::search::{DocFields, SearchIndex};
        let bytes = SearchIndex::build(engine.courses.iter().map(|c| DocFields {
            name: &c.nm,
            subtitle: c.sub.as_deref(),
            instructor: &c.prof,
            code: &c.cd,
            keywords: "",
        }))
        .encode();
        engine.load_search_index(&bytes).expect("index loads");
    }

    /// Courses with distinct names/instructors for ranking assertions.
    fn named() -> Vec<Course> {
        vec![
            course(
                "001",
                &[(0, 0, 1)],
                1,
                0,
                "微分積分学 山田 太郎 001 理工学部",
            ),
            course("002", &[(0, 1, 2)], 1, 0, "線形代数 田中 花子 002 理工学部"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, mut c)| {
            c.nm = ["微分積分学", "線形代数"][i].into();
            c.prof = ["山田 太郎", "田中 花子"][i].into();
            c
        })
        .collect()
    }

    #[test]
    fn search_ranks_and_carries_spans() {
        let mut e = engine_of(named());
        load_index(&mut e);
        let hits = e.search(&Filters {
            query: "田",
            ..Default::default()
        });
        // 田 appears in course 0's instructor (山田) and course 1's instructor
        // (田中): both hit, and every hit carries at least one span.
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| !h.spans.is_empty()));
    }

    #[test]
    fn search_falls_back_to_st_before_the_index_loads() {
        let e = engine_of(sample());
        // No index loaded: still finds the course, unranked (no spans).
        let hits = e.search(&Filters {
            query: "微分",
            ..Default::default()
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].course, CourseIndex::new(0));
        assert!(hits[0].spans.is_empty());
    }

    #[test]
    fn empty_query_search_is_every_candidate_unranked() {
        let mut e = engine_of(sample());
        load_index(&mut e);
        let hits = e.search(&Filters::default());
        assert_eq!(hits.len(), 3);
        assert!(hits.iter().all(|h| h.spans.is_empty() && h.score == 0.0));
        // Ascending index order (browse view).
        assert_eq!(
            hits.iter().map(|h| h.course.get()).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn search_grid_orders_cells_by_score() {
        // Both courses meet 月1 and both match "田中" — course 0 in its instructor
        // (weight 2.0), course 1 in its name (weight 3.0). The higher score must
        // come first in the shared cell: [1, 0], not index order.
        let mut c0 = course("001", &[(0, 0, 1)], 1, 0, "物理 田中 001 理工学部");
        c0.nm = "物理学".into();
        c0.prof = "田中 太郎".into();
        let mut c1 = course("002", &[(0, 0, 1)], 1, 0, "田中理論 佐藤 002 理工学部");
        c1.nm = "田中理論".into();
        c1.prof = "佐藤 花子".into();
        let mut e = engine_of(vec![c0, c1]);
        load_index(&mut e);

        let hits = e.search(&Filters {
            query: "田中",
            ..Default::default()
        });
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].course, CourseIndex::new(1), "name hit ranks first");

        let grid = e.search_grid(&hits, Some("1学期"));
        assert_eq!(
            grid.cell(
                crate::index::Day::new(0),
                crate::index::Period::new(1).unwrap()
            ),
            &[CourseIndex::new(1), CourseIndex::new(0)],
        );
    }

    #[test]
    fn load_search_index_rejects_a_bad_blob() {
        let mut e = engine_of(sample());
        assert!(e.load_search_index(b"garbage").is_err());
    }

    // === plan ===

    #[test]
    fn resolve_cds_drops_unknown_dedups_and_sorts() {
        let e = engine_of(sample()); // cds 001, 002, 003 at indices 0, 1, 2
        let got = e.resolve_cds(&[
            "003".into(),
            "999".into(), // not in the dataset — dropped
            "001".into(),
            "001".into(), // duplicate — collapsed
        ]);
        assert_eq!(got, [CourseIndex::new(0), CourseIndex::new(2)]);
    }

    #[test]
    fn plan_summary_flags_a_same_cell_collision() {
        // Two 1学期 courses both at 月1 collide; a third elsewhere does not.
        let mut a = course("001", &[(0, 0, 1)], 1, 0, "A 001");
        a.unit = Some("2".into());
        let mut b = course("002", &[(0, 0, 1)], 1, 0, "B 002");
        b.unit = Some("1.5".into());
        let c = course("003", &[(0, 2, 3)], 1, 0, "C 003");
        let e = engine_of(vec![a, b, c]);

        let plan = e.resolve_cds(&["001".into(), "002".into(), "003".into()]);
        let summary = e.plan_summary(&plan);
        assert_eq!(summary.conflicts.len(), 1);
        assert_eq!(
            summary.conflicts[0].courses,
            [CourseIndex::new(0), CourseIndex::new(1)]
        );
        assert_eq!(summary.credits.total_courses, 3);
        assert!((summary.credits.total_credits - 3.5).abs() < 1e-6);
        assert_eq!(summary.credits.uncredited, 1); // course C has no unit
    }

    #[test]
    fn plan_summary_does_not_collide_across_semesters() {
        // Same 月1 cell but different terms — not a conflict.
        let a = course("001", &[(0, 0, 1)], 1, 0, "A 001"); // 1学期
        let b = course("002", &[(1, 0, 1)], 1, 0, "B 002"); // 2学期
        let e = engine_of(vec![a, b]);
        let plan = e.resolve_cds(&["001".into(), "002".into()]);
        assert!(e.plan_summary(&plan).conflicts.is_empty());
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
        assert!(matches!(err, super::EngineError::NotV3Format));
    }
}
