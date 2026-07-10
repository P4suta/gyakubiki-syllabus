//! DTOs for the v3 JSON wire format (`data.json`) the `convert` pipeline emits
//! and the engine consumes.
//!
//! Kept deliberately faithful to the wire format; the richer in-memory domain
//! lives in the `engine` layer.

use serde::{Deserialize, Deserializer, Serialize};

/// Top-level v3 payload. Deserialized by the consumer (the engine) and
/// serialized by the producer (the native `convert` CLI).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessedData {
    pub version: u32,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    /// Academic year (`kaikoNendo`, e.g. `"2026"`) shared by the whole dataset;
    /// used to build the official syllabus deep link. `default` tolerates
    /// `data.json` without the field.
    #[serde(default)]
    pub year: String,
    #[serde(rename = "totalRaw")]
    pub total_raw: u32,
    pub dicts: Dictionaries,
    pub indices: IndicesMap,
    pub courses: Vec<Course>,
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

/// Precomputed base64 bitsets per filter dimension, one positional `Vec` per
/// dimension: element `i` is the bitset for dictionary index `i`.
///
/// The dictionaries are dense (every value has ≥1 course), so the vectors have
/// no holes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndicesMap {
    pub semester: Vec<String>,
    pub department: Vec<String>,
    pub campus: Vec<String>,
}

/// A single time slot, using dictionary indices instead of strings.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Slot {
    /// Index into `Dictionaries::semesters`.
    pub s: u32,
    /// Day index: 0=月, 1=火, 2=水, 3=木, 4=金, 5=土, 6=日.
    pub d: i32,
    /// Period (1-8).
    pub p: i32,
}

/// A course optimized for the frontend. Serializable so the WASM layer can
/// hand a faithful view-model to the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Course {
    pub cd: String,
    pub nm: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    pub prof: String,
    pub raw: String,
    pub slots: Vec<Slot>,
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
    /// `syllabusKomokuPatternId` (e.g. `"4"`/`"5"`), needed to build the official
    /// syllabus deep link. Varies per course, so it is carried through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pat: Option<String>,
    // --- Compact detail summary for the card (full detail lives in details/{cd}.json) ---
    /// 単位数 (from the syllabus detail page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Delivery mode (`onsite`/`online`/`ondemand`/`hybrid`) for the card icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm: Option<String>,
    /// Assessment-type summary for the card, e.g. `["attendance:40","exam:60"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ev: Option<Vec<String>>,
    pub st: String,
}

// --- Raw KULAS wire format (producer input) ---
//
// The shape the `convert` pipeline ingests. Unknown keys are ignored (no
// `deny_unknown_fields`), and JSON scalars are tolerated: required strings fall
// back to `""`, optional pointers become `None`, and stray numbers/bools are
// coerced to strings so one odd field never fails the record. The
// `selectKogiDtoList` envelope is unwrapped in the CLI's `io` module.

/// A single course as delivered by KULAS, before normalization.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawCourse {
    #[serde(rename = "kogiCd", default, deserialize_with = "lenient_string")]
    pub kogi_cd: String,
    #[serde(rename = "kogiNm", default, deserialize_with = "lenient_string")]
    pub kogi_nm: String,
    #[serde(default, deserialize_with = "lenient_opt_string")]
    pub fukudai: Option<String>,
    #[serde(rename = "tantoKyoin", default, deserialize_with = "lenient_string")]
    pub tanto_kyoin: String,
    #[serde(default, deserialize_with = "lenient_string")]
    pub jikanwari: String,
    #[serde(
        rename = "kogiKaikojikiNm",
        default,
        deserialize_with = "lenient_string"
    )]
    pub kogi_kaikojiki_nm: String,
    #[serde(rename = "kogiKubunNm", default, deserialize_with = "lenient_string")]
    pub kogi_kubun_nm: String,
    #[serde(
        rename = "sekininBushoNm",
        default,
        deserialize_with = "lenient_string"
    )]
    pub sekinin_busho_nm: String,
    #[serde(rename = "kochiNm", default, deserialize_with = "lenient_string")]
    pub kochi_nm: String,
    #[serde(
        rename = "gakusokuKamokuNm",
        default,
        deserialize_with = "lenient_string"
    )]
    pub gakusoku_kamoku_nm: String,
    #[serde(
        rename = "taishoGakka",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub taisho_gakka: Option<String>,
    #[serde(
        rename = "taishoNenji",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub taisho_nenji: Option<String>,
    #[serde(
        rename = "kamokuBunrui",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub kamoku_bunrui: Option<String>,
    #[serde(
        rename = "kamokuBunya",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub kamoku_bunya: Option<String>,
    #[serde(
        rename = "syllabusKomokuPatternId",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub syllabus_komoku_pattern_id: Option<String>,
    #[serde(
        rename = "kaikoNendo",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub kaiko_nendo: Option<String>,
    /// Last-updated timestamp (`"20260310175914381"`), used by `fetch-details` to
    /// skip courses whose syllabus is unchanged since the previous crawl.
    #[serde(
        rename = "lastUpdate",
        default,
        deserialize_with = "lenient_opt_string"
    )]
    pub last_update: Option<String>,
}

/// Coerce any JSON scalar into an owned `String`, so a field KULAS sometimes
/// sends as a number (e.g. `"taishoNenji": 1`, `"kaikoNendo": 2026`) does not
/// fail the whole record. `null` and missing map to `None`; arrays/objects fall
/// back to their JSON text rather than erroring.
fn lenient_opt_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    Ok(match Value::deserialize(deserializer)? {
        Value::Null => None,
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    })
}

/// Deserialize a required string field, coercing `null`/numbers/bools to a
/// string and falling back to `""` when absent. Optional fields use
/// [`lenient_opt_string`] directly.
fn lenient_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(lenient_opt_string(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::RawCourse;

    fn parse(json: &str) -> RawCourse {
        serde_json::from_str(json).expect("RawCourse should deserialize")
    }

    #[test]
    fn required_string_accepts_null_as_empty() {
        let c = parse(r#"{"kogiCd":null,"kogiNm":"A"}"#);
        assert_eq!(c.kogi_cd, "");
        assert_eq!(c.kogi_nm, "A");
    }

    #[test]
    fn required_string_coerces_number() {
        // KULAS occasionally sends a numeric code; it must not fail the record.
        let c = parse(r#"{"kogiCd":123}"#);
        assert_eq!(c.kogi_cd, "123");
    }

    #[test]
    fn optional_numeric_field_does_not_kill_the_record() {
        // The regression: `taishoNenji` as a JSON number used to abort the whole
        // deserialize. It must now coerce to a string.
        let c = parse(r#"{"kogiCd":"1","taishoNenji":1,"kaikoNendo":2026}"#);
        assert_eq!(c.taisho_nenji.as_deref(), Some("1"));
        assert_eq!(c.kaiko_nendo.as_deref(), Some("2026"));
    }

    #[test]
    fn optional_null_and_missing_are_none() {
        let c = parse(r#"{"kogiCd":"1","fukudai":null}"#);
        assert!(c.fukudai.is_none());
        assert!(c.last_update.is_none());
    }

    #[test]
    fn optional_bool_is_stringified() {
        let c = parse(r#"{"kogiCd":"1","kamokuBunrui":true}"#);
        assert_eq!(c.kamoku_bunrui.as_deref(), Some("true"));
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let c = parse(r#"{"kogiCd":"1","somethingNew":{"a":1}}"#);
        assert_eq!(c.kogi_cd, "1");
    }
}
