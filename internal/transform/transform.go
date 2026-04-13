package transform

import (
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/livec/gyakubiki-syllabus/internal/model"
	"github.com/livec/gyakubiki-syllabus/internal/parser"
)

var semesterOrder = map[string]int{
	"1学期":    0,
	"1学期前半": 1,
	"1学期後半": 2,
	"2学期":    3,
	"2学期前半": 4,
	"2学期後半": 5,
	"通年":     6,
	"前期":     7,
	"後期":     8,
}

// buildSearchText creates a pre-normalized search haystack for a course.
// This mirrors the frontend normalize() logic: full-width space → half-width, lowercase.
func buildSearchText(c model.Course) string {
	parts := []string{c.KogiNm}
	if c.Fukudai != nil {
		parts = append(parts, *c.Fukudai)
	}
	parts = append(parts, c.TantoKyoin, c.KogiCd, c.SekininBushoNm)
	joined := strings.Join(parts, " ")
	return strings.ToLower(strings.ReplaceAll(joined, "\u3000", " "))
}

// ConvertResult holds the processed data and any warnings encountered during conversion.
type ConvertResult struct {
	Data     model.ProcessedData
	Warnings []string
}

// Convert transforms raw courses into the processed output format.
// Warnings are collected for issues that don't prevent conversion but may indicate data problems.
func Convert(raw []model.RawCourse) ConvertResult {
	seen := make(map[string]int) // kogiCd → index in courses
	var courses []model.Course
	var warnings []string
	semesterSet := make(map[string]struct{})
	deptSet := make(map[string]struct{})

	for i, r := range raw {
		// Validate required fields
		if strings.TrimSpace(r.KogiCd) == "" {
			warnings = append(warnings,
				fmt.Sprintf("  [%d件目] 授業コード(kogiCd)が空です。スキップします", i+1))
			continue
		}
		if strings.TrimSpace(r.KogiNm) == "" {
			warnings = append(warnings,
				fmt.Sprintf("  [%s] 科目名(kogiNm)が空です", r.KogiCd))
		}

		parsed := parser.ParseJikanwari(r.Jikanwari)

		// Propagate parser warnings with course context
		for _, w := range parsed.Warnings {
			warnings = append(warnings,
				fmt.Sprintf("  [%s] %s: %s", r.KogiCd, r.KogiNm, w))
		}

		if r.Jikanwari != "" && len(parsed.Slots) == 0 {
			warnings = append(warnings,
				fmt.Sprintf("  [%s] %s: 時間割情報がありますがパースできませんでした: %q", r.KogiCd, r.KogiNm, r.Jikanwari))
		}

		for _, s := range parsed.Slots {
			if s.Semester != "" {
				semesterSet[s.Semester] = struct{}{}
			}
		}

		if dept := strings.TrimSpace(r.SekininBushoNm); dept != "" {
			deptSet[dept] = struct{}{}
		}

		if idx, ok := seen[r.KogiCd]; ok {
			// Merge slots (deduplicate)
			existing := courses[idx].Slots
			for _, s := range parsed.Slots {
				if !containsSlot(existing, s) {
					existing = append(existing, s)
				}
			}
			courses[idx].Slots = existing
			continue
		}

		seen[r.KogiCd] = len(courses)
		c := model.Course{
			KogiCd:           strings.TrimSpace(r.KogiCd),
			KogiNm:           strings.TrimSpace(r.KogiNm),
			Fukudai:          trimPtr(r.Fukudai),
			TantoKyoin:       strings.TrimSpace(r.TantoKyoin),
			JikanwariRaw:     r.Jikanwari,
			Slots:            ensureSlots(parsed.Slots),
			KogiKaikojikiNm:  strings.TrimSpace(r.KogiKaikojikiNm),
			KogiKubunNm:      strings.TrimSpace(r.KogiKubunNm),
			SekininBushoNm:   strings.TrimSpace(r.SekininBushoNm),
			KochiNm:          strings.TrimSpace(r.KochiNm),
			GakusokuKamokuNm: strings.TrimSpace(r.GakusokuKamokuNm),
			TaishoGakka:      trimPtr(r.TaishoGakka),
			TaishoNenji:      trimPtr(r.TaishoNenji),
			KamokuBunrui:     trimPtr(r.KamokuBunrui),
			KamokuBunya:      trimPtr(r.KamokuBunya),
		}
		c.SearchText = buildSearchText(c)
		courses = append(courses, c)
	}

	return ConvertResult{
		Data: model.ProcessedData{
			Version:     1,
			GeneratedAt: time.Now(),
			TotalRaw:    len(raw),
			Courses:     courses,
			Semesters:   sortSemesters(semesterSet),
			Departments: sortDepartments(deptSet),
		},
		Warnings: warnings,
	}
}

// trimPtr trims a *string, returning nil if the result is empty.
func trimPtr(s *string) *string {
	if s == nil {
		return nil
	}
	trimmed := strings.TrimSpace(*s)
	if trimmed == "" {
		return nil
	}
	return &trimmed
}

// ensureSlots guarantees a non-nil slice so JSON outputs [] instead of null.
func ensureSlots(slots []model.Slot) []model.Slot {
	if slots == nil {
		return []model.Slot{}
	}
	return slots
}

func containsSlot(slots []model.Slot, s model.Slot) bool {
	for _, existing := range slots {
		if existing == s {
			return true
		}
	}
	return false
}

func sortSemesters(set map[string]struct{}) []string {
	semesters := make([]string, 0, len(set))
	for s := range set {
		semesters = append(semesters, s)
	}
	sort.Slice(semesters, func(i, j int) bool {
		oi, oki := semesterOrder[semesters[i]]
		oj, okj := semesterOrder[semesters[j]]
		if !oki {
			oi = 99
		}
		if !okj {
			oj = 99
		}
		return oi < oj
	})
	return semesters
}

func sortDepartments(set map[string]struct{}) []string {
	depts := make([]string, 0, len(set))
	for d := range set {
		depts = append(depts, d)
	}
	sort.Strings(depts)
	return depts
}
