//! Library half of `syllabus-cli`, so integration tests and the thin `main`
//! binary share one implementation:
//! - [`convert`] — raw KULAS JSON → canonical `data.json` bytes (pure).
//! - [`fetch`] / [`fetch_details`] — download findPage pages / detail pages.
//! - [`detail`] — parse「シラバス参照」HTML into structured records.
//! - [`fields`] — the display-field spec and its doc/TS generator.
//! - [`io`] — read and parse the raw input files `convert` ingests.

pub mod banner;
pub mod commit;
pub mod convert;
pub mod detail;
pub mod fetch;
pub mod fetch_details;
pub mod fields;
pub mod gen_sample;
pub mod io;
pub mod net;
pub mod palette;
pub mod term;
