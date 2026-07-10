//! `gen-sample` — synthesize a rich, deterministic dummy dataset for local UI
//! development, so the frontend can be exercised without ever touching KULAS.
//!
//! It writes raw KULAS-shaped courses (an envelope JSON) plus per-course detail
//! JSON, then the normal `convert` pipeline turns them into `web/public/data.json`
//! and `web/public/details/`. Structural coverage (every delivery mode / eval
//! type, Saturday, 通年, concentrated no-slot courses, and some courses with no
//! detail at all) is assigned by index, so it is guaranteed regardless of the
//! `--seed`; the seed only garnishes names and keywords.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::json;

use crate::detail::{Delivery, Eval, EvalRow, Labelled, OfficeHour, PlanItem, SanshoDetail};

#[derive(Args)]
pub struct GenSampleArgs {
    /// How many courses to synthesize. Defaults to a production-scale few
    /// thousand so local dev and E2E exercise the real load; pass a small
    /// `--count` for a quick dataset.
    #[arg(long, default_value_t = 3000)]
    count: usize,
    /// Seed for the cosmetic randomness (names/keywords); coverage is seed-independent.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Output path for the raw KULAS-shaped JSON (the `convert` input).
    #[arg(long = "out-raw", default_value = "dev-data/sample-raw.json")]
    out_raw: PathBuf,
    /// Output directory for per-course detail JSON.
    #[arg(long = "out-details", default_value = "dev-data/sample-details")]
    out_details: PathBuf,
}

