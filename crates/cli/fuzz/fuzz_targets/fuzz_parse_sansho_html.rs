//! Fuzz the「シラバス参照」HTML parser (regex + scraper over recursive text
//! walking). The goal is liveness: no panic, no hang, no OOM on adversarial or
//! truncated markup. Seed the corpus with `sansho_sample.html` for fast reach.
#![no_main]

use libfuzzer_sys::fuzz_target;
use syllabus_cli::detail::parse_sansho_html;

fuzz_target!(|data: &str| {
    let _ = parse_sansho_html("fuzz", data);
});
