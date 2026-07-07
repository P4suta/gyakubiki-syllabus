# KULAS API response fields

Describes each field inside `selectKogiDtoList` in the KULAS (Kochi University syllabus system) API response.

## Pagination (top level)

| Field | Type | Description |
|-----------|-----|------|
| `pageNo` | int | Current page number |
| `maxPageNo` | int | Last page number |
| `total` | int | Total number of results |
| `pageSize` | int | Results per page (usually 500) |

## Course record fields

### Currently used (by gyakubiki-syllabus)

| Field | Type | Example | Description |
|-----------|-----|-----|------|
| `kogiCd` | string | `"12345"` | Course code. Unique identifier |
| `kogiNm` | string | `"プログラミング入門"` | Course name |
| `fukudai` | string? | `"（情報科学）"` | Subtitle. May be null |
| `tantoKyoin` | string | `"山田 太郎, 鈴木 花子"` | Instructors. Comma-separated for multiple |
| `jikanwari` | string | `"1学期: 水曜日１時限"` | Timetable string. Structured by the parser |
| `kogiKaikojikiNm` | string | `"1学期"` | Term name |
| `kogiKubunNm` | string | `"講義"` | Course type (lecture/seminar/experiment/practicum/practical) |
| `sekininBushoNm` | string | `"共通教育"` | Offering department name |
| `kochiNm` | string | `"朝倉キャンパス"` | Campus name |
| `gakusokuKamokuNm` | string | `"プログラミング入門"` | Official course name. Same as `kogiNm` in most cases |
| `taishoGakka` | string? | `"全学部共通"` | Target department |
| `taishoNenji` | string? | `"1年"` | Target year |
| `kamokuBunrui` | string? | `"専門"` | Course classification |
| `kamokuBunya` | string? | `"情報"` | Course field |

### Codes (paired with name fields)

| Field | Type | Example | Paired name |
|-----------|-----|-----|-------------|
| `gakusokuKamokuCd` | string | `"K99001"` | Official course code |
| `sekininBushoCd` | string | `"000xxx"` | Department code |
| `kochiCd` | string | `"01"` | Campus code (01=Asakura, 02=Monobe, 03=Oko) |
| `kogiKaikojikiCd` | string | `"2"` | Term code |
| `kogiKubunCd` | string | `"01"` | Course type code (01=lecture, 02=seminar, etc.) |
| `daihyoKyoinCd` | string | `"k00012345"` | Representative instructor code |
| `daihyoKyoinNm` | string | `"山田 太郎"` | Representative instructor. First of `tantoKyoin` |

### Timetable (auxiliary)

| Field | Type | Example | Description |
|-----------|-----|-----|------|
| `daihyoYobiNm` | string | `"水曜日"` | Representative weekday. First weekday in `jikanwari` |
| `daihyoJigenNm` | string | `"１時限"` | Representative period. First period in `jikanwari` |
| `yobi` | string | `"水曜日"` | Weekday (often same as `daihyoYobiNm`) |
| `jigen` | string | `"１時限"` | Period (often same as `daihyoJigenNm`) |

### Enrollment / management flags

| Field | Type | Example | Description |
|-----------|-----|-----|------|
| `select` | bool | `false` | UI selection state (client-side) |
| `isOpened` | bool | `false` | UI expansion state (client-side) |
| `rishuKarteFlg` | string | `"0"` | Enrollment record flag |
| `chusenTaishoFlg` | string | `"0"` | Lottery-eligible flag |
| `nendoKeizokuFlg` | string | `"0"` | Year-continuation flag |
| `lockFlg` | string | `"0"` | Lock flag |
| `lockFlgNm` | string | `"しない"` | Lock flag name |
| `webkogiSeisekiKyokaFlg` | string | `"1"` | Web grade permission flag |
| `webkogiSeisekiKyokaFlgNm` | string | `"する"` | Web grade permission flag name |
| `webrishuTaishogaiFlg` | string | `"0"` | Web enrollment-excluded flag |
| `webrishuTaishogaiFlgNm` | string | `"対象"` | Web enrollment-excluded flag name |
| `webrishuTorikeshifukaFlg` | string | `"0"` | Web enrollment cancel-not-allowed flag |
| `webrishuTorikeshifukaFlgNm` | string | `"取り消し可能"` | Web enrollment cancel-not-allowed flag name |
| `gpcaKeisanTaishoFlg` | string | `"1"` | GPA calculation-eligible flag |
| `gpcaKeisanTaishoFlgNm` | string | `"対象"` | GPA calculation-eligible flag name |
| `kokaiFlg` | string | `"1"` | Publish flag |
| `kokaiFlgNm` | string | `"公開"` | Publish flag name |
| `daihyoKogiFlg` | int | `0` | Representative-course flag |
| `sagyoKanryoFlg` | int | `1` | Work-complete flag |
| `sagyoKanryoFlgNm` | string | `"完了"` | Work-complete flag name |
| `torokuType` | string | `"1"` | Registration type |

### Syllabus content

