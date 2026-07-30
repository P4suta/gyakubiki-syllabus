//! Structured shape of a KULAS「シラバス参照」detail page, emitted to
//! `raw-details/{kogiCd}.json` and the manifest-addressed public detail assets.
//!
//! Field keys match `web/src/lib/syllabus-fields`. Crawl-only state stays in
//! [`SanshoDetail`]; public assets are serialized exclusively from
//! [`PublicDetail`], whose field and additional-label allowlists are explicit.

use serde::{Deserialize, Serialize};

/// One course's full syllabus detail.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
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

    // --- Derived at convert time by `enrich` (absent in raw-details). Faithful
    // re-presentations of the fields above; the originals are kept for fallback. ---
    /// 教科書 split into 教科書/参考書/… sections; `None` when there is no textbook
    /// text. Book titles are linkified at render, not here.
    #[serde(
        rename = "textbookInfo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub textbook_info: Option<TextbookInfo>,
    /// 授業時間外の学習: an extracted study-time and 予習/復習 split, when the text
    /// states them unambiguously. The full `prep` text is still shown.
    #[serde(rename = "prepInfo", default, skip_serializing_if = "Option::is_none")]
    pub prep_info: Option<PrepInfo>,
}

impl SanshoDetail {
    /// Whether parsing found any syllabus content beyond the join key and crawl
    /// timestamp. An empty result usually means the upstream HTML/protocol
    /// changed, so publishing it would turn an outage into silent data loss.
    #[must_use]
    pub fn has_public_content(&self) -> bool {
        self.unit.as_ref().is_some_and(|value| !value.is_empty())
            || self.delivery.is_some()
            || self.eval.is_some()
            || self.summary.as_ref().is_some_and(|value| !value.is_empty())
            || self.aims.as_ref().is_some_and(|value| !value.is_empty())
            || !self.goals.is_empty()
            || !self.plan.is_empty()
            || self
                .textbooks
                .as_ref()
                .is_some_and(|value| !value.is_empty())
            || self.prereq.as_ref().is_some_and(|value| !value.is_empty())
            || self.prep.as_ref().is_some_and(|value| !value.is_empty())
            || !self.office_hour.is_empty()
            || !self.keywords.is_empty()
            || !self.teachers.is_empty()
            || !self.numbering.is_empty()
            || !self.sdgs.is_empty()
            || !self.extra.is_empty()
    }
}

/// Explicit public representation. It deliberately has no `lastUpdate`, GUID,
/// entry context, diagnostic response, or other crawler/session state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PublicDetail {
    pub cd: String,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<Labelled>,
    #[serde(
        rename = "textbookInfo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub textbook_info: Option<TextbookInfo>,
    #[serde(rename = "prepInfo", default, skip_serializing_if = "Option::is_none")]
    pub prep_info: Option<PrepInfo>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnknownPublicLabel(pub String);

impl std::fmt::Display for UnknownPublicLabel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown public syllabus label {:?}", self.0)
    }
}

impl std::error::Error for UnknownPublicLabel {}

/// Every currently understood additional row. A KULAS addition must be reviewed
/// and added here before it can enter a public asset or the search index.
pub const PUBLIC_EXTRA_LABELS: &[&str] = &[
    "【テーマ（英語）】(IN ENGLISH)",
    "【テーマ（日本語）】(IN JAPANESE)",
    "COC＋フェーズ",
    "Webテキスト\n【WEB TEXTBOOK / HOMEPAGE URL 】",
    "オフィスアワーに関する補足",
    "この授業で身につける「10+1の能力」",
    "英文科目名",
    "関連科目名、関連科目コード番号\n【COMPUTER LINK / RELATED COURSES】",
    "教員の実務経験の有無",
    "区分1",
    "区分2",
    "講義副題",
    "資格等",
    "授業形態",
    "成績評価に関する補足",
    "地域関連科目",
    "履修における注意点",
    "履修に係わる注意事項\n【NOTES ON CLASS ENROLLMENT】",
];

impl TryFrom<&SanshoDetail> for PublicDetail {
    type Error = UnknownPublicLabel;

    fn try_from(detail: &SanshoDetail) -> Result<Self, Self::Error> {
        if let Some(unknown) = detail
            .extra
            .iter()
            .find(|value| !PUBLIC_EXTRA_LABELS.contains(&value.label.as_str()))
        {
            return Err(UnknownPublicLabel(unknown.label.clone()));
        }
        Ok(Self {
            cd: detail.cd.clone(),
            unit: detail.unit.clone(),
            delivery: detail.delivery.clone(),
            eval: detail.eval.clone(),
            summary: detail.summary.clone(),
            aims: detail.aims.clone(),
            goals: detail.goals.clone(),
            plan: detail.plan.clone(),
            textbooks: detail.textbooks.clone(),
            prereq: detail.prereq.clone(),
            prep: detail.prep.clone(),
            office_hour: detail.office_hour.clone(),
            keywords: detail.keywords.clone(),
            teachers: detail.teachers.clone(),
            numbering: detail.numbering.clone(),
            sdgs: detail.sdgs.clone(),
            extra: detail.extra.clone(),
            textbook_info: detail.textbook_info.clone(),
            prep_info: detail.prep_info.clone(),
        })
    }
}

