package transform

import (
	"encoding/base64"
	"encoding/binary"
	"fmt"
	"strings"
	"testing"

	"github.com/livec/gyakubiki-syllabus/internal/model"
)

func TestConvertV2Basic(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:           "001",
			KogiNm:           "基礎数学",
			TantoKyoin:       "山田 太郎",
			Jikanwari:        "1学期: 月曜日１時限",
			KogiKaikojikiNm:  "1学期",
			KogiKubunNm:      "講義",
			SekininBushoNm:   "理工学部",
			KochiNm:          "朝倉キャンパス",
			GakusokuKamokuNm: "基礎数学",
		},
		{
			KogiCd:           "002",
			KogiNm:           "政治学概論",
			TantoKyoin:       "小川 寛貴",
			Jikanwari:        "2学期: 火曜日２時限",
			KogiKaikojikiNm:  "2学期",
			KogiKubunNm:      "演習",
			SekininBushoNm:   "人文社会科学部",
			KochiNm:          "物部キャンパス",
			GakusokuKamokuNm: "政治学概論",
		},
	}

	result := ConvertV2(raw)

	if result.Data.Version != 2 {
		t.Errorf("version = %d, want 2", result.Data.Version)
	}
	if result.Data.TotalRaw != 2 {
		t.Errorf("totalRaw = %d, want 2", result.Data.TotalRaw)
	}
	if len(result.Data.Courses) != 2 {
		t.Fatalf("courses len = %d, want 2", len(result.Data.Courses))
	}
}

func TestConvertV2Dictionaries(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:          "001",
			KogiNm:          "A",
			Jikanwari:       "1学期: 月曜日１時限",
			KogiKaikojikiNm: "1学期",
			KogiKubunNm:     "講義",
			SekininBushoNm:  "理工学部",
			KochiNm:         "朝倉キャンパス",
		},
		{
			KogiCd:          "002",
			KogiNm:          "B",
			Jikanwari:       "2学期: 火曜日２時限",
			KogiKaikojikiNm: "2学期",
			KogiKubunNm:     "演習",
			SekininBushoNm:  "人文社会科学部",
			KochiNm:         "物部キャンパス",
		},
	}

	result := ConvertV2(raw)
	dicts := result.Data.Dicts

	// Semesters sorted by semesterOrder
	if len(dicts.Semesters) != 2 {
		t.Fatalf("semesters len = %d, want 2", len(dicts.Semesters))
	}
	if dicts.Semesters[0] != "1学期" || dicts.Semesters[1] != "2学期" {
		t.Errorf("semesters = %v, want [1学期 2学期]", dicts.Semesters)
	}

	// Departments sorted alphabetically
	if len(dicts.Departments) != 2 {
		t.Fatalf("departments len = %d, want 2", len(dicts.Departments))
	}

	// Campuses sorted by fixed order
	if len(dicts.Campuses) != 2 {
		t.Fatalf("campuses len = %d, want 2", len(dicts.Campuses))
	}
	if dicts.Campuses[0] != "朝倉キャンパス" || dicts.Campuses[1] != "物部キャンパス" {
		t.Errorf("campuses = %v, want [朝倉キャンパス 物部キャンパス]", dicts.Campuses)
	}

	// Kubun
	if len(dicts.Kubun) != 2 {
		t.Fatalf("kubun len = %d, want 2", len(dicts.Kubun))
	}

	// Kaikojiki
	if len(dicts.Kaikojiki) != 2 {
		t.Fatalf("kaikojiki len = %d, want 2", len(dicts.Kaikojiki))
	}
}

