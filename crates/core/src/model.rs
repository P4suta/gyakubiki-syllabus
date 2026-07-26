//! DTOs for the v4 JSON wire format (`data.json`) the dataset builder emits
//! and the engine consumes.
//!
//! Kept deliberately faithful to the wire format; the richer in-memory domain
//! lives in the `engine` layer.

#[cfg(any(feature = "producer", test))]
use std::collections::BTreeSet;
#[cfg(any(feature = "producer", test))]
use std::fmt;

#[cfg(any(feature = "producer", test))]
use serde::Deserializer;
#[cfg(any(feature = "producer", test))]
use serde::de::{self, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

/// Top-level v4 payload. `offerings[i]` belongs to `courses[i]`; keeping the
/// scheduling union outside the compact card view avoids repeating course
/// metadata while still preserving scheduled, intensive, and TBA offerings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessedData {
    pub version: u32,
    #[serde(rename = "datasetId")]
    pub dataset_id: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    /// Academic year (`kaikoNendo`, e.g. `"2026"`) shared by the whole dataset;
    /// used to build the official syllabus deep link.
    pub year: String,
    #[serde(rename = "totalRaw")]
    pub total_raw: u32,
    pub dicts: Dictionaries,
    pub indices: IndicesMap,
    pub courses: Vec<Course>,
    pub offerings: Vec<Vec<Offering>>,
}

/// Lookup tables for the dictionary-indexed fields.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct IndicesMap {
    pub semester: Vec<String>,
    pub department: Vec<String>,
    pub campus: Vec<String>,
}

/// A scheduled time slot, using dictionary indices instead of strings.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    /// Index into `Dictionaries::semesters`.
    pub s: u32,
    /// Day index: 0=月, 1=火, 2=水, 3=木, 4=金, 5=土, 6=日.
    pub d: i32,
    /// Period (1-8).
    pub p: i32,
}

/// One way in which a course is offered in a semester.
///
/// The tagged union is deliberately explicit: an intensive/TBA course is data,
/// not a malformed scheduled slot. The compact field names keep the public
/// dataset small; serde's tag makes runtime validation straightforward.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Offering {
    Scheduled {
        /// Index into `Dictionaries::semesters`.
        s: u32,
        /// Day index: 0=月 … 6=日.
        d: u8,
        /// Period: 1…8.
        p: u8,
    },
    Intensive {
        /// Index into `Dictionaries::semesters`.
        s: u32,
    },
    Tba {
        /// Index into `Dictionaries::semesters`.
        s: u32,
        /// Original non-empty label, retained for display/debugging.
        label: String,
    },
}

impl Offering {
    #[must_use]
    pub const fn semester(&self) -> usize {
        match self {
            Self::Scheduled { s, .. } | Self::Intensive { s } | Self::Tba { s, .. } => *s as usize,
        }
    }

    #[must_use]
    pub const fn is_scheduled(&self) -> bool {
        matches!(self, Self::Scheduled { .. })
    }
}

/// A validated course code. Public file paths are never derived directly from
/// this value (the dataset manifest maps codes to content hashes), but rejecting
/// separators/control characters here also protects logs and internal maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CourseCode(String);

/// Failure to construct a safe [`CourseCode`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CourseCodeError {
    /// The trimmed code is empty.
    #[error("course code is empty")]
    Empty,
    /// The code is longer than 128 UTF-16 code units.
    #[error("course code exceeds 128 characters")]
    TooLong,
    /// The code contains a control character, `/`, or `\`.
    #[error("course code contains a control character or path separator")]
    UnsafeCharacter,
}

