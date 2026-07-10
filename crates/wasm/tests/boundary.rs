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

/// Reuse the committed v3 golden as the boundary fixture (9 courses, year 2026).
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
    assert_eq!(e.year(), "2026");
    assert!(!e.generated_at().is_empty());
    let _ = e.has_saturday(); // just must not trap
}

#[wasm_bindgen_test]
fn from_json_rejects_non_v3_and_raw_kulas() {
    assert!(SyllabusEngine::from_json("{ not valid json").is_err());
    assert!(SyllabusEngine::from_json(r#"{"version":2}"#).is_err());
    // A raw KULAS response must be rejected with a helpful error, not parsed.
    assert!(SyllabusEngine::from_json(r#"{"selectKogiDtoList":[]}"#).is_err());
}

#[wasm_bindgen_test]
fn filter_all_returns_every_course() {
    let all = engine().filter("all", "all", "all", "");
    assert_eq!(all.len(), 9);
    // Ascending, unique indices.
    assert!(all.windows(2).all(|w| w[0] < w[1]));
}

#[wasm_bindgen_test]
fn grid_result_is_camel_case() {
    let e = engine();
    let idx = e.filter("all", "all", "all", "");
    let obj = as_value(e.grid(idx, "all").expect("grid"));
    let map = obj.as_object().expect("grid is an object");
    // The frontend reads `.countUnique` and `.cells`; the snake_case Rust name
    // must not leak across the boundary.
    assert!(map.contains_key("countUnique"), "missing countUnique");
    assert!(!map.contains_key("count_unique"), "snake_case leaked");
    assert!(map.contains_key("cells"));
}

#[wasm_bindgen_test]
fn dicts_expose_all_five_dimensions() {
    let map = as_value(engine().dicts().expect("dicts"));
    let obj = map.as_object().expect("dicts is an object");
    for key in ["semesters", "departments", "campuses", "kubun", "kaikojiki"] {
        assert!(obj.contains_key(key), "missing dict key {key}");
        assert!(obj[key].is_array(), "{key} should be an array");
    }
}

#[wasm_bindgen_test]
fn all_course_views_carry_the_expected_keys() {
    let views = as_value(engine().all_course_views().expect("views"));
    let arr = views.as_array().expect("views is an array");
    assert_eq!(arr.len(), 9);
    let first = arr[0].as_object().expect("course is an object");
    for key in ["cd", "nm"] {
        assert!(first.contains_key(key), "course missing {key}");
    }
    // The search haystack is no longer a wire field.
    assert!(!first.contains_key("st"), "st must not cross the boundary");
}

#[wasm_bindgen_test]
fn query_result_is_camel_case_with_highlights_key() {
    let e = engine();
    let obj = as_value(e.query("all", "all", "all", "x").expect("query"));
    let map = obj.as_object().expect("query is an object");
    assert!(map.contains_key("countUnique"), "missing countUnique");
    assert!(!map.contains_key("count_unique"), "snake_case leaked");
    assert!(map.contains_key("cells"));
    assert!(
        map["highlights"].is_array(),
        "highlights should be an array"
    );
}

#[wasm_bindgen_test]
fn empty_query_yields_no_highlights() {
    let obj = as_value(engine().query("all", "all", "all", "").expect("query"));
    let highlights = obj["highlights"].as_array().expect("array");
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
    let blob = SearchIndex::build(docs).encode();

    let mut e = engine();
    e.load_search_index(&blob).expect("index loads");
    let obj = as_value(e.query("all", "all", "all", "zebra").expect("query"));
    let highlights = obj["highlights"].as_array().expect("array");
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
fn plan_summary_shape_is_camel_case() {
    let obj = as_value(engine().plan_summary(vec![]).expect("plan"));
    let map = obj.as_object().expect("plan summary is an object");
    assert!(map.contains_key("conflicts"));
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
fn resolve_plan_drops_unknown_and_resolves_known() {
    let e = engine();
    assert!(
        e.resolve_plan(vec!["definitely-not-a-code".into()])
            .is_empty()
    );
    // A real cd from the dataset resolves to exactly one index.
    let views = as_value(e.all_course_views().expect("views"));
    let cd = views.as_array().unwrap()[0]["cd"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(e.resolve_plan(vec![cd]).len(), 1);
}