func TestConvertV2CampusSortOrder(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", KochiNm: "その他"},
		{KogiCd: "002", KogiNm: "B", KochiNm: "岡豊キャンパス"},
		{KogiCd: "003", KogiNm: "C", KochiNm: "朝倉キャンパス"},
		{KogiCd: "004", KogiNm: "D", KochiNm: "物部キャンパス"},
	}

	result := ConvertV2(raw)
	want := []string{"朝倉キャンパス", "物部キャンパス", "岡豊キャンパス", "その他"}

	if len(result.Data.Dicts.Campuses) != len(want) {
		t.Fatalf("campuses len = %d, want %d", len(result.Data.Dicts.Campuses), len(want))
	}
	for i, c := range result.Data.Dicts.Campuses {
		if c != want[i] {
			t.Errorf("campuses[%d] = %s, want %s", i, c, want[i])
		}
	}
}

func TestConvertV2CourseIndices(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:          "001",
			KogiNm:          "基礎数学",
			TantoKyoin:      "山田 太郎",
			Jikanwari:       "1学期: 月曜日１時限",
			KogiKaikojikiNm: "1学期",
			KogiKubunNm:     "講義",
			SekininBushoNm:  "理工学部",
			KochiNm:         "朝倉キャンパス",
		},
	}

	result := ConvertV2(raw)
	c := result.Data.Courses[0]
	dicts := result.Data.Dicts

	// Check that indices resolve to correct strings
	if dicts.Kaikojiki[c.Kaikojiki] != "1学期" {
		t.Errorf("kaikojiki index %d = %s, want 1学期", c.Kaikojiki, dicts.Kaikojiki[c.Kaikojiki])
	}
	if dicts.Kubun[c.Kubun] != "講義" {
		t.Errorf("kubun index %d = %s, want 講義", c.Kubun, dicts.Kubun[c.Kubun])
	}
	if dicts.Departments[c.Department] != "理工学部" {
		t.Errorf("dept index %d = %s, want 理工学部", c.Department, dicts.Departments[c.Department])
	}
	if dicts.Campuses[c.Campus] != "朝倉キャンパス" {
		t.Errorf("campus index %d = %s, want 朝倉キャンパス", c.Campus, dicts.Campuses[c.Campus])
	}
}

func TestConvertV2SlotIndices(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:    "001",
			KogiNm:    "A",
			Jikanwari: "1学期: 月曜日１時限, 2学期: 火曜日２時限",
		},
	}

	result := ConvertV2(raw)
	c := result.Data.Courses[0]
	dicts := result.Data.Dicts

	if len(c.Slots) != 2 {
		t.Fatalf("slots len = %d, want 2", len(c.Slots))
	}

	// First slot: 1学期, 月, 1
	s0 := c.Slots[0]
	if dicts.Semesters[s0.Semester] != "1学期" {
		t.Errorf("slot[0].semester idx %d = %s, want 1学期", s0.Semester, dicts.Semesters[s0.Semester])
	}
	if s0.Day != 0 { // 月=0
		t.Errorf("slot[0].day = %d, want 0 (月)", s0.Day)
	}
	if s0.Period != 1 {
		t.Errorf("slot[0].period = %d, want 1", s0.Period)
	}

	// Second slot: 2学期, 火, 2
	s1 := c.Slots[1]
	if dicts.Semesters[s1.Semester] != "2学期" {
		t.Errorf("slot[1].semester idx %d = %s, want 2学期", s1.Semester, dicts.Semesters[s1.Semester])
	}
	if s1.Day != 1 { // 火=1
		t.Errorf("slot[1].day = %d, want 1 (火)", s1.Day)
	}
	if s1.Period != 2 {
		t.Errorf("slot[1].period = %d, want 2", s1.Period)
	}
}

func TestConvertV2GakusokuNmDiff(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:           "001",
			KogiNm:           "基礎数学",
			GakusokuKamokuNm: "基礎数学", // same as KogiNm
		},
		{
			KogiCd:           "002",
			KogiNm:           "基礎数学A",
			GakusokuKamokuNm: "基礎数学", // different
		},
	}

	result := ConvertV2(raw)

	// Same → nil (omitted in JSON)
	if result.Data.Courses[0].GakusokuNm != nil {
		t.Errorf("courses[0].GakusokuNm should be nil, got %q", *result.Data.Courses[0].GakusokuNm)
	}

	// Different → preserved
	if result.Data.Courses[1].GakusokuNm == nil {
		t.Fatal("courses[1].GakusokuNm should not be nil")
	}
	if *result.Data.Courses[1].GakusokuNm != "基礎数学" {
		t.Errorf("courses[1].GakusokuNm = %q, want 基礎数学", *result.Data.Courses[1].GakusokuNm)
	}
}

