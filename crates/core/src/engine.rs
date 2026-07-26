//! The domain layer: an [`Engine`] that owns the parsed dataset and answers the
//! two questions the UI asks — *which courses match these filters* and *how do
//! they lay out on the timetable*.
//!
//! Parsing, bitset decoding and index construction all happen in
//! [`Engine::from_json`], so the WASM layer only ever marshals **indices**
//! across the boundary.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::bitset::BitSet;
use crate::grid::{Grid, GridSlot, build_grid};
use crate::index::{CourseIndex, SemesterIndex};
use crate::model::{CourseCode, Dictionaries, IndicesMap, Offering, ProcessedData};
use crate::plan::{PlanSummary, conflicts_in_grid, summarize_credits};
use crate::search::{IndexError, SearchHit, SearchIndex};
use crate::text::{normalize, search_text};

/// The semester label whose courses appear under every *other* semester filter.
const TSUUNEN_LABEL: &str = "通年";
const MAX_DATA_JSON_BYTES: usize = 128 * 1024 * 1024;
const MAX_COURSES: usize = 100_000;
const MAX_DICTIONARY_ENTRIES: usize = 20_000;
const MAX_DICTIONARY_VALUE_UTF16: usize = 1_024;
const MAX_COURSE_FIELD_UTF16: usize = 4_096;
const MAX_PATTERN_UTF16: usize = 128;
const MAX_EVALUATION_ITEMS: usize = 32;
const MAX_EVALUATION_ITEM_UTF16: usize = 256;
const MAX_OFFERINGS_PER_COURSE: usize = 128;
const MAX_TOTAL_OFFERINGS: usize = 1_000_000;

