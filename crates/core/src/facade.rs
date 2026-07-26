//! Stable, code-based public facade.
//!
//! Numeric wire/dictionary indices and conversion internals stay behind feature
//! gated producer/WASM boundaries. General callers use validated course codes,
//! weekdays, and periods only.

use std::collections::HashSet;

use crate::CourseCode;
use crate::engine::{Engine, EngineError, Filters, QueryError};
use crate::index::CourseIndex;
use crate::search::{Field, IndexError};

/// Failure while validating or querying a dataset.
///
/// No public operation panics for malformed data or caller input. Invalid wire
/// data, index data, and search readiness are reported as separate variants.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DatasetError {
    /// The v4 JSON envelope or one of its invariants is invalid.
    #[error("invalid dataset: {0}")]
    InvalidDataset(String),
    /// The search index is malformed or belongs to another dataset generation.
    #[error("invalid search index: {0}")]
    InvalidSearchIndex(String),
    /// A non-empty text query was attempted before the lazy index was loaded.
    #[error("full-text search index is not ready")]
    SearchIndexNotReady,
    /// A validated internal relationship was unexpectedly inconsistent.
    #[error("validated dataset invariant failed: {0}")]
    Invariant(&'static str),
}

impl From<EngineError> for DatasetError {
    fn from(error: EngineError) -> Self {
        Self::InvalidDataset(error.to_string())
    }
}

impl From<IndexError> for DatasetError {
    fn from(error: IndexError) -> Self {
        Self::InvalidSearchIndex(error.to_string())
    }
}

impl From<QueryError> for DatasetError {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::SearchIndexNotReady => Self::SearchIndexNotReady,
        }
    }
}

/// A complete validated dataset and its optional lazy search index.
#[derive(Debug)]
pub struct Dataset {
    engine: Engine,
}

/// Code-based query selectors. `None` means all values for that dimension.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// Exact semester dictionary value, or every semester when absent.
    pub semester: Option<String>,
    /// Exact department dictionary value, or every department when absent.
    pub department: Option<String>,
    /// Exact campus dictionary value, or every campus when absent.
    pub campus: Option<String>,
    /// Unicode-normalized substring query. Empty text browses all candidates.
    pub text: String,
}

/// Complete query output, including both grid and non-grid offerings.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    /// Number of distinct matching courses.
    pub total: u32,
    /// Number of matching courses placed in at least one grid cell.
    pub scheduled_count: u32,
    /// Number of matching courses with an intensive or time-undecided offering.
    pub unscheduled_count: u32,
    /// Populated timetable cells in data order.
    pub cells: Vec<QueryCell>,
    /// Intensive/time-undecided courses, including mixed courses also in cells.
    pub unscheduled: Vec<CourseCode>,
    /// Ranked matches and their field-level highlight spans.
    pub matches: Vec<QueryMatch>,
}

/// One populated timetable cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryCell {
    /// Day of the week.
    pub day: Weekday,
    /// One-based class period.
    pub period: Period,
    /// Courses occupying this cell, in query rank order.
    pub courses: Vec<CourseCode>,
}

/// One ranked full-text match.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryMatch {
    /// Stable course code.
    pub course: CourseCode,
    /// Weighted relevance score; larger values rank earlier.
    pub score: f32,
    /// Matching UTF-16 spans grouped by syllabus field.
    pub spans: Vec<MatchSpan>,
}

/// A highlight span using browser-compatible UTF-16 offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchSpan {
    /// Syllabus field containing the match.
    pub field: MatchField,
    /// Zero-based start offset in UTF-16 code units.
    pub start_utf16: u32,
    /// Span length in UTF-16 code units.
    pub length_utf16: u32,
}

/// Searchable syllabus field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MatchField {
    /// Course name.
    Name,
    /// Course subtitle.
    Subtitle,
    /// Instructor summary on the course card.
    Instructor,
    /// Course code.
    Code,
    /// Department and taxonomy labels.
    Department,
    /// Course summary.
    Summary,
    /// Course aims/purpose.
    Aims,
    /// Learning goals.
    Goals,
    /// Lesson plan.
    Plan,
    /// Textbooks and references.
    Textbooks,
    /// Prerequisites and enrolment conditions.
    Prerequisite,
    /// Preparation and review work.
    Preparation,
    /// Office hours.
    OfficeHour,
    /// Keywords.
    Keywords,
    /// Detailed teacher field.
    Teachers,
    /// Course numbering.
    Numbering,
    /// Sustainable Development Goals.
    Sdgs,
    /// Evaluation method.
    Evaluation,
    /// Delivery method.
    Delivery,
    /// Allowlisted additional syllabus fields.
    Extra,
}

