package fetch

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// fakeFetcher returns canned bytes per page.
type fakeFetcher struct {
	pages map[int][]byte
	err   error
}

func (f *fakeFetcher) FetchPage(_ context.Context, pageNo int) ([]byte, error) {
	if f.err != nil {
		return nil, f.err
	}
	b, ok := f.pages[pageNo]
	if !ok {
		return nil, fmt.Errorf("fake: no canned response for page %d", pageNo)
	}
	return b, nil
}

// buildPage assembles a realistic-shaped KULAS findPage response.
func buildPage(t *testing.T, pageNo, maxPageNo, total, listLen int, extraField string) []byte {
	t.Helper()
	courses := make([]map[string]any, listLen)
	for i := range courses {
		// extraField simulates raw API fields beyond what RawCourse defines —
		// the round-trip must preserve them.
		courses[i] = map[string]any{
			"kogiCd": fmt.Sprintf("%05d", pageNo*1000+i),
			"kogiNm": "テスト講義",
			"extra":  extraField,
		}
	}
	resp := map[string]any{
		"pageNo":            pageNo,
		"maxPageNo":         maxPageNo,
		"total":             total,
		"pageSize":          500,
		"selectKogiDtoList": courses,
	}
	b, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("buildPage marshal: %v", err)
	}
	return b
}

func TestAll_HappyPath(t *testing.T) {
	tmpDir := t.TempDir()
	fetcher := &fakeFetcher{
		pages: map[int][]byte{
			1: buildPage(t, 1, 3, 1200, 500, "page1"),
			2: buildPage(t, 2, 3, 1200, 500, "page2"),
			3: buildPage(t, 3, 3, 1200, 200, "page3"), // last page partial
		},
	}

	result, err := All(context.Background(), Options{
		OutDir:   tmpDir,
		MinTotal: 100,
	}, fetcher)
	if err != nil {
		t.Fatalf("All returned error: %v", err)
	}
	if result.Total != 1200 || result.MaxPageNo != 3 {
		t.Errorf("unexpected Total/MaxPageNo: got total=%d, max=%d", result.Total, result.MaxPageNo)
	}
	if len(result.Pages) != 3 {
		t.Fatalf("expected 3 pages, got %d", len(result.Pages))
	}

	// Verify on-disk files
	wantFiles := []string{"講義データ.json", "講義データ-02.json", "講義データ-03.json"}
	for _, name := range wantFiles {
		path := filepath.Join(tmpDir, name)
		data, err := os.ReadFile(path)
		if err != nil {
			t.Errorf("expected file %s: %v", name, err)
			continue
		}
		// Verify the "extra" field round-trips (no field loss)
		var got map[string]any
		if err := json.Unmarshal(data, &got); err != nil {
			t.Errorf("failed to parse %s: %v", name, err)
			continue
		}
		courses, _ := got["selectKogiDtoList"].([]any)
		if len(courses) == 0 {
			t.Errorf("%s has no courses", name)
			continue
		}
		first, _ := courses[0].(map[string]any)
		if first["extra"] == nil {
			t.Errorf("%s lost the 'extra' field — RawMessage round-trip is broken", name)
		}
	}
}

func TestAll_BelowMinTotal(t *testing.T) {
	fetcher := &fakeFetcher{
		pages: map[int][]byte{
			1: buildPage(t, 1, 1, 50, 50, "small"),
		},
	}
	_, err := All(context.Background(), Options{
		OutDir:   t.TempDir(),
		MinTotal: 100,
	}, fetcher)
	if err == nil {
		t.Fatal("expected error when total < MinTotal, got nil")
	}
}

func TestAll_MidPageWrongSize(t *testing.T) {
	// 3 ページ構成で page 2 (中間ページ) が部分件数 → 検証エラーになるべき
	fetcher := &fakeFetcher{
		pages: map[int][]byte{
			1: buildPage(t, 1, 3, 1300, 500, "p1"),
			2: buildPage(t, 2, 3, 1300, 300, "p2"), // partial mid page → invalid
		},
	}
	_, err := All(context.Background(), Options{
		OutDir:   t.TempDir(),
		MinTotal: 100,
	}, fetcher)
	if err == nil {
		t.Fatal("expected error when mid-page listLen != pageSize, got nil")
	}
}

