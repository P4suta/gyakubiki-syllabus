package fetch

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
)

// Options controls a All run.
type Options struct {
	OutDir   string
	MinTotal int
	DryRun   bool
}

// Result describes what All did.
type Result struct {
	Total     int
	MaxPageNo int
	Pages     []PageResult
	Cleaned   []string
}

// PageResult is per-page metadata for the run.
type PageResult struct {
	PageNo   int
	ListLen  int
	FileName string
	Changed  bool
}

// PageFetcher abstracts the HTTP layer so tests can swap in a fake.
type PageFetcher interface {
	FetchPage(ctx context.Context, pageNo int) ([]byte, error)
}

type pageMeta struct {
	PageNo            int             `json:"pageNo"`
	MaxPageNo         int             `json:"maxPageNo"`
	Total             int             `json:"total"`
	PageSize          int             `json:"pageSize"`
	SelectKogiDtoList json.RawMessage `json:"selectKogiDtoList"`
}

// All downloads every page, validates counts, and writes raw JSON to disk.
// Returns without writing if opts.DryRun is set.
func All(ctx context.Context, opts Options, fetcher PageFetcher) (*Result, error) {
	firstBytes, err := fetcher.FetchPage(ctx, 1)
	if err != nil {
		return nil, err
	}

	var first pageMeta
	if err := json.Unmarshal(firstBytes, &first); err != nil {
		return nil, fmt.Errorf("page 1 のレスポンスを JSON として解析できません: %w", err)
	}
	if first.MaxPageNo < 1 {
		return nil, fmt.Errorf("page 1 の maxPageNo が無効です (= %d)", first.MaxPageNo)
	}
	if first.Total < opts.MinTotal {
		return nil, fmt.Errorf("page 1 の total が閾値を下回っています (%d < %d) — API 不調の可能性",
			first.Total, opts.MinTotal)
	}

	pages := map[int][]byte{1: firstBytes}
	result := &Result{
		Total:     first.Total,
		MaxPageNo: first.MaxPageNo,
		Pages: []PageResult{{
			PageNo:   1,
			ListLen:  countList(first.SelectKogiDtoList),
			FileName: RawFileName(1),
		}},
	}

	for pageNo := 2; pageNo <= first.MaxPageNo; pageNo++ {
		fmt.Fprintf(os.Stderr, "  取得中: page %d/%d\n", pageNo, first.MaxPageNo)
		b, err := fetcher.FetchPage(ctx, pageNo)
		if err != nil {
			return nil, err
		}
		var meta pageMeta
		if err := json.Unmarshal(b, &meta); err != nil {
			return nil, fmt.Errorf("page %d のレスポンスを JSON として解析できません: %w", pageNo, err)
		}
		if meta.PageNo != pageNo {
			return nil, fmt.Errorf("page %d を要求したが pageNo=%d が返ってきました", pageNo, meta.PageNo)
		}
		if meta.MaxPageNo != first.MaxPageNo {
			return nil, fmt.Errorf("page %d で maxPageNo が変化 (%d → %d)", pageNo, first.MaxPageNo, meta.MaxPageNo)
		}
		listLen := countList(meta.SelectKogiDtoList)
		if pageNo < first.MaxPageNo && listLen != meta.PageSize {
			return nil, fmt.Errorf("中間 page %d の件数が不足 (listLen=%d, pageSize=%d)",
				pageNo, listLen, meta.PageSize)
		}
		pages[pageNo] = b
		result.Pages = append(result.Pages, PageResult{
			PageNo:   pageNo,
			ListLen:  listLen,
			FileName: RawFileName(pageNo),
		})
	}

	if opts.DryRun {
		fmt.Fprintln(os.Stderr, "(dry-run) ファイルは書き込みません")
		sortPagesByNo(result.Pages)
		return result, nil
	}

	if err := os.MkdirAll(opts.OutDir, 0o750); err != nil {
		return nil, fmt.Errorf("出力ディレクトリの作成に失敗 %s: %w", opts.OutDir, err)
	}

	for pageNo, b := range pages {
		path := filepath.Join(opts.OutDir, RawFileName(pageNo))
		changed := fileChanged(path, b)
		// raw/ は GitHub Pages から読まれるので 0o644 が必要
		if err := os.WriteFile(path, b, 0o644); err != nil { //nolint:gosec
			return nil, fmt.Errorf("ファイル書き込みに失敗 %s: %w", path, err)
		}
		for i := range result.Pages {
			if result.Pages[i].PageNo == pageNo {
				result.Pages[i].Changed = changed
			}
		}
	}

	sortPagesByNo(result.Pages)

	cleaned, err := cleanupStalePages(opts.OutDir, first.MaxPageNo)
	if err != nil {
		return nil, fmt.Errorf("古いファイルのクリーンアップに失敗: %w", err)
	}
	result.Cleaned = cleaned

	return result, nil
}

// RawFileName returns the on-disk file name for the given page number.
// Page 1 keeps the legacy unsuffixed name; pages 2+ get a zero-padded suffix.
func RawFileName(pageNo int) string {
	if pageNo == 1 {
		return "講義データ.json"
	}
	return fmt.Sprintf("講義データ-%02d.json", pageNo)
}

// rawFileNamePattern parses a raw filename back into its page number.
// Returns (pageNo, true) on match, (0, false) otherwise.
var rawFileNamePattern = regexp.MustCompile(`^講義データ-(\d{2})\.json$`)

func cleanupStalePages(outDir string, maxPageNo int) ([]string, error) {
	entries, err := os.ReadDir(outDir)
	if err != nil {
		return nil, err
	}
	var cleaned []string
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		m := rawFileNamePattern.FindStringSubmatch(entry.Name())
		if m == nil {
			continue
		}
		pageNo, err := strconv.Atoi(m[1])
		if err != nil || pageNo <= maxPageNo {
			continue
		}
		path := filepath.Join(outDir, entry.Name())
		if err := os.Remove(path); err != nil {
			return nil, err
		}
		cleaned = append(cleaned, entry.Name())
	}
	sort.Strings(cleaned)
	return cleaned, nil
}

func countList(raw json.RawMessage) int {
	var arr []json.RawMessage
	if err := json.Unmarshal(raw, &arr); err != nil {
		return 0
	}
	return len(arr)
}

func fileChanged(path string, newContent []byte) bool {
	existing, err := os.ReadFile(path) //nolint:gosec // path は raw/ 内の既知 file 名のみ
	if err != nil {
		return true
	}
	return !bytesEqual(existing, newContent)
}

func bytesEqual(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func sortPagesByNo(pages []PageResult) {
	sort.Slice(pages, func(i, j int) bool {
		return pages[i].PageNo < pages[j].PageNo
	})
}
