use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use syllabus_cli::test_support::{Asset, BuildDatasetOptions, PublicDetail, build_dataset};
use syllabus_core::{Engine, Filters, Offering, ProcessedData};

struct TempOutput(PathBuf);

impl TempOutput {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("gyakubiki-latest-snapshot-{}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale snapshot test output");
        }
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempOutput {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn latest_snapshot_is_complete_searchable_and_plannable() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_owned();
    let mut files: Vec<PathBuf> = fs::read_dir(repository.join("raw"))
        .expect("raw directory")
        .map(|entry| entry.expect("raw entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();
    let output = TempOutput::new();
    let manifest = build_dataset(&BuildDatasetOptions {
        files,
        details_dir: Some(repository.join("raw-details")),
        output_dir: output.path().to_owned(),
        generated_at: "2026-07-26T12:00:00+09:00".into(),
        source_commit: "0000000000000000000000000000000000000000".into(),
        compact: true,
        allow_incomplete_details: false,
    })
    .expect("latest snapshot must build");

    assert_eq!(manifest.counts.courses, 3_928);
    assert_eq!(manifest.counts.details, 3_928);
    assert_eq!(manifest.counts.detail_coverage, 1.0);
    assert_eq!(manifest.counts.scheduled_courses, 2_155);
    assert_eq!(manifest.counts.unscheduled_courses, 1_775);
    assert_eq!(manifest.range.max_period, Some(7));

    let generation = output.path().join(&manifest.base_path);
    let data_bytes = fs::read(generation.join(&manifest.assets.data.path)).expect("data asset");
    let data: ProcessedData = serde_json::from_slice(&data_bytes).expect("typed v4 data");
    assert_eq!(data.courses.len(), 3_928);
    assert_eq!(data.offerings.len(), data.courses.len());

    let scheduled: HashSet<&str> = data
        .courses
        .iter()
        .zip(&data.offerings)
        .filter(|(_, offerings)| offerings.iter().any(Offering::is_scheduled))
        .map(|(course, _)| course.cd.as_str())
        .collect();
    let unscheduled: HashSet<&str> = data
        .courses
        .iter()
        .zip(&data.offerings)
        .filter(|(_, offerings)| offerings.iter().any(|offering| !offering.is_scheduled()))
        .map(|(course, _)| course.cd.as_str())
        .collect();
    assert_eq!(scheduled.len(), 2_155);
    assert_eq!(unscheduled.len(), 1_775);
    assert!(
        data.courses
            .iter()
            .all(|course| scheduled.contains(course.cd.as_str())
                || unscheduled.contains(course.cd.as_str()))
    );
    let unscheduled_only: Vec<&str> = unscheduled.difference(&scheduled).copied().collect();
    assert_eq!(unscheduled_only.len(), 1_773);

    let seventh_period: HashSet<&str> = data
        .courses
        .iter()
        .zip(&data.offerings)
        .filter(|(_, offerings)| {
            offerings
                .iter()
                .any(|offering| matches!(offering, Offering::Scheduled { p: 7, .. }))
        })
        .map(|(course, _)| course.cd.as_str())
        .collect();
    assert_eq!(seventh_period.len(), 19);

    let detail_index: BTreeMap<String, Asset> = serde_json::from_slice(
        &fs::read(generation.join(&manifest.assets.details.path)).expect("detail index"),
    )
    .expect("typed detail index");
    assert_eq!(detail_index.len(), 3_928);

    let mut compressed = fs::File::open(generation.join(&manifest.assets.index.path))
        .expect("compressed search index");
    let mut compressed_bytes = Vec::new();
    compressed
        .read_to_end(&mut compressed_bytes)
        .expect("read compressed search index");
    let mut index_bytes = Vec::new();
    brotli::Decompressor::new(compressed_bytes.as_slice(), 32 * 1024)
        .read_to_end(&mut index_bytes)
        .expect("decompress search index");
    assert_eq!(
        Some(index_bytes.len() as u64),
        manifest.assets.index.decoded_bytes
    );

    let mut engine = Engine::from_json(std::str::from_utf8(&data_bytes).expect("UTF-8 data"))
        .expect("validated engine");
    engine
        .load_search_index(&index_bytes)
        .expect("matching search generation");

    for code in unscheduled_only {
        let asset = detail_index
            .get(code)
            .unwrap_or_else(|| panic!("missing detail asset for {code}"));
        let detail: PublicDetail =
            serde_json::from_slice(&fs::read(generation.join(&asset.path)).expect("detail bytes"))
                .expect("strict public detail");
        assert_eq!(detail.cd, code);

        let hits = engine
            .search(&Filters {
                query: code,
                ..Filters::default()
            })
            .expect("search index is loaded");
        assert!(
            hits.iter()
                .any(|hit| engine.courses()[hit.course.get()].cd == code),
            "{code} must be searchable"
        );

        let plan = engine.resolve_cds(&[code.to_owned()]);
        assert_eq!(plan.len(), 1, "{code} must be addable to a plan");
        assert_eq!(
            engine.unscheduled_indices(&plan, None),
            plan,
            "{code} must remain visible in the unscheduled plan section"
        );
    }
}
