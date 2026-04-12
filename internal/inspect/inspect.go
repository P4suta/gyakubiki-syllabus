package inspect

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"github.com/livec/gyakubiki-syllabus/internal/parser"
)

// Report holds the full inspection result.
type Report struct {
	FileName string `json:"fileName"`
	FileSize int64  `json:"fileSize"`
	Format   string `json:"format"` // "APIレスポンス" | "配列"

	// Pagination (API response only)
	PageNo    *int `json:"pageNo,omitempty"`
	MaxPageNo *int `json:"maxPageNo,omitempty"`
	Total     *int `json:"total,omitempty"`
	PageSize  *int `json:"pageSize,omitempty"`

	// Field inspection
	TotalFields   int         `json:"totalFields"`
	CourseCount   int         `json:"courseCount"`
	FieldCoverage []FieldStat `json:"fieldCoverage"`

	// Data summary
	UniqueKogiCd int      `json:"uniqueKogiCd"`
	Semesters    []string `json:"semesters"`
	Departments  []string `json:"departments"`
	KogiKubuns   []string `json:"kogiKubuns"`
	Campuses     []string `json:"campuses"`

	// Parse trial
	ParseSuccess  int            `json:"parseSuccess"`
	ParseFailures []ParseFailure `json:"parseFailures"`

	// Verdict
	CanConvert bool     `json:"canConvert"`
	Warnings   []string `json:"warnings"`
}

// FieldStat holds fill rate for a single field.
type FieldStat struct {
	Name   string `json:"name"`
	Filled int    `json:"filled"`
	Total  int    `json:"total"`
}

// ParseFailure holds info about a course whose jikanwari couldn't be parsed.
type ParseFailure struct {
	KogiCd  string `json:"kogiCd"`
	KogiNm  string `json:"kogiNm"`
	Message string `json:"message"`
}

// usedFields is the ordered list of fields that convert uses.
var usedFields = []string{
	"kogiCd", "kogiNm", "fukudai", "tantoKyoin", "jikanwari",
	"kogiKaikojikiNm", "kogiKubunNm", "sekininBushoNm", "kochiNm",
	"gakusokuKamokuNm", "taishoGakka", "taishoNenji", "kamokuBunrui", "kamokuBunya",
}

// Inspect analyzes raw JSON data and returns a report.
func Inspect(data []byte, fileName string, fileSize int64) (*Report, error) {
	data = []byte(strings.TrimSpace(string(data)))
	if len(data) == 0 {
		return nil, fmt.Errorf("入力が空です")
	}

	if data[0] == '<' {
		return nil, fmt.Errorf("HTMLが入力されました。JSONデータを指定してください")
	}

	report := &Report{
		FileName: fileName,
		FileSize: fileSize,
	}

	courses, err := extractCourses(data, report)
	if err != nil {
		return nil, err
	}

	report.CourseCount = len(courses)

	if len(courses) == 0 {
		report.CanConvert = true
		report.Warnings = append(report.Warnings, "講義データが0件です")
		return report, nil
	}

	// Field inspection
	report.TotalFields = len(courses[0])
	report.FieldCoverage = computeFieldCoverage(courses)

	// Data summary
	computeSummary(courses, report)

	// Parse trial
	computeParseTrial(courses, report)

	// Pagination warning
	if report.PageNo != nil && report.MaxPageNo != nil && *report.MaxPageNo > 1 {
		pct := float64(len(courses)) / float64(*report.Total) * 100
		report.Warnings = append(report.Warnings,
			fmt.Sprintf("このファイルは全体の%.1f%%です (%d/%dページ)。残り%dページのデータが含まれていません",
				pct, *report.PageNo, *report.MaxPageNo, *report.MaxPageNo-1))
	}

	report.CanConvert = true
	return report, nil
}