/// Generate the dataset and write it to disk.
pub fn run(args: GenSampleArgs) -> Result<()> {
    let generated = generate(args.count, args.seed);

    if let Some(parent) = args.out_raw.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let envelope = json!({ "selectKogiDtoList": generated.raw });
    fs::write(&args.out_raw, serde_json::to_vec_pretty(&envelope)?)
        .with_context(|| format!("failed to write {}", args.out_raw.display()))?;

    fs::create_dir_all(&args.out_details)
        .with_context(|| format!("failed to create {}", args.out_details.display()))?;
    for detail in &generated.details {
        let path = args.out_details.join(format!("{}.json", detail.cd));
        fs::write(&path, serde_json::to_vec(detail)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    eprintln!(
        "gen-sample: wrote {} courses to {} and {} details to {}",
        generated.raw.len(),
        args.out_raw.display(),
        generated.details.len(),
        args.out_details.display()
    );
    Ok(())
}

/// The synthesized dataset: raw course objects and the details for a subset.
struct Generated {
    raw: Vec<serde_json::Value>,
    details: Vec<SanshoDetail>,
}

const CAMPUSES: &[&str] = &["朝倉キャンパス", "物部キャンパス", "岡豊キャンパス", ""];
const DEPARTMENTS: &[&str] = &[
    "理工学部",
    "人文社会科学部",
    "農林海洋科学部",
    "医学部 医学科",
    "教育学部",
    "共通教育",
];
const KUBUN: &[&str] = &["講義", "演習", "実習", "実技", "実験"];
const SEMESTERS: &[&str] = &["1学期", "2学期", "1学期前半", "2学期前半", "通年"];
const DAYS: &[&str] = &["月", "火", "水", "木", "金", "土"];
const SUBJECTS: &[&str] = &[
    "微分積分学",
    "線形代数",
    "日本国憲法",
    "有機化学",
    "西洋史概論",
    "データ構造とアルゴリズム",
    "分子生物学",
    "英語コミュニケーション",
    "経済学入門",
    "心理学概論",
    "電磁気学",
    "統計学",
    "環境科学",
    "現代日本文学",
    "情報理論",
    "民法総則",
    "生化学",
    "地球科学",
    "海洋生物学概論",
    "教育心理学",
];
const INSTRUCTORS: &[&str] = &[
    "山田 太郎",
    "佐藤 花子",
    "鈴木 一郎",
    "高橋 実",
    "田中 桂",
    "伊藤 恵子",
    "渡辺 健",
    "小林 由美",
];
const KEYWORDS: &[&str] = &[
    "基礎", "応用", "演習", "理論", "実践", "分析", "設計", "歴史", "国際", "地域",
];

/// Full-width period digit (`１`-`８`) for a 1-based period.
fn fw_period(period: u32) -> char {
    char::from_u32('０' as u32 + period).expect("period 1..=8")
}

/// Build the `jikanwari` string for course `i` (fills the grid; a few edge cases).
fn jikanwari_for(i: usize) -> String {
    let sem = SEMESTERS[i % SEMESTERS.len()];
    // Every 9th course is a no-slot concentrated course.
    if i % 9 == 8 {
        return format!("{sem}: 集中講義");
    }
    let day = DAYS[i % DAYS.len()];
    let period = (i % 6) as u32 + 1;
    let first = format!("{sem}: {day}曜日{}時限", fw_period(period));
    // Every 7th course meets twice a week.
    if i % 7 == 3 {
        let day2 = DAYS[(i + 2) % DAYS.len()];
        let period2 = ((i + 2) % 6) as u32 + 1;
        return format!("{first}, {sem}: {day2}曜日{}時限", fw_period(period2));
    }
    first
}

/// Assemble one raw KULAS-shaped course object.
fn raw_course(i: usize, rng: &mut StdRng) -> serde_json::Value {
    let cd = format!("{:05}", i + 1);
    // A couple of deliberate edge cases for the UI to survive.
    let (name, instructor, dept) = match i {
        3 => (
            "理論 & 実践 <入門>".to_owned(),
            "Smith John".to_owned(),
            DEPARTMENTS[0],
        ),
        7 => (SUBJECTS[i % SUBJECTS.len()].to_owned(), String::new(), ""), // empty prof + dept
        _ => {
            let base = SUBJECTS[i % SUBJECTS.len()];
            let n = if i.is_multiple_of(4) {
                format!("{base}Ⅰ")
            } else {
                base.to_owned()
            };
            let a = INSTRUCTORS[rng.random_range(0..INSTRUCTORS.len())];
            // Some courses are team-taught (comma-separated), so the card shows「… ほか」.
            let instructor = if i.is_multiple_of(3) {
                let b = INSTRUCTORS[rng.random_range(0..INSTRUCTORS.len())];
                format!("{a}, {b}")
            } else {
                a.to_owned()
            };
            (n, instructor, DEPARTMENTS[i % DEPARTMENTS.len()])
        }
    };
    let fukudai = (i % 5 == 2).then(|| "基礎から応用まで".to_owned());

    json!({
        "kogiCd": cd,
        "kogiNm": name,
        "tantoKyoin": instructor,
        "jikanwari": jikanwari_for(i),
        "kogiKaikojikiNm": SEMESTERS[i % SEMESTERS.len()],
        "kogiKubunNm": KUBUN[i % KUBUN.len()],
        "sekininBushoNm": dept,
        "kochiNm": CAMPUSES[i % CAMPUSES.len()],
        "fukudai": fukudai,
        "taishoGakka": format!("{}学科", DEPARTMENTS[i % DEPARTMENTS.len()].trim_end_matches("学部")),
        "taishoNenji": format!("{}年", (i % 4) + 1),
        "kamokuBunrui": if i.is_multiple_of(2) { "専門" } else { "教養" },
        "kamokuBunya": "総合",
        "syllabusKomokuPatternId": "4",
        "kaikoNendo": "2026",
    })
}

/// The 6 assessment types, cycled so every type appears across the details.
fn eval_for(i: usize) -> Eval {
    fn row(item: &str, weight: Option<i64>, kind: &str) -> EvalRow {
        EvalRow {
            item: item.to_owned(),
            weight,
            kind: kind.to_owned(),
        }
    }
    let (rows, note) = match i % 6 {
        0 => (
            vec![
                row("期末試験", Some(60), "exam"),
                row("レポート", Some(40), "report"),
            ],
            None,
        ),
        1 => (
            vec![
                row("小テスト", Some(20), "quiz"),
                row("定期試験", Some(50), "exam"),
                row("受講態度", Some(30), "attendance"),
            ],
            Some("小テストは毎回実施".to_owned()),
        ),
        2 => (
            vec![
                row("最終発表", Some(50), "presentation"),
                row("レポート", Some(50), "report"),
            ],
            None,
        ),
        3 => (
            vec![
                row("平常点", Some(40), "attendance"),
                row("その他", Some(60), "other"),
            ],
            Some("出席が3分の2に満たない場合は評価対象外".to_owned()),
        ),
        4 => (
            // A weightless row: the card must not fabricate ":0".
            vec![
                row("レポート", None, "report"),
                row("期末試験", Some(60), "exam"),
            ],
            None,
        ),
        _ => (
            vec![
                row("課題", Some(30), "report"),
                row("プレゼンテーション", Some(30), "presentation"),
                row("試験", Some(40), "exam"),
            ],
            None,
        ),
    };
    Eval { rows, note }
}

/// Delivery mode for course `i`; index 4 in the cycle is the `unknown` case.
fn delivery_for(i: usize) -> Delivery {
    let (mode, raw, is_media) = match i % 5 {
        0 => ("onsite", "すべて対面", false),
        1 => ("online", "オンライン（同時双方向型）", true),
        2 => ("ondemand", "オンデマンド配信のみ", true),
        3 => ("hybrid", "主に対面、一部オンライン", false),
        _ => ("unknown", "別途連絡", false),
    };
    Delivery {
        mode: mode.to_owned(),
        raw: raw.to_owned(),
        is_media,
    }
}

const LONG_SUMMARY: &str = "この授業では、基礎的な概念から出発し、具体的な事例や演習を通じて理解を深めます。前半では理論的な枠組みを丁寧に扱い、後半ではそれを実際の課題へ応用する力を養います。受講者は各回の予習・復習に取り組むことで、体系的な知識と実践的な技能の双方を身につけることが期待されます。";

/// Build one course's detail, cycling through the full range of UI states.
fn detail_for(i: usize, rng: &mut StdRng) -> SanshoDetail {
    let cd = format!("{:05}", i + 1);
    let plan_len = if i.is_multiple_of(10) {
        15
    } else {
        3 + (i % 6)
    };
    let plan = (1..=plan_len)
        .map(|n| PlanItem {
            n: n as i64,
            text: format!("第{n}回の授業内容（テーマ{n}）"),
        })
        .collect();
    let goal_count = 3 + (i % 3);
    let goals = (1..=goal_count)
        .map(|g| format!("到達目標その{g}を説明・応用できる"))
        .collect();
    let summary = if i.is_multiple_of(2) {
        Some(LONG_SUMMARY.to_owned())
    } else {
        Some("基礎的な内容を扱う入門的な授業です。".to_owned())
    };
    let teacher_count = 1 + (i % 3);
    let teachers = (0..teacher_count)
        .map(|t| INSTRUCTORS[(i + t) % INSTRUCTORS.len()].to_owned())
        .collect();
    let keywords = (0..3)
        .map(|_| KEYWORDS[rng.random_range(0..KEYWORDS.len())].to_owned())
        .collect();
    // One course carries an unmodelled label to show it degrades into `extra`.
    let extra = if i == 5 {
        vec![Labelled {
            label: "特記事項".to_owned(),
            text: "この科目は隔年開講です。".to_owned(),
        }]
    } else {
        Vec::new()
    };

    SanshoDetail {
        cd,
        last_update: String::new(),
        unit: Some(["1.0", "2.0", "4.0"][i % 3].to_owned()),
        delivery: Some(delivery_for(i)),
        eval: Some(eval_for(i)),
        summary,
        aims: Some("この授業の目的は、対象分野の基礎を体系的に理解することです。".to_owned()),
        goals,
        plan,
        textbooks: Some("教科書『入門テキスト』（○○出版）、参考書は初回に指示。".to_owned()),
        prereq: Some("特になし（関連する基礎科目の履修を推奨）。".to_owned()),
        prep: Some("毎回およそ1時間の予習・復習を行うこと。".to_owned()),
        office_hour: vec![OfficeHour {
            name: INSTRUCTORS[i % INSTRUCTORS.len()].to_owned(),
            day: DAYS[i % DAYS.len()].to_owned(),
            time: "12:00-13:00".to_owned(),
            place: format!("研究室{}", (i % 30) + 101),
        }],
        keywords,
        teachers,
        numbering: vec![format!("GEN-{:03}", (i % 400) + 100)],
        sdgs: vec!["4 質の高い教育をみんなに".to_owned()],
        extra,
    }
}

/// Deterministically synthesize `count` courses; ~70% get a detail record.
fn generate(count: usize, seed: u64) -> Generated {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut raw = Vec::with_capacity(count);
    let mut details = Vec::new();
    for i in 0..count {
        raw.push(raw_course(i, &mut rng));
        // Leave ~30% without a detail so the graceful-degradation path shows too.
        if i % 10 < 7 {
            details.push(detail_for(i, &mut rng));
        }
    }
    Generated { raw, details }
}

#[cfg(test)]
mod tests {
    use super::generate;
    use std::collections::BTreeSet;
    use syllabus_core::model::RawCourse;
    use syllabus_core::{convert_v3, Engine, Filters};

    fn raw_courses(g: &super::Generated) -> Vec<RawCourse> {
        serde_json::from_value(serde_json::Value::Array(g.raw.clone()))
            .expect("generated raw parses as RawCourse")
    }

    #[test]
    fn generates_the_requested_count_with_some_details_missing() {
        let g = generate(40, 42);
        assert_eq!(g.raw.len(), 40);
        // ~70% have details, so strictly between 0 and count.
        assert!(g.details.len() < g.raw.len());
        assert!(g.details.len() >= 20);
    }

    #[test]
    fn generation_is_deterministic() {
        let a = generate(30, 42);
        let b = generate(30, 42);
        assert_eq!(a.raw, b.raw);
        assert_eq!(a.details, b.details);
    }

    #[test]
    fn details_cover_every_delivery_mode_and_eval_type() {
        let g = generate(40, 42);
        let modes: BTreeSet<&str> = g
            .details
            .iter()
            .filter_map(|d| d.delivery.as_ref())
            .map(|d| d.mode.as_str())
            .collect();
        for m in ["onsite", "online", "ondemand", "hybrid", "unknown"] {
            assert!(modes.contains(m), "delivery mode {m} not covered");
        }
        let kinds: BTreeSet<&str> = g
            .details
            .iter()
            .filter_map(|d| d.eval.as_ref())
            .flat_map(|e| e.rows.iter())
            .map(|r| r.kind.as_str())
            .collect();
        for k in [
            "exam",
            "report",
            "attendance",
            "presentation",
            "quiz",
            "other",
        ] {
            assert!(kinds.contains(k), "eval type {k} not covered");
        }
    }

    #[test]
    fn detail_files_round_trip_and_key_the_grid() {
        // Every detail serializes and re-parses, and its cd matches a raw course.
        let g = generate(40, 42);
        let codes: BTreeSet<String> = g
            .raw
            .iter()
            .map(|c| c["kogiCd"].as_str().unwrap().to_owned())
            .collect();
        for d in &g.details {
            let json = serde_json::to_string(d).unwrap();
            let back: crate::detail::SanshoDetail = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, d);
            assert!(codes.contains(&d.cd), "detail cd {} has no course", d.cd);
        }
    }

    #[test]
    fn converts_to_valid_v3_with_saturday_and_tsuunen() {
        let g = generate(40, 42);
        let raw = raw_courses(&g);
        let data = convert_v3(&raw, "2026-01-01T00:00:00Z".to_owned()).data;

        // Grid variety: 通年 propagation and a Saturday column both present.
        assert!(data.dicts.semesters.iter().any(|s| s == "通年"));
        assert!(
            data.courses
                .iter()
                .any(|c| c.slots.iter().any(|s| s.d == 5)),
            "no Saturday slot generated"
        );
        // At least one no-slot (concentrated) course exists.
        assert!(data.courses.iter().any(|c| c.slots.is_empty()));

        // Producer→consumer round-trip: the engine rebuilds and sees every course.
        let json = serde_json::to_string(&data).unwrap();
        let engine = Engine::from_json(&json).expect("engine builds from generated data");
        assert_eq!(engine.filter(&Filters::default()).len(), data.courses.len());
    }

    #[test]
    fn enriched_cards_expose_delivery_and_eval() {
        // Through the real convert path, cards gain dm/ev for detailed courses,
        // and the weightless eval row renders as bare kind (no ":0").
        let g = generate(40, 42);
        let raw = raw_courses(&g);
        let details: std::collections::HashMap<String, crate::detail::SanshoDetail> = g
            .details
            .iter()
            .map(|d| (d.cd.clone(), d.clone()))
            .collect();
        let rendered = crate::convert::render_data_json(&raw, "t".into(), true, &details).unwrap();
        let json = String::from_utf8(rendered.bytes).unwrap();
        assert!(json.contains(r#""dm":"#));
        assert!(json.contains(r#""ev":["#));
        assert!(!json.contains(":0\""), "fabricated :0 weight leaked");
        // HTML metacharacters in a course name are escaped, never raw.
        assert!(!json.contains("<入門>"));
    }
}
