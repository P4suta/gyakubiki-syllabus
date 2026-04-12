package inspect

import (
	"encoding/json"
	"strings"
	"testing"
)

// === ヘルパー: テスト用JSON生成 ===

func makeAPIResponse(courses []map[string]interface{}, pageNo, maxPageNo, total, pageSize int) []byte {
	resp := map[string]interface{}{
		"selectKogiDtoList": courses,
		"pageNo":            pageNo,
		"maxPageNo":         maxPageNo,
		"total":             total,
		"pageSize":          pageSize,
	}
	b, _ := json.Marshal(resp)
	return b
}

func makeCourseMap(kogiCd, kogiNm, jikanwari string) map[string]interface{} {
	return map[string]interface{}{
		"kogiCd":           kogiCd,
		"kogiNm":           kogiNm,
		"fukudai":          nil,
		"tantoKyoin":       "教員",
		"jikanwari":        jikanwari,
		"kogiKaikojikiNm":  "1学期",
		"kogiKubunNm":      "講義",
		"sekininBushoNm":   "理工学部",
		"kochiNm":          "朝倉キャンパス",
		"gakusokuKamokuNm": kogiNm,
		"taishoGakka":      nil,
		"taishoNenji":      nil,
		"kamokuBunrui":     nil,
		"kamokuBunya":      nil,
	}
}

func makeBareArray(courses []map[string]interface{}) []byte {
	b, _ := json.Marshal(courses)
	return b
}

// === 入力形式の判定 ===

