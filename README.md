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
