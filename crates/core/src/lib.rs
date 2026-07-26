//! Validated, code-based API for the gyakubiki-syllabus dataset.
//!
//! General callers use [`Dataset`], [`Query`], [`QueryResult`], and
//! [`PlanResult`]. The numeric wire model, conversion pipeline, and WASM bridge
//! are deliberately feature-gated so unchecked dictionary indices do not leak
//! into the supported public API.
//!
//! # Example
//!
//! ```no_run
//! use syllabus_core::{Dataset, Query};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data = std::fs::read_to_string("data.content-hash.json")?;
//! let index = std::fs::read("search.content-hash.idx")?;
//! let mut dataset = Dataset::from_json(&data)?;
//! dataset.load_search_index(&index)?;
//!
//! let result = dataset.query(&Query {
//!     semester: Some("1学期".into()),
//!     text: "微分積分".into(),
//!     ..Query::default()
//! })?;
//! println!("{} results", result.total);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod bitset;
#[cfg(any(feature = "producer", test))]
mod convert;
#[cfg(any(feature = "producer", test))]
mod dict;
mod engine;
mod facade;
mod grid;
mod index;
mod model;
#[cfg(any(feature = "producer", test))]
mod parser;
mod plan;
mod search;
mod text;

pub use facade::{
    CreditTally, Dataset, DatasetError, MatchField, MatchSpan, Period, PeriodError, PlanConflict,
    PlanCredits, PlanResult, Query, QueryCell, QueryMatch, QueryResult, Weekday,
};
pub use model::{CourseCode, CourseCodeError};

#[cfg(feature = "producer")]
#[doc(hidden)]
pub use convert::{ConvertError, ConvertResult, convert_v4};
#[cfg(any(feature = "producer", feature = "wasm-internal"))]
#[doc(hidden)]
pub use engine::{Engine, EngineError, Filters, QueryError};
#[cfg(any(feature = "producer", feature = "wasm-internal"))]
#[doc(hidden)]
pub use grid::Grid;
#[cfg(feature = "producer")]
#[doc(hidden)]
pub use index::{
    CampusIndex, CourseIndex, Day, DepartmentIndex, Period as WirePeriod, SemesterIndex,
};
#[cfg(feature = "producer")]
#[doc(hidden)]
pub use model::{Course, Offering, ProcessedData, RawCourse};
#[cfg(feature = "producer")]
#[doc(hidden)]
pub use parser::{
    ParseResult as TimetableParseResult, ParsedSlot, ParsedUnscheduled, UnscheduledKind,
    parse_jikanwari,
};
#[cfg(any(feature = "producer", feature = "wasm-internal"))]
#[doc(hidden)]
pub use plan::{CategoryTally, Conflict, CreditSummary, PlanSummary};
#[cfg(feature = "producer")]
#[doc(hidden)]
pub use search::{DocFields, Field, IndexError, SearchHit, SearchIndex, Span};
#[cfg(feature = "producer")]
#[doc(hidden)]
pub use text::{fold_char, normalize, search_text};
