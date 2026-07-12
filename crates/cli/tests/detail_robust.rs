//! Robustness of the「シラバス参照」HTML parser beyond the single happy-path
//! fixture: malformed markup, unknown labels, the mobile duplicate, missing
//! sections, positional tables, and a never-panic property over adversarial
//! input. Uses small inline synthetic HTML (the parser is label-driven, so a
//! minimal `<table>` exercises each branch) plus a fuzz-lite property test.

use proptest::prelude::*;
use syllabus_cli::detail::parse_sansho_html;

#[test]
fn malformed_html_never_panics_and_extracts_what_it_can() {
    // Unclosed cells / stray tags: the parser must still recover the unit.
    let html = "<table><tr><th>単位数</th><td>3.0<td></tr></table><table><tr><th>broken";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.unit.as_deref(), Some("3.0"));
}

#[test]
fn unknown_label_is_preserved_in_extra() {
    // A label the parser doesn't model must survive in `extra`, never be dropped.
    let html = "<table><tr><th>謎のラベル</th><td>大事な値</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert!(
        d.extra
            .iter()
            .any(|e| e.label == "謎のラベル" && e.text == "大事な値"),
        "unknown label lost: {:?}",
        d.extra
    );
}

#[test]
fn mobile_duplicate_table_is_not_processed() {
    // The `is-hidden-tablet` copy must be skipped so a field isn't counted twice.
    let html = "<table class=\"is-hidden-tablet\"><tr><th>モバイル専用</th><td>skip</td></tr></table>\
                <table><tr><th>通常ラベル</th><td>keep</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert!(d.extra.iter().any(|e| e.label == "通常ラベル"));
    assert!(
        !d.extra.iter().any(|e| e.label == "モバイル専用"),
        "mobile duplicate was processed"
    );
}

#[test]
fn missing_sections_yield_empty_detail() {
    let d = parse_sansho_html("99999", "<html><body>no tables here</body></html>");
    assert!(d.eval.is_none());
    assert!(d.delivery.is_none());
    assert!(d.plan.is_empty());
    assert!(d.goals.is_empty());
    assert_eq!(d.cd, "99999");
}

