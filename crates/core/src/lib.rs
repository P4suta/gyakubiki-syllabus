//! Pure, platform-agnostic core for the gyakubiki-syllabus viewer: no WASM or DOM
//! dependency, so it compiles to WASM for the browser and runs natively in the CLI.
//!
//! The producer side (`convert` CLI) builds `data.json` from raw KULAS JSON; the
//! consumer side ([`Engine`]) reads it back and answers the UI's filter/grid
//! queries. [`model`] is the v3 wire format the two sides share.

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