func TestConvertV2SearchText(t *testing.T) {
	subtitle := "副題テスト"
	raw := []model.RawCourse{
		{
			KogiCd:         "ABC",
			KogiNm:         "English Communication",
			Fukudai:        &subtitle,
			TantoKyoin:     "Smith\u3000John",
			SekininBushoNm: "共通教育",
		},
	}

	result := ConvertV2(raw)
	st := result.Data.Courses[0].SearchText

	if strings.Contains(st, "English") {
		t.Errorf("SearchText should be lowercased: %q", st)
	}
	if !strings.Contains(st, "english communication") {
		t.Errorf("SearchText should contain lowered name: %q", st)
	}
	if strings.Contains(st, "\u3000") {
		t.Errorf("SearchText should not contain full-width spaces: %q", st)
	}
	if !strings.Contains(st, "副題テスト") {
		t.Errorf("SearchText should contain fukudai: %q", st)
	}
}

func TestConvertV2Dedup(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限"},
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限, 1学期: 水曜日３時限"},
	}

	result := ConvertV2(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1 (dedup)", len(result.Data.Courses))
	}
	if len(result.Data.Courses[0].Slots) != 2 {
		t.Errorf("slots len = %d, want 2 (merged)", len(result.Data.Courses[0].Slots))
	}
}

func TestConvertV2SkipsEmptyKogiCd(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "", KogiNm: "空コード"},
		{KogiCd: "001", KogiNm: "正常"},
	}

	result := ConvertV2(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(result.Data.Courses))
	}
	if len(result.Warnings) == 0 {
		t.Error("expected warning for empty kogiCd")
	}
}

func TestConvertV2Empty(t *testing.T) {
	result := ConvertV2(nil)
	if len(result.Data.Courses) != 0 {
		t.Errorf("expected empty courses, got %d", len(result.Data.Courses))
	}
	if result.Data.Version != 2 {
		t.Errorf("version = %d, want 2", result.Data.Version)
	}
}

// decodeBitset decodes a base64 bitset into a []uint64 for testing.
func decodeBitset(encoded string) ([]uint64, error) {
	data, err := base64.StdEncoding.DecodeString(encoded)
	if err != nil {
		return nil, err
	}
	words := make([]uint64, len(data)/8)
	for i := range words {
		words[i] = binary.LittleEndian.Uint64(data[i*8 : (i+1)*8])
	}
	return words, nil
}

func TestConvertV2BitsetIndices(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限", SekininBushoNm: "理工学部", KochiNm: "朝倉キャンパス"},
		{KogiCd: "002", KogiNm: "B", Jikanwari: "2学期: 火曜日２時限", SekininBushoNm: "人文社会科学部", KochiNm: "物部キャンパス"},
		{KogiCd: "003", KogiNm: "C", Jikanwari: "1学期: 水曜日３時限", SekininBushoNm: "理工学部", KochiNm: "朝倉キャンパス"},
	}

	result := ConvertV2(raw)
	indices := result.Data.Indices

	// Find semester indices
	semIdx := -1
	for i, s := range result.Data.Dicts.Semesters {
		if s == "1学期" {
			semIdx = i
			break
		}
	}
	if semIdx == -1 {
		t.Fatal("1学期 not found in semesters dict")
	}

	// Decode the bitset for 1学期
	bs, err := decodeBitset(indices.Semester[fmt.Sprintf("%d", semIdx)])
	if err != nil {
		t.Fatalf("decode bitset: %v", err)
	}

	// Courses 0 and 2 should be set (1学期), course 1 should not
	if bs[0]&(1<<0) == 0 {
		t.Error("course 0 should be in 1学期 bitset")
	}
	if bs[0]&(1<<1) != 0 {
		t.Error("course 1 should NOT be in 1学期 bitset")
	}
	if bs[0]&(1<<2) == 0 {
		t.Error("course 2 should be in 1学期 bitset")
	}

	// Check campus bitset
	campIdx := -1
	for i, c := range result.Data.Dicts.Campuses {
		if c == "朝倉キャンパス" {
			campIdx = i
			break
		}
	}
	campBs, err := decodeBitset(indices.Campus[fmt.Sprintf("%d", campIdx)])
	if err != nil {
		t.Fatalf("decode campus bitset: %v", err)
	}
	if campBs[0]&(1<<0) == 0 {
		t.Error("course 0 should be in 朝倉 campus bitset")
	}
	if campBs[0]&(1<<1) != 0 {
		t.Error("course 1 should NOT be in 朝倉 campus bitset")
	}
}

