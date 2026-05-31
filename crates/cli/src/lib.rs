//! Library half of `syllabus-cli`.
//!
//! The two subcommands live here as plain modules so that integration tests and
//! the thin `main` binary share one implementation:
//! - [`convert`] — raw KULAS JSON → the canonical `data.json` bytes (pure).
//! - [`fetch`] — download every findPage page from KULAS into `raw/`.
//! - [`io`] — read and parse the raw input files `convert` ingests.

pub mod convert;
pub mod fetch;
pub mod io;
