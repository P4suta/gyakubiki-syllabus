//! Parse the「シラバス参照」HTML into [`SanshoDetail`].
//!
//! The page is a stack of sibling `<table>` sections. Extraction is driven by
//! `<th>` labels rather than table position, so layout differences and new
//! labels degrade gracefully — anything unrecognised lands in `extra`. The
//! `is-hidden-tablet` mobile duplicate is skipped so each field is read once.

use std::sync::LazyLock;

use ego_tree::NodeRef;
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};

use super::classify::{delivery_mode, eval_type};
use super::model::{Delivery, Eval, EvalRow, Labelled, OfficeHour, PlanItem, SanshoDetail};

static TABLE_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse("table").expect("sel"));
static TR_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse("tr").expect("sel"));
static CELL_SEL: LazyLock<Selector> = LazyLock::new(|| Selector::parse("th, td").expect("sel"));
static BR_BLOCK: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)<br\s*/?>|</(p|div|li|tr|h[1-6])>").expect("re"));
static LEADING_INT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(\d+)").expect("re"));

/// Fields already in `data.json` — skipped so `details/{cd}.json` carries only
/// what is unique to the detail page.
const REDUNDANT_LABELS: &[&str] = &[
    "年度",
    "授業コード",
    "授業科目",
    "開講責任部署",
    "講義区分",
    "時間割",
    "講義開講時期",
    "履修開始年次",
];

/// One parsed table cell.
struct Cell {
    is_th: bool,
    text: String,
}

impl Cell {
    fn th(&self) -> Option<&str> {
        self.is_th.then_some(self.text.as_str())
    }
}

/// Parse the「シラバス参照」HTML for course `cd` into structured detail.
#[must_use]
pub fn parse_sansho_html(cd: &str, html: &str) -> SanshoDetail {
    // Turn line-break / block-closing tags into newlines so `.text()` keeps the
    // structure (KULAS uses <br> inside long cells and 授業計画 rows).
    let prepared = BR_BLOCK.replace_all(html, "\n");
    let doc = Html::parse_document(&prepared);

    let mut detail = SanshoDetail {
        cd: cd.to_owned(),
        ..Default::default()
    };

    for table in doc.select(&TABLE_SEL) {
        // Process each *top-level* table once. KULAS wraps some sections
        // (到達目標・成績評価…) in an outer table whose label header and inner data
        // table form one logical section, so reading the whole group keeps
        // label-driven dispatch working; skipping nested tables avoids
        // double-counting them as their own section.
        if !is_top_level(&table) || is_mobile_dup(&table) {
            continue;
        }
        let rows = collect_rows(&table);
        classify_table(&class_of(&table), &rows, &mut detail);
    }

    detail
}

/// A table with no ancestor `<table>` (see loop comment).
fn is_top_level(table: &ElementRef) -> bool {
    table
        .ancestors()
        .filter_map(ElementRef::wrap)
        .all(|a| a.value().name() != "table")
}

/// The `is-hidden-tablet` copy duplicates the desktop meta table on mobile.
fn is_mobile_dup(table: &ElementRef) -> bool {
    class_of(table).contains("is-hidden-tablet")
}

fn class_of(el: &ElementRef) -> String {
    el.value().attr("class").unwrap_or_default().to_owned()
}

/// Collect a table's rows as cells, preserving `th`/`td` and normalized text.
///
/// Handles KULAS's 2-deep nesting: walk every descendant `<tr>`, but keep each
/// cell only for the `<tr>` it *directly* belongs to (a nested table's cells are
/// handled on their own iteration). A cell that merely *wraps* a nested table is
/// dropped — its content arrives via that table's own rows.
fn collect_rows(table: &ElementRef) -> Vec<Vec<Cell>> {
    table
        .select(&TR_SEL)
        .map(|tr| {
            let tr_id = tr.id();
            tr.select(&CELL_SEL)
                // Keep only cells belonging directly to this row, not a nested
                // table's cells.
                .filter(|c| {
                    c.ancestors()
                        .filter_map(ElementRef::wrap)
                        .find(|a| a.value().name() == "tr")
                        .map(|a| a.id())
                        == Some(tr_id)
                })
                .map(|c| Cell {
                    is_th: c.value().name() == "th",
                    // own text, excluding any nested table, so we get the cell's
                    // value rather than a flattened blob.
                    text: normalize(&own_text(*c)),
                })
                .collect()
        })
        .collect()
}

