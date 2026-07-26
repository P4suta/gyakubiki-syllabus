//! wasm-bindgen wrapper around [`syllabus_core::Engine`].
//!
//! The boundary is deliberately **indices-out**: the dataset lives once in WASM
//! linear memory, queries return only course *indices*, and the rich
//! view-models cross the boundary once via [`SyllabusEngine::init_snapshot`].
//! The JS side caches those and resolves indices against them, so no per-query
//! data marshaling happens.

#![forbid(unsafe_code)]

use serde::Serialize;
use syllabus_core::{Engine, Filters};
use wasm_bindgen::prelude::*;

/// Treat the UI's `"all"` sentinel as "no filter".
fn selector(value: &str) -> Option<&str> {
    (value != "all").then_some(value)
}

fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|_| JsError::new("WASM result serialization failed"))
}

/// One populated timetable cell: course indices at a (day, period) coordinate.
#[derive(Serialize)]
struct GridCell {
    /// Day column index (0=月 … 6=日).
    day: u8,
    /// Period (1限‥8限).
    period: u8,
    /// Course indices in this cell, ascending.
    courses: Vec<u32>,
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

/// A complete query result. Every match is represented in the grid,
/// `unscheduled`, or both; `total` is the distinct hit count.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryResult {
    total: u32,
    scheduled_count: u32,
    unscheduled_count: u32,
    cells: Vec<GridCell>,
    unscheduled: Vec<u32>,
    matches: Vec<Highlight>,
}

/// A timetable collision: the cell coordinate and the colliding course indices.
#[derive(Serialize)]
struct ConflictView {
    day: u8,
    period: u8,
    courses: Vec<u32>,
}

/// One category's rolled-up credits and course count.
#[derive(Serialize)]
struct TallyView {
    key: String,
    credits: f32,
    count: u32,
}

/// Credit totals plus the per-axis tallies (授業形態 / 科目分類 / 対象年次).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreditsView {
    total_credits: f32,
    total_courses: u32,
    uncredited: u32,
    by_kubun: Vec<TallyView>,
    by_bunrui: Vec<TallyView>,
    by_nen: Vec<TallyView>,
}

/// Atomic, course-code based plan result. Raw numeric indices are output-only;
/// callers cannot feed unchecked indices back into the WASM API.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanResultView {
    valid_codes: Vec<String>,
    unknown_codes: Vec<String>,
    cells: Vec<GridCell>,
    unscheduled: Vec<u32>,
    conflicts: Vec<ConflictView>,
    credits: CreditsView,
}

/// One atomic initialization snapshot. Keeping this as a single boundary call
/// prevents consumers from combining metadata, dictionaries, and courses from
/// different engine instances.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitSnapshot<'a, C, D> {
    courses: C,
    dicts: D,
    generated_at: &'a str,
    year: &'a str,
    dataset_id: &'a str,
    day_count: u8,
    max_period: u8,
}

/// The browser-facing handle to a loaded dataset.
#[wasm_bindgen]
pub struct SyllabusEngine {
    inner: Engine,
}

