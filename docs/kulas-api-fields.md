# KULAS API レスポンスフィールド一覧

高知大学シラバスシステム（KULAS）のAPIレスポンス `selectKogiDtoList` 内の各フィールドを解説する。

## ページネーション（トップレベル）

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `pageNo` | int | 現在のページ番号 |
| `maxPageNo` | int | 最大ページ番号 |
| `total` | int | 検索結果の全件数 |
| `pageSize` | int | 1ページあたりの件数（通常500） |

## 講義レコードフィールド

### ★ 現在使用中（逆引きシラバスで利用）

| フィールド | 型 | 例 | 説明 |
|-----------|-----|-----|------|
| `kogiCd` | string | `"12345"` | 授業コード。一意識別子 |
| `kogiNm` | string | `"プログラミング入門"` | 科目名 |
| `fukudai` | string? | `"（情報科学）"` | 副題。nullの場合あり |
| `tantoKyoin` | string | `"山田 太郎, 鈴木 花子"` | 担当教員。カンマ区切りで複数名 |
| `jikanwari` | string | `"1学期: 水曜日１時限"` | 時間割文字列。パーサーで構造化 |
| `kogiKaikojikiNm` | string | `"1学期"` | 開講時期の名称 |
| `kogiKubunNm` | string | `"講義"` | 講義区分（講義/演習/実験/実習/実技） |
| `sekininBushoNm` | string | `"共通教育"` | 開講責任部署の名称 |
| `kochiNm` | string | `"朝倉キャンパス"` | 校地（キャンパス）名称 |
| `gakusokuKamokuNm` | string | `"プログラミング入門"` | 学則上の科目名。99.2%がkogiNmと同一 |
| `taishoGakka` | string? | `"全学部共通"` | 対象学科 |
| `taishoNenji` | string? | `"1年"` | 対象年次 |
| `kamokuBunrui` | string? | `"専門"` | 科目分類 |
| `kamokuBunya` | string? | `"情報"` | 科目分野 |

### コード系（名称フィールドと対応）

| フィールド | 型 | 例 | 対応する名称 |
|-----------|-----|-----|-------------|
| `gakusokuKamokuCd` | string | `"K99001"` | 学則科目コード |
| `sekininBushoCd` | string | `"000xxx"` | 責任部署コード |
| `kochiCd` | string | `"01"` | 校地コード（01=朝倉, 02=物部, 03=岡豊） |
| `kogiKaikojikiCd` | string | `"2"` | 開講時期コード |
| `kogiKubunCd` | string | `"01"` | 講義区分コード（01=講義, 02=演習, etc.） |
| `daihyoKyoinCd` | string | `"k00012345"` | 代表教員コード |
| `daihyoKyoinNm` | string | `"山田 太郎"` | 代表教員名。tantoKyoinの最初の教員 |

### 時間割関連（補助）

| フィールド | 型 | 例 | 説明 |
|-----------|-----|-----|------|
| `daihyoYobiNm` | string | `"水曜日"` | 代表曜日。jikanwariの最初の曜日 |
| `daihyoJigenNm` | string | `"１時限"` | 代表時限。jikanwariの最初の時限 |
| `yobi` | string | `"水曜日"` | 曜日（daihyoYobiNmと同じことが多い） |
| `jigen` | string | `"１時限"` | 時限（daihyoJigenNmと同じことが多い） |

### 履修・管理フラグ

| フィールド | 型 | 例 | 説明 |
|-----------|-----|-----|------|
| `select` | bool | `false` | UI選択状態（クライアント用） |
| `isOpened` | bool | `false` | UI展開状態（クライアント用） |
| `rishuKarteFlg` | string | `"0"` | 履修カルテフラグ |
| `chusenTaishoFlg` | string | `"0"` | 抽選対象フラグ |
| `nendoKeizokuFlg` | string | `"0"` | 年度継続フラグ |
| `lockFlg` | string | `"0"` | ロックフラグ |
| `lockFlgNm` | string | `"しない"` | ロックフラグ名称 |
| `webkogiSeisekiKyokaFlg` | string | `"1"` | Web成績許可フラグ |
| `webkogiSeisekiKyokaFlgNm` | string | `"する"` | Web成績許可フラグ名称 |
| `webrishuTaishogaiFlg` | string | `"0"` | Web履修対象外フラグ |
| `webrishuTaishogaiFlgNm` | string | `"対象"` | Web履修対象外フラグ名称 |
| `webrishuTorikeshifukaFlg` | string | `"0"` | Web履修取消不可フラグ |
| `webrishuTorikeshifukaFlgNm` | string | `"取り消し可能"` | Web履修取消不可フラグ名称 |
| `gpcaKeisanTaishoFlg` | string | `"1"` | GPA計算対象フラグ |
| `gpcaKeisanTaishoFlgNm` | string | `"対象"` | GPA計算対象フラグ名称 |
| `kokaiFlg` | string | `"1"` | 公開フラグ |
| `kokaiFlgNm` | string | `"公開"` | 公開フラグ名称 |
| `daihyoKogiFlg` | int | `0` | 代表講義フラグ |
| `sagyoKanryoFlg` | int | `1` | 作業完了フラグ |
| `sagyoKanryoFlgNm` | string | `"完了"` | 作業完了フラグ名称 |
| `torokuType` | string | `"1"` | 登録タイプ |

