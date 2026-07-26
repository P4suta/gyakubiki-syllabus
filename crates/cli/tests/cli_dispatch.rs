//! Process-level CLI behaviour: subcommand dispatch, help, and clean failure.
//!
//! Safety: no test here may reach KULAS. The only `fetch`/`fetch-details` path
//! exercised is the early return when zero courses are selected — it stops
//! before any session/HTTP is established.

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("syllabus-cli").expect("binary builds")
}

/// A fresh empty directory under the test tmp dir.
fn empty_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn help_lists_every_subcommand() {
    bin().arg("--help").assert().success().stdout(
        predicate::str::contains("build-dataset")
            .and(predicate::str::contains("fetch"))
            .and(predicate::str::contains("fetch-details"))
            .and(predicate::str::contains("gen-field-docs")),
    );
}

#[test]
fn build_dataset_empty_input_fails_cleanly() {
    let root = empty_dir("cli_dispatch_empty_dataset");
    let raw = root.join("empty.json");
    fs::write(&raw, "[]").expect("write empty fixture");
    bin()
        .arg("build-dataset")
        .arg(&raw)
        .args([
            "--source-commit",
            "0000000000000000000000000000000000000000",
        ])
        .arg("--output")
        .arg(root.join("public"))
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No course data").or(predicate::str::contains("no course")),
        );
}

#[test]
fn build_dataset_unrecognized_json_fails_cleanly() {
    let root = empty_dir("cli_dispatch_bad_dataset");
    let raw = root.join("bad.json");
    fs::write(&raw, r#"{"unexpected": true}"#).expect("write bad fixture");
    bin()
        .arg("build-dataset")
        .arg(&raw)
        .args([
            "--source-commit",
            "0000000000000000000000000000000000000000",
        ])
        .arg("--output")
        .arg(root.join("public"))
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot parse typed findPage response")
                .and(predicate::str::contains("unknown field `unexpected`")),
        );
}

#[test]
fn unknown_subcommand_is_rejected() {
    bin().arg("bogus").assert().failure();
}

#[test]
fn gen_palette_stdout_is_one_machine_readable_json_document() {
    let output = bin()
        .arg("gen-palette")
        .output()
        .expect("gen-palette process runs");
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is strict JSON");
    assert_eq!(value["light"].as_array().map(Vec::len), Some(10));
    assert_eq!(value["dark"].as_array().map(Vec::len), Some(10));
    assert_eq!(value["eval"].as_array().map(Vec::len), Some(6));
}

#[test]
fn fetch_details_defaults_are_documented() {
    // The politeness defaults are a load-safety contract; surface them in --help.
    // Base sleep 3000ms + up to 2000ms jitter (3-5s/course).
    bin()
        .args(["fetch-details", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("3000").and(predicate::str::contains("2000")));
}

#[test]
fn fetch_details_with_no_courses_stops_before_any_network() {
    // A raw file with a real course, but `--only` selects a code that isn't
    // there → 0 selected → early return. Crucially this never constructs a
    // SanshoClient, so KULAS is never contacted.
    let raw = empty_dir("cli_dispatch_raw");
    let out = empty_dir("cli_dispatch_out");
    fs::write(
        raw.join("courses.json"),
        r#"{"pageNo":1,"maxPageNo":1,"total":1,"pageSize":1,"selectKogiDtoList":[{"kogiCd":"001","kogiNm":"A","kaikoNendo":"2026","syllabusKomokuPatternId":"4"}]}"#,
    )
    .expect("write raw fixture");
    bin()
        .args(["fetch-details"])
        .arg("--raw-dir")
        .arg(&raw)
        .arg("--out-dir")
        .arg(&out)
        .args(["--only", "__NONE__"])
        .assert()
        .success()
        .stderr(predicate::str::contains("nothing to fetch"));
}