func extractCourses(data []byte, report *Report) ([]map[string]interface{}, error) {
	// Try API response format
	var apiResp map[string]json.RawMessage
	if err := json.Unmarshal(data, &apiResp); err == nil {
		if listRaw, ok := apiResp["selectKogiDtoList"]; ok {
			report.Format = "APIレスポンス"

			// Extract pagination
			report.PageNo = extractInt(apiResp, "pageNo")
			report.MaxPageNo = extractInt(apiResp, "maxPageNo")
			report.Total = extractInt(apiResp, "total")
			report.PageSize = extractInt(apiResp, "pageSize")

			var courses []map[string]interface{}
			if err := json.Unmarshal(listRaw, &courses); err != nil {
				return nil, fmt.Errorf("selectKogiDtoListのパースに失敗: %w", err)
			}
			return courses, nil
		}
	}

	// Try bare array
	var courses []map[string]interface{}
	if err := json.Unmarshal(data, &courses); err == nil {
		report.Format = "配列"
		return courses, nil
	}

	return nil, fmt.Errorf("JSONの解析に失敗しました。APIレスポンスまたは配列が必要です")
}

func extractInt(m map[string]json.RawMessage, key string) *int {
	raw, ok := m[key]
	if !ok {
		return nil
	}
	var v int
	if err := json.Unmarshal(raw, &v); err != nil {
		return nil
	}
	return &v
}

func computeFieldCoverage(courses []map[string]interface{}) []FieldStat {
	total := len(courses)
	stats := make([]FieldStat, len(usedFields))

	for i, field := range usedFields {
		filled := 0
		for _, c := range courses {
			v, exists := c[field]
			if exists && v != nil {
				// Also check for empty string
				if s, ok := v.(string); ok && s == "" {
					continue
				}
				filled++
			}
		}
		stats[i] = FieldStat{Name: field, Filled: filled, Total: total}
	}

	return stats
}

func computeSummary(courses []map[string]interface{}, report *Report) {
	kogiCdSet := make(map[string]struct{})
	semesterSet := make(map[string]struct{})
	deptSet := make(map[string]struct{})
	kubunSet := make(map[string]struct{})
	campusSet := make(map[string]struct{})

	for _, c := range courses {
		if cd, ok := getString(c, "kogiCd"); ok {
			kogiCdSet[cd] = struct{}{}
		}
		if jikanwari, ok := getString(c, "jikanwari"); ok && jikanwari != "" {
			result := parser.ParseJikanwari(jikanwari)
			for _, s := range result.Slots {
				if s.Semester != "" {
					semesterSet[s.Semester] = struct{}{}
				}
			}
		}
		if dept, ok := getString(c, "sekininBushoNm"); ok && dept != "" {
			deptSet[dept] = struct{}{}
		}
		if kubun, ok := getString(c, "kogiKubunNm"); ok && kubun != "" {
			kubunSet[kubun] = struct{}{}
		}
		if campus, ok := getString(c, "kochiNm"); ok && campus != "" {
			campusSet[campus] = struct{}{}
		}
	}

	report.UniqueKogiCd = len(kogiCdSet)
	report.Semesters = sortedKeys(semesterSet)
	report.Departments = sortedKeys(deptSet)
	report.KogiKubuns = sortedKeys(kubunSet)
	report.Campuses = sortedKeys(campusSet)
}

func computeParseTrial(courses []map[string]interface{}, report *Report) {
	for _, c := range courses {
		jikanwari, ok := getString(c, "jikanwari")
		if !ok || jikanwari == "" {
			continue // Not a failure, just no schedule
		}

		result := parser.ParseJikanwari(jikanwari)
		if len(result.Slots) > 0 {
			report.ParseSuccess++
		} else {
			kogiCd, _ := getString(c, "kogiCd")
			kogiNm, _ := getString(c, "kogiNm")
			msg := strings.Join(result.Warnings, "; ")
			if msg == "" {
				msg = "パース結果が空です"
			}
			report.ParseFailures = append(report.ParseFailures, ParseFailure{
				KogiCd:  kogiCd,
				KogiNm:  kogiNm,
				Message: msg,
			})
		}
	}
}

func getString(m map[string]interface{}, key string) (string, bool) {
	v, exists := m[key]
	if !exists || v == nil {
		return "", false
	}
	s, ok := v.(string)
	return s, ok
}

func sortedKeys(set map[string]struct{}) []string {
	keys := make([]string, 0, len(set))
	for k := range set {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
