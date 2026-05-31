//! DTOs for the v2 JSON wire format emitted by the Go pipeline (`data.json`).
//!
//! Field renames mirror the Go `json:"..."` tags in `internal/model/model.go`
//! and the TS interfaces in `web/src/types/course.ts`. These types are kept
//! deliberately faithful to the wire format; the richer in-memory domain lives
//! in the `engine` layer.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

/// Top-level v2 payload. Deserialized by the consumer (the engine) and
/// serialized by the producer (the native `convert` CLI).
#[derive(Debug, Clone, Deserialize, Serialize)]
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
///
/// A [`BTreeMap`] (not a `HashMap`) so serialization emits keys in a stable,
/// lexical order — byte-identical to Go's `encoding/json`, which sorts map keys.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndicesMap {
    pub semester: BTreeMap<String, String>,
    pub department: BTreeMap<String, String>,
    pub campus: BTreeMap<String, String>,
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

// --- Raw KULAS wire format (producer input) ---
//
// The shapes the `convert` pipeline ingests. Field names mirror the Go
// `json:"..."` tags in `internal/model/model.go`. Unknown keys are ignored
// (no `deny_unknown_fields`), and JSON `null` is tolerated everywhere the Go
// side tolerated it: required strings fall back to `""` (Go leaves the zero
// value), optional pointers become `None`.

/// The KULAS API envelope: `{ "selectKogiDtoList": [...] }`.
#[derive(Debug, Clone, Deserialize)]
pub struct RawResponse {
    /// `Some` (even if empty) when the key is present and non-null — mirroring
    /// the Go `resp.SelectKogiDtoList != nil` check that selects this shape.
    #[serde(rename = "selectKogiDtoList")]
    pub select_kogi_dto_list: Option<Vec<RawCourse>>,
}

/// A single course as delivered by KULAS, before normalization.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawCourse {
    #[serde(rename = "kogiCd", default, deserialize_with = "string_or_null")]
    pub kogi_cd: String,
    #[serde(rename = "kogiNm", default, deserialize_with = "string_or_null")]
    pub kogi_nm: String,
    #[serde(default)]
    pub fukudai: Option<String>,
    #[serde(rename = "tantoKyoin", default, deserialize_with = "string_or_null")]
    pub tanto_kyoin: String,
    #[serde(default, deserialize_with = "string_or_null")]
    pub jikanwari: String,
    #[serde(
        rename = "kogiKaikojikiNm",
        default,
        deserialize_with = "string_or_null"
    )]
    pub kogi_kaikojiki_nm: String,
    #[serde(rename = "kogiKubunNm", default, deserialize_with = "string_or_null")]
    pub kogi_kubun_nm: String,
    #[serde(
        rename = "sekininBushoNm",
        default,
        deserialize_with = "string_or_null"
    )]
    pub sekinin_busho_nm: String,
    #[serde(rename = "kochiNm", default, deserialize_with = "string_or_null")]
    pub kochi_nm: String,
    #[serde(
        rename = "gakusokuKamokuNm",
        default,
        deserialize_with = "string_or_null"
    )]
    pub gakusoku_kamoku_nm: String,
    #[serde(rename = "taishoGakka", default)]
    pub taisho_gakka: Option<String>,
    #[serde(rename = "taishoNenji", default)]
    pub taisho_nenji: Option<String>,
    #[serde(rename = "kamokuBunrui", default)]
    pub kamoku_bunrui: Option<String>,
    #[serde(rename = "kamokuBunya", default)]
    pub kamoku_bunya: Option<String>,
}

/// Deserialize a string field that the producer treats as required, accepting a
/// JSON `null` as `""` (Go's `encoding/json` leaves the zero value on null).
fn string_or_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}
