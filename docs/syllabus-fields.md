# シラバス項目の表示優先度

> このファイルは `syllabus-cli gen-field-docs` が `crates/cli/src/fields.rs` の
> `FIELD_SPEC` から自動生成します。**手で編集しないでください。**
> 並び順・tier を変えるには `FIELD_SPEC` を編集し `just gen-field-docs` を実行します。

| tier | group | key | ラベル | 表示 |
|---|---|---|---|---|
| 1 |  | `eval` | 成績評価 | `eval-chart` |
| 1 |  | `delivery` | 授業実施方法 | `delivery-badge` |
| 1 |  | `unit` | 単位数 | `meta` |
| 1 |  | `summary` | 授業の概要 | `longtext` |
| 1 | 授業内容 | `aims` | 授業の目的 | `longtext` |
| 2 | 授業内容 | `goals` | 到達目標 | `list` |
| 2 | 授業内容 | `plan` | 授業計画 | `plan-timeline` |
| 2 | 授業内容 | `textbooks` | 教科書・参考書 | `longtext` |
| 2 | 授業内容 | `prereq` | 履修に求めるもの | `longtext` |
| 2 | 授業内容 | `prep` | 授業時間外の学習 | `longtext` |
| 2 | 授業内容 | `officeHour` | オフィスアワー | `office-table` |
| 3 | その他 | `teachers` | 担当教員 | `list` |
| 3 | その他 | `keywords` | キーワード | `chips` |
| 3 | その他 | `numbering` | ナンバリング | `chips` |
| 3 | その他 | `sdgs` | SDGs | `sdgs` |