func TestAll_DryRunDoesNotWrite(t *testing.T) {
	tmpDir := t.TempDir()
	fetcher := &fakeFetcher{
		pages: map[int][]byte{
			1: buildPage(t, 1, 1, 300, 300, "dry"),
		},
	}
	_, err := All(context.Background(), Options{
		OutDir:   tmpDir,
		MinTotal: 100,
		DryRun:   true,
	}, fetcher)
	if err != nil {
		t.Fatalf("dry-run failed: %v", err)
	}
	entries, _ := os.ReadDir(tmpDir)
	if len(entries) != 0 {
		t.Errorf("dry-run wrote %d files (expected 0)", len(entries))
	}
}

func TestAll_CleansUpStalePages(t *testing.T) {
	tmpDir := t.TempDir()

	// Pre-populate with stale page 4 and 5 (left over from a previous run)
	for _, n := range []string{"講義データ-04.json", "講義データ-05.json"} {
		if err := os.WriteFile(filepath.Join(tmpDir, n), []byte("stale"), 0o644); err != nil {
			t.Fatal(err)
		}
	}

	fetcher := &fakeFetcher{
		pages: map[int][]byte{
			1: buildPage(t, 1, 3, 1200, 500, "p1"),
			2: buildPage(t, 2, 3, 1200, 500, "p2"),
			3: buildPage(t, 3, 3, 1200, 200, "p3"),
		},
	}
	result, err := All(context.Background(), Options{
		OutDir:   tmpDir,
		MinTotal: 100,
	}, fetcher)
	if err != nil {
		t.Fatalf("All error: %v", err)
	}

	if len(result.Cleaned) != 2 {
		t.Errorf("expected 2 cleaned files, got %d: %v", len(result.Cleaned), result.Cleaned)
	}
	for _, n := range []string{"講義データ-04.json", "講義データ-05.json"} {
		if _, err := os.Stat(filepath.Join(tmpDir, n)); !os.IsNotExist(err) {
			t.Errorf("expected %s to be removed", n)
		}
	}
}

func TestRawFileName(t *testing.T) {
	cases := []struct {
		pageNo int
		want   string
	}{
		{1, "講義データ.json"},
		{2, "講義データ-02.json"},
		{8, "講義データ-08.json"},
		{10, "講義データ-10.json"},
	}
	for _, c := range cases {
		if got := RawFileName(c.pageNo); got != c.want {
			t.Errorf("RawFileName(%d) = %q, want %q", c.pageNo, got, c.want)
		}
	}
}

func TestExtractTokenFromHTML(t *testing.T) {
	// "{\"token\":\"abc123\"}" を base64 化したもの
	validHTML := []byte(`<html><body>
<script>
cpSmartVueStartup('dash-app-main', '2025-03-26-13-31-19-072', true, 'eyJ0b2tlbiI6ImFiYzEyMyJ9')
</script>
</body></html>`)
	tok, err := extractTokenFromHTML(validHTML)
	if err != nil {
		t.Fatalf("expected success, got error: %v", err)
	}
	if tok != "abc123" {
		t.Errorf("token = %q, want %q", tok, "abc123")
	}
}

func TestExtractTokenFromHTML_NoMatch(t *testing.T) {
	// cpSmartVueStartup が無い
	_, err := extractTokenFromHTML([]byte(`<html><body>no startup script</body></html>`))
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !strings.Contains(err.Error(), "cpSmartVueStartup") {
		t.Errorf("error should mention cpSmartVueStartup: %v", err)
	}
}

func TestExtractTokenFromHTML_BadBase64(t *testing.T) {
	html := []byte(`cpSmartVueStartup('dash-app-main', 'v', true, 'not!base64')`)
	_, err := extractTokenFromHTML(html)
	if err == nil {
		t.Fatal("expected error for unparsable base64")
	}
}

func TestExtractTokenFromHTML_EmptyToken(t *testing.T) {
	// {"token":""} の base64
	html := []byte(`cpSmartVueStartup('dash-app-main', 'v', true, 'eyJ0b2tlbiI6IiJ9')`)
	_, err := extractTokenFromHTML(html)
	if err == nil {
		t.Fatal("expected error for empty token")
	}
}

func TestExtractTokenFromHTML_WrongComponent(t *testing.T) {
	// dash-header の token は無視されるべき (dash-app-main のみマッチ)
	html := []byte(`cpSmartVueStartup('dash-header', 'v', true, 'eyJ0b2tlbiI6Im90aGVyIn0=')`)
	_, err := extractTokenFromHTML(html)
	if err == nil {
		t.Fatal("expected error when only non-dash-app-main script exists")
	}
}