func TestConvertV2TsuunenInSemesterBitsets(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限"},
		{KogiCd: "002", KogiNm: "B", Jikanwari: "通年: 火曜日２時限"},
		{KogiCd: "003", KogiNm: "C", Jikanwari: "2学期: 水曜日３時限"},
	}

	result := ConvertV2(raw)

	// Find 1学期 index
	semIdx := -1
	for i, s := range result.Data.Dicts.Semesters {
		if s == "1学期" {
			semIdx = i
			break
		}
	}
	if semIdx == -1 {
		t.Fatal("1学期 not found")
	}

	bs, err := decodeBitset(result.Data.Indices.Semester[fmt.Sprintf("%d", semIdx)])
	if err != nil {
		t.Fatal(err)
	}

	// 通年 course (002) should appear in both 1学期 and 2学期 bitsets
	if bs[0]&(1<<0) == 0 {
		t.Error("course 0 (1学期) should be in 1学期 bitset")
	}
	if bs[0]&(1<<1) == 0 {
		t.Error("course 1 (通年) should be in 1学期 bitset")
	}
	if bs[0]&(1<<2) != 0 {
		t.Error("course 2 (2学期) should NOT be in 1学期 bitset")
	}
}

func TestConvertV2PreservesJikanwariRaw(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "集中講義", Jikanwari: "1学期: 集中講義"},
	}

	result := ConvertV2(raw)
	if result.Data.Courses[0].JikanwariRaw != "1学期: 集中講義" {
		t.Errorf("JikanwariRaw = %q, want %q", result.Data.Courses[0].JikanwariRaw, "1学期: 集中講義")
	}
}

func TestConvertV2FieldNameShortened(t *testing.T) {
	raw := []model.RawCourse{
		{
			KogiCd:     "001",
			KogiNm:     "テスト",
			TantoKyoin: "教員",
		},
	}

	result := ConvertV2(raw)
	c := result.Data.Courses[0]

	if c.KogiCd != "001" {
		t.Errorf("KogiCd = %q", c.KogiCd)
	}
	if c.KogiNm != "テスト" {
		t.Errorf("KogiNm = %q", c.KogiNm)
	}
	if c.TantoKyoin != "教員" {
		t.Errorf("TantoKyoin = %q", c.TantoKyoin)
	}
}

func TestConvertV2UnknownCampusSortedLast(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", KochiNm: "未知のキャンパスB"},
		{KogiCd: "002", KogiNm: "B", KochiNm: "未知のキャンパスA"},
		{KogiCd: "003", KogiNm: "C", KochiNm: "朝倉キャンパス"},
	}

	result := ConvertV2(raw)
	campuses := result.Data.Dicts.Campuses

	// 朝倉 first, then unknown sorted alphabetically
	if campuses[0] != "朝倉キャンパス" {
		t.Errorf("campuses[0] = %s, want 朝倉キャンパス", campuses[0])
	}
	if campuses[1] != "未知のキャンパスA" {
		t.Errorf("campuses[1] = %s, want 未知のキャンパスA", campuses[1])
	}
	if campuses[2] != "未知のキャンパスB" {
		t.Errorf("campuses[2] = %s, want 未知のキャンパスB", campuses[2])
	}
}

