//! Byte-exact golden over a committed fixture: runs the built binary on
//! `fixtures/sample_raw.json` and asserts byte-identical output to
//! `fixtures/sample_data.golden.json` — compact JSON, HTML escaping, key order,
//! and the no-trailing-newline `-o <file>` write. A synthetic fixture is used
//! because the real `data.json` is a gitignored, monthly-changing artifact.
//!
//! Regenerate: UPDATE_GOLDEN=1 cargo test -p syllabus-cli --test golden_convert

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
