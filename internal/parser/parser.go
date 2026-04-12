package parser

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/livec/gyakubiki-syllabus/internal/model"
)

var (
	semesterRe = regexp.MustCompile(`^(.*?):\s*`)
	dayRe      = regexp.MustCompile(`(月|火|水|木|金|土|日)曜日`)
	periodRe   = regexp.MustCompile(`([１２３４５６７８])時限`)
)

var fullWidthToInt = map[rune]int{
	'１': 1, '２': 2, '３': 3, '４': 4,
	'５': 5, '６': 6, '７': 7, '８': 8,
}

// ParseResult holds parsed slots and any warnings from unparseable parts.
type ParseResult struct {
	Slots    []model.Slot
	Warnings []string
}

// ParseJikanwari parses a jikanwari string into a slice of Slots.
// Unparseable parts are collected as warnings rather than silently dropped.
func ParseJikanwari(jikanwari string) ParseResult {
	if jikanwari == "" {
		return ParseResult{}
	}

	parts := strings.Split(jikanwari, ",")
	var result ParseResult

	for _, part := range parts {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}

		var semester string
		rest := part

		if m := semesterRe.FindStringSubmatch(part); m != nil {
			semester = strings.TrimSpace(m[1])
			rest = part[len(m[0]):]
		}

		dayMatch := dayRe.FindStringSubmatch(rest)
		periodMatch := periodRe.FindStringSubmatch(rest)

		if dayMatch == nil && periodMatch == nil {
			result.Warnings = append(result.Warnings,
				fmt.Sprintf("曜日・時限が見つかりません: %q", part))
			continue
		}
		if dayMatch == nil {
			result.Warnings = append(result.Warnings,
				fmt.Sprintf("曜日が見つかりません: %q", part))
			continue
		}
		if periodMatch == nil {
			result.Warnings = append(result.Warnings,
				fmt.Sprintf("時限が見つかりません: %q", part))
			continue
		}

		period, ok := fullWidthToInt[[]rune(periodMatch[1])[0]]
		if !ok {
			result.Warnings = append(result.Warnings,
				fmt.Sprintf("不明な時限番号 %q: %q", periodMatch[1], part))
			continue
		}

		result.Slots = append(result.Slots, model.Slot{
			Semester: semester,
			Day:      dayMatch[1],
			Period:   period,
		})
	}

	return result
}