#[wasm_bindgen]
impl SyllabusEngine {
    /// Parse a v4 `data.json` payload (parsing happens here, in WASM).
    ///
    /// # Errors
    /// Rejects raw KULAS responses, non-v4 documents, and malformed bitsets.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<SyllabusEngine, JsError> {
        let inner = Engine::from_json(json).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner })
    }

    /// Load the companion `search.idx` (fetched separately from `data.json`),
    /// enabling ranked [`SyllabusEngine::query`]. Until this is called, a
    /// non-empty query fails explicitly; no incomplete fallback is exposed.
    ///
    /// # Errors
    /// Rejects a blob that is not a valid `search.idx`.
    #[wasm_bindgen(js_name = loadSearchIndex)]
    pub fn load_search_index(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.inner
            .load_search_index(bytes)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Filter, rank, and lay out in one hop: returns scheduled cells,
    /// unscheduled matches, totals, and highlights. Cells are ordered best-first
    /// within each timetable
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
        let hits = self
            .inner
            .search(&Filters {
                semester: selector(semester),
                department: selector(department),
                campus: selector(campus),
                query,
            })
            .map_err(|error| JsError::new(&error.to_string()))?;
        let grid = self.inner.search_grid(&hits, selector(semester));

        let cells = grid
            .cells()
            .map(|(day, period, courses)| GridCell {
                day: day.get(),
                period: period.get(),
                courses: courses.iter().map(|&i| i.get() as u32).collect(),
            })
            .collect();

        let matches = hits
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

        let unscheduled: Vec<u32> = self
            .inner
            .unscheduled(&hits, selector(semester))
            .into_iter()
            .map(|value| value.get() as u32)
            .collect();
        to_js(&QueryResult {
            total: hits.len() as u32,
            scheduled_count: grid.count_unique() as u32,
            unscheduled_count: unscheduled.len() as u32,
            cells,
            unscheduled,
            matches,
        })
    }

    /// Resolve, validate, lay out, and summarize a plan in one atomic call.
    /// Unknown or retired course codes are returned explicitly.
    ///
    /// # Errors
    /// Fails only if the result cannot be serialized to a JS value.
    #[wasm_bindgen]
    pub fn plan(&self, cds: Vec<String>, semester: &str) -> Result<JsValue, JsError> {
        let mut requested = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for code in cds {
            if seen.insert(code.clone()) {
                requested.push(code);
            }
        }
        let indices = self.inner.resolve_cds(&requested);
        let valid_codes: Vec<String> = indices
            .iter()
            .filter_map(|index| self.inner.courses().get(index.get()))
            .map(|course| course.cd.clone())
            .collect();
        let valid: std::collections::HashSet<&str> =
            valid_codes.iter().map(String::as_str).collect();
        let unknown_codes = requested
            .into_iter()
            .filter(|code| !valid.contains(code.as_str()))
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
        let unscheduled = self
            .inner
            .unscheduled_indices(&indices, selector(semester))
            .into_iter()
            .map(|index| index.get() as u32)
            .collect();
        let summary = self.inner.plan_summary(&indices);

        let tally = |ts: &[syllabus_core::CategoryTally]| {
            ts.iter()
                .map(|t| TallyView {
                    key: t.key.clone(),
                    credits: t.credits,
                    count: t.count,
                })
                .collect()
        };
        let view = PlanResultView {
            valid_codes,
            unknown_codes,
            cells,
            unscheduled,
            conflicts: summary
                .conflicts
                .iter()
                .map(|c| ConflictView {
                    day: c.day.get(),
                    period: c.period.get(),
                    courses: c.courses.iter().map(|i| i.get() as u32).collect(),
                })
                .collect(),
            credits: CreditsView {
                total_credits: summary.credits.total_credits,
                total_courses: summary.credits.total_courses,
                uncredited: summary.credits.uncredited,
                by_kubun: tally(&summary.credits.by_kubun),
                by_bunrui: tally(&summary.credits.by_bunrui),
                by_nen: tally(&summary.credits.by_nen),
            },
        };
        to_js(&view)
    }

    /// Return courses, dictionaries, and dataset metadata as one consistent
    /// initialization snapshot.
    ///
    /// # Errors
    /// Fails only if the validated snapshot cannot be serialized to JS.
    #[wasm_bindgen(js_name = initSnapshot)]
    pub fn init_snapshot(&self) -> Result<JsValue, JsError> {
        to_js(&InitSnapshot {
            courses: self.inner.courses(),
            dicts: self.inner.dicts(),
            generated_at: self.inner.generated_at(),
            year: self.inner.year(),
            dataset_id: self.inner.dataset_id(),
            day_count: self.inner.day_count(),
            max_period: self.inner.max_period(),
        })
    }
}

/// Route Rust panics to `console.error` in development builds. The release
/// boundary returns structured errors and omits the hook from the payload.
#[cfg(debug_assertions)]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