impl From<Field> for MatchField {
    fn from(value: Field) -> Self {
        match value {
            Field::Name => Self::Name,
            Field::Subtitle => Self::Subtitle,
            Field::Instructor => Self::Instructor,
            Field::Code => Self::Code,
            Field::Department => Self::Department,
            Field::Summary => Self::Summary,
            Field::Aims => Self::Aims,
            Field::Goals => Self::Goals,
            Field::Plan => Self::Plan,
            Field::Textbooks => Self::Textbooks,
            Field::Prerequisite => Self::Prerequisite,
            Field::Preparation => Self::Preparation,
            Field::OfficeHour => Self::OfficeHour,
            Field::Keywords => Self::Keywords,
            Field::Teachers => Self::Teachers,
            Field::Numbering => Self::Numbering,
            Field::Sdgs => Self::Sdgs,
            Field::Evaluation => Self::Evaluation,
            Field::Delivery => Self::Delivery,
            Field::Extra => Self::Extra,
        }
    }
}

/// Day of the week represented without a numeric wire index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    fn from_wire(value: u8) -> Result<Self, DatasetError> {
        match value {
            0 => Ok(Self::Monday),
            1 => Ok(Self::Tuesday),
            2 => Ok(Self::Wednesday),
            3 => Ok(Self::Thursday),
            4 => Ok(Self::Friday),
            5 => Ok(Self::Saturday),
            6 => Ok(Self::Sunday),
            _ => Err(DatasetError::Invariant("weekday outside Monday-Sunday")),
        }
    }
}

/// Error returned when constructing a [`Period`] outside 1–8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("period must be between 1 and 8, got {0}")]
pub struct PeriodError(u8);

/// Validated one-based class period in the inclusive range 1–8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Period(u8);

impl Period {
    /// Construct a validated one-based class period.
    ///
    /// # Errors
    ///
    /// Returns [`PeriodError`] when `value` is outside 1–8.
    pub fn new(value: u8) -> Result<Self, PeriodError> {
        if (1..=8).contains(&value) {
            Ok(Self(value))
        } else {
            Err(PeriodError(value))
        }
    }

    #[must_use]
    /// Return the one-based numeric period.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Atomic result of resolving and analysing a requested study plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanResult {
    /// Known, de-duplicated codes in dataset order.
    pub valid_codes: Vec<CourseCode>,
    /// Unknown or retired requested codes in request order.
    pub unknown_codes: Vec<String>,
    /// Scheduled valid courses laid out in timetable cells.
    pub cells: Vec<QueryCell>,
    /// Valid intensive/time-undecided courses.
    pub unscheduled: Vec<CourseCode>,
    /// Every timetable collision among the valid courses.
    pub conflicts: Vec<PlanConflict>,
    /// Credit totals for the valid courses.
    pub credits: PlanCredits,
}

/// Two or more selected courses that occupy the same timetable cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanConflict {
    /// Collision day.
    pub day: Weekday,
    /// Collision period.
    pub period: Period,
    /// All selected courses in the cell.
    pub courses: Vec<CourseCode>,
}

/// Aggregate credit statistics for a study plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanCredits {
    /// Sum of parseable course credits.
    pub total_credits: f32,
    /// Number of valid selected courses.
    pub total_courses: u32,
    /// Valid courses whose credit value was unavailable/unparseable.
    pub uncredited: u32,
    /// Totals by lecture/exercise/practical kind.
    pub by_kind: Vec<CreditTally>,
    /// Totals by course classification.
    pub by_classification: Vec<CreditTally>,
    /// Totals by target year.
    pub by_year: Vec<CreditTally>,
}

/// Credit/count aggregate for one category label.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTally {
    /// Category label from the dataset.
    pub key: String,
    /// Sum of parseable credits in this category.
    pub credits: f32,
    /// Number of courses in this category.
    pub count: u32,
}