/// Route one table to the right extractor, keyed by its `<th>` labels.
fn classify_table(class: &str, rows: &[Vec<Cell>], detail: &mut SanshoDetail) {
    let th_labels: Vec<&str> = rows
        .iter()
        .flat_map(|r| r.iter().filter_map(Cell::th))
        .collect();
    let has = |needle: &str| th_labels.iter().any(|t| t.contains(needle));

    if class.contains("tbl_status_jugyo") {
        parse_plan(rows, detail);
    } else if has("ナンバリングコード") {
        detail.numbering = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| !c.is_th && !c.text.is_empty())
            .map(|c| c.text.clone())
            .collect();
    } else if has("氏名") && has("所属") {
        parse_teachers(rows, detail);
    } else if has("到達目標") {
        parse_goals(rows, detail);
    } else if has("比重") || has("配分") {
        parse_eval(rows, detail);
    } else if has("曜日") && (has("場所") || has("時間")) {
        parse_office_hours(rows, detail);
    } else {
        // Generic label → value rows (meta table, 概要, 目的, キーワード, …).
        for row in rows {
            route_row(row, detail);
        }
    }
}

/// 授業計画: rows of `第N回 | 授業概要 …`.
fn parse_plan(rows: &[Vec<Cell>], detail: &mut SanshoDetail) {
    for row in rows {
        let mut cells = row.iter().filter(|c| !c.text.is_empty());
        let Some(head) = cells.next() else { continue };
        let Some(n) = kai_number(&head.text) else {
            continue;
        };
        let text = cells
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .trim_start_matches("授業概要")
            .trim()
            .to_owned();
        if !text.is_empty() {
            detail.plan.push(PlanItem {
                n,
                text,
                ..Default::default()
            });
        }
    }
}

/// 担当教員: header `氏名 | 所属`, then name/dept rows.
fn parse_teachers(rows: &[Vec<Cell>], detail: &mut SanshoDetail) {
    for row in rows {
        // The header row is all-th; data rows lead with the name in a td.
        if row.iter().all(|c| c.is_th) {
            continue;
        }
        if let Some(name) = row.iter().find(|c| !c.is_th && !c.text.is_empty()) {
            detail.teachers.push(name.text.clone());
        }
    }
}

/// 到達目標: rows of `N | goal text`.
fn parse_goals(rows: &[Vec<Cell>], detail: &mut SanshoDetail) {
    for row in rows {
        let is_number = |c: &Cell| {
            !c.text.is_empty()
                && to_ascii_digits(&c.text)
                    .chars()
                    .all(|ch| ch.is_ascii_digit())
        };
        if row.iter().any(is_number)
            && let Some(text) = row.iter().find(|c| !c.is_th && !c.text.is_empty())
        {
            detail.goals.push(text.text.clone());
        }
    }
}

/// 成績評価: header `比重・配分`, then `item | NN点` rows.
fn parse_eval(rows: &[Vec<Cell>], detail: &mut SanshoDetail) {
    let mut eval = Eval::default();
    for row in rows {
        let Some(item) = row.iter().find_map(Cell::th) else {
            continue;
        };
        if item.contains("比重") || item.contains("配分") {
            continue;
        }
        let Some(value) = row.iter().find(|c| !c.is_th).map(|c| c.text.as_str()) else {
            continue;
        };
        eval.rows.push(EvalRow {
            item: item.to_owned(),
            weight: LEADING_INT
                .captures(&to_ascii_digits(value))
                .and_then(|c| c[1].parse().ok()),
            kind: eval_type(item).to_owned(),
        });
    }
    if !eval.rows.is_empty() {
        detail.eval = Some(eval);
    }
}