func TestConvertV2EmptyCampus(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", KochiNm: ""},
		{KogiCd: "002", KogiNm: "B", KochiNm: "朝倉キャンパス"},
	}

	result := ConvertV2(raw)

	// Empty campus should still work (mapped to "" in dicts)
	if len(result.Data.Courses) != 2 {
		t.Fatalf("courses len = %d, want 2", len(result.Data.Courses))
	}
}

func TestConvertV2UnknownSemesterSortedLast(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: "1学期: 月曜日１時限"},
		{KogiCd: "002", KogiNm: "B", Jikanwari: "特殊学期B: 火曜日２時限"},
		{KogiCd: "003", KogiNm: "C", Jikanwari: "特殊学期A: 水曜日３時限"},
	}

	result := ConvertV2(raw)
	semesters := result.Data.Dicts.Semesters

	// 1学期 should come before unknown semesters
	if len(semesters) < 3 {
		t.Fatalf("semesters len = %d, want >= 3", len(semesters))
	}
	if semesters[0] != "1学期" {
		t.Errorf("semesters[0] = %s, want 1学期", semesters[0])
	}
	// Both unknown semesters should be present after known ones
	unknowns := semesters[1:]
	hasA := false
	hasB := false
	for _, s := range unknowns {
		if s == "特殊学期A" {
			hasA = true
		}
		if s == "特殊学期B" {
			hasB = true
		}
	}
	if !hasA || !hasB {
		t.Errorf("unknown semesters not found: %v", unknowns)
	}
}

func TestConvertV2WarnsUnparseableJikanwari(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "集中講義", Jikanwari: "集中"},
	}

	result := ConvertV2(raw)

	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(result.Data.Courses))
	}
	if len(result.Warnings) == 0 {
		t.Error("expected warnings for unparseable jikanwari")
	}
}

func TestConvertV2WarnsEmptyKogiNm(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: ""},
	}

	result := ConvertV2(raw)

	if len(result.Warnings) == 0 {
		t.Error("expected warning for empty kogiNm")
	}
}

func TestConvertV2NoJikanwariNoCrash(t *testing.T) {
	raw := []model.RawCourse{
		{KogiCd: "001", KogiNm: "A", Jikanwari: ""},
	}

	result := ConvertV2(raw)
	if len(result.Data.Courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(result.Data.Courses))
	}
	if len(result.Data.Courses[0].Slots) != 0 {
		t.Errorf("slots len = %d, want 0", len(result.Data.Courses[0].Slots))
	}
	if len(result.Warnings) != 0 {
		t.Errorf("no warnings expected for empty jikanwari, got %d", len(result.Warnings))
	}
}

func TestConvertV2PreservesOptionalFields(t *testing.T) {
	gakka := "理工学部"
	nenji := "1年"
	bunrui := "専門"
	bunya := "数学"
	raw := []model.RawCourse{
		{
			KogiCd:      "001",
			KogiNm:      "テスト",
			TaishoGakka: &gakka,
			TaishoNenji: &nenji,
			KamokuBunrui: &bunrui,
			KamokuBunya: &bunya,
		},
	}

	result := ConvertV2(raw)
	c := result.Data.Courses[0]

	if c.TaishoGakka == nil || *c.TaishoGakka != "理工学部" {
		t.Error("TaishoGakka not preserved")
	}
	if c.TaishoNenji == nil || *c.TaishoNenji != "1年" {
		t.Error("TaishoNenji not preserved")
	}
	if c.KamokuBunrui == nil || *c.KamokuBunrui != "専門" {
		t.Error("KamokuBunrui not preserved")
	}
	if c.KamokuBunya == nil || *c.KamokuBunya != "数学" {
		t.Error("KamokuBunya not preserved")
	}
}