#[test]
fn empty_detail_serializes_to_just_the_code() {
    // Sparse syllabus → tiny JSON (only `cd`), thanks to skip_serializing_if.
    let d = parse_sansho_html("42", "<html></html>");
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(json, r#"{"cd":"42"}"#);
}

#[test]
fn teachers_and_office_hours_are_extracted_positionally() {
    let html = "<table><tr><th>氏名</th><th>所属</th></tr>\
                <tr><td>山田 太郎</td><td>理工学部</td></tr>\
                <tr><td>佐藤 花子</td><td>医学部</td></tr></table>\
                <table><tr><th>氏名</th><th>曜日</th><th>時間</th><th>場所</th></tr>\
                <tr><td>山田 太郎</td><td>月</td><td>12:00-13:00</td><td>研究室A</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.teachers, vec!["山田 太郎", "佐藤 花子"]);
    assert_eq!(d.office_hour.len(), 1);
    assert_eq!(d.office_hour[0].name, "山田 太郎");
    assert_eq!(d.office_hour[0].day, "月");
    assert_eq!(d.office_hour[0].place, "研究室A");
}

#[test]
fn round_trips_through_json() {
    // Parse → serialize → deserialize is identity (serde skip/default symmetry).
    let html = "<table><tr><th>単位数</th><td>2.0</td></tr>\
                <tr><th>授業実施方法</th><td>すべて対面</td></tr></table>\
                <table class='tbl_status_jugyo'><tr><td>第1回</td><td>導入</td></tr></table>";
    let d = parse_sansho_html("77", html);
    let json = serde_json::to_string(&d).unwrap();
    let back: syllabus_cli::detail::SanshoDetail = serde_json::from_str(&json).unwrap();
    assert_eq!(d, back);
}

#[test]
fn office_hours_route_on_weekday_plus_time_alone() {
    // 曜日 + 時間 (no 場所) still routes to the office-hour extractor.
    let html = "<table><tr><th>氏名</th><th>曜日</th><th>時間</th></tr>\
                <tr><td>山田</td><td>月</td><td>10:00</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.office_hour.len(), 1);
    assert_eq!(d.office_hour[0].day, "月");
}

#[test]
fn office_hour_data_row_with_basho_in_a_value_is_not_dropped_as_header() {
    // Only a <th> cell naming 曜日/場所 marks the header — a data value that
    // merely contains 場所 must still be kept.
    let html = "<table><tr><th>氏名</th><th>曜日</th><th>時間</th><th>場所</th></tr>\
                <tr><td>山田</td><td>月</td><td>10:00</td><td>第二場所</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.office_hour.len(), 1);
    assert_eq!(d.office_hour[0].place, "第二場所");
}

#[test]
fn teacher_name_skips_a_leading_empty_cell() {
    // The name is the first non-empty data cell, not merely the first non-th one.
    let html = "<table><tr><th>氏名</th><th>所属</th></tr>\
                <tr><td></td><td>山田太郎</td><td>理工学部</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.teachers, vec!["山田太郎"]);
}

#[test]
fn goals_require_a_numbered_row() {
    // A 到達目標 row without a number is not captured (both signals required).
    let html = "<table><tr><th>番号</th><th>到達目標</th></tr>\
                <tr><th>1</th><td>目標A</td></tr>\
                <tr><th>あ</th><td>目標B</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.goals, vec!["目標A"]);
}

#[test]
fn eval_header_row_is_skipped_by_either_keyword() {
    // A 配分-only header (no 比重) must still be dropped, not read as a weight row.
    let html = "<table><tr><th>配分</th><td>割合</td></tr>\
                <tr><th>期末試験</th><td>60点</td></tr></table>";
    let d = parse_sansho_html("X", html);
    let eval = d.eval.expect("eval present");
    assert_eq!(eval.rows.len(), 1);
    assert!(eval.rows[0].item.contains("期末試験"));
}

#[test]
fn generic_meta_rows_route_to_their_own_fields() {
    let html = "<table>\
        <tr><th>メディア授業科目</th><td>該当する</td></tr>\
        <tr><th>受講生に求めるもの</th><td>予習必須</td></tr>\
        <tr><th>授業時間外の学習</th><td>毎回2時間</td></tr>\
        <tr><th>SDGs</th><td>4</td><td>7</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert!(d.delivery.expect("delivery present").is_media); // メディア: !value.is_empty()
    assert_eq!(d.prereq.as_deref(), Some("予習必須")); // 求めるもの ||
    assert_eq!(d.prep.as_deref(), Some("毎回2時間")); // 授業時間外 ||
    assert_eq!(d.sdgs, vec!["4", "7"]); // parse_sdgs pulls the goal numbers
}

#[test]
fn keywords_split_respects_a_stray_close_paren() {
    // A lone ')' must not drive the paren depth negative and swallow the next
    // delimiter — 「用語)」and「次の語」stay two keywords.
    let html = "<table><tr><th>キーワード</th><td>用語)、次の語</td></tr></table>";
    let d = parse_sansho_html("X", html);
    assert_eq!(d.keywords, vec!["用語)", "次の語"]);
}

#[test]
fn adversarial_inputs_do_not_panic() {
    for s in [
        "",
        "<<<",
        "<table><table><table>",
        "<tr><th>単位数",
        "\u{0}\u{fffd}<table>",
        "<table class='tbl_status_jugyo'><tr><td>第",
        "&lt;script&gt;",
    ] {
        let _ = parse_sansho_html("X", s); // returning at all is the assertion
    }
}

proptest! {
    /// Never panics on arbitrary bytes or HTML-ish fragments (the parser layers
    /// regex + scraper over recursive text walking; this is the stable-Rust
    /// bridge to the cargo-fuzz target).
    #[test]
    fn parse_never_panics(s in ".*") {
        let _ = parse_sansho_html("X", &s);
    }

    #[test]
    fn parse_never_panics_on_html_ish(
        s in prop::collection::vec(
            prop::sample::select(vec![
                "<table>", "</table>", "<tr>", "<th>単位数</th>", "<td>2.0</td>",
                "<br>", "第１回", "４０点", "\u{3000}", "<table class='is-hidden-tablet'>",
                "<table class='tbl_status_jugyo'>", "selectGroupTable", "<<>>",
            ]),
            0..40,
        ).prop_map(|v| v.concat())
    ) {
        let _ = parse_sansho_html("X", &s);
    }
}