/// Errors that can arise while constructing an [`Engine`] from JSON.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The input is a raw KULAS API response, not a converted v4 dataset.
    #[error("This is a raw KULAS response; run `syllabus-cli build-dataset` on it first.")]
    RawKulasResponse,
    /// No usable v4 envelope.
    #[error("Not a v4 dataset.")]
    NotV4Format,
    /// A `version` was present but unsupported.
    #[error("Unsupported version {0}; version 4 is required.")]
    UnsupportedVersion(u32),
    /// The JSON itself could not be parsed / did not match the v4 schema.
    #[error("Failed to parse JSON: {0}")]
    Parse(#[from] serde_json::Error),
    /// A base64 bitset could not be decoded.
    #[error("Failed to decode bitset: {0}")]
    Bitset(#[from] crate::bitset::DecodeError),
    #[error("dataset ID is empty")]
    EmptyDatasetId,
    #[error("dataset identity field {field} is invalid")]
    InvalidIdentity { field: &'static str },
    #[error("declared course count {declared} does not match actual count {actual}")]
    DeclaredCourseCount { declared: u32, actual: usize },
    #[error("{field} count/length {actual} exceeds the limit {max}")]
    LimitExceeded {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("{field} dictionary contains an invalid value at index {index}")]
    InvalidDictionaryValue { field: &'static str, index: usize },
    #[error("{field} dictionary contains duplicate value {value:?}")]
    DuplicateDictionaryValue { field: &'static str, value: String },
    #[error("offerings length {actual} does not match course count {expected}")]
    OfferingCount { expected: usize, actual: usize },
    #[error("duplicate course code {0:?}")]
    DuplicateCourseCode(String),
    #[error("invalid course code {course:?}: {reason}")]
    InvalidCourseCode { course: String, reason: String },
    #[error("course {0:?} has an empty name")]
    EmptyCourseName(String),
    #[error("course {0:?} has no syllabus pattern ID")]
    MissingPatternId(String),
    #[error("course {course:?} has an invalid {field} field")]
    InvalidCourseField { course: String, field: &'static str },
    #[error("dataset academic year is invalid")]
    InvalidYear,
    #[error("course {course:?} references an invalid {field} dictionary index {index}")]
    DictionaryIndex {
        course: String,
        field: &'static str,
        index: u32,
    },
    #[error("course {course:?} has an invalid offering: {reason}")]
    Offering {
        course: String,
        reason: &'static str,
    },
    #[error("{field} index count {actual} does not match dictionary count {expected}")]
    DimensionCount {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
}

/// Query-time failures after a dataset itself has been validated.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryError {
    #[error("full-text search index is not ready")]
    SearchIndexNotReady,
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
    offerings: Vec<Vec<Offering>>,
    /// Each course's validated timetable (parallel to `courses`) — the domain
    /// form the grid consumes, range-checked once here instead of per `grid` call.
    timetables: Vec<Vec<GridSlot>>,
    dicts: Dictionaries,
    generated_at: String,
    dataset_id: String,
    /// Academic year shared by the dataset (for the official syllabus deep link).
    year: String,
    semester_bitsets: Vec<BitSet>,
    department_bitsets: Vec<BitSet>,
    campus_bitsets: Vec<BitSet>,
    /// `1` bits for every course index, the starting point of an AND filter.
    all_bits: BitSet,
    /// The 通年 semester, if the dataset has it.
    tsuunen_index: Option<SemesterIndex>,
    day_count: u8,
    max_period: u8,
    /// A normalized per-course search haystack (name/subtitle/instructor/code/
    /// department/taxonomy), built here at load — the wire format no longer
    /// carries it. Used by [`Engine::filter`] and as [`Engine::search`]'s
    /// pre-index text used only for non-query internal filtering.
    haystack: Vec<String>,
    /// The full-text index, loaded from the companion `search.idx` after the
    /// engine is built (it ships separately from `data.json`). `None` until then;
    /// text queries fail explicitly meanwhile.
    search_index: Option<SearchIndex>,
    /// `cd` → course index, for resolving a shared plan (a list of stable course
    /// codes) back to indices. Built once here so `resolve_cds` is O(n).
    cd_to_index: HashMap<String, CourseIndex>,
}

impl Engine {
    /// Parse a v4 `data.json` payload and build the queryable engine.
    ///
    /// # Errors
    /// Returns an [`EngineError`] if the text is a raw KULAS response, is not a
    /// supported v4 document, or fails schema/bitset decoding.
    pub fn from_json(json: &str) -> Result<Self, EngineError> {
        if json.len() > MAX_DATA_JSON_BYTES {
            return Err(EngineError::LimitExceeded {
                field: "data JSON bytes",
                actual: json.len(),
                max: MAX_DATA_JSON_BYTES,
            });
        }
        if json.contains("\"selectKogiDtoList\"") {
            return Err(EngineError::RawKulasResponse);
        }
        let data: ProcessedData = serde_json::from_str(json).map_err(|error| {
            if !json.contains("\"version\"") {
                EngineError::NotV4Format
            } else {
                EngineError::Parse(error)
            }
        })?;
        if data.version != 4 {
            return Err(EngineError::UnsupportedVersion(data.version));
        }
        Self::build(data)
    }

    /// Construct an engine from an already-deserialized payload (decoding the
    /// base64 bitsets and deriving the cached lookups).
    fn build(data: ProcessedData) -> Result<Self, EngineError> {
        let ProcessedData {
            dicts,
            indices,
            courses,
            offerings,
            generated_at,
            dataset_id,
            year,
            total_raw,
            version: _,
        } = data;
        if dataset_id.trim().is_empty() {
            return Err(EngineError::EmptyDatasetId);
        }
        if dataset_id.len() != 64
            || !dataset_id
                .bytes()
                .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
        {
            return Err(EngineError::InvalidIdentity { field: "datasetId" });
        }
        if generated_at.encode_utf16().count() > 128 || !is_rfc3339(&generated_at) {
            return Err(EngineError::InvalidIdentity {
                field: "generatedAt",
            });
        }
        if year.len() != 4 || !year.chars().all(|value| value.is_ascii_digit()) {
            return Err(EngineError::InvalidYear);
        }
        validate_limit("courses", courses.len(), MAX_COURSES)?;
        if total_raw as usize != courses.len() {
            return Err(EngineError::DeclaredCourseCount {
                declared: total_raw,
                actual: courses.len(),
            });
        }
        if offerings.len() != courses.len() {
            return Err(EngineError::OfferingCount {
                expected: courses.len(),
                actual: offerings.len(),
            });
        }
        let total_offerings = offerings.iter().try_fold(0usize, |total, values| {
            validate_limit(
                "offerings for one course",
                values.len(),
                MAX_OFFERINGS_PER_COURSE,
            )?;
            total
                .checked_add(values.len())
                .ok_or(EngineError::LimitExceeded {
                    field: "total offerings",
                    actual: usize::MAX,
                    max: MAX_TOTAL_OFFERINGS,
                })
        })?;
        validate_limit("total offerings", total_offerings, MAX_TOTAL_OFFERINGS)?;
        validate_dictionary("semester", &dicts.semesters)?;
        validate_dictionary("department", &dicts.departments)?;
        validate_dictionary("campus", &dicts.campuses)?;
        validate_dictionary("kubun", &dicts.kubun)?;
        validate_dictionary("kaikojiki", &dicts.kaikojiki)?;

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

        let mut seen_codes = HashSet::new();
        for (course, course_offerings) in courses.iter().zip(&offerings) {
            CourseCode::parse(&course.cd).map_err(|error| EngineError::InvalidCourseCode {
                course: course.cd.clone(),
                reason: error.to_string(),
            })?;
            if !seen_codes.insert(course.cd.as_str()) {
                return Err(EngineError::DuplicateCourseCode(course.cd.clone()));
            }
            if course.nm.trim().is_empty() {
                return Err(EngineError::EmptyCourseName(course.cd.clone()));
            }
            for (field, value) in [
                ("name", Some(course.nm.as_str())),
                ("subtitle", course.sub.as_deref()),
                ("instructor", Some(course.prof.as_str())),
                ("raw timetable", Some(course.raw.as_str())),
                ("faculty", course.gaku.as_deref()),
                ("department label", course.gakka.as_deref()),
                ("year label", course.nen.as_deref()),
                ("classification", course.bunrui.as_deref()),
                ("field", course.bunya.as_deref()),
                ("credits", course.unit.as_deref()),
            ] {
                if value.is_some_and(|value| value.encode_utf16().count() > MAX_COURSE_FIELD_UTF16)
                {
                    return Err(EngineError::InvalidCourseField {
                        course: course.cd.clone(),
                        field,
                    });
                }
            }
            if course
                .pat
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(EngineError::MissingPatternId(course.cd.clone()));
            }
            if course
                .pat
                .as_deref()
                .is_some_and(|value| value.encode_utf16().count() > MAX_PATTERN_UTF16)
            {
                return Err(EngineError::InvalidCourseField {
                    course: course.cd.clone(),
                    field: "pattern ID",
                });
            }
            if course
                .dm
                .as_deref()
                .is_some_and(|value| !matches!(value, "onsite" | "online" | "ondemand" | "hybrid"))
            {
                return Err(EngineError::InvalidCourseField {
                    course: course.cd.clone(),
                    field: "delivery mode",
                });
            }
            if course.ev.as_ref().is_some_and(|values| {
                values.len() > MAX_EVALUATION_ITEMS
                    || values
                        .iter()
                        .any(|value| value.encode_utf16().count() > MAX_EVALUATION_ITEM_UTF16)
            }) {
                return Err(EngineError::InvalidCourseField {
                    course: course.cd.clone(),
                    field: "evaluation summary",
                });
            }
            validate_dictionary_index(course, "department", course.dept, dicts.departments.len())?;
            validate_dictionary_index(course, "campus", course.campus, dicts.campuses.len())?;
            validate_dictionary_index(course, "kubun", course.kbn, dicts.kubun.len())?;
            validate_dictionary_index(course, "kaikojiki", course.ki, dicts.kaikojiki.len())?;
            if course_offerings.is_empty() {
                return Err(EngineError::Offering {
                    course: course.cd.clone(),
                    reason: "no offering",
                });
            }
            for offering in course_offerings {
                if offering.semester() >= dicts.semesters.len() {
                    return Err(EngineError::Offering {
                        course: course.cd.clone(),
                        reason: "semester index is out of range",
                    });
                }
                if let Offering::Scheduled { d, p, .. } = offering
                    && (*d > 6 || !(1..=8).contains(p))
                {
                    return Err(EngineError::Offering {
                        course: course.cd.clone(),
                        reason: "scheduled day/period is out of range",
                    });
                }
                if let Offering::Tba { label, .. } = offering
                    && (label.trim().is_empty()
                        || label.encode_utf16().count() > MAX_DICTIONARY_VALUE_UTF16)
                {
                    return Err(EngineError::Offering {
                        course: course.cd.clone(),
                        reason: "TBA label is empty or too long",
                    });
                }
            }
        }

        // Validate each course's v4 offerings into grid slots once, here.
        let timetables: Vec<Vec<GridSlot>> = courses
            .iter()
            .zip(&offerings)
            .map(|(_, values)| values.iter().filter_map(GridSlot::from_offering).collect())
            .collect();
        let max_day = offerings
            .iter()
            .flatten()
            .filter_map(|offering| match offering {
                Offering::Scheduled { d, .. } => Some(*d),
                _ => None,
            })
            .max()
            .unwrap_or(4);
        let day_count = max_day.saturating_add(1).max(5);
        let max_period = offerings
            .iter()
            .flatten()
            .filter_map(|offering| match offering {
                Offering::Scheduled { p, .. } => Some(*p),
                _ => None,
            })
            .max()
            .unwrap_or(1);
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
        let bitset_words = all_bits_word_len(courses.len());
        let semester_bitsets =
            decode_dimension("semester", &semester, dicts.semesters.len(), bitset_words)?;
        let department_bitsets = decode_dimension(
            "department",
            &department,
            dicts.departments.len(),
            bitset_words,
        )?;
        let campus_bitsets =
            decode_dimension("campus", &campus, dicts.campuses.len(), bitset_words)?;

        Ok(Self {
            courses,
            offerings,
            timetables,
            dicts,
            generated_at,
            dataset_id,
            year,
            semester_bitsets,
            department_bitsets,
            campus_bitsets,
            all_bits,
            tsuunen_index,
            day_count,
            max_period,
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
    /// enabling ranked search with match spans. Text queries return
    /// [`QueryError::SearchIndexNotReady`] until this succeeds.
    ///
    /// # Errors
    /// Returns an [`IndexError`] if the blob is not a valid `search.idx`.
    pub fn load_search_index(&mut self, bytes: &[u8]) -> Result<(), IndexError> {
        let index = SearchIndex::decode(bytes)?;
        if index.dataset_id() != self.dataset_id {
            return Err(IndexError::DatasetMismatch);
        }
        if index.document_count() != self.courses.len() {
            return Err(IndexError::DocumentCountMismatch {
                expected: self.courses.len(),
                actual: index.document_count(),
            });
        }
        self.search_index = Some(index);
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
    /// loaded `search.idx`. An incomplete fallback is never presented as a
    /// finished result.
    pub fn search(&self, filters: &Filters) -> Result<Vec<SearchHit>, QueryError> {
        if self.courses.is_empty() {
            return Ok(Vec::new());
        }
        let bits = self.candidate_bits(filters);
        let candidates = bits.iter_ones().map(CourseIndex::new);

        if filters.query.is_empty() {
            return Ok(candidates.map(SearchHit::unranked).collect());
        }
        match &self.search_index {
            Some(index) => Ok(index.search(filters.query, candidates)),
            None => Err(QueryError::SearchIndexNotReady),
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
            self.day_count,
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
                .filter_map(|&i| self.timetables.get(i.get()).map(|t| (i, t.as_slice()))),
            semester_index,
            self.tsuunen_index,
            self.day_count,
        )
    }

    /// Courses in a query result that have an intensive/TBA offering in the
    /// selected semester. A course with both kinds remains visible in both the
    /// grid and this list; `total` remains the distinct hit count.
    #[must_use]
    pub fn unscheduled(&self, hits: &[SearchHit], semester: Option<&str>) -> Vec<CourseIndex> {
        let indices: Vec<CourseIndex> = hits.iter().map(|hit| hit.course).collect();
        self.unscheduled_indices(&indices, semester)
    }

    /// Return the selected plan courses that have an intensive/TBA offering in
    /// the requested semester. Invalid indices are ignored at this defensive
    /// boundary rather than indexing the course arrays.
    #[must_use]
    pub fn unscheduled_indices(
        &self,
        indices: &[CourseIndex],
        semester: Option<&str>,
    ) -> Vec<CourseIndex> {
        let selected =
            semester.and_then(|value| self.dicts.semesters.iter().position(|s| s == value));
        indices
            .iter()
            .filter(|index| {
                self.offerings.get(index.get()).is_some_and(|offerings| {
                    offerings.iter().any(|offering| {
                        !offering.is_scheduled()
                            && selected.is_none_or(|s| {
                                offering.semester() == s
                                    || self
                                        .tsuunen_index
                                        .is_some_and(|t| offering.semester() == t.get())
                            })
                    })
                })
            })
            .copied()
            .collect()
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

    #[must_use]
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    /// Number of weekday columns derived from the dataset (5…7).
    #[must_use]
    pub fn day_count(&self) -> u8 {
        self.day_count
    }

    /// Highest period derived from the dataset (1…8).
    #[must_use]
    pub fn max_period(&self) -> u8 {
        self.max_period
    }

    /// Whether the timetable needs a Saturday or Sunday column.
    #[must_use]
    pub fn has_saturday(&self) -> bool {
        self.day_count >= 6
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
fn decode_dimension(
    field: &'static str,
    encoded: &[String],
    expected_count: usize,
    expected_words: usize,
) -> Result<Vec<BitSet>, EngineError> {
    if encoded.len() != expected_count {
        return Err(EngineError::DimensionCount {
            field,
            expected: expected_count,
            actual: encoded.len(),
        });
    }
    encoded
        .iter()
        .map(|value| BitSet::from_base64_words(value, expected_words).map_err(EngineError::from))
        .collect()
}

fn validate_limit(field: &'static str, actual: usize, max: usize) -> Result<(), EngineError> {
    if actual > max {
        return Err(EngineError::LimitExceeded { field, actual, max });
    }
    Ok(())
}

fn validate_dictionary(field: &'static str, values: &[String]) -> Result<(), EngineError> {
    validate_limit(field, values.len(), MAX_DICTIONARY_ENTRIES)?;
    let mut seen = HashSet::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if value.trim().is_empty()
            || value.encode_utf16().count() > MAX_DICTIONARY_VALUE_UTF16
            || value.chars().any(char::is_control)
        {
            return Err(EngineError::InvalidDictionaryValue { field, index });
        }
        if !seen.insert(value.as_str()) {
            return Err(EngineError::DuplicateDictionaryValue {
                field,
                value: value.clone(),
            });
        }
    }
    Ok(())
}

fn is_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !value.is_ascii()
        || bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }
    let Some(year) = decimal(bytes, 0, 4) else {
        return false;
    };
    let Some(month) = decimal(bytes, 5, 7) else {
        return false;
    };
    let Some(day) = decimal(bytes, 8, 10) else {
        return false;
    };
    let Some(hour) = decimal(bytes, 11, 13) else {
        return false;
    };
    let Some(minute) = decimal(bytes, 14, 16) else {
        return false;
    };
    let Some(second) = decimal(bytes, 17, 19) else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if !(1..=max_day).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }

    let mut cursor = 19;
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == start {
            return false;
        }
    }
    if bytes.get(cursor) == Some(&b'Z') {
        return cursor + 1 == bytes.len();
    }
    if !matches!(bytes.get(cursor), Some(b'+' | b'-'))
        || cursor + 6 != bytes.len()
        || bytes.get(cursor + 3) != Some(&b':')
    {
        return false;
    }
    decimal(bytes, cursor + 1, cursor + 3).is_some_and(|offset_hour| offset_hour <= 23)
        && decimal(bytes, cursor + 4, cursor + 6).is_some_and(|offset_minute| offset_minute <= 59)
}

fn decimal(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
    let digits = bytes.get(start..end)?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    digits.iter().try_fold(0u32, |value, digit| {
        value.checked_mul(10)?.checked_add(u32::from(digit - b'0'))
    })
}

const fn all_bits_word_len(course_count: usize) -> usize {
    course_count.div_ceil(64)
}

fn validate_dictionary_index(
    course: &crate::model::Course,
    field: &'static str,
    index: u32,
    len: usize,
) -> Result<(), EngineError> {
    if index as usize >= len {
        return Err(EngineError::DictionaryIndex {
            course: course.cd.clone(),
            field,
            index,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Fixture indices are built here as little-endian `u64` words, base64-encoded,
    //! so `from_base64` round-trips them bit-for-bit.

    use super::{Engine, Filters};
    use crate::index::CourseIndex;
    use crate::model::{Course, Dictionaries, IndicesMap, Offering, ProcessedData, Slot};
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
            pat: Some("4".into()),
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
        let num_words = n.div_ceil(64);
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

    fn processed(courses: Vec<Course>) -> ProcessedData {
        let d = dicts();
        let indices = build_test_indices(&courses, &d);
        let offerings = courses
            .iter()
            .map(|course| {
                let scheduled: Vec<Offering> = course
                    .slots
                    .iter()
                    .map(|slot| Offering::Scheduled {
                        s: slot.s,
                        d: slot.d as u8,
                        p: slot.p as u8,
                    })
                    .collect();
                if scheduled.is_empty() {
                    vec![Offering::Tba {
                        s: 0,
                        label: "時間未定".into(),
                    }]
                } else {
                    scheduled
                }
            })
            .collect();
        ProcessedData {
            version: 4,
            dataset_id: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            generated_at: "2026-05-31T00:00:00Z".into(),
            year: "2026".into(),
            total_raw: courses.len() as u32,
            dicts: d,
            indices,
            courses,
            offerings,
        }
    }

    fn engine_of(courses: Vec<Course>) -> Engine {
        Engine::build(processed(courses)).expect("engine builds")
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
    /// so `search` takes the ranked path rather than reporting index-not-ready.
    fn load_index(engine: &mut Engine) {
        use crate::search::{DocFields, SearchIndex};
        let bytes = SearchIndex::build_for_dataset(
            engine.dataset_id(),
            engine.courses.iter().map(|c| DocFields {
                name: &c.nm,
                subtitle: c.sub.as_deref(),
                instructor: &c.prof,
                code: &c.cd,
                ..DocFields::default()
            }),
        )
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
        let hits = e
            .search(&Filters {
                query: "田",
                ..Default::default()
            })
            .unwrap();
        // 田 appears in course 0's instructor (山田) and course 1's instructor
        // (田中): both hit, and every hit carries at least one span.
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| !h.spans.is_empty()));
    }

    #[test]
    fn search_reports_not_ready_before_the_index_loads() {
        let e = engine_of(sample());
        assert_eq!(
            e.search(&Filters {
                query: "微分",
                ..Default::default()
            })
            .unwrap_err(),
            super::QueryError::SearchIndexNotReady
        );
    }

    #[test]
    fn empty_query_search_is_every_candidate_unranked() {
        let mut e = engine_of(sample());
        load_index(&mut e);
        let hits = e.search(&Filters::default()).unwrap();
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

        let hits = e
            .search(&Filters {
                query: "田中",
                ..Default::default()
            })
            .unwrap();
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
        let mut data = serde_json::to_value(
            crate::convert::convert_v4(&[], "2026-01-01T00:00:00Z".into(), "test".into())
                .unwrap()
                .data,
        )
        .unwrap();
        data["version"] = serde_json::json!(1);
        let err = Engine::from_json(&serde_json::to_string(&data).unwrap()).unwrap_err();
        assert!(matches!(err, super::EngineError::UnsupportedVersion(1)));
    }

    #[test]
    fn rejects_missing_version() {
        let err = Engine::from_json(r#"{"courses": []}"#).unwrap_err();
        assert!(matches!(err, super::EngineError::NotV4Format));
    }

    #[test]
    fn rejects_dictionary_reference_outside_the_dictionary() {
        let mut data = processed(sample());
        data.courses[0].dept = 99;
        assert!(matches!(
            Engine::build(data).unwrap_err(),
            super::EngineError::DictionaryIndex {
                field: "department",
                ..
            }
        ));
    }

    #[test]
    fn rejects_declared_course_count_mismatch() {
        let mut data = processed(sample());
        data.total_raw += 1;
        assert!(matches!(
            Engine::build(data).unwrap_err(),
            super::EngineError::DeclaredCourseCount {
                declared: 4,
                actual: 3
            }
        ));
    }

    #[test]
    fn rejects_duplicate_and_unsafe_dictionary_values() {
        let mut duplicate = processed(sample());
        duplicate
            .dicts
            .semesters
            .push(duplicate.dicts.semesters[0].clone());
        assert!(matches!(
            Engine::build(duplicate).unwrap_err(),
            super::EngineError::DuplicateDictionaryValue {
                field: "semester",
                ..
            }
        ));

        let mut unsafe_value = processed(sample());
        unsafe_value.dicts.departments[0] = "unsafe\nvalue".into();
        assert!(matches!(
            Engine::build(unsafe_value).unwrap_err(),
            super::EngineError::InvalidDictionaryValue {
                field: "department",
                index: 0
            }
        ));
    }

    #[test]
    fn rejects_invalid_identity_fields() {
        let mut bad_id = processed(sample());
        bad_id.dataset_id = "not-a-content-hash".into();
        assert!(matches!(
            Engine::build(bad_id).unwrap_err(),
            super::EngineError::InvalidIdentity { field: "datasetId" }
        ));

        let mut bad_timestamp = processed(sample());
        bad_timestamp.generated_at = "2026-02-30T00:00:00Z".into();
        assert!(matches!(
            Engine::build(bad_timestamp).unwrap_err(),
            super::EngineError::InvalidIdentity {
                field: "generatedAt"
            }
        ));
    }

    #[test]
    fn rejects_excessive_offerings_and_course_fields() {
        let mut excessive = processed(sample());
        excessive.offerings[0] =
            vec![Offering::Intensive { s: 0 }; super::MAX_OFFERINGS_PER_COURSE + 1];
        assert!(matches!(
            Engine::build(excessive).unwrap_err(),
            super::EngineError::LimitExceeded {
                field: "offerings for one course",
                ..
            }
        ));

        let mut invalid_mode = processed(sample());
        invalid_mode.courses[0].dm = Some("telepathy".into());
        assert!(matches!(
            Engine::build(invalid_mode).unwrap_err(),
            super::EngineError::InvalidCourseField {
                field: "delivery mode",
                ..
            }
        ));
    }

    #[test]
    fn rejects_short_bitsets_for_multiword_datasets() {
        let courses = (0..65)
            .map(|index| course(&format!("C{index:03}"), &[(0, 0, 1)], 0, 0, "bitset"))
            .collect();
        let mut data = processed(courses);
        data.indices.semester[0] = encode(&[1]);
        assert!(matches!(
            Engine::build(data).unwrap_err(),
            super::EngineError::Bitset(_)
        ));
    }

    #[test]
    fn rejects_index_from_another_dataset_or_document_count() {
        let mut engine = engine_of(sample());
        let wrong_generation = crate::search::SearchIndex::build_for_dataset(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            [crate::search::DocFields::default(); 3],
        )
        .encode();
        assert_eq!(
            engine.load_search_index(&wrong_generation).unwrap_err(),
            crate::search::IndexError::DatasetMismatch
        );

        let wrong_count = crate::search::SearchIndex::build_for_dataset(
            engine.dataset_id(),
            [crate::search::DocFields::default()],
        )
        .encode();
        assert!(matches!(
            engine.load_search_index(&wrong_count).unwrap_err(),
            crate::search::IndexError::DocumentCountMismatch {
                expected: 3,
                actual: 1
            }
        ));
    }
}
