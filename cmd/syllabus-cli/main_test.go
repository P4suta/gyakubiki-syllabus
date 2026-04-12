package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestParseInputWrappedResponse(t *testing.T) {
	input := `{"selectKogiDtoList": [{"kogiCd": "001", "kogiNm": "テスト"}]}`
	courses, err := parseInput([]byte(input))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(courses))
	}
	if courses[0].KogiCd != "001" {
		t.Errorf("kogiCd = %s, want 001", courses[0].KogiCd)
	}
}

func TestParseInputBareArray(t *testing.T) {
	input := `[{"kogiCd": "001", "kogiNm": "テスト"}]`
	courses, err := parseInput([]byte(input))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 1 {
		t.Fatalf("courses len = %d, want 1", len(courses))
	}
}

func TestParseInputEmptyArray(t *testing.T) {
	input := `[]`
	courses, err := parseInput([]byte(input))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 0 {
		t.Errorf("courses len = %d, want 0", len(courses))
	}
}

func TestParseInputEmptyString(t *testing.T) {
	_, err := parseInput([]byte(""))
	if err == nil {
		t.Fatal("expected error for empty input")
	}
	if !strings.Contains(err.Error(), "空") {
		t.Errorf("error should mention empty input: %v", err)
	}
}

func TestParseInputWhitespaceOnly(t *testing.T) {
	_, err := parseInput([]byte("   \n  "))
	if err == nil {
		t.Fatal("expected error for whitespace-only input")
	}
}

func TestParseInputHTML(t *testing.T) {
	_, err := parseInput([]byte("<html><body>Error</body></html>"))
	if err == nil {
		t.Fatal("expected error for HTML input")
	}
	if !strings.Contains(err.Error(), "HTML") {
		t.Errorf("error should mention HTML: %v", err)
	}
}

func TestParseInputInvalidJSON(t *testing.T) {
	_, err := parseInput([]byte("this is not json"))
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
	if !strings.Contains(err.Error(), "JSON形式ではありません") {
		t.Errorf("error should mention not JSON: %v", err)
	}
}

func TestParseInputObjectWithoutSelectKogiDtoList(t *testing.T) {
	_, err := parseInput([]byte(`{"data": [{"kogiCd": "001"}]}`))
	if err == nil {
		t.Fatal("expected error for object without selectKogiDtoList")
	}
	if !strings.Contains(err.Error(), "selectKogiDtoList") {
		t.Errorf("error should mention selectKogiDtoList: %v", err)
	}
}

func TestParseInputWrappedWithNullList(t *testing.T) {
	// selectKogiDtoList is null — should fall through to bare array attempt, then fail
	_, err := parseInput([]byte(`{"selectKogiDtoList": null}`))
	if err == nil {
		t.Fatal("expected error for null selectKogiDtoList")
	}
}

func TestParseInputTrimsWhitespace(t *testing.T) {
	input := `  [{"kogiCd": "001", "kogiNm": "テスト"}]  `
	courses, err := parseInput([]byte(input))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 1 {
		t.Errorf("courses len = %d, want 1", len(courses))
	}
}

// === loadAndMerge テスト ===

func writeTempJSON(t *testing.T, dir, name, content string) string {
	t.Helper()
	p := filepath.Join(dir, name)
	if err := os.WriteFile(p, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}
	return p
}

func TestLoadAndMerge_SingleFile(t *testing.T) {
	dir := t.TempDir()
	f := writeTempJSON(t, dir, "page1.json",
		`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A"}]}`)

	courses, err := loadAndMerge([]string{f})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 1 {
		t.Errorf("courses len = %d, want 1", len(courses))
	}
}

func TestLoadAndMerge_MultipleFiles(t *testing.T) {
	dir := t.TempDir()
	f1 := writeTempJSON(t, dir, "page1.json",
		`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A"},{"kogiCd":"002","kogiNm":"B"}], "pageNo":1, "maxPageNo":2, "total":3, "pageSize":2}`)
	f2 := writeTempJSON(t, dir, "page2.json",
		`{"selectKogiDtoList": [{"kogiCd":"003","kogiNm":"C"}], "pageNo":2, "maxPageNo":2, "total":3, "pageSize":2}`)

	courses, err := loadAndMerge([]string{f1, f2})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 3 {
		t.Errorf("courses len = %d, want 3", len(courses))
	}
}

func TestLoadAndMerge_BareArrayFiles(t *testing.T) {
	dir := t.TempDir()
	f1 := writeTempJSON(t, dir, "a.json", `[{"kogiCd":"001","kogiNm":"A"}]`)
	f2 := writeTempJSON(t, dir, "b.json", `[{"kogiCd":"002","kogiNm":"B"}]`)

	courses, err := loadAndMerge([]string{f1, f2})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 2 {
		t.Errorf("courses len = %d, want 2", len(courses))
	}
}

func TestLoadAndMerge_MixedFormats(t *testing.T) {
	dir := t.TempDir()
	f1 := writeTempJSON(t, dir, "api.json",
		`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A"}]}`)
	f2 := writeTempJSON(t, dir, "bare.json",
		`[{"kogiCd":"002","kogiNm":"B"}]`)

	courses, err := loadAndMerge([]string{f1, f2})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 2 {
		t.Errorf("courses len = %d, want 2", len(courses))
	}
}

func TestLoadAndMerge_FileNotFound(t *testing.T) {
	_, err := loadAndMerge([]string{"/nonexistent/file.json"})
	if err == nil {
		t.Fatal("expected error for missing file")
	}
}

func TestLoadAndMerge_InvalidJSON(t *testing.T) {
	dir := t.TempDir()
	f := writeTempJSON(t, dir, "bad.json", "not json")

	_, err := loadAndMerge([]string{f})
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
}

func TestLoadAndMerge_ReportsFileCount(t *testing.T) {
	dir := t.TempDir()
	f1 := writeTempJSON(t, dir, "p1.json",
		`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A"}]}`)
	f2 := writeTempJSON(t, dir, "p2.json",
		`{"selectKogiDtoList": [{"kogiCd":"002","kogiNm":"B"}]}`)
	f3 := writeTempJSON(t, dir, "p3.json",
		`{"selectKogiDtoList": [{"kogiCd":"003","kogiNm":"C"}]}`)

	courses, err := loadAndMerge([]string{f1, f2, f3})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 3 {
		t.Errorf("courses len = %d, want 3", len(courses))
	}
}

func TestLoadAndMerge_EmptyFiles(t *testing.T) {
	dir := t.TempDir()
	f := writeTempJSON(t, dir, "empty.json",
		`{"selectKogiDtoList": []}`)

	courses, err := loadAndMerge([]string{f})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(courses) != 0 {
		t.Errorf("courses len = %d, want 0", len(courses))
	}
}

func TestLoadAndMerge_SecondFileFails(t *testing.T) {
	dir := t.TempDir()
	f1 := writeTempJSON(t, dir, "good.json",
		`{"selectKogiDtoList": [{"kogiCd":"001","kogiNm":"A"}]}`)
	f2 := writeTempJSON(t, dir, "bad.json", "{{invalid}}")

	_, err := loadAndMerge([]string{f1, f2})
	if err == nil {
		t.Fatal("expected error when second file is invalid")
	}
	// Error should mention which file failed
	if !strings.Contains(err.Error(), "bad.json") {
		t.Errorf("error should mention failing file: %v", err)
	}
}
