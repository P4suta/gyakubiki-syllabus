use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run_build(raw: &Path, output: &Path, failure: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_syllabus-cli"));
    command
        .arg("build-dataset")
        .arg(raw)
        .arg("--output")
        .arg(output)
        .arg("--generated-at")
        .arg(if failure.is_some() {
            "2026-01-02T00:00:00+09:00"
        } else {
            "2026-01-01T00:00:00+09:00"
        })
        .arg("--source-commit")
        .arg("0000000000000000000000000000000000000000")
        .arg("--allow-incomplete-details");
    if let Some(point) = failure {
        command.env("SYLLABUS_FAIL_AT", point);
    }
    command.output().expect("run build-dataset")
}

fn snapshot_tree(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        root: &Path,
        current: &Path,
        output: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(current).expect("read snapshot directory") {
            let path = entry.expect("snapshot entry").path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root).expect("relative path").to_owned(),
                    fs::read(&path).expect("snapshot file"),
                );
            }
        }
    }
    let mut output = std::collections::BTreeMap::new();
    visit(root, root, &mut output);
    output
}

#[test]
fn every_injected_failure_preserves_the_published_generation() {
    let root: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("dataset-atomic-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove old test directory");
    }
    fs::create_dir_all(&root).expect("create test directory");
    let raw = root.join("raw.json");
    fs::write(
        &raw,
        r#"[{
            "kogiCd":"ATOMIC01",
            "kogiNm":"Atomic Dataset",
            "tantoKyoin":"Test",
            "jikanwari":"1学期: 月曜日１時限",
            "kogiKaikojikiNm":"1学期",
            "kogiKubunNm":"講義",
            "sekininBushoNm":"Test",
            "kochiNm":"朝倉キャンパス",
            "syllabusKomokuPatternId":"4",
            "kaikoNendo":"2026"
        }]"#,
    )
    .expect("write raw fixture");
    let public = root.join("public");

    let initial = run_build(&raw, &public, None);
    assert!(
        initial.status.success(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );
    let published_before = snapshot_tree(&public);

    for point in [
        "data",
        "index",
        "details",
        "manifest",
        "promote-manifest",
        "promote-dataset",
    ] {
        let failed = run_build(&raw, &public, Some(point));
        assert!(!failed.status.success(), "{point} failure must abort");
        assert_eq!(
            snapshot_tree(&public),
            published_before,
            "{point} failure changed the published tree"
        );
    }

    fs::remove_dir_all(&root).expect("clean test directory");
}

#[test]
fn partial_detail_crawl_cannot_publish() {
    let root: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("dataset-incomplete-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove old incomplete-test directory");
    }
    fs::create_dir_all(&root).expect("create incomplete-test directory");
    let raw = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_raw.json");
    let public = root.join("public");
    let output = Command::new(env!("CARGO_BIN_EXE_syllabus-cli"))
        .arg("build-dataset")
        .arg(raw)
        .arg("--output")
        .arg(&public)
        .arg("--generated-at")
        .arg("2026-01-01T00:00:00Z")
        .arg("--source-commit")
        .arg("0000000000000000000000000000000000000000")
        .output()
        .expect("run incomplete build");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("detail crawl is incomplete"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!public.join("manifest.json").exists());
    fs::remove_dir_all(&root).expect("clean incomplete-test directory");
}