impl CourseCode {
    /// Parse a trimmed, path-safe course code.
    ///
    /// # Errors
    ///
    /// Returns [`CourseCodeError`] for an empty, oversized, or unsafe value.
    pub fn parse(value: &str) -> Result<Self, CourseCodeError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(CourseCodeError::Empty);
        }
        if value.encode_utf16().count() > 128 {
            return Err(CourseCodeError::TooLong);
        }
        if value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
        {
            return Err(CourseCodeError::UnsafeCharacter);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    /// Borrow the validated code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CourseCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for CourseCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::str::FromStr for CourseCode {
    type Err = CourseCodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for CourseCode {
    type Error = CourseCodeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// A course optimized for the frontend. Serializable so the WASM layer can
/// hand a faithful view-model to the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Course {
    pub cd: String,
    pub nm: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    pub prof: String,
    pub raw: String,
    /// Legacy in-memory convenience for native callers. v4 serializes the
    /// lossless `ProcessedData::offerings` union instead.
    #[serde(skip)]
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
    // --- Compact detail summary for the card (full detail is manifest-addressed) ---
    /// 単位数 (from the syllabus detail page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Delivery mode (`onsite`/`online`/`ondemand`/`hybrid`) for the card icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm: Option<String>,
    /// Assessment-type summary for the card, e.g. `["attendance:40","exam:60"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ev: Option<Vec<String>>,
}

// --- Raw KULAS wire format (producer input) ---
//
// The shape the producer ingests. KULAS currently returns many fields that are
// intentionally not published. Their names are still explicitly allowlisted:
// a newly introduced source field is a schema change and fails the build until
// somebody decides whether it belongs in the public model. String-like scalar
// fields accept JSON strings/numbers/bools/null, but arrays/objects are rejected.
// This custom visitor performs one typed deserialize and never builds a
// `serde_json::Value` tree.

/// A single course as delivered by KULAS, before normalization.
#[cfg(any(feature = "producer", test))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct RawCourse {
    #[serde(rename = "kogiCd")]
    pub kogi_cd: String,
    #[serde(rename = "kogiNm")]
    pub kogi_nm: String,
    #[serde(default)]
    pub fukudai: Option<String>,
    #[serde(rename = "tantoKyoin")]
    pub tanto_kyoin: String,
    #[serde(default)]
    pub jikanwari: String,
    #[serde(rename = "kogiKaikojikiNm")]
    pub kogi_kaikojiki_nm: String,
    #[serde(rename = "kogiKubunNm")]
    pub kogi_kubun_nm: String,
    #[serde(rename = "sekininBushoNm")]
    pub sekinin_busho_nm: String,
    #[serde(rename = "kochiNm")]
    pub kochi_nm: String,
    #[serde(rename = "gakusokuKamokuNm")]
    pub gakusoku_kamoku_nm: String,
    #[serde(rename = "taishoGakka", default)]
    pub taisho_gakka: Option<String>,
    #[serde(rename = "taishoNenji", default)]
    pub taisho_nenji: Option<String>,
    #[serde(rename = "kamokuBunrui", default)]
    pub kamoku_bunrui: Option<String>,
    #[serde(rename = "kamokuBunya", default)]
    pub kamoku_bunya: Option<String>,
    #[serde(rename = "syllabusKomokuPatternId", default)]
    pub syllabus_komoku_pattern_id: Option<String>,
    #[serde(rename = "kaikoNendo", default)]
    pub kaiko_nendo: Option<String>,
    /// Last-updated timestamp (`"20260310175914381"`), used by `fetch-details` to
    /// skip courses whose syllabus is unchanged since the previous crawl.
    #[serde(rename = "lastUpdate", default)]
    pub last_update: Option<String>,
}

