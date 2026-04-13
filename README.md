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

- 本ツールは高知大学の学生が個人的に作成した**非公式**のツールであり、大学とは一切関係ありません。
- 表示されるデータは[高知大学シラバス](https://www.kochi-u.ac.jp/education-support/courses/syllabus/)の公開情報を元に手動で収集したものです (2026年4月13日時点)。
- **内容の正確性・完全性・最新性について一切保証しません。**
- 本ツールの利用により生じたいかなる損害についても、作成者は責任を負いません。
- 履修に関する判断は必ず大学公式の情報を確認してください。

---

## 開発者向け

### 必要なもの

- **Go** (go.mod参照)
- **Bun**

### ローカル開発

```bash
go build -o bin/syllabus-cli ./cmd/syllabus-cli
./bin/syllabus-cli convert raw/*.json -o web/public/data.json
cd web && bun install && bun run dev
```

### テスト

```bash
go test ./...
cd web && bun run test && bun run check
```

### デプロイ

`main` ブランチへのpushで GitHub Actions が自動ビルド・デプロイを実行します。