/// オフィスアワー: header `氏名 曜日 時間 場所`, then positional data rows.
fn parse_office_hours(rows: &[Vec<Cell>], detail: &mut SanshoDetail) {
    for row in rows {
        if row
            .iter()
            .any(|c| c.is_th && (c.text.contains("曜日") || c.text.contains("場所")))
        {
            continue; // header row
        }
        let vals: Vec<&str> = row.iter().map(|c| c.text.as_str()).collect();
        if vals.iter().all(|v| v.is_empty()) {
            continue;
        }
        detail.office_hour.push(OfficeHour {
            name: vals.first().copied().unwrap_or_default().to_owned(),
            day: vals.get(1).copied().unwrap_or_default().to_owned(),
            time: vals.get(2).copied().unwrap_or_default().to_owned(),
            place: vals.get(3).copied().unwrap_or_default().to_owned(),
        });
    }
}

/// Route a generic `label | value` row into a known field or `extra`.
fn route_row(row: &[Cell], detail: &mut SanshoDetail) {
    let Some(label) = row.iter().find_map(Cell::th) else {
        return;
    };
    let value = row
        .iter()
        .filter(|c| !c.is_th)
        .map(|c| c.text.as_str())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");

    let has = |n: &str| label.contains(n);
    if has("単位数") {
        set_opt(&mut detail.unit, &value);
    } else if has("メディア授業科目") {
        delivery(detail).is_media = !value.is_empty();
    } else if has("授業実施方法") {
        let d = delivery(detail);
        d.raw = value.clone();
        d.mode = delivery_mode(&value).to_owned();
    } else if has("授業の概要") {
        set_opt(&mut detail.summary, &value);
    } else if has("授業の目的") {
        set_opt(&mut detail.aims, &value);
    } else if has("キーワード") {
        detail.keywords = split_keywords(&value);
    } else if has("求めるもの") || has("PREREQUISITES") {
        set_opt(&mut detail.prereq, &value);
    } else if has("授業時間外") || has("PREPARATION") {
        set_opt(&mut detail.prep, &value);
    } else if has("教科書") {
        set_opt(&mut detail.textbooks, &value);
    } else if has("SDGs") {
        detail.sdgs = parse_sdgs(&value);
    } else if REDUNDANT_LABELS.iter().any(|l| label.contains(l)) {
        // already in the grid dataset — drop
    } else if !value.is_empty() {
        detail.extra.push(Labelled {
            label: label.to_owned(),
            text: value,
        });
    }
}

/// Lazily create the delivery record so `授業実施方法` and `メディア授業科目`
/// (separate rows) merge into one object.
fn delivery(detail: &mut SanshoDetail) -> &mut Delivery {
    detail.delivery.get_or_insert_with(Delivery::default)
}

fn set_opt(slot: &mut Option<String>, value: &str) {
    if !value.is_empty() {
        *slot = Some(value.to_owned());
    }
}

/// Split a keyword blob on separators that are never *inside* a term (spaces and
/// Japanese/ASCII commas — deliberately not `・`, which joins compound terms).
fn split_keywords(value: &str) -> Vec<String> {
    value
        .split(['、', '，', ',', ' ', '\u{3000}', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Extract the SDG goal numbers from a `4 質の高い教育…` style value.
fn parse_sdgs(value: &str) -> Vec<String> {
    value
        .split(['\n', '/'])
        .filter_map(|seg| LEADING_INT.captures(seg.trim()).map(|c| c[1].to_owned()))
        .collect()
}

/// Map full-width digits (`０`-`９`) to ASCII so number extraction works whether
/// KULAS renders numerals half- or full-width. Non-digits pass through.
fn to_ascii_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '０'..='９' => char::from(b'0' + (c as u32 - '０' as u32) as u8),
            other => other,
        })
        .collect()
}

