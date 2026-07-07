//! Structured shape of a KULAS「シラバス参照」detail page, emitted to
//! `raw-details/{kogiCd}.json` and (via `convert`) `web/public/details/{cd}.json`.
//!
//! Field keys match `web/src/lib/syllabus-fields`. Everything is
//! optional/skippable so a sparse syllabus yields a small file, and unknown
//! labels survive in `extra` rather than being dropped.

use serde::{Deserialize, Serialize};

/// One course's full syllabus detail.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SanshoDetail {
    /// 授業コード (kogiCd) — the join key back to the grid dataset.
    pub cd: String,
    /// `lastUpdate` timestamp this detail was scraped from, so `fetch-details`
    /// can skip courses whose grid record is unchanged. Not shown in the UI.
    #[serde(
        rename = "lastUpdate",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub last_update: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval: Option<Eval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aims: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan: Vec<PlanItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub textbooks: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prep: Option<String>,
    #[serde(rename = "officeHour", default, skip_serializing_if = "Vec::is_empty")]
    pub office_hour: Vec<OfficeHour>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teachers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbering: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdgs: Vec<String>,
    /// Labelled rows not modelled explicitly — kept so a KULAS layout change
    /// degrades gracefully instead of dropping data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Labelled>,
}

/// How the class is delivered. `mode` is classified from `raw`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Delivery {
    /// `onsite` | `online` | `ondemand` | `hybrid` | `unknown`.
    pub mode: String,
    /// The original「授業実施方法」text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw: String,
    /// Whether it is a「メディア授業科目」.
    #[serde(rename = "isMedia", default)]
    pub is_media: bool,
}

/// The grade breakdown, rendered as a ratio chart.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Eval {
    pub rows: Vec<EvalRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One grade-weight row.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EvalRow {
    pub item: String,
    /// Numeric weight when parseable from e.g. "40点" / "40%".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<i64>,
    /// `exam` | `report` | `attendance` | `presentation` | `quiz` | `other`.
    #[serde(rename = "type")]
    pub kind: String,
}

/// One session in the授業計画.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PlanItem {
    /// Session number parsed from 第N回 (half- or full-width). Rows whose number
    /// can't be parsed are skipped, so this is always the real session number.
    pub n: i64,
    pub text: String,
}

/// One オフィスアワー entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OfficeHour {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub day: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub time: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub place: String,
}

/// A generic label/text pair (for `extra`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Labelled {
    pub label: String,
    pub text: String,
}
