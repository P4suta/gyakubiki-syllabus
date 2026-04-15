package transform

import (
	"encoding/base64"
	"encoding/binary"
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/livec/gyakubiki-syllabus/internal/model"
	"github.com/livec/gyakubiki-syllabus/internal/parser"
)

var campusOrder = map[string]int{
	"朝倉キャンパス": 0,
	"物部キャンパス": 1,
	"岡豊キャンパス": 2,
	"その他":       3,
}

var dayIndex = map[string]int{
	"月": 0,
	"火": 1,
	"水": 2,
	"木": 3,
	"金": 4,
	"土": 5,
	"日": 6,
}

// ConvertV2Result holds the v2 processed data and any warnings.
type ConvertV2Result struct {
	Data     model.ProcessedDataV2
	Warnings []string
}

// ConvertV2 transforms raw courses into the optimized v2 output format.
func ConvertV2(raw []model.RawCourse) ConvertV2Result {
	seen := make(map[string]int) // kogiCd → index in courses
	var courses []model.CourseV2
	var warnings []string

	// Collect sets for dictionaries
	semesterSet := make(map[string]struct{})
	deptSet := make(map[string]struct{})
	campusSet := make(map[string]struct{})
	kubunSet := make(map[string]struct{})
	kaikojikiSet := make(map[string]struct{})

	// First pass: collect all unique values and build courses
	type rawSlots struct {
		parsed []model.Slot
	}
	var allRawSlots []rawSlots

	for i, r := range raw {
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

		dept := strings.TrimSpace(r.SekininBushoNm)
		if dept != "" {
			deptSet[dept] = struct{}{}
		}
		campus := strings.TrimSpace(r.KochiNm)
		campusSet[campus] = struct{}{}
		kubun := strings.TrimSpace(r.KogiKubunNm)
		if kubun != "" {
			kubunSet[kubun] = struct{}{}
		}
		kaikojiki := strings.TrimSpace(r.KogiKaikojikiNm)
		if kaikojiki != "" {
			kaikojikiSet[kaikojiki] = struct{}{}
		}

		if idx, ok := seen[r.KogiCd]; ok {
			// Merge slots
			existing := allRawSlots[idx].parsed
			for _, s := range parsed.Slots {
				if !containsSlot(existing, s) {
					existing = append(existing, s)
				}
			}
			allRawSlots[idx] = rawSlots{parsed: existing}
			continue
		}

		seen[r.KogiCd] = len(courses)

		kogiNm := strings.TrimSpace(r.KogiNm)
		gakusokuNm := strings.TrimSpace(r.GakusokuKamokuNm)

		var gakusokuPtr *string
		if gakusokuNm != "" && gakusokuNm != kogiNm {
			gakusokuPtr = &gakusokuNm
		}

		c := model.CourseV2{
			KogiCd:       strings.TrimSpace(r.KogiCd),
			KogiNm:       kogiNm,
			Fukudai:      trimPtr(r.Fukudai),
			TantoKyoin:   strings.TrimSpace(r.TantoKyoin),
			JikanwariRaw: r.Jikanwari,
			// Slots, indices set in second pass
			GakusokuNm:   gakusokuPtr,
			TaishoGakka:  trimPtr(r.TaishoGakka),
			TaishoNenji:  trimPtr(r.TaishoNenji),
			KamokuBunrui: trimPtr(r.KamokuBunrui),
			KamokuBunya:  trimPtr(r.KamokuBunya),
		}
		c.SearchText = buildSearchTextV2(c, dept)
		courses = append(courses, c)
		allRawSlots = append(allRawSlots, rawSlots{parsed: ensureSlots(parsed.Slots)})
	}

	// Build sorted dictionaries
	semesters := sortSemesters(semesterSet)
	departments := sortDepartments(deptSet)
	campuses := sortCampuses(campusSet)
	kubun := sortKubun(kubunSet)
	kaikojiki := sortKaikojiki(kaikojikiSet)

	// Build lookup maps for second pass
	semesterIdx := indexMap(semesters)
	deptIdx := indexMap(departments)
	campusIdx := indexMap(campuses)
	kubunIdx := indexMap(kubun)
	kaikojikiIdx := indexMap(kaikojiki)

	// Second pass: set indices and convert slots
	n := len(courses)
	numWords := (n + 63) / 64

	// Bitset arrays for each filter value
	semBitsets := make(map[int][]uint64)
	deptBitsets := make(map[int][]uint64)
	campBitsets := make(map[int][]uint64)

	// Track 通年 courses for semester bitset propagation
	var tsuunenCourseIndices []int

	// Build kogiCd → first raw index map for O(1) lookup
	rawIdxMap := make(map[string]int, len(raw))
	for i, r := range raw {
		cd := strings.TrimSpace(r.KogiCd)
		if _, exists := rawIdxMap[cd]; !exists {
			rawIdxMap[cd] = i
		}
	}

	for i := range courses {
		r := raw[rawIdxMap[courses[i].KogiCd]]
		dept := strings.TrimSpace(r.SekininBushoNm)
		campus := strings.TrimSpace(r.KochiNm)
		kubunStr := strings.TrimSpace(r.KogiKubunNm)
		kaikojikiStr := strings.TrimSpace(r.KogiKaikojikiNm)

		courses[i].Department = lookupIndex(deptIdx, dept)
		courses[i].Campus = lookupIndex(campusIdx, campus)
		courses[i].Kubun = lookupIndex(kubunIdx, kubunStr)
		courses[i].Kaikojiki = lookupIndex(kaikojikiIdx, kaikojikiStr)

		// Convert slots from string-based to index-based
		var slotsV2 []model.SlotV2
		hasTsuunen := false
		for _, s := range allRawSlots[i].parsed {
			si, ok := semesterIdx[s.Semester]
			if !ok {
				continue
			}
			di, ok := dayIndex[s.Day]
			if !ok {
				continue
			}
			slotsV2 = append(slotsV2, model.SlotV2{
				Semester: si,
				Day:      di,
				Period:   s.Period,
			})

			// Add to semester bitset
			setBit(semBitsets, si, i, numWords)

			if s.Semester == "通年" {
				hasTsuunen = true
			}
		}
		if slotsV2 == nil {
			slotsV2 = []model.SlotV2{}
		}
		courses[i].Slots = slotsV2

		if hasTsuunen {
			tsuunenCourseIndices = append(tsuunenCourseIndices, i)
		}

		// Department bitset
		setBit(deptBitsets, courses[i].Department, i, numWords)

		// Campus bitset
		setBit(campBitsets, courses[i].Campus, i, numWords)
	}

	// Propagate 通年 courses to all non-通年 semester bitsets
	tsuunenSemIdx, hasTsuunenSem := semesterIdx["通年"]
	if hasTsuunenSem {
		for semI := range semBitsets {
			if semI == tsuunenSemIdx {
				continue
			}
			for _, courseI := range tsuunenCourseIndices {
				setBit(semBitsets, semI, courseI, numWords)
			}
		}
	}

	// Encode bitsets to base64
	indices := model.IndicesMap{
		Semester:   encodeBitsets(semBitsets),
		Department: encodeBitsets(deptBitsets),
		Campus:     encodeBitsets(campBitsets),
	}

	return ConvertV2Result{
		Data: model.ProcessedDataV2{
			Version:     2,
			GeneratedAt: time.Now(),
			TotalRaw:    len(raw),
			Dicts: model.Dictionaries{
				Semesters:   semesters,
				Departments: departments,
				Campuses:    campuses,
				Kubun:       kubun,
				Kaikojiki:   kaikojiki,
			},
			Indices: indices,
			Courses: courses,
		},
		Warnings: warnings,
	}
}

