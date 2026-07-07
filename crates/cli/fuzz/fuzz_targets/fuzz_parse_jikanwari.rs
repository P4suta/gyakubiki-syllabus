//! Fuzz the timetable-string parser: it must never panic, and it must account
//! for every non-empty comma part (as either a slot or a warning) — the "never
//! silently drop" contract, checked here on adversarial input too.
#![no_main]

use libfuzzer_sys::fuzz_target;
use syllabus_core::parser::parse_jikanwari;

fuzz_target!(|data: &str| {
    let result = parse_jikanwari(data);
    let parts = data.split(',').filter(|p| !p.trim().is_empty()).count();
    assert_eq!(
        result.slots.len() + result.warnings.len(),
        parts,
        "a comma part was dropped or double-counted: {data:?}"
    );
});