func TestInspect_FormatAPIResponse(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeAPIResponse(courses, 1, 1, 1, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.Format != "APIレスポンス" {
		t.Errorf("Format = %q, want %q", report.Format, "APIレスポンス")
	}
}

func TestInspect_FormatBareArray(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.Format != "配列" {
		t.Errorf("Format = %q, want %q", report.Format, "配列")
	}
}

func TestInspect_InvalidJSON(t *testing.T) {
	_, err := Inspect([]byte("not json"), "test.json", 8)
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
}

func TestInspect_EmptyInput(t *testing.T) {
	_, err := Inspect([]byte(""), "test.json", 0)
	if err == nil {
		t.Fatal("expected error for empty input")
	}
}

func TestInspect_HTMLInput(t *testing.T) {
	_, err := Inspect([]byte("<html><body>Error</body></html>"), "test.html", 30)
	if err == nil {
		t.Fatal("expected error for HTML input")
	}
	if !strings.Contains(err.Error(), "HTML") {
		t.Errorf("error should mention HTML: %v", err)
	}
}

// === ファイル情報 ===

func TestInspect_FileInfo(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeAPIResponse(courses, 1, 1, 1, 500)

	report, err := Inspect(data, "朝倉キャンパス.json", 2549965)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.FileName != "朝倉キャンパス.json" {
		t.Errorf("FileName = %q", report.FileName)
	}
	if report.FileSize != 2549965 {
		t.Errorf("FileSize = %d", report.FileSize)
	}
}

// === ページネーション ===

func TestInspect_PaginationExtracted(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeAPIResponse(courses, 1, 5, 2310, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.PageNo == nil || *report.PageNo != 1 {
		t.Errorf("PageNo = %v", report.PageNo)
	}
	if report.MaxPageNo == nil || *report.MaxPageNo != 5 {
		t.Errorf("MaxPageNo = %v", report.MaxPageNo)
	}
	if report.Total == nil || *report.Total != 2310 {
		t.Errorf("Total = %v", report.Total)
	}
	if report.PageSize == nil || *report.PageSize != 500 {
		t.Errorf("PageSize = %v", report.PageSize)
	}
}

func TestInspect_PaginationWarningWhenPartial(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeAPIResponse(courses, 1, 5, 2310, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Should warn about partial data
	hasPartialWarning := false
	for _, w := range report.Warnings {
		if strings.Contains(w, "ページ") || strings.Contains(w, "全体") {
			hasPartialWarning = true
			break
		}
	}
	if !hasPartialWarning {
		t.Errorf("expected warning about partial pagination, got: %v", report.Warnings)
	}
}

func TestInspect_PaginationNoWarningWhenComplete(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeAPIResponse(courses, 1, 1, 1, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	for _, w := range report.Warnings {
		if strings.Contains(w, "ページ") {
			t.Errorf("should not have pagination warning for complete data: %s", w)
		}
	}
}

func TestInspect_BareArrayNoPagination(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.PageNo != nil {
		t.Errorf("PageNo should be nil for bare array")
	}
	if report.MaxPageNo != nil {
		t.Errorf("MaxPageNo should be nil for bare array")
	}
}

// === フィールド検査 ===

func TestInspect_FieldCoverage_AllFilled(t *testing.T) {
	course := makeCourseMap("001", "テスト", "1学期: 月曜日１時限")
	course["fukudai"] = "副題あり"
	course["taishoGakka"] = "理工学部"
	course["taishoNenji"] = "1年"
	course["kamokuBunrui"] = "専門"
	course["kamokuBunya"] = "数学"
	data := makeAPIResponse([]map[string]interface{}{course}, 1, 1, 1, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	for _, fs := range report.FieldCoverage {
		if fs.Filled != 1 {
			t.Errorf("field %s: Filled = %d, want 1", fs.Name, fs.Filled)
		}
		if fs.Total != 1 {
			t.Errorf("field %s: Total = %d, want 1", fs.Name, fs.Total)
		}
	}
}

func TestInspect_FieldCoverage_NullFields(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "テスト", "1学期: 月曜日１時限"),
		makeCourseMap("002", "テスト2", "2学期: 火曜日２時限"),
	}
	// fukudai is nil in both
	data := makeAPIResponse(courses, 1, 1, 2, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	for _, fs := range report.FieldCoverage {
		if fs.Name == "fukudai" {
			if fs.Filled != 0 {
				t.Errorf("fukudai Filled = %d, want 0 (both are null)", fs.Filled)
			}
			if fs.Total != 2 {
				t.Errorf("fukudai Total = %d, want 2", fs.Total)
			}
		}
	}
}

func TestInspect_TotalFieldsCount(t *testing.T) {
	course := map[string]interface{}{
		"kogiCd":   "001",
		"kogiNm":   "テスト",
		"field3":   "a",
		"field4":   "b",
		"field5":   nil,
		"jikanwari": "1学期: 月曜日１時限",
		"kogiKaikojikiNm": "1学期",
		"kogiKubunNm": "講義",
		"sekininBushoNm": "理工学部",
		"kochiNm": "朝倉",
		"gakusokuKamokuNm": "テスト",
		"tantoKyoin": "教員",
	}
	data := makeAPIResponse([]map[string]interface{}{course}, 1, 1, 1, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.TotalFields != 12 {
		t.Errorf("TotalFields = %d, want 12", report.TotalFields)
	}
}

func TestInspect_CourseCount(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
		makeCourseMap("002", "B", "1学期: 火曜日２時限"),
		makeCourseMap("003", "C", "1学期: 水曜日３時限"),
	}
	data := makeAPIResponse(courses, 1, 1, 3, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.CourseCount != 3 {
		t.Errorf("CourseCount = %d, want 3", report.CourseCount)
	}
}

func TestInspect_EmptyArray(t *testing.T) {
	data := makeAPIResponse([]map[string]interface{}{}, 1, 1, 0, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.CourseCount != 0 {
		t.Errorf("CourseCount = %d, want 0", report.CourseCount)
	}
	if !report.CanConvert {
		t.Error("CanConvert should be true even for empty array")
	}
	// Should have a warning about 0 courses
	hasEmptyWarning := false
	for _, w := range report.Warnings {
		if strings.Contains(w, "0件") {
			hasEmptyWarning = true
		}
	}
	if !hasEmptyWarning {
		t.Errorf("expected warning about 0 courses, got: %v", report.Warnings)
	}
}

// === データ概要 ===

func TestInspect_UniqueKogiCd(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
		makeCourseMap("002", "B", "1学期: 火曜日２時限"),
		makeCourseMap("001", "A", "1学期: 水曜日３時限"), // duplicate
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.UniqueKogiCd != 2 {
		t.Errorf("UniqueKogiCd = %d, want 2", report.UniqueKogiCd)
	}
}

func TestInspect_Semesters(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
		makeCourseMap("002", "B", "2学期: 火曜日２時限"),
		makeCourseMap("003", "C", "通年: 水曜日３時限"),
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(report.Semesters) != 3 {
		t.Fatalf("Semesters len = %d, want 3: %v", len(report.Semesters), report.Semesters)
	}
}

func TestInspect_Departments(t *testing.T) {
	c1 := makeCourseMap("001", "A", "1学期: 月曜日１時限")
	c1["sekininBushoNm"] = "理工学部"
	c2 := makeCourseMap("002", "B", "1学期: 火曜日２時限")
	c2["sekininBushoNm"] = "人文社会科学部"
	data := makeBareArray([]map[string]interface{}{c1, c2})

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(report.Departments) != 2 {
		t.Fatalf("Departments len = %d, want 2: %v", len(report.Departments), report.Departments)
	}
}

func TestInspect_KogiKubuns(t *testing.T) {
	c1 := makeCourseMap("001", "A", "1学期: 月曜日１時限")
	c1["kogiKubunNm"] = "講義"
	c2 := makeCourseMap("002", "B", "1学期: 火曜日２時限")
	c2["kogiKubunNm"] = "演習"
	data := makeBareArray([]map[string]interface{}{c1, c2})

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(report.KogiKubuns) != 2 {
		t.Fatalf("KogiKubuns len = %d, want 2: %v", len(report.KogiKubuns), report.KogiKubuns)
	}
}

func TestInspect_Campuses(t *testing.T) {
	c1 := makeCourseMap("001", "A", "1学期: 月曜日１時限")
	c1["kochiNm"] = "朝倉キャンパス"
	c2 := makeCourseMap("002", "B", "1学期: 火曜日２時限")
	c2["kochiNm"] = "岡豊キャンパス"
	data := makeBareArray([]map[string]interface{}{c1, c2})

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(report.Campuses) != 2 {
		t.Fatalf("Campuses len = %d, want 2: %v", len(report.Campuses), report.Campuses)
	}
}

// === パース試行 ===

func TestInspect_ParseAllSuccess(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
		makeCourseMap("002", "B", "2学期: 火曜日２時限"),
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.ParseSuccess != 2 {
		t.Errorf("ParseSuccess = %d, want 2", report.ParseSuccess)
	}
	if len(report.ParseFailures) != 0 {
		t.Errorf("ParseFailures should be empty: %v", report.ParseFailures)
	}
}

func TestInspect_ParseSomeFailures(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
		makeCourseMap("002", "集中講義", "集中"),
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.ParseSuccess != 1 {
		t.Errorf("ParseSuccess = %d, want 1", report.ParseSuccess)
	}
	if len(report.ParseFailures) != 1 {
		t.Fatalf("ParseFailures len = %d, want 1", len(report.ParseFailures))
	}
	if report.ParseFailures[0].KogiCd != "002" {
		t.Errorf("ParseFailures[0].KogiCd = %q", report.ParseFailures[0].KogiCd)
	}
	if report.ParseFailures[0].KogiNm != "集中講義" {
		t.Errorf("ParseFailures[0].KogiNm = %q", report.ParseFailures[0].KogiNm)
	}
}

func TestInspect_EmptyJikanwariNotCountedAsFailure(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", ""),
	}
	data := makeBareArray(courses)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Empty jikanwari should NOT be a parse failure — it's simply not scheduled
	if len(report.ParseFailures) != 0 {
		t.Errorf("empty jikanwari should not be a parse failure: %v", report.ParseFailures)
	}
	// But it shouldn't count as success either
	if report.ParseSuccess != 0 {
		t.Errorf("ParseSuccess = %d, want 0 (empty jikanwari)", report.ParseSuccess)
	}
}

// === 総合判定 ===

func TestInspect_CanConvertTrue(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
	}
	data := makeAPIResponse(courses, 1, 1, 1, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !report.CanConvert {
		t.Error("CanConvert should be true for valid data")
	}
}

func TestInspect_CanConvertTrueEvenWithWarnings(t *testing.T) {
	courses := []map[string]interface{}{
		makeCourseMap("001", "A", "1学期: 月曜日１時限"),
		makeCourseMap("002", "B", "集中"),
	}
	data := makeAPIResponse(courses, 1, 5, 2310, 500)

	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !report.CanConvert {
		t.Error("CanConvert should be true even with parse failures and partial pagination")
	}
	if len(report.Warnings) == 0 {
		t.Error("expected warnings for partial pagination")
	}
}

// === カバレッジ補完: 未到達分岐 ===

func TestInspect_BrokenSelectKogiDtoList(t *testing.T) {
	// selectKogiDtoList exists but is not a valid array
	data := []byte(`{"selectKogiDtoList": "not an array", "pageNo": 1}`)
	_, err := Inspect(data, "test.json", int64(len(data)))
	if err == nil {
		t.Fatal("expected error for broken selectKogiDtoList")
	}
	if !strings.Contains(err.Error(), "selectKogiDtoList") {
		t.Errorf("error should mention selectKogiDtoList: %v", err)
	}
}

func TestInspect_ObjectWithoutSelectKogiDtoList(t *testing.T) {
	// Valid JSON object but no selectKogiDtoList and not an array
	_, err := Inspect([]byte(`{"data": "something"}`), "test.json", 21)
	if err == nil {
		t.Fatal("expected error for object without selectKogiDtoList")
	}
}

func TestInspect_PaginationFieldMissing(t *testing.T) {
	// API response without pagination fields
	data := []byte(`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A","jikanwari":"1学期: 月曜日１時限","tantoKyoin":"x","kogiKaikojikiNm":"1学期","kogiKubunNm":"講義","sekininBushoNm":"理工学部","kochiNm":"朝倉","gakusokuKamokuNm":"A"}]}`)
	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.PageNo != nil {
		t.Errorf("PageNo should be nil when not in response")
	}
}

func TestInspect_PaginationFieldWrongType(t *testing.T) {
	// pageNo is a string instead of int
	data := []byte(`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A","jikanwari":"1学期: 月曜日１時限","tantoKyoin":"x","kogiKaikojikiNm":"1学期","kogiKubunNm":"講義","sekininBushoNm":"理工学部","kochiNm":"朝倉","gakusokuKamokuNm":"A"}], "pageNo": "not a number"}`)
	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if report.PageNo != nil {
		t.Errorf("PageNo should be nil when field is wrong type")
	}
}

func TestInspect_NonStringFieldInCourse(t *testing.T) {
	// kogiNm is a number instead of string — getString should handle gracefully
	courses := []map[string]interface{}{
		{
			"kogiCd":           "001",
			"kogiNm":           12345, // not a string
			"jikanwari":        "1学期: 月曜日１時限",
			"tantoKyoin":       "x",
			"kogiKaikojikiNm":  "1学期",
			"kogiKubunNm":      "講義",
			"sekininBushoNm":   "理工学部",
			"kochiNm":          "朝倉",
			"gakusokuKamokuNm": "A",
		},
	}
	data := makeBareArray(courses)
	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Should still work, just with reduced field coverage
	if report.CourseCount != 1 {
		t.Errorf("CourseCount = %d, want 1", report.CourseCount)
	}
}

func TestInspect_MissingFieldsInCourse(t *testing.T) {
	// Course with some fields entirely missing (not null, but absent)
	courses := []map[string]interface{}{
		{
			"kogiCd":    "001",
			"kogiNm":    "テスト",
			"jikanwari": "1学期: 月曜日１時限",
			// tantoKyoin, sekininBushoNm, kochiNm etc. are absent
		},
	}
	data := makeBareArray(courses)
	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Should count missing fields as unfilled
	for _, fs := range report.FieldCoverage {
		if fs.Name == "tantoKyoin" && fs.Filled != 0 {
			t.Errorf("tantoKyoin Filled = %d, want 0 (field absent)", fs.Filled)
		}
	}
}

func TestInspect_JikanwariWhitespaceOnly(t *testing.T) {
	// jikanwari is whitespace-only — parser returns no slots and no warnings
	courses := []map[string]interface{}{
		{
			"kogiCd":           "001",
			"kogiNm":           "テスト",
			"jikanwari":        "   ",
			"tantoKyoin":       "x",
			"kogiKaikojikiNm":  "1学期",
			"kogiKubunNm":      "講義",
			"sekininBushoNm":   "理工学部",
			"kochiNm":          "朝倉",
			"gakusokuKamokuNm": "A",
		},
	}
	data := makeBareArray(courses)
	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Whitespace jikanwari should result in parse failure with fallback message
	if len(report.ParseFailures) != 1 {
		t.Fatalf("ParseFailures len = %d, want 1", len(report.ParseFailures))
	}
	if report.ParseFailures[0].Message == "" {
		t.Error("ParseFailure message should not be empty")
	}
}

func TestInspect_JikanwariNullInCourse(t *testing.T) {
	// jikanwari is null (not string) — getString returns ("", false)
	courses := []map[string]interface{}{
		{
			"kogiCd":           "001",
			"kogiNm":           "テスト",
			"jikanwari":        nil,
			"tantoKyoin":       "x",
			"kogiKaikojikiNm":  "1学期",
			"kogiKubunNm":      "講義",
			"sekininBushoNm":   "理工学部",
			"kochiNm":          "朝倉",
			"gakusokuKamokuNm": "A",
		},
	}
	data := makeBareArray(courses)
	report, err := Inspect(data, "test.json", int64(len(data)))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// null jikanwari → not parsed, not a failure
	if report.ParseSuccess != 0 {
		t.Errorf("ParseSuccess = %d, want 0", report.ParseSuccess)
	}
	if len(report.ParseFailures) != 0 {
		t.Errorf("ParseFailures should be empty for null jikanwari: %v", report.ParseFailures)
	}
}
