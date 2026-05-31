//! Pure, platform-agnostic core for the gyakubiki-syllabus viewer.
//!
//! This crate is the single source of truth for the runtime logic that used to
//! live in the TypeScript frontend (`web/src/lib`). It deliberately has **no**
//! WASM or DOM dependency so it can be compiled to WASM today (Phase 1) and
//! reused natively to replace the Go pipeline tomorrow (Phase 2).
//!
//! Layering:
//! - [`model`] — faithful DTOs for the v2 JSON wire format (`data.json`).
//! - [`index`] — strongly-typed domain indices (course / dictionary / grid).
//! - [`text`] — query/haystack normalization.
//! - [`bitset`] — the compact filter index primitive.
//! - [`grid`] — the timetable layout (display-agnostic cells).
//! - [`engine`] — the domain layer tying it all together ([`Engine`]).

#![forbid(unsafe_code)]

pub mod bitset;
pub mod engine;
pub mod grid;
pub mod index;
pub mod model;
pub mod text;

pub use engine::{Engine, EngineError, Filters};
pub use grid::Grid;
pub use index::{CampusIndex, CourseIndex, Day, DepartmentIndex, Period, SemesterIndex};
pub use text::normalize;
