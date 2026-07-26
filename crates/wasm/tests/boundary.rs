//! WASM boundary tests: run in Node via `wasm-pack test --node crates/wasm`.
//!
//! The core Engine is already covered natively (fingerprint + invariants); here
//! we pin only the JS-facing *shapes* `serde_wasm_bindgen` produces — the
//! camelCase keys, dictionary keys, and error propagation the frontend depends
//! on. `#![cfg(target_arch = "wasm32")]` keeps native `cargo test` from
//! compiling this (it needs the wasm-only harness).
#![cfg(target_arch = "wasm32")]

use serde_json::Value;
use syllabus_wasm::SyllabusEngine;
use wasm_bindgen_test::*;

/// Reuse the committed v4 golden as the boundary fixture (9 courses, year 2026).
const DATA: &str = include_str!("../../cli/tests/fixtures/sample_data.golden.json");

fn engine() -> SyllabusEngine {
    SyllabusEngine::from_json(DATA).expect("golden data.json builds an engine")
}

fn as_value(js: wasm_bindgen::JsValue) -> Value {
    serde_wasm_bindgen::from_value(js).expect("JsValue deserializes")
}

#[wasm_bindgen_test]
fn from_json_ok_exposes_meta() {
    let e = engine();
    let snapshot = as_value(e.init_snapshot().expect("snapshot"));
    assert_eq!(snapshot["year"], "2026");
    assert!(
        snapshot["generatedAt"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert_eq!(snapshot["datasetId"].as_str().map(str::len), Some(64));
    assert!(snapshot["dayCount"].is_number());
    assert!(snapshot["maxPeriod"].is_number());
}

#[wasm_bindgen_test]
fn from_json_rejects_non_v4_and_raw_kulas() {
    assert!(SyllabusEngine::from_json("{ not valid json").is_err());
    assert!(SyllabusEngine::from_json(r#"{"version":2}"#).is_err());
    // A raw KULAS response must be rejected with a helpful error, not parsed.
    assert!(SyllabusEngine::from_json(r#"{"selectKogiDtoList":[]}"#).is_err());
}

#[wasm_bindgen_test]
fn query_all_returns_every_course() {
    let result = as_value(engine().query("all", "all", "all", "").expect("query all"));
    assert_eq!(result["total"], 9);
}

#[wasm_bindgen_test]
fn query_result_includes_scheduled_and_unscheduled_counts() {
    let obj = as_value(engine().query("all", "all", "all", "").expect("query all"));
    let map = obj.as_object().expect("query is an object");
    for key in [
        "total",
        "scheduledCount",
        "unscheduledCount",
        "cells",
        "unscheduled",
    ] {
        assert!(map.contains_key(key), "missing {key}");
    }
}

#[wasm_bindgen_test]
fn dicts_expose_all_five_dimensions() {
    let snapshot = as_value(engine().init_snapshot().expect("snapshot"));
    let obj = snapshot["dicts"].as_object().expect("dicts is an object");
    for key in ["semesters", "departments", "campuses", "kubun", "kaikojiki"] {
        assert!(obj.contains_key(key), "missing dict key {key}");
        assert!(obj[key].is_array(), "{key} should be an array");
    }
}

#[wasm_bindgen_test]
fn init_snapshot_courses_carry_the_expected_keys() {
    let snapshot = as_value(engine().init_snapshot().expect("snapshot"));
    let arr = snapshot["courses"].as_array().expect("views is an array");
    assert_eq!(arr.len(), 9);
    let first = arr[0].as_object().expect("course is an object");
    for key in ["cd", "nm"] {
        assert!(first.contains_key(key), "course missing {key}");
    }
    // The search haystack is no longer a wire field.
    assert!(!first.contains_key("st"), "st must not cross the boundary");
}

#[wasm_bindgen_test]
fn query_result_has_matches_key() {
    use syllabus_core::{DocFields, SearchIndex};

    let mut e = engine();
    let mut docs = vec![DocFields::default(); 9];
    docs[0] = DocFields {
        name: "x",
        ..Default::default()
    };
    let snapshot = as_value(e.init_snapshot().expect("snapshot"));
    let dataset_id = snapshot["datasetId"].as_str().expect("dataset ID");
    let blob = SearchIndex::build_for_dataset(dataset_id, docs).encode();
    e.load_search_index(&blob).expect("index loads");
    let obj = as_value(e.query("all", "all", "all", "x").expect("query"));
    let map = obj.as_object().expect("query is an object");
    assert!(map.contains_key("total"), "missing total");
    assert!(map.contains_key("scheduledCount"), "missing scheduledCount");
    assert!(
        map.contains_key("unscheduledCount"),
        "missing unscheduledCount"
    );
    assert!(map.contains_key("cells"));
    assert!(map["matches"].is_array(), "matches should be an array");
}

#[wasm_bindgen_test]
fn empty_query_yields_no_highlights() {
    let obj = as_value(engine().query("all", "all", "all", "").expect("query"));
    let highlights = obj["matches"].as_array().expect("array");
    assert!(highlights.is_empty(), "empty query must not highlight");
}

#[wasm_bindgen_test]
fn loaded_index_drives_ranked_highlights() {
    // Build a synthetic index over 9 docs (aligned to the golden's course
    // indices); a query for the token planted in doc 0 must come back with a
    // highlight span for course 0, in the terse {i, spans:[{f,o,l}]} shape.
    use syllabus_core::{DocFields, SearchIndex};
    let mut docs = vec![DocFields::default(); 9];
    docs[0] = DocFields {
        name: "ZEBRA",
        ..Default::default()
    };
    let mut e = engine();
    let snapshot = as_value(e.init_snapshot().expect("snapshot"));
    let dataset_id = snapshot["datasetId"].as_str().expect("dataset ID");
    let blob = SearchIndex::build_for_dataset(dataset_id, docs).encode();
    e.load_search_index(&blob).expect("index loads");
    let obj = as_value(e.query("all", "all", "all", "zebra").expect("query"));
    let highlights = obj["matches"].as_array().expect("array");
    assert_eq!(highlights.len(), 1, "exactly course 0 matches");
    let hit = highlights[0].as_object().expect("highlight is an object");
    assert_eq!(hit["i"], 0);
    let span = hit["spans"][0].as_object().expect("span is an object");
    for key in ["f", "o", "l"] {
        assert!(span.contains_key(key), "span missing {key}");
    }
    assert_eq!(span["o"], 0);
    assert_eq!(span["l"], 5); // "ZEBRA" → 5 UTF-16 units
}

#[wasm_bindgen_test]
fn load_search_index_rejects_a_bad_blob() {
    let mut e = engine();
    assert!(e.load_search_index(b"garbage").is_err());
}

#[wasm_bindgen_test]
fn plan_result_is_atomic_and_camel_case() {
    let obj = as_value(
        engine()
            .plan(vec!["definitely-not-a-code".into()], "all")
            .expect("plan"),
    );
    let map = obj.as_object().expect("plan result is an object");
    for key in [
        "validCodes",
        "unknownCodes",
        "cells",
        "unscheduled",
        "conflicts",
        "credits",
    ] {
        assert!(map.contains_key(key), "plan result missing {key}");
    }
    assert_eq!(map["unknownCodes"][0], "definitely-not-a-code");
    let credits = map["credits"].as_object().expect("credits object");
    for key in [
        "totalCredits",
        "totalCourses",
        "uncredited",
        "byKubun",
        "byBunrui",
        "byNen",
    ] {
        assert!(credits.contains_key(key), "credits missing {key}");
    }
}

#[wasm_bindgen_test]
fn plan_reports_known_and_unknown_codes() {
    let e = engine();
    let snapshot = as_value(e.init_snapshot().expect("snapshot"));
    let cd = snapshot["courses"].as_array().unwrap()[0]["cd"]
        .as_str()
        .unwrap()
        .to_owned();
    let result = as_value(
        e.plan(vec![cd.clone(), "retired-code".into()], "all")
            .expect("plan"),
    );
    assert_eq!(result["validCodes"], serde_json::json!([cd]));
    assert_eq!(result["unknownCodes"], serde_json::json!(["retired-code"]));
}