// buildSearchTextV2 creates a normalized search haystack for a v2 course.
func buildSearchTextV2(c model.CourseV2, dept string) string {
	parts := []string{c.KogiNm}
	if c.Fukudai != nil {
		parts = append(parts, *c.Fukudai)
	}
	parts = append(parts, c.TantoKyoin, c.KogiCd, dept)
	joined := strings.Join(parts, " ")
	return strings.ToLower(strings.ReplaceAll(joined, "\u3000", " "))
}

func sortCampuses(set map[string]struct{}) []string {
	campuses := make([]string, 0, len(set))
	for c := range set {
		campuses = append(campuses, c)
	}
	sort.Slice(campuses, func(i, j int) bool {
		oi, oki := campusOrder[campuses[i]]
		oj, okj := campusOrder[campuses[j]]
		if !oki {
			oi = 99
		}
		if !okj {
			oj = 99
		}
		if oi != oj {
			return oi < oj
		}
		return campuses[i] < campuses[j]
	})
	return campuses
}

func sortKubun(set map[string]struct{}) []string {
	kubun := make([]string, 0, len(set))
	for k := range set {
		kubun = append(kubun, k)
	}
	sort.Strings(kubun)
	return kubun
}

func sortKaikojiki(set map[string]struct{}) []string {
	return sortSemesters(set) // same ordering
}

func indexMap(strs []string) map[string]int {
	m := make(map[string]int, len(strs))
	for i, s := range strs {
		m[s] = i
	}
	return m
}

func lookupIndex(m map[string]int, key string) int {
	if idx, ok := m[key]; ok {
		return idx
	}
	return 0
}

// setBit sets bit courseIdx in the bitset for dictIdx.
func setBit(bitsets map[int][]uint64, dictIdx, courseIdx, numWords int) {
	if _, ok := bitsets[dictIdx]; !ok {
		bitsets[dictIdx] = make([]uint64, numWords)
	}
	bitsets[dictIdx][courseIdx/64] |= 1 << uint(courseIdx%64)
}


func encodeBitsets(bitsets map[int][]uint64) map[string]string {
	result := make(map[string]string, len(bitsets))
	for dictIdx, words := range bitsets {
		data := make([]byte, len(words)*8)
		for i, w := range words {
			binary.LittleEndian.PutUint64(data[i*8:(i+1)*8], w)
		}
		result[fmt.Sprintf("%d", dictIdx)] = base64.StdEncoding.EncodeToString(data)
	}
	return result
}