### シラバス内容

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `keyword` | string? | キーワード |
| `gakushuMokuhyo` | string? | 学習目標 |
| `gairyaku` | string? | 概略 |
| `shosai` | string? | 詳細 |
| `jugyoKeishiki` | string? | 授業形式 |
| `hyokaHoho` | string? | 評価方法 |
| `text` | string? | テキスト |
| `textIsbn` | string? | テキストISBN |
| `sankoBunken` | string? | 参考文献 |
| `officeHour` | string? | オフィスアワー |
| `gakuseiMessage` | string? | 学生へのメッセージ |
| `junbiGakushu` | string? | 準備学習 |
| `url` | string? | URL |
| `sanshoUrl` | string? | 参照URL |

### 成績・入力パターン

| フィールド | 型 | 例 | 説明 |
|-----------|-----|-----|------|
| `nyuryokuKikanPatternCd` | string | `"0000001"` | 入力期間パターンコード |
| `nyuryokuKikanPatternNm` | string | `"標準パターン"` | 入力期間パターン名称 |
| `tsuisaishiKikanPatternCd` | string | `"0000001"` | 追再試期間パターンコード |
| `tsuisaishiKikanPatternNm` | string | `"標準パターン"` | 追再試期間パターン名称 |
| `kogiseisekiShikenshubetsuPatternCd` | string | `"0000004"` | 成績試験種別パターンコード |
| `kogiseisekiShikenshubetsuPatternNm` | string | `"素点入力パターン"` | 成績試験種別パターン名称 |
| `tsuisaishikanriMode` | string | `"1"` | 追再試管理モード |
| `tsuisaishikanriModeNm` | string | `"成績絞込無・申請管理無"` | 追再試管理モード名称 |
| `shikenanketoTaishoFlgNm` | string | `"対象外"` | 試験案件対象フラグ名称 |
| `kogiseisekiShikenhohoNm` | string? | null | 成績試験方法名称 |
| `shikenshubetsuRyokinPatternCd` | string? | null | 試験種別料金パターンコード |

### その他メタ情報

| フィールド | 型 | 例 | 説明 |
|-----------|-----|-----|------|
| `kaikoNendo` | string | `"2026"` | 開講年度 |
| `kanaKogiNm` | string? | null | 科目名カナ |
| `kogiRnm` | string? | null | 科目名略称 |
| `chuyaKubunCd` | string? | null | 昼夜区分コード |
| `chuyaKubunNm` | string? | null | 昼夜区分名称 |
| `kogiJiyuCd` | string? | null | 講義事由コード |
| `kogiJiyuNm` | string? | null | 講義事由名称 |
| `rishu` | string? | null | 履修情報 |
| `biko1`, `biko2` | string? | null | 備考1, 2 |
| `kogiGroup` | string? | null | 講義グループ |
| `kogiGroupCd` | string? | null | 講義グループコード |
| `kogiGroupNm` | string? | null | 講義グループ名称 |
| `yotoCd` | string? | null | 用途コード |
| `syllabusKanren` | string? | null | シラバス関連 |
| `syllabusKomokuPatternId` | string | `"4"` | シラバス項目パターンID |
| `syllabusKomokuPatternNm` | string | `"【学部】2024年度以降..."` | シラバス項目パターン名称 |
| `sosaKyoin` | string? | null | 操作教員 |
| `sosaNichiji` | string? | null | 操作日時 |
| `lastUpdate` | string | `"20260310175914381"` | 最終更新タイムスタンプ |
| `lastUser` | string | `"k00012345"` | 最終更新ユーザー |
| `kyoinSosaStatusNm` | string | `"更新した"` | 教員操作状態 |
| `daihyoNumberingCd` | string | `""` | 代表ナンバリングコード |
| `shozokubetsuNumberingCd` | string? | null | 所属別ナンバリングコード |
| `kyoinShimei` | string? | null | 教員氏名 |
| `kyointantoKubunNm` | string? | null | 教員担当区分名称 |
| `sokaikoJikansu` | int | `0` | 総開講時間数 |
| `kogiKaisu` | int | `0` | 講義回数 |
| `shippitsuTantoKyoin` | string? | null | 執筆担当教員 |
| `kamokuKaiso1` | string? | null | 科目階層1 |
| `nyuryokuKanryoFlg` | string | `"1"` | 入力完了フラグ |
| `nyuryokuKanryoFlgNm` | string | `"完了"` | 入力完了フラグ名称 |
| `kJitsumuKeiken` | string? | null | 実務経験 |
| `kOfficeHour` | string? | null | オフィスアワー（k接頭） |
| `kYobi` | string? | null | 予備 |
| `kKoji` | string? | null | 告示 |
| `kBasho` | string? | null | 場所 |
| `kBiko1`〜`kBiko10` | string? | null | 備考1〜10 |

### 配当関連

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `haitoShozokuCdStart` | string? | 配当所属コード（開始） |
| `haitoShozokuCdEnd` | string? | 配当所属コード（終了） |
| `haitoGakunen` | string? | 配当学年 |
| `haitoSemester` | string? | 配当セメスター |
| `haitoClassCdStart` | string? | 配当クラスコード（開始） |
| `haitoClassCdEnd` | string? | 配当クラスコード（終了） |

### ヘッダー項目

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `headerKomoku1`〜`headerKomoku4` | string? | カスタムヘッダー項目1〜4 |