/// Session number from「第N回」(returns `None` when the cell isn't a session).
/// Accepts full-width digits (`第１回`) as well as ASCII.
fn kai_number(text: &str) -> Option<i64> {
    let inner = text.strip_prefix('第')?;
    let inner = to_ascii_digits(inner);
    let digits: String = inner.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// A cell's own text. Nested tables are pruned *except* KULAS's `selectGroupTable`
/// value wrapper, which holds some field values (授業実施方法・SDGs…) inside the
/// label's cell — its text is kept. Structured sections (到達目標・成績評価…) use a
/// `tbl_status` table parsed row-by-row instead, so they are dropped here.
fn own_text(node: NodeRef<Node>) -> String {
    let mut out = String::new();
    fn walk(node: NodeRef<Node>, out: &mut String) {
        for child in node.children() {
            match child.value() {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) if e.name() == "table" => {
                    if e.attr("class")
                        .is_some_and(|c| c.contains("selectGroupTable"))
                    {
                        walk(child, out); // value wrapper — keep its text
                        out.push(' ');
                    }
                    // otherwise a data/guide table — prune it
                }
                _ => walk(child, out),
            }
        }
    }
    walk(node, &mut out);
    out
}

/// Collapse whitespace within each line but keep line breaks (so long fields and
/// multi-line lists stay readable). Trims ideographic spaces too.
fn normalize(text: &str) -> String {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|line| line.trim_matches('\u{3000}').to_owned())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{kai_number, parse_sansho_html, to_ascii_digits};

    #[test]
    fn to_ascii_digits_converts_full_width_only() {
        assert_eq!(to_ascii_digits("４０点"), "40点");
        assert_eq!(to_ascii_digits("第１回"), "第1回");
        assert_eq!(to_ascii_digits("ABC 123"), "ABC 123"); // ASCII untouched
    }

    #[test]
    fn kai_number_accepts_full_width_and_ascii() {
        assert_eq!(kai_number("第1回"), Some(1));
        assert_eq!(kai_number("第１回"), Some(1)); // full-width
        assert_eq!(kai_number("第15回"), Some(15));
        assert_eq!(kai_number("補講"), None);
    }

    #[test]
    fn plan_keeps_full_width_session_number() {
        // Regression: 「第１回」(full-width) used to yield None and drop the whole
        // plan row.
        let html = "<table class='tbl_status_jugyo'><tr><td>第１回</td><td>ガイダンス</td></tr>\
                    <tr><td>第２回</td><td>基礎理論</td></tr></table>";
        let d = parse_sansho_html("x", html);
        assert_eq!(d.plan.len(), 2);
        assert_eq!(d.plan[0].n, 1);
        assert_eq!(d.plan[0].text, "ガイダンス");
        assert_eq!(d.plan[1].n, 2);
    }

    #[test]
    fn eval_weight_parses_full_width_digits() {
        // Regression: 「４０点」(full-width) used to yield weight None → card ":0".
        let html = "<table><tr><th>比重</th><th>項目</th></tr>\
                    <tr><th>レポート</th><td>４０点</td></tr>\
                    <tr><th>期末試験</th><td>60%</td></tr></table>";
        let d = parse_sansho_html("x", html);
        let eval = d.eval.expect("eval table parsed");
        assert_eq!(eval.rows[0].weight, Some(40));
        assert_eq!(eval.rows[1].weight, Some(60));
    }

    #[test]
    fn eval_weight_overflow_stays_none_not_zero() {
        // A pathological huge number must not panic nor coerce to 0; it stays
        // None so the card omits the weight rather than claiming a real 0.
        let html = "<table><tr><th>比重</th><th>項目</th></tr>\
                    <tr><th>レポート</th><td>99999999999999999999点</td></tr></table>";
        let d = parse_sansho_html("x", html);
        assert_eq!(d.eval.unwrap().rows[0].weight, None);
    }
}
