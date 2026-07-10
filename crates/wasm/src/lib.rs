//! wasm-bindgen wrapper around [`syllabus_core::Engine`].
//!
//! The boundary is deliberately **indices-out**: the dataset lives once in WASM
//! linear memory, `filter`/`grid` return only course *indices*, and the rich
//! view-models cross the boundary once via [`SyllabusEngine::all_course_views`].
//! The JS side caches those and resolves indices against them, so no per-query
//! data marshaling happens.

#![forbid(unsafe_code)]

use serde::Serialize;
use syllabus_core::{CourseIndex, Engine, Filters};
use wasm_bindgen::prelude::*;

/// Treat the UI's `"all"` sentinel as "no filter".
fn selector(value: &str) -> Option<&str> {
    (value != "all").then_some(value)
}

fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}

/// One populated timetable cell: course indices at a (day, period) coordinate.
#[derive(Serialize)]
struct GridCell {
    /// Day column index (0=月 … 5=土).
    day: u8,
    /// Period (1限‥6限).
    period: u8,
    /// Course indices in this cell, ascending.
    courses: Vec<u32>,
}

/// The grid plus its distinct-course count, handed back in one object.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GridResult {
    cells: Vec<GridCell>,
    count_unique: u32,
}

/// One match span: field discriminant, and UTF-16 offset/length into that
/// field's original display text. Terse keys keep the per-query payload small.
#[derive(Serialize)]
struct HlSpan {
    f: u8,
    o: u32,
    l: u32,
}

/// Match spans for one course (referenced by index `i`).
#[derive(Serialize)]
struct Highlight {
    i: u32,
    spans: Vec<HlSpan>,
}

/// A full-text query result: the score-ordered grid, its distinct-course count,
/// and per-course highlight spans (empty when the query is empty).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryResult {
    cells: Vec<GridCell>,
    count_unique: u32,
    highlights: Vec<Highlight>,
}

/// The browser-facing handle to a loaded dataset.
#[wasm_bindgen]
pub struct SyllabusEngine {
    inner: Engine,
}

#[wasm_bindgen]
impl SyllabusEngine {
    /// Parse a v3 `data.json` payload (parsing happens here, in WASM).
    ///
    /// # Errors
    /// Rejects raw KULAS responses, non-v3 documents, and malformed bitsets.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<SyllabusEngine, JsError> {
        let inner = Engine::from_json(json).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner })
    }

    /// Load the companion `search.idx` (fetched separately from `data.json`),
    /// enabling ranked [`SyllabusEngine::query`]. Until this is called, `query`
    /// falls back to an unranked substring scan.
    ///
    /// # Errors
    /// Rejects a blob that is not a valid `search.idx`.
    #[wasm_bindgen(js_name = loadSearchIndex)]
    pub fn load_search_index(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.inner
            .load_search_index(bytes)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Filter, rank, and lay out in one hop: returns `{ cells, countUnique,
    /// highlights }`, cells already ordered best-first within each timetable
    /// slot. `highlights` carries per-course match spans (empty for an empty
    /// query). Scores never cross the boundary — the ordering already encodes
    /// them.
    ///
    /// # Errors
    /// Fails only if the result cannot be serialized to a JS value.
    #[wasm_bindgen]
    pub fn query(
        &self,
        semester: &str,
        department: &str,
        campus: &str,
        query: &str,
    ) -> Result<JsValue, JsError> {
        let hits = self.inner.search(&Filters {
            semester: selector(semester),
            department: selector(department),
            campus: selector(campus),
            query,
        });
        let grid = self.inner.search_grid(&hits, selector(semester));

        let cells = grid
            .cells()
            .map(|(day, period, courses)| GridCell {
                day: day.get(),
                period: period.get(),
                courses: courses.iter().map(|&i| i.get() as u32).collect(),
            })
            .collect();

        let highlights = hits
            .iter()
            .filter(|h| !h.spans.is_empty())
            .map(|h| Highlight {
                i: h.course.get() as u32,
                spans: h
                    .spans
                    .iter()
                    .map(|s| HlSpan {
                        f: s.field as u8,
                        o: s.start,
                        l: s.len,
                    })
                    .collect(),
            })
            .collect();

        to_js(&QueryResult {
            cells,
            count_unique: grid.count_unique() as u32,
            highlights,
        })
    }

    /// Indices of courses matching the filters (`"all"` = no filter), ascending.
    #[wasm_bindgen]
    pub fn filter(&self, semester: &str, department: &str, campus: &str, query: &str) -> Vec<u32> {
        self.inner
            .filter(&Filters {
                semester: selector(semester),
                department: selector(department),
                campus: selector(campus),
                query,
            })
            .into_iter()
            .map(|i| i.get() as u32)
            .collect()
    }

    /// Lay the given (already-filtered) indices onto the timetable, returning
    /// `{ cells, countUnique }`.
    ///
    /// # Errors
    /// Fails only if the result cannot be serialized to a JS value.
    #[wasm_bindgen]
    pub fn grid(&self, course_indices: Vec<u32>, semester: &str) -> Result<JsValue, JsError> {
        let indices: Vec<CourseIndex> = course_indices
            .into_iter()
            .map(|i| CourseIndex::new(i as usize))
            .collect();
        let grid = self.inner.grid(&indices, selector(semester));

        let cells = grid
            .cells()
            .map(|(day, period, courses)| GridCell {
                day: day.get(),
                period: period.get(),
                courses: courses.iter().map(|&i| i.get() as u32).collect(),
            })
            .collect();

        to_js(&GridResult {
            cells,
            count_unique: grid.count_unique() as u32,
        })
    }

    /// All course view-models, in index order — fetched once to seed the JS
    /// read-only cache.
    ///
    /// # Errors
    /// Fails only if the courses cannot be serialized to a JS value.
    #[wasm_bindgen(js_name = allCourseViews)]
    pub fn all_course_views(&self) -> Result<JsValue, JsError> {
        to_js(self.inner.courses())
    }

    /// The dictionaries (semesters / departments / campuses / kubun / kaikojiki).
    ///
    /// # Errors
    /// Fails only if the dictionaries cannot be serialized to a JS value.
    #[wasm_bindgen]
    pub fn dicts(&self) -> Result<JsValue, JsError> {
        to_js(self.inner.dicts())
    }

    /// Whether the timetable needs a Saturday column (derived from the data).
    #[wasm_bindgen(js_name = hasSaturday)]
    pub fn has_saturday(&self) -> bool {
        self.inner.has_saturday()
    }

    /// When the dataset was generated (RFC 3339 string).
    #[wasm_bindgen(js_name = generatedAt)]
    pub fn generated_at(&self) -> String {
        self.inner.generated_at().to_owned()
    }

    /// The dataset's academic year (`kaikoNendo`), for the official deep link.
    #[wasm_bindgen(js_name = year)]
    pub fn year(&self) -> String {
        self.inner.year().to_owned()
    }
}

/// Route Rust panics to `console.error` for legible stack traces in the browser.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
