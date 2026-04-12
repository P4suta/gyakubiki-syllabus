package transform

import (
	"strings"
	"testing"

	"github.com/livec/gyakubiki-syllabus/internal/model"
)

func ptr(s string) *string { return &s }

func TestConvert(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:         "001",
			KogiNm:         "基礎数学",
			TantoKyoin:     "山田 太郎",
			Jikanwari:      "1学期: 月曜日１時限",
			SekininBushoNm: "理工学部",
			KochiNm:        "朝倉",
		},
		{
			KogiCd:         "002",
			KogiNm:         "政治学概論",
			TantoKyoin:     "小川 寛貴",
			Jikanwari:      "2学期: 火曜日２時限, 2学期: 木曜日１時限",
			SekininBushoNm: "人文社会科学部",
			KochiNm:        "朝倉",
		},
		{
			KogiCd:         "001", // duplicate
			KogiNm:         "基礎数学",
			TantoKyoin:     "山田 太郎",
			Jikanwari:      "1学期: 月曜日１時限, 1学期: 水曜日３時限",
			SekininBushoNm: "理工学部",
			KochiNm:        "朝倉",
		},
	}

	result := Convert(raw)

	if result.Data.Version != 1 {
		t.Errorf("version = %d, want 1", result.Data.Version)
	}
	if result.Data.TotalRaw != 3 {
		t.Errorf("totalRaw = %d, want 3", result.Data.TotalRaw)
	}

	// Should deduplicate to 2 courses
	if len(result.Data.Courses) != 2 {
		t.Fatalf("courses len = %d, want 2", len(result.Data.Courses))
	}

	// First course should have merged slots (月1 + 水3)
	c1 := result.Data.Courses[0]
	if c1.KogiCd != "001" {
		t.Errorf("courses[0].kogiCd = %s, want 001", c1.KogiCd)
	}
	if len(c1.Slots) != 2 {
		t.Errorf("courses[0] slots len = %d, want 2", len(c1.Slots))
	}

	// Second course should have 2 slots
	c2 := result.Data.Courses[1]
	if len(c2.Slots) != 2 {
		t.Errorf("courses[1] slots len = %d, want 2", len(c2.Slots))
	}

	// Semesters should be sorted
	if len(result.Data.Semesters) != 2 {
		t.Fatalf("semesters len = %d, want 2", len(result.Data.Semesters))
	}
	if result.Data.Semesters[0] != "1学期" || result.Data.Semesters[1] != "2学期" {
		t.Errorf("semesters = %v, want [1学期 2学期]", result.Data.Semesters)
	}

	// Departments should be sorted alphabetically
	if len(result.Data.Departments) != 2 {
		t.Fatalf("departments len = %d, want 2", len(result.Data.Departments))
	}

	// No warnings expected for valid data
	if len(result.Warnings) != 0 {
		t.Errorf("unexpected warnings: %v", result.Warnings)
	}
}

func TestConvertEmpty(t *testing.T) {
	result := Convert(nil)
	if len(result.Data.Courses) != 0 {
		t.Errorf("expected empty courses, got %d", len(result.Data.Courses))
	}
	if result.Data.TotalRaw != 0 {
		t.Errorf("totalRaw = %d, want 0", result.Data.TotalRaw)
	}
}

func TestConvertSkipsEmptyKogiCd(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "", KogiNm: "空コード科目", Jikanwari: "1学期: 月曜日１時限"},
		{KogiCd: "  ", KogiNm: "空白コード科目", Jikanwari: "1学期: 火曜日２時限"},
		{KogiCd: "001", KogiNm: "正常な科目", Jikanwari: "1学期: 水曜日３時限"},
	}

	result := Convert(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1 (should skip empty kogiCd)", len(result.Data.Courses))
	}
	if result.Data.Courses[0].KogiCd != "001" {
		t.Errorf("expected remaining course to be 001, got %s", result.Data.Courses[0].KogiCd)
	}
	if len(result.Warnings) != 2 {
		t.Errorf("expected 2 warnings for empty kogiCd, got %d: %v", len(result.Warnings), result.Warnings)
	}
}

func TestConvertWarnsEmptyKogiNm(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "", Jikanwari: "1学期: 月曜日１時限"},
	}

	result := Convert(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(result.Data.Courses))
	}
	if len(result.Warnings) == 0 {
		t.Error("expected warning for empty kogiNm")
	}
}

func TestConvertWarnsUnparseableJikanwari(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "集中講義", Jikanwari: "集中"},
	}

	result := Convert(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(result.Data.Courses))
	}
	if len(result.Data.Courses[0].Slots) != 0 {
		t.Errorf("expected 0 slots for unparseable jikanwari")
	}
	if len(result.Warnings) == 0 {
		t.Error("expected warning for unparseable jikanwari")
	}
}

func TestConvertNoWarningForEmptyJikanwari(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "オンデマンド講義", Jikanwari: ""},
	}

	result := Convert(raw)

	// Empty jikanwari is not a warning — some courses genuinely have no schedule
	if len(result.Warnings) != 0 {
		t.Errorf("unexpected warnings for empty jikanwari: %v", result.Warnings)
	}
}

