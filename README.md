# 逆引きシラバス

高知大学の講義を「曜日 x 時限」の時間割グリッドで一覧できるツール。

**[https://p4suta.github.io/gyakubiki-syllabus/](https://p4suta.github.io/gyakubiki-syllabus/)**

---

## 機能

| 機能 | 操作 |
|------|------|
| **学期切り替え** | 上部タブで学期を切り替え |
| **部署フィルタ** | セレクトボックスで開講責任部署を絞り込み |
| **検索** | 科目名、教員名、授業コードなどで検索 |
| **詳細表示** | 講義カードをクリックで詳細を表示 |

---

## 免責事項

本ツールは個人が作成した非公式のものであり、高知大学とは一切の関係がなく、同大学による承認又は推奨を受けたものではありません。表示されるデータは、[同大学が一般に公開しているシラバス情報](https://www.kochi-u.ac.jp/education-support/courses/syllabus/)のみに基づいており（2026年4月13日時点）、非公開情報は一切使用していません。

本ツールは現状有姿（AS IS）で提供され、明示又は黙示を問わず、正確性、完全性、最新性、特定目的への適合性その他一切の保証をいたしません。本ツールの利用又は利用不能により生じた直接的又は間接的な損害について、作成者は一切の責任を負いません。履修登録その他の判断は、必ず大学公式の情報に基づいて行ってください。

---

## 開発者向け

### 必要なもの

- **Go** (`go.mod` 参照)
- **Bun**
- **just** ([just-rs](https://github.com/casey/just) — タスクランナー)

### よく使うコマンド

`just` でコマンド一覧を表示できます。主なもの:

```bash
just dev            # Web の dev server を起動
just build          # Go バイナリをビルド
just convert        # raw/ → web/public/data.json
just test           # Go + Web のテストを通す
just lint           # 全 linter (Go / typos / actionlint / markdown)
just fmt            # 自動 format (gofumpt + typos --write-changes)
just check          # CI と等価のチェック (lint + test)
just install-tools  # 開発ツール一括 install
just install-hooks  # lefthook で git hooks を有効化
```

### 開発ツール (モダン構成)

| 種別 | ツール | 設定ファイル |
|---|---|---|
| タスクランナー | `just` | `justfile` |
| Git hooks | `lefthook` | `lefthook.yml` |
| Go formatter | `gofumpt` (gofmt より厳格) | `.golangci.yml` |
| Go meta-linter | `golangci-lint` v2 (`default: all`) | `.golangci.yml` |
| Spell check | `typos` (Rust 製, 高速) | `.typos.toml` |
| Actions lint | `actionlint` | — |
| Markdown lint | `markdownlint-cli2` | `.markdownlint.yaml` |

`lefthook` の `pre-commit` / `pre-push` は CI と同じ範囲を走らせます (PR でしか落ちない事故を防ぐため)。`just install-hooks` で有効化。

### 自動取得 (月次)

`raw/*.json` は **`.github/workflows/fetch-syllabus.yml`** が月次 cron で自動更新します (毎月 2 日 04:00 JST)。手動実行は GitHub の Actions タブから `Fetch syllabus monthly` → `Run workflow` で `dry-run` も選択可能。

詳細仕様は [`docs/kulas-api-spec.md`](docs/kulas-api-spec.md) 参照。

### デプロイ

`main` ブランチへの push で `.github/workflows/deploy.yml` が自動ビルド・デプロイを実行します。`fetch-syllabus.yml` は変更を commit して push した後、明示的に deploy を起動します (`GITHUB_TOKEN` の push は `on: push` を起動しない仕様への対応)。