| Field | Type | Description |
|-----------|-----|------|
| `keyword` | string? | Keywords |
| `gakushuMokuhyo` | string? | Learning objectives |
| `gairyaku` | string? | Summary |
| `shosai` | string? | Details |
| `jugyoKeishiki` | string? | Class format |
| `hyokaHoho` | string? | Grading method |
| `text` | string? | Textbook |
| `textIsbn` | string? | Textbook ISBN |
| `sankoBunken` | string? | References |
| `officeHour` | string? | Office hours |
| `gakuseiMessage` | string? | Message to students |
| `junbiGakushu` | string? | Preparatory study |
| `url` | string? | URL |
| `sanshoUrl` | string? | Reference URL |

### Grading / input patterns

| Field | Type | Example | Description |
|-----------|-----|-----|------|
| `nyuryokuKikanPatternCd` | string | `"0000001"` | Input-period pattern code |
| `nyuryokuKikanPatternNm` | string | `"標準パターン"` | Input-period pattern name |
| `tsuisaishiKikanPatternCd` | string | `"0000001"` | Makeup/re-exam period pattern code |
| `tsuisaishiKikanPatternNm` | string | `"標準パターン"` | Makeup/re-exam period pattern name |
| `kogiseisekiShikenshubetsuPatternCd` | string | `"0000004"` | Grade exam-type pattern code |
| `kogiseisekiShikenshubetsuPatternNm` | string | `"素点入力パターン"` | Grade exam-type pattern name |
| `tsuisaishikanriMode` | string | `"1"` | Makeup/re-exam management mode |
| `tsuisaishikanriModeNm` | string | `"成績絞込無・申請管理無"` | Makeup/re-exam management mode name |
| `shikenanketoTaishoFlgNm` | string | `"対象外"` | Exam-survey eligibility flag name |
| `kogiseisekiShikenhohoNm` | string? | null | Grade exam-method name |
| `shikenshubetsuRyokinPatternCd` | string? | null | Exam-type fee pattern code |

### Other metadata

| Field | Type | Example | Description |
|-----------|-----|-----|------|
| `kaikoNendo` | string | `"2026"` | Academic year |
| `kanaKogiNm` | string? | null | Course name in kana |
| `kogiRnm` | string? | null | Course name abbreviation |
| `chuyaKubunCd` | string? | null | Day/night division code |
| `chuyaKubunNm` | string? | null | Day/night division name |
| `kogiJiyuCd` | string? | null | Course reason code |
| `kogiJiyuNm` | string? | null | Course reason name |
| `rishu` | string? | null | Enrollment info |
| `biko1`, `biko2` | string? | null | Remarks 1, 2 |
| `kogiGroup` | string? | null | Course group |
| `kogiGroupCd` | string? | null | Course group code |
| `kogiGroupNm` | string? | null | Course group name |
| `yotoCd` | string? | null | Purpose code |
| `syllabusKanren` | string? | null | Syllabus relation |
| `syllabusKomokuPatternId` | string | `"4"` | Syllabus item pattern ID |
| `syllabusKomokuPatternNm` | string | `"【学部】2024年度以降..."` | Syllabus item pattern name |
| `sosaKyoin` | string? | null | Operating instructor |
| `sosaNichiji` | string? | null | Operation timestamp |
| `lastUpdate` | string | `"20260310175914381"` | Last-update timestamp |
| `lastUser` | string | `"k00012345"` | Last-update user |
| `kyoinSosaStatusNm` | string | `"更新した"` | Instructor operation status |
| `daihyoNumberingCd` | string | `""` | Representative numbering code |
| `shozokubetsuNumberingCd` | string? | null | Per-affiliation numbering code |
| `kyoinShimei` | string? | null | Instructor full name |
| `kyointantoKubunNm` | string? | null | Instructor role name |
| `sokaikoJikansu` | int | `0` | Total teaching hours |
| `kogiKaisu` | int | `0` | Number of sessions |
| `shippitsuTantoKyoin` | string? | null | Authoring instructor |
| `kamokuKaiso1` | string? | null | Course hierarchy 1 |
| `nyuryokuKanryoFlg` | string | `"1"` | Input-complete flag |
| `nyuryokuKanryoFlgNm` | string | `"完了"` | Input-complete flag name |
| `kJitsumuKeiken` | string? | null | Practical experience |
| `kOfficeHour` | string? | null | Office hours (k-prefixed) |
| `kYobi` | string? | null | Reserved |
| `kKoji` | string? | null | Notice |
| `kBasho` | string? | null | Location |
| `kBiko1`–`kBiko10` | string? | null | Remarks 1–10 |

### Assignment

| Field | Type | Description |
|-----------|-----|------|
| `haitoShozokuCdStart` | string? | Assigned affiliation code (start) |
| `haitoShozokuCdEnd` | string? | Assigned affiliation code (end) |
| `haitoGakunen` | string? | Assigned grade |
| `haitoSemester` | string? | Assigned semester |
| `haitoClassCdStart` | string? | Assigned class code (start) |
| `haitoClassCdEnd` | string? | Assigned class code (end) |

### Header items

| Field | Type | Description |
|-----------|-----|------|
| `headerKomoku1`–`headerKomoku4` | string? | Custom header items 1–4 |