impl Dataset {
    /// Parse and validate a v4 data asset.
    ///
    /// # Errors
    ///
    /// Returns [`DatasetError::InvalidDataset`] for malformed JSON, unsupported
    /// schema versions, unsafe values, invalid references, or exceeded limits.
    pub fn from_json(json: &str) -> Result<Self, DatasetError> {
        Ok(Self {
            engine: Engine::from_json(json)?,
        })
    }

    /// Attach the manifest-selected positional search index.
    ///
    /// # Errors
    ///
    /// Returns [`DatasetError::InvalidSearchIndex`] for malformed, oversized,
    /// truncated, trailing, document-count-mismatched, or generation-mismatched
    /// index data.
    pub fn load_search_index(&mut self, bytes: &[u8]) -> Result<(), DatasetError> {
        self.engine.load_search_index(bytes)?;
        Ok(())
    }

    #[must_use]
    /// Return the content-addressed dataset generation ID.
    pub fn dataset_id(&self) -> &str {
        self.engine.dataset_id()
    }

    #[must_use]
    /// Return the four-digit academic year.
    pub fn year(&self) -> &str {
        self.engine.year()
    }

    #[must_use]
    /// Return the number of validated courses.
    pub fn course_count(&self) -> usize {
        self.engine.courses().len()
    }

