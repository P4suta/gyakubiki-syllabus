//! Private implementation library for the `syllabus-cli` binary.
//!
//! This crate is not published and does not expose its crawler, converter,
//! terminal, or filesystem modules as a supported Rust API.

mod banner;
mod cli;
mod commit;
mod convert;
mod dataset;
mod detail;
mod fetch;
mod fetch_details;
mod fields;
mod gen_sample;
mod io;
mod net;
mod palette;
mod term;

/// Run the command-line application.
///
/// This exists solely for the minimal binary shim and is not a general-purpose
/// library API.
#[doc(hidden)]
pub fn run() -> std::process::ExitCode {
    cli::run()
}

/// Narrow, unsupported surface used by this package's integration tests and
/// out-of-workspace fuzz targets. Product callers must use the binary.
#[doc(hidden)]
pub mod test_support {
    pub use crate::convert::render_data_json;
    pub use crate::dataset::{Asset, BuildDatasetOptions, DatasetManifest, build as build_dataset};
    pub use crate::detail::{PublicDetail, SanshoDetail, parse_sansho_html};
}
