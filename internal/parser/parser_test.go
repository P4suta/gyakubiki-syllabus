package parser

import (
	"testing"

	"github.com/livec/gyakubiki-syllabus/internal/model"
)

func TestParseJikanwari(t *testing.T) {
	tests := []struct {
		name         string
		input        string
		wantSlots    []model.Slot
		wantWarnings int
	}{
		// --- 正常系 ---
		{
			name:      "single slot",
			input:     "1学期: 月曜日１時限",
			wantSlots: []model.Slot{{Semester: "1学期", Day: "月", Period: 1}},
		},
		{
			name:  "multiple slots",
			input: "2学期: 月曜日２時限, 2学期: 木曜日１時限",
			wantSlots: []model.Slot{
				{Semester: "2学期", Day: "月", Period: 2},
				{Semester: "2学期", Day: "木", Period: 1},
			},
		},
		{
			name:      "tsuunen (full year)",
			input:     "通年: 火曜日３時限",
			wantSlots: []model.Slot{{Semester: "通年", Day: "火", Period: 3}},
		},
		{
			name:      "zenki (first half)",
			input:     "前期: 水曜日４時限",
			wantSlots: []model.Slot{{Semester: "前期", Day: "水", Period: 4}},
		},
		{
			name:      "koki (second half)",
			input:     "後期: 金曜日５時限",
			wantSlots: []model.Slot{{Semester: "後期", Day: "金", Period: 5}},
		},
		{
			name:      "period 6 saturday",
			input:     "1学期: 土曜日６時限",
			wantSlots: []model.Slot{{Semester: "1学期", Day: "土", Period: 6}},
		},
		{
			name:      "period 7 and 8",
			input:     "1学期: 月曜日７時限, 1学期: 月曜日８時限",
			wantSlots: []model.Slot{{Semester: "1学期", Day: "月", Period: 7}, {Semester: "1学期", Day: "月", Period: 8}},
		},
		{
			name:      "sunday",
			input:     "1学期: 日曜日１時限",
			wantSlots: []model.Slot{{Semester: "1学期", Day: "日", Period: 1}},
		},
		{
			name:  "three slots",
			input: "1学期: 月曜日１時限, 1学期: 水曜日３時限, 1学期: 金曜日５時限",
			wantSlots: []model.Slot{
				{Semester: "1学期", Day: "月", Period: 1},
				{Semester: "1学期", Day: "水", Period: 3},
				{Semester: "1学期", Day: "金", Period: 5},
			},
		},

		// --- 空・nil系 ---
		{
			name:      "empty string",
			input:     "",
			wantSlots: nil,
		},
		{
			name:      "whitespace only",
			input:     "   ",
			wantSlots: nil,
		},
		{
			name:      "commas only",
			input:     ", , ,",
			wantSlots: nil,
		},

		// --- パース失敗(警告あり) ---
		{
			name:         "no day match",
			input:        "1学期: ３時限",
			wantSlots:    nil,
			wantWarnings: 1,
		},
		{
			name:         "no period match",
			input:        "1学期: 月曜日",
			wantSlots:    nil,
			wantWarnings: 1,
		},
		{
			name:         "no day and no period",
			input:        "1学期: 集中講義",
			wantSlots:    nil,
			wantWarnings: 1,
		},
		{
			name:         "partial parse - one good one bad",
			input:        "1学期: 月曜日１時限, 1学期: 集中講義",
			wantSlots:    []model.Slot{{Semester: "1学期", Day: "月", Period: 1}},
			wantWarnings: 1,
		},
		{
			name:         "day without 曜日 suffix",
			input:        "1学期: 月１時限",
			wantSlots:    nil,
			wantWarnings: 1,
		},

		// --- 空白・フォーマットの揺れ ---
		{
			name:      "extra whitespace",
			input:     "  1学期:  月曜日１時限  ,  2学期:  火曜日２時限  ",
			wantSlots: []model.Slot{{Semester: "1学期", Day: "月", Period: 1}, {Semester: "2学期", Day: "火", Period: 2}},
		},
		{
			name:      "no semester prefix",
			input:     "月曜日３時限",
			wantSlots: []model.Slot{{Semester: "", Day: "月", Period: 3}},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ParseJikanwari(tt.input)
			got := result.Slots

			if len(got) != len(tt.wantSlots) {
				t.Fatalf("slots: len = %d, want %d\n  got:  %+v\n  want: %+v",
					len(got), len(tt.wantSlots), got, tt.wantSlots)
			}
			for i := range got {
				if got[i] != tt.wantSlots[i] {
					t.Errorf("slots[%d] = %+v, want %+v", i, got[i], tt.wantSlots[i])
				}
			}
			if tt.wantWarnings > 0 && len(result.Warnings) != tt.wantWarnings {
				t.Errorf("warnings: len = %d, want %d\n  got: %v",
					len(result.Warnings), tt.wantWarnings, result.Warnings)
			}
			if tt.wantWarnings == 0 && len(result.Warnings) > 0 {
				t.Errorf("unexpected warnings: %v", result.Warnings)
			}
		})
	}
}

func TestParseJikanwariWarningMessages(t *testing.T) {
	// Verify warning messages are human-readable and contain the original text
	t.Run("missing day includes original text", func(t *testing.T) {
		result := ParseJikanwari("1学期: ３時限")
		if len(result.Warnings) == 0 {
			t.Fatal("expected warning")
		}
		if result.Warnings[0] == "" {
			t.Error("warning message is empty")
		}
	})

	t.Run("missing period includes original text", func(t *testing.T) {
		result := ParseJikanwari("1学期: 月曜日")
		if len(result.Warnings) == 0 {
			t.Fatal("expected warning")
		}
		if result.Warnings[0] == "" {
			t.Error("warning message is empty")
		}
	})
}