    /// Execute a browse/filter/full-text query.
    ///
    /// # Errors
    ///
    /// A non-empty text query returns [`DatasetError::SearchIndexNotReady`]
    /// until [`Dataset::load_search_index`] succeeds. Internal invariant errors
    /// are returned rather than panicking.
    pub fn query(&self, query: &Query) -> Result<QueryResult, DatasetError> {
        let semester = query.semester.as_deref();
        let hits = self.engine.search(&Filters {
            semester,
            department: query.department.as_deref(),
            campus: query.campus.as_deref(),
            query: &query.text,
        })?;
        let grid = self.engine.search_grid(&hits, semester);
        let cells = grid
            .cells()
            .map(|(day, period, courses)| {
                Ok(QueryCell {
                    day: Weekday::from_wire(day.get())?,
                    period: Period::new(period.get())
                        .map_err(|_| DatasetError::Invariant("period outside 1-8"))?,
                    courses: courses
                        .iter()
                        .copied()
                        .map(|index| self.code_at(index))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, DatasetError>>()?;
        let unscheduled = self
            .engine
            .unscheduled(&hits, semester)
            .into_iter()
            .map(|index| self.code_at(index))
            .collect::<Result<Vec<_>, _>>()?;
        let matches = hits
            .iter()
            .map(|hit| {
                Ok(QueryMatch {
                    course: self.code_at(hit.course)?,
                    score: hit.score,
                    spans: hit
                        .spans
                        .iter()
                        .map(|span| MatchSpan {
                            field: span.field.into(),
                            start_utf16: span.start,
                            length_utf16: span.len,
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, DatasetError>>()?;
        Ok(QueryResult {
            total: hits.len() as u32,
            scheduled_count: grid.count_unique() as u32,
            unscheduled_count: unscheduled.len() as u32,
            cells,
            unscheduled,
            matches,
        })
    }

    /// Resolve codes, report unknown values, lay out all valid selections, and
    /// calculate conflicts/credits atomically.
    ///
    /// Unknown/retired and duplicate input codes are safe caller input. They are
    /// reported or de-duplicated, never treated as numeric indices.
    ///
    /// # Errors
    ///
    /// Returns an invariant error if validated internal relationships become
    /// inconsistent; malformed caller codes do not panic.
    pub fn plan(
        &self,
        requested_codes: &[String],
        semester: Option<&str>,
    ) -> Result<PlanResult, DatasetError> {
        let mut requested = Vec::new();
        let mut seen = HashSet::new();
        for code in requested_codes {
            if seen.insert(code.as_str()) {
                requested.push(code.clone());
            }
        }
        let indices = self.engine.resolve_cds(&requested);
        let valid_codes = indices
            .iter()
            .copied()
            .map(|index| self.code_at(index))
            .collect::<Result<Vec<_>, _>>()?;
        let valid: HashSet<&str> = valid_codes.iter().map(CourseCode::as_str).collect();
        let unknown_codes = requested
            .into_iter()
            .filter(|code| !valid.contains(code.as_str()))
            .collect();
        let grid = self.engine.grid(&indices, semester);
        let cells = grid
            .cells()
            .map(|(day, period, courses)| {
                Ok(QueryCell {
                    day: Weekday::from_wire(day.get())?,
                    period: Period::new(period.get())
                        .map_err(|_| DatasetError::Invariant("period outside 1-8"))?,
                    courses: courses
                        .iter()
                        .copied()
                        .map(|index| self.code_at(index))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, DatasetError>>()?;
        let unscheduled = self
            .engine
            .unscheduled_indices(&indices, semester)
            .into_iter()
            .map(|index| self.code_at(index))
            .collect::<Result<Vec<_>, _>>()?;
        let summary = self.engine.plan_summary(&indices);
        let conflicts = summary
            .conflicts
            .into_iter()
            .map(|conflict| {
                Ok(PlanConflict {
                    day: Weekday::from_wire(conflict.day.get())?,
                    period: Period::new(conflict.period.get())
                        .map_err(|_| DatasetError::Invariant("period outside 1-8"))?,
                    courses: conflict
                        .courses
                        .into_iter()
                        .map(|index| self.code_at(index))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, DatasetError>>()?;
        Ok(PlanResult {
            valid_codes,
            unknown_codes,
            cells,
            unscheduled,
            conflicts,
            credits: PlanCredits {
                total_credits: summary.credits.total_credits,
                total_courses: summary.credits.total_courses,
                uncredited: summary.credits.uncredited,
                by_kind: tallies(summary.credits.by_kubun),
                by_classification: tallies(summary.credits.by_bunrui),
                by_year: tallies(summary.credits.by_nen),
            },
        })
    }

    fn code_at(&self, index: CourseIndex) -> Result<CourseCode, DatasetError> {
        let course = self
            .engine
            .courses()
            .get(index.get())
            .ok_or(DatasetError::Invariant("course index outside dataset"))?;
        CourseCode::parse(&course.cd)
            .map_err(|_| DatasetError::Invariant("stored course code was not validated"))
    }
}

fn tallies(values: Vec<crate::plan::CategoryTally>) -> Vec<CreditTally> {
    values
        .into_iter()
        .map(|value| CreditTally {
            key: value.key,
            credits: value.credits,
            count: value.count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::convert_v4;
    use crate::model::RawCourse;
    use crate::search::{DocFields, SearchIndex};

    fn raw(code: &str, timetable: &str) -> RawCourse {
        RawCourse {
            kogi_cd: code.into(),
            kogi_nm: format!("科目{code}"),
            tanto_kyoin: "教員".into(),
            jikanwari: timetable.into(),
            kogi_kaikojiki_nm: "1学期".into(),
            kogi_kubun_nm: "講義".into(),
            sekinin_busho_nm: "理工学部".into(),
            kochi_nm: "朝倉キャンパス".into(),
            syllabus_komoku_pattern_id: Some("4".into()),
            kaiko_nendo: Some("2026".into()),
            ..RawCourse::default()
        }
    }

    #[test]
    fn public_query_and_plan_use_codes_not_numeric_input() {
        const DATASET_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let converted = convert_v4(
            &[
                raw("SCHEDULED", "1学期: 月曜日１時限"),
                raw("INTENSIVE", "1学期: 集中講義"),
            ],
            "2026-07-26T00:00:00+09:00".into(),
            DATASET_ID.into(),
        )
        .unwrap();
        let json = serde_json::to_string(&converted.data).unwrap();
        let mut dataset = Dataset::from_json(&json).unwrap();
        let index = SearchIndex::build_for_dataset(
            DATASET_ID,
            [
                DocFields {
                    name: "科目SCHEDULED",
                    code: "SCHEDULED",
                    ..DocFields::default()
                },
                DocFields {
                    name: "科目INTENSIVE",
                    code: "INTENSIVE",
                    ..DocFields::default()
                },
            ],
        );
        dataset.load_search_index(&index.encode()).unwrap();

        let result = dataset
            .query(&Query {
                text: "INTENSIVE".into(),
                ..Query::default()
            })
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.unscheduled[0].as_str(), "INTENSIVE");

        let plan = dataset
            .plan(&["INTENSIVE".into(), "RETIRED".into()], None)
            .unwrap();
        assert_eq!(plan.valid_codes[0].as_str(), "INTENSIVE");
        assert_eq!(plan.unknown_codes, ["RETIRED"]);
        assert_eq!(plan.unscheduled[0].as_str(), "INTENSIVE");
    }
}