/// Source fields intentionally ignored by the published dataset. Keeping the
/// complete list here turns an upstream addition into a loud schema error.
#[cfg(any(feature = "producer", test))]
const KNOWN_IGNORED_SOURCE_FIELDS: &[&str] = &[
    "biko1",
    "biko2",
    "chusenTaishoFlg",
    "chuyaKubunCd",
    "chuyaKubunNm",
    "daihyoJigenNm",
    "daihyoKogiFlg",
    "daihyoKyoinCd",
    "daihyoKyoinNm",
    "daihyoNumberingCd",
    "daihyoYobiNm",
    "gairyaku",
    "gakuseiMessage",
    "gakushuMokuhyo",
    "gakusokuKamokuCd",
    "gpcaKeisanTaishoFlg",
    "gpcaKeisanTaishoFlgNm",
    "haitoClassCdEnd",
    "haitoClassCdStart",
    "haitoGakunen",
    "haitoSemester",
    "haitoShozokuCdEnd",
    "haitoShozokuCdStart",
    "headerKomoku1",
    "headerKomoku2",
    "headerKomoku3",
    "headerKomoku4",
    "hyokaHoho",
    "isOpened",
    "jigen",
    "jugyoKeishiki",
    "junbiGakushu",
    "kamokuKaiso1",
    "kanaKogiNm",
    "kBasho",
    "kBiko1",
    "kBiko10",
    "kBiko2",
    "kBiko3",
    "kBiko4",
    "kBiko5",
    "kBiko6",
    "kBiko7",
    "kBiko8",
    "kBiko9",
    "keyword",
    "kJitsumuKeiken",
    "kKoji",
    "kochiCd",
    "kOfficeHour",
    "kogiGroup",
    "kogiGroupCd",
    "kogiGroupNm",
    "kogiJiyuCd",
    "kogiJiyuNm",
    "kogiKaikojikiCd",
    "kogiKaisu",
    "kogiKubunCd",
    "kogiRnm",
    "kogiseisekiShikenhohoNm",
    "kogiseisekiShikenshubetsuPatternCd",
    "kogiseisekiShikenshubetsuPatternNm",
    "kokaiFlg",
    "kokaiFlgNm",
    "kYobi",
    "kyoin",
    "kyoinShimei",
    "kyoinSosaStatusNm",
    "kyointantoKubunNm",
    "lastUser",
    "lockFlg",
    "lockFlgNm",
    "nendoKeizokuFlg",
    "nyuryokuKanryoFlg",
    "nyuryokuKanryoFlgNm",
    "nyuryokuKikanPatternCd",
    "nyuryokuKikanPatternNm",
    "officeHour",
    "rishu",
    "rishuKarteFlg",
    "sagyoKanryoFlg",
    "sagyoKanryoFlgNm",
    "sankoBunken",
    "sanshoUrl",
    "sekininBushoCd",
    "select",
    "shikenanketoTaishoFlgNm",
    "shikenshubetsuRyokinPatternCd",
    "shippitsuTantoKyoin",
    "shosai",
    "shozokubetsuNumberingCd",
    "sokaikoJikansu",
    "sosaKyoin",
    "sosaNichiji",
    "syllabusKanren",
    "syllabusKomokuPatternNm",
    "text",
    "textIsbn",
    "torokuType",
    "tsuisaishikanriMode",
    "tsuisaishikanriModeNm",
    "tsuisaishiKikanPatternCd",
    "tsuisaishiKikanPatternNm",
    "url",
    "webkogiSeisekiKyokaFlg",
    "webkogiSeisekiKyokaFlgNm",
    "webrishuTaishogaiFlg",
    "webrishuTaishogaiFlgNm",
    "webrishuTorikeshifukaFlg",
    "webrishuTorikeshifukaFlgNm",
    "yobi",
    "yotoCd",
];

#[cfg(any(feature = "producer", test))]
struct ScalarString(Option<String>);

#[cfg(any(feature = "producer", test))]
impl<'de> Deserialize<'de> for ScalarString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ScalarVisitor;

        impl<'de> Visitor<'de> for ScalarVisitor {
            type Value = ScalarString;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string-like JSON scalar or null")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(ScalarString(None))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(ScalarString(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(ScalarString(Some(value.to_owned())))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(ScalarString(Some(value)))
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(ScalarString(Some(value.to_string())))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(ScalarString(Some(value.to_string())))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(ScalarString(Some(value.to_string())))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(ScalarString(Some(value.to_string())))
            }
        }

        deserializer.deserialize_any(ScalarVisitor)
    }
}

