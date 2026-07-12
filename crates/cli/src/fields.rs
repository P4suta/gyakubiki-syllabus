//! **Single source of truth** for how syllabus detail fields are ranked and
//! rendered in the UI — nothing else hard-codes tier/label.
//!
//! `syllabus-cli gen-field-docs` renders `FIELD_SPEC` into two artifacts, kept in
//! sync by a CI check:
//! - `web/src/lib/syllabus-fields.generated.ts` — drives the frontend modal.
//! - `docs/syllabus-fields.md` — a human-readable table.
//!
//! `key` matches a field of [`crate::detail::SanshoDetail`]; the test below
//! guarantees they never drift.

use std::path::Path;

use anyhow::{Context, Result, bail};

/// One displayable syllabus field.
pub struct FieldSpec {
    /// Matches the JSON key in `details/{cd}.json` (a `SanshoDetail` field).
    pub key: &'static str,
    /// Japanese section label.
    pub label: &'static str,
    /// Display priority: 1 = hero, 2 = standard, 3 = low (collapsed).
    pub tier: u8,
    /// Modal grouping label: `""` = hero (no subheading, open by default);
    /// otherwise the subheading the field is bundled under.
    pub group: &'static str,
    /// How the frontend renders the value (a component/style selector).
    pub render: &'static str,
}

/// The ordered display spec. Order = display order; tier = prominence.
///
/// Reorder / retier by editing this list, then run `just gen-field-docs`.
pub const FIELD_SPEC: &[FieldSpec] = &[
    // Tier 1 — hero: what a student decides on first. `eval`/`summary` open at
    // the top with no subheading; `delivery`/`unit` render as header chips.
    FieldSpec {
        key: "eval",
        label: "成績評価",
        tier: 1,
        group: "",
        render: "eval-chart",
    },
    FieldSpec {
        key: "delivery",
        label: "授業実施方法",
        tier: 1,
        group: "",
        render: "delivery-badge",
    },
    FieldSpec {
        key: "unit",
        label: "単位数",
        tier: 1,
        group: "",
        render: "meta",
    },
    FieldSpec {
        key: "summary",
        label: "授業の概要",
        tier: 1,
        group: "",
        render: "longtext",
    },
    // Tier 2 — standard: bundled under the「授業内容」subheading.
    FieldSpec {
        key: "aims",
        label: "授業の目的",
        tier: 1,
        group: "授業内容",
        render: "longtext",
    },
    FieldSpec {
        key: "goals",
        label: "到達目標",
        tier: 2,
        group: "授業内容",
        render: "checklist",
    },
    FieldSpec {
        key: "plan",
        label: "授業計画",
        tier: 2,
        group: "授業内容",
        render: "plan-timeline",
    },
    FieldSpec {
        key: "textbooks",
        label: "教科書・参考書",
        tier: 2,
        group: "授業内容",
        render: "textbooks",
    },
    FieldSpec {
        key: "prereq",
        label: "履修に求めるもの",
        tier: 2,
        group: "授業内容",
        render: "longtext",
    },
    FieldSpec {
        key: "prep",
        label: "授業時間外の学習",
        tier: 2,
        group: "授業内容",
        render: "prep",
    },
    FieldSpec {
        key: "officeHour",
        label: "オフィスアワー",
        tier: 2,
        group: "授業内容",
        render: "office-table",
    },
    // Tier 3 — low priority: bundled under the「その他」subheading, collapsed.
    FieldSpec {
        key: "teachers",
        label: "担当教員",
        tier: 3,
        group: "その他",
        render: "people",
    },
    FieldSpec {
        key: "keywords",
        label: "キーワード",
        tier: 3,
        group: "その他",
        render: "keywords",
    },
    FieldSpec {
        key: "numbering",
        label: "ナンバリング",
        tier: 3,
        group: "その他",
        render: "numbering",
    },
    FieldSpec {
        key: "sdgs",
        label: "SDGs",
        tier: 3,
        group: "その他",
        render: "sdgs",
    },
];

const TS_PATH: &str = "web/src/lib/syllabus-fields.generated.ts";
const MD_PATH: &str = "docs/syllabus-fields.md";

/// Render `FIELD_SPEC` to the two generated artifacts under `root`.
///
/// When `check` is true, verify the on-disk files already match (for CI) instead
/// of writing — erroring if they are stale.
pub fn generate(root: &Path, check: bool) -> Result<()> {
    let outputs = [(TS_PATH, render_ts()), (MD_PATH, render_md())];
    for (rel, content) in outputs {
        let path = root.join(rel);
        if check {
            let current = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read {rel} (may not be generated yet)"))?;
            if normalize_eol(&current) != normalize_eol(&content) {
                bail!("{rel} does not match FIELD_SPEC. Run `just gen-field-docs`");
            }
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&path, content).with_context(|| format!("failed to write {rel}"))?;
            eprintln!("wrote {rel}");
        }
    }
    Ok(())
}

