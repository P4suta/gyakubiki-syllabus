//! Pure, platform-agnostic core for the gyakubiki-syllabus viewer.
//!
//! This crate is the single source of truth for the runtime logic that used to
//! live in the TypeScript frontend (`web/src/lib`) and the Go pipeline. It
//! deliberately has **no** WASM or DOM dependency, so it both compiles to WASM
//! for the browser and runs natively in the `convert` CLI.
//!
//! Layering:
//! - [`model`] — faithful DTOs for the v3 JSON wire format (`data.json`).
//! - [`index`] — strongly-typed domain indices (course / dictionary / grid).
//! - [`text`] — query/haystack normalization (shared by producer and consumer).
//! - [`bitset`] — the compact filter index primitive.
//!
//! Consumer side (read `data.json`, answer the UI):
//! - [`grid`] — the timetable layout (display-agnostic cells).
//! - [`engine`] — the domain layer tying it all together ([`Engine`]).
//!
//! Producer side (build `data.json` from raw KULAS JSON — the native pipeline
//! that replaces the Go `internal/transform`):
//! - [`parser`] — jikanwari → semester/day/period slots.
//! - [`dict`] — dictionary ordering.
//! - [`convert`] — raw courses → [`model::ProcessedData`] ([`convert_v2`]).

#![forbid(unsafe_code)]

pub mod bitset;
pub mod convert;
pub mod dict;
pub mod engine;
pub mod grid;
pub mod index;
pub mod model;
pub mod parser;
pub mod text;

pub use convert::{convert_v2, ConvertResult};
pub use engine::{Engine, EngineError, Filters};
pub use grid::Grid;
pub use index::{CampusIndex, CourseIndex, Day, DepartmentIndex, Period, SemesterIndex};
pub use text::{normalize, search_text};
