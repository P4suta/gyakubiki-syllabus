# KULAS findPage API 仕様メモ

`syllabus-cli fetch` が呼び出す KULAS シラバス検索 API の仕様。

## エンドポイント

```http
POST https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Gkm.Com.KogiKensaku.App.KogiKensakuWebApi/findPage
```

## セッション確立フロー

KULAS は GUEST ユーザーでもセッション cookie + CSRF token を要求する。

1. **GET** `https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku`
   - レスポンス: HTML
   - `Set-Cookie: CPSMART_PUBLIC_AUTH=...; GCLB=...` を取得
   - HTML 内に CSRF token が埋め込まれている（要観察 — 多分 `<script>` 内）
2. **POST** findPage（上記）
   - Cookie: `CPSMART_PUBLIC_AUTH`, `GCLB`
   - body: JSON（後述）
   - body 内の `tempData.entryContext.token` に CSRF token を埋める

## リクエストヘッダ

```http
accept: */*
accept-language: ja
content-type: application/json
origin: https://kulas.kochi-u.ac.jp
referer: https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku
user-agent: <Chrome 系の現実的な UA>
```

## リクエスト body

巨大（約 60KB のミニファイ JSON）。`internal/fetch/findpage_body.json` にテンプレートとして保存し、以下のプレースホルダだけ差し替える：

| プレースホルダ | 場所 | 内容 |
|---|---|---|
| `{{PAGE_NO}}` | `methodParams.kensakuJoken.pageNo` および `methodParams.kensakuJoken.values.pageNo` | 1, 2, 3, ... |
| `{{KAIKO_NENDO}}` | `methodParams.kensakuJoken.values.kaikoNendo.values[0]` | `"2026"` 等。年度 |
| `{{TOKEN}}` | `tempData.entryContext.token` | GET レスポンスから抽出した CSRF token |

それ以外の数百フィールドは検索条件のスキーマ宣言（空 values）で、KULAS 側が body 全体を要求するため温存する。

## レスポンス

```json
{
  "pageNo": 1,
  "maxPageNo": 8,
  "total": 3850,
  "pageSize": 500,
  "selectKogiDtoList": [ ... 500 件の RawCourse ... ]
}
```

ページごとに `pageNo` を 1..maxPageNo で叩く。レスポンスフィールド定義は `docs/kulas-api-fields.md` を参照。

## 保存ファイル名規約（既存踏襲）

| pageNo | ファイル名 |
|---|---|
| 1 | `raw/講義データ.json` |
| 2 以降 | `raw/講義データ-{pageNo:02d}.json` |

## 既知の制約

- ログイン認証は不要（GUEST ユーザー）
- 国内大学システムが GitHub Actions runner の IP を弾く可能性は未検証。初回 `workflow_dispatch` の `dry-run` で 200 が返るか必ず確認すること。
- `kaikoNendo` は毎年更新が必要。デフォルトは「現在年 (4月以降) または前年 (1-3月)」のロジックで自動計算し、`--year` フラグで上書き可能にする。

## TLS chain について

KULAS (IIS 10) は TLS handshake で **leaf 証明書のみ送信**し中間 CA を配信しない。
Chrome / Firefox は AIA fetching で chain を補完するが、Go の `crypto/tls`
は補完しないため `x509: certificate signed by unknown authority` で fail する。

対応として `internal/fetch/kulas_ca.pem` に中間 CA (`NII Open Domain CA - G7 RSA`)
を bundle して `internal/fetch/tls.go` の `newKulasTLSConfig()` で `RootCAs` に
積んでいる。Root CA (`Security Communication RootCA2`) はシステム標準 bundle に
含まれるので追加不要。

### 中間 CA の有効期限と更新手順

現行 bundle の有効期限: **2029-05-29** (確認: `openssl x509 -in internal/fetch/kulas_ca.pem -noout -dates`)

期限切れまたは KULAS の証明書 chain が更新されて fetch が再び
`unknown authority` で fail し始めたら、以下で再取得:

```sh
# 1. 現行 leaf 証明書の AIA URL を確認
openssl s_client -connect kulas.kochi-u.ac.jp:443 \
  -servername kulas.kochi-u.ac.jp </dev/null 2>/dev/null \
  | openssl x509 -noout -text | grep -A1 'Authority Information Access'

# 2. CA Issuers URL から DER を取得
curl -sS -o /tmp/intermediate.cer "<上で得た URL>"

# 3. PEM に変換して上書き
openssl x509 -inform DER -in /tmp/intermediate.cer -outform PEM \
  -out internal/fetch/kulas_ca.pem
```
