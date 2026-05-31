//! DTOs for the v2 JSON wire format emitted by the Go pipeline (`data.json`).
//!
//! Field renames mirror the Go `json:"..."` tags in `internal/model/model.go`
//! and the TS interfaces in `web/src/types/course.ts`. These types are kept
//! deliberately faithful to the wire format; the richer in-memory domain lives
//! in the `engine` layer.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Top-level v2 payload.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessedDataV2 {
    pub version: u32,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    #[serde(rename = "totalRaw")]
    pub total_raw: u32,
    pub dicts: Dictionaries,
    pub indices: IndicesMap,
    pub courses: Vec<CourseV2>,
}

/// Lookup tables for the dictionary-indexed fields.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dictionaries {
    pub semesters: Vec<String>,
    pub departments: Vec<String>,
    pub campuses: Vec<String>,
    pub kubun: Vec<String>,
    pub kaikojiki: Vec<String>,
}

/// Precomputed base64 bitsets per filter dimension, keyed by dictionary index.
#[derive(Debug, Clone, Deserialize)]
pub struct IndicesMap {
    pub semester: HashMap<String, String>,
    pub department: HashMap<String, String>,
    pub campus: HashMap<String, String>,
}

/// A single time slot, using dictionary indices instead of strings.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct SlotV2 {
    /// Index into `Dictionaries::semesters`.
    pub s: u32,
    /// Day index: 0=月, 1=火, 2=水, 3=木, 4=金, 5=土, 6=日.
    pub d: i32,
    /// Period (1-8).
    pub p: i32,
}

/// A course optimized for the frontend (v2). Serializable so the WASM layer can
/// hand a faithful view-model to the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CourseV2 {
    pub cd: String,
    pub nm: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    pub prof: String,
    pub raw: String,
    pub slots: Vec<SlotV2>,
    pub ki: u32,
    pub kbn: u32,
    pub dept: u32,
    pub campus: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gaku: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gakka: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bunrui: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bunya: Option<String>,
    pub st: String,
}
