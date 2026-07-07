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