/// 教科書・参考書, split by label. All source lines are preserved verbatim.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TextbookInfo {
    /// The whole value is a "not specified" statement (なし・適宜指示…), so the UI
    /// shows a quiet badge instead of a book list.
    #[serde(rename = "isNone", default)]
    pub is_none: bool,
    pub sections: Vec<TextbookSection>,
}

/// One labelled block of the 教科書 field (label `None` = text before any label).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TextbookSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub lines: Vec<String>,
}

/// Study-time and 予習/復習 extracted from 授業時間外の学習.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrepInfo {
    /// Study hours per session, only when the text states it unambiguously.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<f64>,
    /// Text after a 予習[:：] label, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yoshu: Option<String>,
    /// Text after a 復習[:：] label, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fukushu: Option<String>,
}

/// How the class is delivered. `mode` is classified from `raw`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Eval {
    pub rows: Vec<EvalRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One grade-weight row.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvalRow {
    pub item: String,
    /// Numeric weight when parseable from e.g. "40点" / "40%".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<i64>,
    /// `exam` | `report` | `minireport` | `attendance` | `quiz` | `other`.
    #[serde(rename = "type")]
    pub kind: String,
}

/// One session in the授業計画.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlanItem {
    /// Session number parsed from 第N回 (half- or full-width). Rows whose number
    /// can't be parsed are skipped, so this is always the real session number.
    pub n: i64,
    pub text: String,
    /// Highlight hint derived at convert time: `exam` | `milestone` | `start`.
    /// The `text` is never altered; this only tints the timeline node. Absent in
    /// `raw-details` (added by `enrich`), so default/skip keeps those parseable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// One オフィスアワー entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Labelled {
    pub label: String,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_public_field_independently_counts_as_content() {
        let cases = [
            SanshoDetail {
                unit: Some("1".into()),
                ..Default::default()
            },
            SanshoDetail {
                delivery: Some(Delivery::default()),
                ..Default::default()
            },
            SanshoDetail {
                eval: Some(Eval::default()),
                ..Default::default()
            },
            SanshoDetail {
                summary: Some("summary".into()),
                ..Default::default()
            },
            SanshoDetail {
                aims: Some("aims".into()),
                ..Default::default()
            },
            SanshoDetail {
                goals: vec!["goal".into()],
                ..Default::default()
            },
            SanshoDetail {
                plan: vec![PlanItem::default()],
                ..Default::default()
            },
            SanshoDetail {
                textbooks: Some("textbook".into()),
                ..Default::default()
            },
            SanshoDetail {
                prereq: Some("prerequisite".into()),
                ..Default::default()
            },
            SanshoDetail {
                prep: Some("preparation".into()),
                ..Default::default()
            },
            SanshoDetail {
                office_hour: vec![OfficeHour::default()],
                ..Default::default()
            },
            SanshoDetail {
                keywords: vec!["keyword".into()],
                ..Default::default()
            },
            SanshoDetail {
                teachers: vec!["teacher".into()],
                ..Default::default()
            },
            SanshoDetail {
                numbering: vec!["numbering".into()],
                ..Default::default()
            },
            SanshoDetail {
                sdgs: vec!["sdg".into()],
                ..Default::default()
            },
            SanshoDetail {
                extra: vec![Labelled::default()],
                ..Default::default()
            },
        ];

        for (index, detail) in cases.into_iter().enumerate() {
            assert!(
                detail.has_public_content(),
                "public field case {index} must count as content"
            );
        }
    }

    #[test]
    fn empty_optional_strings_do_not_count_as_public_content() {
        let cases = [
            SanshoDetail {
                unit: Some(String::new()),
                ..Default::default()
            },
            SanshoDetail {
                summary: Some(String::new()),
                ..Default::default()
            },
            SanshoDetail {
                aims: Some(String::new()),
                ..Default::default()
            },
            SanshoDetail {
                textbooks: Some(String::new()),
                ..Default::default()
            },
            SanshoDetail {
                prereq: Some(String::new()),
                ..Default::default()
            },
            SanshoDetail {
                prep: Some(String::new()),
                ..Default::default()
            },
        ];

        for (index, detail) in cases.into_iter().enumerate() {
            assert!(
                !detail.has_public_content(),
                "empty optional string case {index} must not count as content"
            );
        }
    }

    #[test]
    fn public_detail_omits_crawler_state() {
        let source = SanshoDetail {
            cd: "ABC1234567".into(),
            last_update: "2026-07-26T00:00:00Z".into(),
            summary: Some("概要".into()),
            ..SanshoDetail::default()
        };
        let public = PublicDetail::try_from(&source).unwrap();
        let json = serde_json::to_value(public).unwrap();
        assert_eq!(json["cd"], "ABC1234567");
        assert_eq!(json["summary"], "概要");
        assert!(json.get("lastUpdate").is_none());
    }

    #[test]
    fn unknown_additional_label_stops_publication() {
        let source = SanshoDetail {
            cd: "ABC1234567".into(),
            extra: vec![Labelled {
                label: "未審査の新項目".into(),
                text: "公開してはいけない".into(),
            }],
            ..SanshoDetail::default()
        };
        assert_eq!(
            PublicDetail::try_from(&source).unwrap_err(),
            UnknownPublicLabel("未審査の新項目".into())
        );
    }

    #[test]
    fn public_detail_rejects_unknown_json_fields() {
        let error =
            serde_json::from_str::<PublicDetail>(r#"{"cd":"ABC1234567","sessionToken":"secret"}"#)
                .unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }
}
