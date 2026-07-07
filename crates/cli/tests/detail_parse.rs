//! Parse a real KULAS「シラバス参照」page (captured from the browser) and pin the
//! high-value fields. The fixture is course `00001` (大学基礎論, pattern 4).

use syllabus_cli::detail::parse_sansho_html;

const SAMPLE: &str = include_str!("fixtures/sansho_sample.html");

fn detail() -> syllabus_cli::detail::SanshoDetail {
    parse_sansho_html("00001", SAMPLE)
}

#[test]
fn extracts_unit_and_delivery() {
    let d = detail();
    assert_eq!(d.unit.as_deref(), Some("2.0"));
    let delivery = d.delivery.expect("delivery present");
    assert_eq!(delivery.mode, "hybrid"); // 主に対面、一部オンライン
    assert!(delivery.raw.contains("対面"));
}

#[test]
fn extracts_eval_breakdown_with_types() {
    let d = detail();
    let eval = d.eval.expect("eval present");
    assert_eq!(eval.rows.len(), 2);
    // 学習意欲・授業参加度 40点 → attendance; 期末試験 60点 → exam
    let attend = &eval.rows[0];
    assert!(attend.item.contains("学習意欲"));
    assert_eq!(attend.weight, Some(40));
    assert_eq!(attend.kind, "attendance");
    let exam = &eval.rows[1];
    assert!(exam.item.contains("期末試験"));
    assert_eq!(exam.weight, Some(60));
    assert_eq!(exam.kind, "exam");
}

#[test]
fn extracts_goals_and_plan() {
    let d = detail();
    assert_eq!(d.goals.len(), 3);
    assert!(d.goals[0].contains("レポート作成方法"));
    // 授業計画 has 第1回..第N回; assert a few came through in order.
    assert!(d.plan.len() >= 5);
    assert_eq!(d.plan[0].n, 1);
    assert!(d.plan[0].text.contains("ガイダンス"));
    assert!(!d.plan[0].text.starts_with("授業概要"));
}

#[test]
fn extracts_summary_aims_keywords_teachers() {
    let d = detail();
    assert!(d
        .summary
        .as_deref()
        .unwrap_or_default()
        .contains("レポート"));
    assert!(d.aims.as_deref().unwrap_or_default().contains("人文科学"));
    assert!(!d.keywords.is_empty());
    assert!(d.teachers.iter().any(|t| t.contains("宮里")));
    // ◎ marks the representative teacher and must survive.
    assert!(d.teachers.iter().any(|t| t.contains('◎')));
}

#[test]
fn extracts_numbering_and_office_hour() {
    let d = detail();
    assert_eq!(d.numbering, ["01-0100-11", "04-0110-11"]);
    assert_eq!(d.office_hour.len(), 1);
    assert!(!d.office_hour[0].day.is_empty());
}

#[test]
fn serializes_compactly_without_empty_fields() {
    let d = parse_sansho_html("99999", "<html><body>no tables</body></html>");
    let json = serde_json::to_string(&d).unwrap();
    // Nothing parsed → only the cd survives (all empties skipped).
    assert_eq!(json, r#"{"cd":"99999"}"#);
}