func TestConvertPreservesOptionalFields(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:           "001",
			KogiNm:           "テスト",
			Fukudai:          ptr("副題テスト"),
			TantoKyoin:       "教員",
			Jikanwari:        "1学期: 月曜日１時限",
			KogiKaikojikiNm:  "2026年度",
			KogiKubunNm:      "講義",
			SekininBushoNm:   "理工学部",
			KochiNm:          "朝倉",
			GakusokuKamokuNm: "テスト",
			TaishoGakka:      ptr("理工学部"),
			TaishoNenji:      ptr("1年"),
			KamokuBunrui:     ptr("専門"),
			KamokuBunya:      ptr("数学"),
		},
	}

	result := Convert(raw)
	c := result.Data.Courses[0]

	if c.Fukudai == nil || *c.Fukudai != "副題テスト" {
		t.Errorf("fukudai not preserved")
	}
	if c.TaishoGakka == nil || *c.TaishoGakka != "理工学部" {
		t.Errorf("taishoGakka not preserved")
	}
}

func TestConvertSemesterSortOrder(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: "後期: 月曜日１時限"},
		{KogiCd: "002", KogiNm: "B", Jikanwari: "通年: 火曜日２時限"},
		{KogiCd: "003", KogiNm: "C", Jikanwari: "1学期: 水曜日３時限"},
		{KogiCd: "004", KogiNm: "D", Jikanwari: "前期: 木曜日４時限"},
		{KogiCd: "005", KogiNm: "E", Jikanwari: "2学期: 金曜日５時限"},
		{KogiCd: "006", KogiNm: "F", Jikanwari: "1学期前半: 土曜日１時限"},
		{KogiCd: "007", KogiNm: "G", Jikanwari: "2学期後半: 土曜日２時限"},
	}

	result := Convert(raw)
	want := []string{"1学期", "1学期前半", "2学期", "2学期後半", "通年", "前期", "後期"}

	if len(result.Data.Semesters) != len(want) {
		t.Fatalf("semesters len = %d, want %d", len(result.Data.Semesters), len(want))
	}
	for i, s := range result.Data.Semesters {
		if s != want[i] {
			t.Errorf("semesters[%d] = %s, want %s", i, s, want[i])
		}
	}
}

func TestConvertDuplicateSlotDedup(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限"},
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限"}, // exact same slot
	}

	result := Convert(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(result.Data.Courses))
	}
	if len(result.Data.Courses[0].Slots) != 1 {
		t.Errorf("slots len = %d, want 1 (should dedup identical slots)", len(result.Data.Courses[0].Slots))
	}
}

func TestConvertWarningMessageIncludesCourseContext(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "XYZ", KogiNm: "特殊講義", Jikanwari: "集中"},
	}

	result := Convert(raw)

	if len(result.Warnings) == 0 {
		t.Fatal("expected warnings")
	}
	// Warning should include course code and name for user context
	w := strings.Join(result.Warnings, "\n")
	if !strings.Contains(w, "XYZ") {
		t.Errorf("warning should include kogiCd 'XYZ': %s", w)
	}
	if !strings.Contains(w, "特殊講義") {
		t.Errorf("warning should include kogiNm '特殊講義': %s", w)
	}
}

func TestConvertTrimsStringFields(t *testing.T) {
	subtitle := "　副題テスト　"
	gakka := "理工学部　"
	raw := []model.RawCourse{
		{
			KogiCd:           "001  ",
			KogiNm:           "　基礎数学　",
			Fukudai:          &subtitle,
			TantoKyoin:       "山田 太郎　　",
			Jikanwari:        "1学期: 月曜日１時限",
			KogiKaikojikiNm:  "2026年度　",
			KogiKubunNm:      "　講義",
			SekininBushoNm:   "理工学部　　",
			KochiNm:          "朝倉キャンパス　",
			GakusokuKamokuNm: "基礎数学  ",
			TaishoGakka:      &gakka,
		},
	}

	result := Convert(raw)
	c := result.Data.Courses[0]

	if c.KogiCd != "001" {
		t.Errorf("KogiCd not trimmed: %q", c.KogiCd)
	}
	if c.KogiNm != "基礎数学" {
		t.Errorf("KogiNm not trimmed: %q", c.KogiNm)
	}
	if c.Fukudai == nil || *c.Fukudai != "副題テスト" {
		t.Errorf("Fukudai not trimmed: %v", c.Fukudai)
	}
	if c.TantoKyoin != "山田 太郎" {
		t.Errorf("TantoKyoin not trimmed: %q", c.TantoKyoin)
	}
	if c.SekininBushoNm != "理工学部" {
		t.Errorf("SekininBushoNm not trimmed: %q", c.SekininBushoNm)
	}
	if c.KochiNm != "朝倉キャンパス" {
		t.Errorf("KochiNm not trimmed: %q", c.KochiNm)
	}
	if c.TaishoGakka == nil || *c.TaishoGakka != "理工学部" {
		t.Errorf("TaishoGakka not trimmed: %v", c.TaishoGakka)
	}

	// Department list should also be trimmed
	if len(result.Data.Departments) != 1 || result.Data.Departments[0] != "理工学部" {
		t.Errorf("Departments not trimmed: %v", result.Data.Departments)
	}
}

func TestConvertTrimPtrEmptyBecomesNil(t *testing.T) {
	spaces := "　　　"
	raw := []model.RawCourse{
		{
			KogiCd:    "001",
			KogiNm:    "テスト",
			Jikanwari: "1学期: 月曜日１時限",
			Fukudai:   &spaces, // all whitespace
		},
	}

	result := Convert(raw)
	c := result.Data.Courses[0]

	if c.Fukudai != nil {
		t.Errorf("Fukudai should be nil when trimmed to empty, got: %q", *c.Fukudai)
	}
}
