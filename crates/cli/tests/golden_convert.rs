//! G0 — byte-exact golden over a small committed fixture.
//!
//! Runs the real built binary (`CARGO_BIN_EXE_syllabus-cli`) on the committed
//! `fixtures/sample_raw.json` and asserts the output is byte-identical to the
//! committed `fixtures/sample_data.golden.json`. This pins the exact
//! serialization — compact JSON, Go-style HTML escaping (the fixture contains an
//! `&`), key/field order, and the no-trailing-newline `-o <file>` write — through
//! the same code path production uses.
//!
//! It is deliberately self-contained: the real `web/public/data.json` is a
//! gitignored build artifact that a fresh CI checkout does not have, and `raw/`
//! changes monthly, so neither is a stable golden. The synthetic fixture is.
//!
//! Regenerate after an intended output change:
//!   UPDATE_GOLDEN=1 cargo test -p syllabus-cli --test golden_convert

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A fixed timestamp so the output is deterministic and the golden stable.
const PINNED_GENERATED_AT: &str = "2026-01-01T00:00:00Z";

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn convert_reproduces_golden() {
    let fixtures = fixtures_dir();
    let raw = fixtures.join("sample_raw.json");
    let golden = fixtures.join("sample_data.golden.json");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sample_out.json");

    let status = Command::new(env!("CARGO_BIN_EXE_syllabus-cli"))
        .arg("convert")
        .arg(&raw)
        .arg("--compact")
        .arg("--generated-at")
        .arg(PINNED_GENERATED_AT)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("run syllabus-cli convert");
    assert!(status.success(), "syllabus-cli convert exited with failure");

    let produced = fs::read(&out).expect("read produced output");
    let _ = fs::remove_file(&out);

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&golden, &produced).expect("write golden fixture");
        eprintln!("golden written to {}", golden.display());
        return;
    }

    let expected = fs::read(&golden).unwrap_or_else(|_| {
        panic!(
            "missing {}; regenerate with UPDATE_GOLDEN=1 cargo test -p syllabus-cli --test golden_convert",
            golden.display()
        )
    });
    assert_eq!(
        produced.len(),
        expected.len(),
        "byte length differs: produced {} vs golden {}",
        produced.len(),
        expected.len()
    );
    assert!(
        produced == expected,
        "produced output is not byte-identical to the committed golden"
    );
}
