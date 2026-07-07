//! Semantic fingerprint over a committed fixture: runs the whole chain
//! (`convert_v2` → serialize → `Engine::from_json` → `filter`/`grid`) over
//! `fixtures/sample_raw.json` and compares to a committed snapshot. The
//! fingerprint is cd/value-based (never raw indices), so it is immune to
//! reordering and drifts only on a real semantic change. Where the byte golden
//! pins serialization, this pins behaviour.
//!
//! Regenerate: UPDATE_FINGERPRINT=1 cargo test -p syllabus-cli --test fingerprint

use std::fs;
use std::path::{Path, PathBuf};

use syllabus_core::model::RawCourse;
use syllabus_core::{convert_v2, Engine, Filters};

/// A fixed timestamp so `generated_at` in the fingerprint is stable.
const PINNED_GENERATED_AT: &str = "2026-01-01T00:00:00Z";

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Build an engine the way the CLI does: parse raw → `convert_v2` → serialize →
/// `Engine::from_json`, exercising the full producer→consumer round-trip.
fn engine_from_fixture() -> Engine {
    let raw_json =
        fs::read_to_string(fixtures_dir().join("sample_raw.json")).expect("read sample_raw.json");
    let raw: Vec<RawCourse> = serde_json::from_str(&raw_json).expect("parse sample_raw.json");
    let data = convert_v2(&raw, PINNED_GENERATED_AT.to_owned()).data;
    let json = serde_json::to_string(&data).expect("serialize v3 payload");
    Engine::from_json(&json).expect("engine builds from converted fixture")
}

/// FNV-1a 64-bit — a tiny digest so the snapshot stays compact and
/// human-diffable. Deterministic across Rust versions, unlike `DefaultHasher`.
fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in s.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The sorted course codes a filter selects.
fn cds_for(engine: &Engine, filters: &Filters) -> Vec<String> {
    let mut cds: Vec<String> = engine
        .filter(filters)
        .iter()
        .map(|&i| engine.courses()[i.get()].cd.clone())
        .collect();
    cds.sort();
    cds
}

/// One `label: count=N hash=…` line over a sorted cd list.
fn filter_line(label: &str, cds: &[String]) -> String {
    format!(
        "{label}: count={} hash={:016x}\n",
        cds.len(),
        fnv1a(&cds.join("\n"))
    )
}

/// One grid line: distinct-course count, non-empty cell count, and a hash over
/// every `(day,period) -> sorted cds` cell so cell membership is pinned too.
fn grid_line(engine: &Engine, label: &str, filters: &Filters, semester: Option<&str>) -> String {
    let indices = engine.filter(filters);
    let grid = engine.grid(&indices, semester);
    let mut cells: Vec<String> = Vec::new();
    for (day, period, idxs) in grid.cells() {
        let mut cds: Vec<&str> = idxs
            .iter()
            .map(|&i| engine.courses()[i.get()].cd.as_str())
            .collect();
        cds.sort_unstable();
        cells.push(format!("d{}p{}:{}", day.get(), period.get(), cds.join(",")));
    }
    format!(
        "{label}: count_unique={} cells={} hash={:016x}\n",
        grid.count_unique(),
        cells.len(),
        fnv1a(&cells.join(";"))
    )
}

/// The full semantic fingerprint of an engine built from the fixture.
fn fingerprint(engine: &Engine) -> String {
    let dicts = engine.dicts();
    let mut out = String::new();

    out.push_str("== meta ==\n");
    out.push_str(&format!("generated_at: {}\n", engine.generated_at()));
    out.push_str(&format!("has_saturday: {}\n", engine.has_saturday()));
    out.push_str(&format!("courses: {}\n", engine.courses().len()));

    out.push_str("== dicts ==\n");
    out.push_str(&format!(
        "semesters[{}]: {}\n",
        dicts.semesters.len(),
        dicts.semesters.join(" | ")
    ));
    out.push_str(&format!(
        "departments[{}]: {}\n",
        dicts.departments.len(),
        dicts.departments.join(" | ")
    ));
    out.push_str(&format!(
        "campuses[{}]: {}\n",
        dicts.campuses.len(),
        dicts.campuses.join(" | ")
    ));
    out.push_str(&format!(
        "kubun[{}]: {}\n",
        dicts.kubun.len(),
        dicts.kubun.join(" | ")
    ));
    out.push_str(&format!(
        "kaikojiki[{}]: {}\n",
        dicts.kaikojiki.len(),
        dicts.kaikojiki.join(" | ")
    ));

    out.push_str("== filters ==\n");
    out.push_str(&filter_line("all", &cds_for(engine, &Filters::default())));
    for s in &dicts.semesters {
        let f = Filters {
            semester: Some(s.as_str()),
            ..Default::default()
        };
        out.push_str(&filter_line(&format!("semester={s}"), &cds_for(engine, &f)));
    }
    for d in &dicts.departments {
        let f = Filters {
            department: Some(d.as_str()),
            ..Default::default()
        };
        out.push_str(&filter_line(
            &format!("department={d}"),
            &cds_for(engine, &f),
        ));
    }
    for c in &dicts.campuses {
        let f = Filters {
            campus: Some(c.as_str()),
            ..Default::default()
        };
        out.push_str(&filter_line(&format!("campus={c}"), &cds_for(engine, &f)));
    }
    for q in ["微分", "英語", "プログラミング", "海洋", "通年", "001"] {
        let f = Filters {
            query: q,
            ..Default::default()
        };
        out.push_str(&filter_line(&format!("query={q}"), &cds_for(engine, &f)));
    }
    // A combo exercising the bitset AND across all three dimensions.
    if dicts.semesters.len() >= 2 && !dicts.departments.is_empty() && !dicts.campuses.is_empty() {
        let f = Filters {
            semester: Some(dicts.semesters[0].as_str()),
            department: Some(dicts.departments[0].as_str()),
            campus: Some(dicts.campuses[0].as_str()),
            query: "",
        };
        out.push_str(&filter_line("combo[sem0,dep0,camp0]", &cds_for(engine, &f)));
    }

    out.push_str("== grid ==\n");
    out.push_str(&grid_line(engine, "grid[all]", &Filters::default(), None));
    for s in &dicts.semesters {
        let f = Filters {
            semester: Some(s.as_str()),
            ..Default::default()
        };
        out.push_str(&grid_line(
            engine,
            &format!("grid[semester={s}]"),
            &f,
            Some(s.as_str()),
        ));
    }

    out
}

#[test]
fn semantic_fingerprint_is_stable() {
    let engine = engine_from_fixture();
    let actual = fingerprint(&engine);

    let snapshot = fixtures_dir().join("fingerprint.snapshot");

    if std::env::var_os("UPDATE_FINGERPRINT").is_some() {
        fs::write(&snapshot, &actual).expect("write fingerprint snapshot");
        eprintln!("fingerprint snapshot written to {}", snapshot.display());
        return;
    }

    let expected = fs::read_to_string(&snapshot).unwrap_or_else(|_| {
        panic!(
            "missing {}; regenerate with UPDATE_FINGERPRINT=1 cargo test -p syllabus-cli --test fingerprint",
            snapshot.display()
        )
    });
    assert_eq!(
        actual, expected,
        "semantic fingerprint drifted from the committed snapshot"
    );
}