#[cfg(any(feature = "producer", test))]
impl<'de> Deserialize<'de> for RawCourse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RawCourseVisitor;

        impl<'de> Visitor<'de> for RawCourseVisitor {
            type Value = RawCourse;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a KULAS course object with only known fields")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut course = RawCourse::default();
                let mut seen = BTreeSet::new();
                while let Some(field) = map.next_key::<String>()? {
                    if !seen.insert(field.clone()) {
                        return Err(de::Error::custom(format!(
                            "duplicate source field {field:?}"
                        )));
                    }
                    let scalar = |map: &mut M| -> Result<Option<String>, M::Error> {
                        Ok(map.next_value::<ScalarString>()?.0)
                    };
                    match field.as_str() {
                        "kogiCd" => course.kogi_cd = scalar(&mut map)?.unwrap_or_default(),
                        "kogiNm" => course.kogi_nm = scalar(&mut map)?.unwrap_or_default(),
                        "fukudai" => course.fukudai = scalar(&mut map)?,
                        "tantoKyoin" => {
                            course.tanto_kyoin = scalar(&mut map)?.unwrap_or_default();
                        }
                        "jikanwari" => {
                            course.jikanwari = scalar(&mut map)?.unwrap_or_default();
                        }
                        "kogiKaikojikiNm" => {
                            course.kogi_kaikojiki_nm = scalar(&mut map)?.unwrap_or_default();
                        }
                        "kogiKubunNm" => {
                            course.kogi_kubun_nm = scalar(&mut map)?.unwrap_or_default();
                        }
                        "sekininBushoNm" => {
                            course.sekinin_busho_nm = scalar(&mut map)?.unwrap_or_default();
                        }
                        "kochiNm" => {
                            course.kochi_nm = scalar(&mut map)?.unwrap_or_default();
                        }
                        "gakusokuKamokuNm" => {
                            course.gakusoku_kamoku_nm = scalar(&mut map)?.unwrap_or_default();
                        }
                        "taishoGakka" => course.taisho_gakka = scalar(&mut map)?,
                        "taishoNenji" => course.taisho_nenji = scalar(&mut map)?,
                        "kamokuBunrui" => course.kamoku_bunrui = scalar(&mut map)?,
                        "kamokuBunya" => course.kamoku_bunya = scalar(&mut map)?,
                        "syllabusKomokuPatternId" => {
                            course.syllabus_komoku_pattern_id = scalar(&mut map)?;
                        }
                        "kaikoNendo" => course.kaiko_nendo = scalar(&mut map)?,
                        "lastUpdate" => course.last_update = scalar(&mut map)?,
                        known if KNOWN_IGNORED_SOURCE_FIELDS.contains(&known) => {
                            map.next_value::<IgnoredAny>()?;
                        }
                        unknown => {
                            return Err(de::Error::unknown_field(
                                unknown,
                                KNOWN_IGNORED_SOURCE_FIELDS,
                            ));
                        }
                    }
                }
                Ok(course)
            }
        }

        deserializer.deserialize_map(RawCourseVisitor)
    }
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
    fn known_non_public_source_fields_are_accepted_without_retention() {
        let c = parse(r#"{"kogiCd":"1","gairyaku":{"upstream":"private"}}"#);
        assert_eq!(c.kogi_cd, "1");
        assert!(!serde_json::to_string(&c).unwrap().contains("gairyaku"));
    }

    #[test]
    fn unknown_source_field_is_a_schema_error() {
        let error = serde_json::from_str::<RawCourse>(r#"{"kogiCd":"1","somethingNew":{"a":1}}"#)
            .unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn object_in_a_string_field_is_rejected() {
        assert!(serde_json::from_str::<RawCourse>(r#"{"kogiCd":{"nested":1}}"#).is_err());
    }

    #[test]
    fn duplicate_source_field_is_rejected() {
        assert!(serde_json::from_str::<RawCourse>(r#"{"kogiCd":"1","kogiCd":"2"}"#).is_err());
    }
}
