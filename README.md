# 逆引きシラバス

高知大学の全講義を「曜日 x 時限」の時間割グリッドで一覧できるツール。
KULASのシラバス検索サイトから手動で取得したJSONを変換して、ブラウザで閲覧する。

---

## セットアップ

### Docker を使う場合 (推奨)

必要なもの: **Docker**

```bash
docker compose build
```

これだけ。Go や Bun のインストールは不要。

### ローカルで動かす場合

必要なもの: **Go** と **Bun**

```bash
# Windows (winget)
winget install GoLang.Go
winget install Oven-sh.Bun

# macOS (Homebrew)
brew install go
brew install oven-sh/bun/bun
```

```bash
go build -o bin/syllabus-cli ./cmd/syllabus-cli
cd web && bun install && cd ..
```

---

## Step 1: KULASからデータを取得する (手動)

1. ブラウザで KULAS のシラバス検索ページを開く:
   https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku

2. 検索条件を設定する
   - **開講年度**: 2026 (見たい年度)
   - **校地**: 朝倉キャンパス (など、見たい校地)
   - 他の条件はお好みで

3. **検索ボタンを押す**

4. **F12** でDevToolsを開く → **Network** タブ

5. 検索結果の一覧が表示されたら、Networkタブに `findPage` というリクエストが出る

6. `findPage` をクリック → **Response** タブ → 中身を全選択(Ctrl+A) → コピー

7. テキストエディタに貼り付けて `page1.json` として保存

### ページが複数ある場合

検索結果が500件を超えると複数ページに分かれる。
一覧画面の「次のページ」ボタンを押すたびに新しい `findPage` リクエストが飛ぶので、
同じ手順でページごとに保存する。

```
page1.json  ← 1ページ目 (最大500件)
page2.json  ← 2ページ目
page3.json  ← 3ページ目
...
```

---

## Step 2: データを変換する

### Docker の場合

```bash
docker compose run --rm cli convert /data/page1.json /data/page2.json ... -o /data/data.json
```

### ローカルの場合

```bash
./bin/syllabus-cli convert page1.json page2.json page3.json -o data.json
```

複数ファイルを渡すと内部でマージして1つにまとめてくれる。

出力例:
```
  読み込み: page1.json (500件)
  読み込み: page2.json (500件)
  読み込み: page3.json (310件)
  合計: 1310件 (3ファイル)

✓ 1310件の講義を変換しました (元データ: 1310件) → data.json
```

### 変換前にデータを確認したい場合

```bash
./bin/syllabus-cli inspect page1.json
```

ファイルの構造、ページネーション情報、フィールドの充填率、時間割のパース成功率を一覧で表示する。
全ページ揃っているか確認するのに便利。

```
=== ページネーション ===
  ページ: 1 / 5
  このファイルの件数: 500
  全件数: 2310

=== 判定 ===
  ✓ convertに使用できます
  ⚠ このファイルは全体の21.6%です (1/5ページ)。残り4ページのデータが含まれていません
```

---

## Step 3: ブラウザで閲覧する

### Docker の場合

```bash
docker compose up web
```

http://localhost:3000 を開く。

### ローカルの場合

```bash
cd web && bun run dev
```

http://localhost:5173 を開く。

---

Step 2で作った `data.json` をページにドラッグ&ドロップするか、クリックしてファイルを選択する。

### ビューアーの機能

| 機能 | 操作 |
|------|------|
| **学期切り替え** | 上部タブで 1学期 / 1学期前半 / 1学期後半 / 2学期 / ... / 通年 を切り替え |
| **部署フィルタ** | セレクトボックスで開講責任部署を絞り込み |
| **検索** | 科目名、教員名、授業コード、部署名 ― 何でも引っかかる。全角/半角スペースはどちらでもOK |
| **詳細表示** | 講義カードをクリック → モーダルで全情報を表示 |

---

## コマンドリファレンス

### syllabus-cli convert

```bash
syllabus-cli convert <file> [file...] -o <output>

# 例
syllabus-cli convert page1.json page2.json page3.json -o data.json
cat raw.json | syllabus-cli convert -o data.json

# オプション
#   -o, --output <file>   出力先ファイル (省略するとstdoutに出力)
#   --compact             改行なしの圧縮JSON
```

### syllabus-cli inspect

```bash
syllabus-cli inspect <file>

# オプション
#   --json    JSON形式で出力 (スクリプトから使う場合)
```

---

## よくあるパターン

### データを更新したい

学期の途中で講義が追加・変更されることがある。
Step 1 からやり直して data.json を上書きするだけ。

### 複数の校地のデータをまとめたい

校地ごとに検索してダウンロードし、全部まとめて convert に渡せばOK。

```bash
./bin/syllabus-cli convert \
  朝倉-page1.json 朝倉-page2.json \
  岡豊-page1.json 岡豊-page2.json \
  物部-page1.json \
  -o all-campuses.json
```

### 「集中講義」が時間割に表示されない

集中講義は曜日x時限が存在しないため、グリッドには表示されない。
データ自体は data.json に含まれている (slotsが空配列)。
inspect で「パース失敗」として確認できる。

---

## トラブルシューティング

### `syllabus-cli` が動かない

```bash
# ビルドし直す
go build -o bin/syllabus-cli ./cmd/syllabus-cli
```

### ビューアーでファイルを選択しても何も起きない

- `syllabus-cli convert` で作ったファイルか確認 (KULASの生JSONを直接読み込むことはできない)
- ブラウザの F12 → Console タブにエラーが出ていないか確認
- `data.json` が 0バイトでないか確認

### ポートが使用中

Viteは自動的に 5174, 5175... と別のポートで起動する。
Docker の場合は固定で 3000番ポート。