/// Tolerate CRLF checkouts when comparing (the golden bytes use LF).
fn normalize_eol(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn render_ts() -> String {
    let mut s = String::new();
    s.push_str(
        "// AUTO-GENERATED by `syllabus-cli gen-field-docs` from crates/cli/src/fields.rs.\n",
    );
    s.push_str(
        "// Do not edit by hand — change FIELD_SPEC there and re-run `just gen-field-docs`.\n\n",
    );
    s.push_str("export type FieldTier = 1 | 2 | 3\n\n");
    s.push_str("export interface FieldSpec {\n");
    s.push_str(
        "\tkey: string\n\tlabel: string\n\ttier: FieldTier\n\tgroup: string\n\trender: string\n",
    );
    s.push_str("}\n\n");
    s.push_str("export const FIELD_SPEC: readonly FieldSpec[] = [\n");
    for f in FIELD_SPEC {
        s.push_str(&format!(
            "\t{{ key: '{}', label: '{}', tier: {}, group: '{}', render: '{}' }},\n",
            f.key, f.label, f.tier, f.group, f.render
        ));
    }
    s.push_str("] as const\n");
    s
}

fn render_md() -> String {
    let mut s = String::new();
    s.push_str("# シラバス項目の表示優先度\n\n");
    s.push_str("> このファイルは `syllabus-cli gen-field-docs` が `crates/cli/src/fields.rs` の\n");
    s.push_str("> `FIELD_SPEC` から自動生成します。**手で編集しないでください。**\n");
    s.push_str("> 並び順・tier を変えるには `FIELD_SPEC` を編集し `just gen-field-docs` を実行します。\n\n");
    s.push_str("| tier | group | key | ラベル | 表示 |\n");
    s.push_str("|---|---|---|---|---|\n");
    for f in FIELD_SPEC {
        s.push_str(&format!(
            "| {} | {} | `{}` | {} | `{}` |\n",
            f.tier, f.group, f.key, f.label, f.render
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detail::SanshoDetail;

    // Snapshot the generated artifacts. `gen-field-docs --check` guards drift in
    // CI, but these pin the exact bytes under `cargo test` (and cover the render
    // paths), so an accidental format change surfaces as a reviewable diff.
    #[test]
    fn render_ts_snapshot() {
        insta::assert_snapshot!(render_ts());
    }

    #[test]
    fn render_md_snapshot() {
        insta::assert_snapshot!(render_md());
    }

    #[test]
    fn generate_writes_then_check_roundtrips() {
        // Exercises the write/verify logic (not just the renderers): write the
        // artifacts, confirm --check accepts them, then confirm a drifted file is
        // rejected. Uses a throwaway root so the real repo files are untouched.
        let dir = std::env::temp_dir().join(format!("syllabus-fieldspec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        generate(&dir, false).expect("write artifacts");
        assert!(
            dir.join(TS_PATH).exists(),
            "generate(false) must write the TS file"
        );
        generate(&dir, true).expect("--check accepts freshly-written files");
        std::fs::write(dir.join(TS_PATH), "drifted\n").unwrap();
        assert!(
            generate(&dir, true).is_err(),
            "--check must reject a stale file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spec_covers_every_populated_detail_field() {
        // A fully-populated detail: every content field non-empty so it serializes.
        let detail = SanshoDetail {
            cd: "x".into(),
            last_update: "t".into(),
            unit: Some("2".into()),
            delivery: Some(Default::default()),
            eval: Some(Default::default()),
            summary: Some("s".into()),
            aims: Some("a".into()),
            goals: vec!["g".into()],
            plan: vec![Default::default()],
            textbooks: Some("t".into()),
            prereq: Some("p".into()),
            prep: Some("p".into()),
            office_hour: vec![Default::default()],
            keywords: vec!["k".into()],
            teachers: vec!["t".into()],
            numbering: vec!["n".into()],
            sdgs: vec!["4".into()],
            extra: vec![Default::default()],
            // Derived fields (textbookInfo/prepInfo) are added by `enrich`, not
            // shown as their own sections — left default so they don't serialize.
            ..Default::default()
        };
        let value = serde_json::to_value(&detail).unwrap();
        let obj = value.as_object().unwrap();
        // Keys not shown as sections by the spec.
        let ignored = ["cd", "lastUpdate", "extra"];
        let spec_keys: Vec<&str> = FIELD_SPEC.iter().map(|f| f.key).collect();
        for key in obj.keys() {
            if ignored.contains(&key.as_str()) {
                continue;
            }
            assert!(
                spec_keys.contains(&key.as_str()),
                "SanshoDetail key `{key}` is missing from FIELD_SPEC (add it in fields.rs)"
            );
        }
        // …and no spec key is a typo with no backing data field.
        for key in spec_keys {
            assert!(
                obj.contains_key(key),
                "FIELD_SPEC key `{key}` has no SanshoDetail field"
            );
        }
    }

    #[test]
    fn tiers_are_valid_and_ordered_nondecreasing() {
        let mut last = 0;
        for f in FIELD_SPEC {
            assert!((1..=3).contains(&f.tier), "{} has bad tier", f.key);
            assert!(
                f.tier >= last,
                "FIELD_SPEC must be grouped by tier ({} out of order)",
                f.key
            );
            last = f.tier;
        }
    }
}
