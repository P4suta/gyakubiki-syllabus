//! Byte-exact golden over a committed fixture: runs the built binary on
//! `fixtures/sample_raw.json` through the sole public `build-dataset` command
//! and asserts that its content-addressed data asset is byte-identical to
//! `fixtures/sample_data.golden.json`. A synthetic fixture is used because the
//! real dataset is a gitignored, monthly-changing artifact.
//!
//! Regenerate: UPDATE_GOLDEN=1 cargo test -p syllabus-cli --test golden_convert

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use syllabus_cli::test_support::DatasetManifest;

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
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("golden-dataset-{}", std::process::id()));
    if out.exists() {
        fs::remove_dir_all(&out).expect("remove stale golden dataset");
    }

    let status = Command::new(env!("CARGO_BIN_EXE_syllabus-cli"))
        .arg("build-dataset")
        .arg(&raw)
        .arg("--output")
        .arg(&out)
        .arg("--generated-at")
        .arg(PINNED_GENERATED_AT)
        .arg("--source-commit")
        .arg("0000000000000000000000000000000000000000")
        .arg("--allow-incomplete-details")
        .status()
        .expect("run syllabus-cli build-dataset");
    assert!(
        status.success(),
        "syllabus-cli build-dataset exited with failure"
    );

    let manifest: DatasetManifest =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).expect("read manifest"))
            .expect("parse manifest");
    let produced = fs::read(
        out.join(&manifest.base_path)
            .join(&manifest.assets.data.path),
    )
    .expect("read content-addressed data asset");
    fs::remove_dir_all(&out).expect("remove golden dataset");

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
