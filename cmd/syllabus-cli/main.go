package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/livec/gyakubiki-syllabus/internal/inspect"
	"github.com/livec/gyakubiki-syllabus/internal/model"
	"github.com/livec/gyakubiki-syllabus/internal/transform"
	"github.com/spf13/cobra"
)

func main() {
	var outputFile string
	var compact bool

	rootCmd := &cobra.Command{
		Use:   "syllabus-cli",
		Short: "高知大学シラバスデータ変換ツール",
	}

	convertCmd := &cobra.Command{
		Use:   "convert <input-file> [input-file...]",
		Short: "KULASのAPIレスポンスJSONをビューアー用に変換する",
		Long: `KULASのシラバス検索APIのレスポンスJSONを読み込み、
時間割ビューアーで使える形式に変換して出力します。

複数ファイルを指定すると、内部でマージして1つの出力にします。
ページ分割されたAPIレスポンスをまとめて変換するのに便利です。

入力はファイル引数またはstdinから読み込みます。
selectKogiDtoListラッパーと生配列の両方に対応しています。`,
		Args: cobra.MinimumNArgs(0),
		RunE: func(cmd *cobra.Command, args []string) error {
			var rawCourses []model.RawCourse
			var err error

			if len(args) > 1 {
				// Multiple files — merge mode
				rawCourses, err = loadAndMerge(args)
				if err != nil {
					return err
				}
			} else {
				data, err := readInput(args)
				if err != nil {
					return err
				}
				rawCourses, err = parseInput(data)
				if err != nil {
					return err
				}
			}

			if len(rawCourses) == 0 {
				return fmt.Errorf("講義データが0件です。入力ファイルの内容を確認してください")
			}

			result := transform.Convert(rawCourses)

			// Print warnings to stderr
			if len(result.Warnings) > 0 {
				fmt.Fprintf(os.Stderr, "⚠ %d件の警告:\n", len(result.Warnings))
				for _, w := range result.Warnings {
					fmt.Fprintln(os.Stderr, w)
				}
				fmt.Fprintln(os.Stderr)
			}

			output, err := marshalOutput(result.Data, compact)
			if err != nil {
				return err
			}

			if outputFile != "" {
				if err := os.WriteFile(outputFile, output, 0644); err != nil {
					return fmt.Errorf("出力ファイルの書き込みに失敗しました: %s\n  原因: %w", outputFile, err)
				}
				fmt.Fprintf(os.Stderr, "✓ %d件の講義を変換しました (元データ: %d件) → %s\n",
					len(result.Data.Courses), result.Data.TotalRaw, outputFile)
			} else {
				fmt.Println(string(output))
			}

			return nil
		},
	}

	convertCmd.Flags().StringVarP(&outputFile, "output", "o", "", "出力先ファイル (デフォルト: stdout)")
	convertCmd.Flags().BoolVar(&compact, "compact", false, "圧縮出力")

	var jsonOutput bool

	inspectCmd := &cobra.Command{
		Use:   "inspect <input-file>",
		Short: "生JSONの構造を検査し、データの概要を表示する",
		Long: `KULASのAPIレスポンスJSONの構造を検査し、
convert前にデータの健全性を確認できます。

ページネーション情報、フィールド充填率、時間割パース試行結果などを表示します。`,
		Args: cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			filePath := args[0]
			data, err := os.ReadFile(filePath)
			if err != nil {
				if os.IsNotExist(err) {
					return fmt.Errorf("ファイルが見つかりません: %s", filePath)
				}
				return fmt.Errorf("ファイルを読み込めません: %s\n  原因: %w", filePath, err)
			}

			fi, _ := os.Stat(filePath)
			var fileSize int64
			if fi != nil {
				fileSize = fi.Size()
			}

			report, err := inspect.Inspect(data, filePath, fileSize)
			if err != nil {
				return err
			}

			if jsonOutput {
				out, err := json.MarshalIndent(report, "", "  ")
				if err != nil {
					return fmt.Errorf("JSON出力に失敗: %w", err)
				}
				fmt.Println(string(out))
			} else {
				printReport(report)
			}
			return nil
		},
	}

	inspectCmd.Flags().BoolVar(&jsonOutput, "json", false, "JSON形式で出力")

	rootCmd.AddCommand(convertCmd)
	rootCmd.AddCommand(inspectCmd)

	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}

func printReport(r *inspect.Report) {
	fmt.Println("=== ファイル情報 ===")
	fmt.Printf("  ファイル: %s (%s)\n", r.FileName, formatSize(r.FileSize))
	fmt.Printf("  形式: %s\n", r.Format)
	fmt.Println()

	if r.PageNo != nil {
		fmt.Println("=== ページネーション ===")
		fmt.Printf("  ページ: %d / %d\n", *r.PageNo, *r.MaxPageNo)
		fmt.Printf("  このファイルの件数: %d\n", r.CourseCount)
		if r.Total != nil {
			fmt.Printf("  全件数: %d\n", *r.Total)
		}
		fmt.Println()
	}

	if r.CourseCount > 0 {
		fmt.Println("=== フィールド検査 ===")
		fmt.Printf("  1件あたりのフィールド数: %d\n", r.TotalFields)
		fmt.Printf("  convertが使用するフィールド: %d\n", len(r.FieldCoverage))
		fmt.Println("  充填率:")
		for _, fs := range r.FieldCoverage {
			pct := float64(fs.Filled) / float64(fs.Total) * 100
			fmt.Printf("    %-20s %d/%d (%.0f%%)\n", fs.Name, fs.Filled, fs.Total, pct)
		}
		fmt.Println()

		fmt.Println("=== データ概要 ===")
		fmt.Printf("  ユニーク授業コード: %d\n", r.UniqueKogiCd)
		fmt.Printf("  学期: %s\n", strings.Join(r.Semesters, ", "))
		fmt.Printf("  開講責任部署: %s\n", strings.Join(r.Departments, ", "))
		fmt.Printf("  講義区分: %s\n", strings.Join(r.KogiKubuns, ", "))
		fmt.Printf("  校地: %s\n", strings.Join(r.Campuses, ", "))
		fmt.Println()

		total := r.ParseSuccess + len(r.ParseFailures)
		if total > 0 {
			fmt.Println("=== 時間割パース試行 ===")
			fmt.Printf("  パース成功: %d/%d (%.1f%%)\n",
				r.ParseSuccess, total, float64(r.ParseSuccess)/float64(total)*100)
			if len(r.ParseFailures) > 0 {
				fmt.Printf("  パース失敗: %d件\n", len(r.ParseFailures))
				for _, f := range r.ParseFailures {
					fmt.Printf("    [%s] %s: %s\n", f.KogiCd, f.KogiNm, f.Message)
				}
			}
			fmt.Println()
		}
	}

	fmt.Println("=== 判定 ===")
	if r.CanConvert {
		fmt.Println("  ✓ convertに使用できます")
	}
	for _, w := range r.Warnings {
		fmt.Printf("  ⚠ %s\n", w)
	}
}

func formatSize(bytes int64) string {
	switch {
	case bytes >= 1024*1024:
		return fmt.Sprintf("%.1f MB", float64(bytes)/1024/1024)
	case bytes >= 1024:
		return fmt.Sprintf("%.1f KB", float64(bytes)/1024)
	default:
		return fmt.Sprintf("%d B", bytes)
	}
}

// loadAndMerge reads multiple JSON files and merges their course lists into one.
func loadAndMerge(paths []string) ([]model.RawCourse, error) {
	var all []model.RawCourse

	for _, p := range paths {
		data, err := os.ReadFile(p)
		if err != nil {
			if os.IsNotExist(err) {
				return nil, fmt.Errorf("ファイルが見つかりません: %s", p)
			}
			return nil, fmt.Errorf("ファイルを読み込めません: %s\n  原因: %w", p, err)
		}

		courses, err := parseInput(data)
		if err != nil {
			return nil, fmt.Errorf("%s: %w", p, err)
		}

		all = append(all, courses...)
		fmt.Fprintf(os.Stderr, "  読み込み: %s (%d件)\n", p, len(courses))
	}

	fmt.Fprintf(os.Stderr, "  合計: %d件 (%dファイル)\n\n", len(all), len(paths))
	return all, nil
}

func readInput(args []string) ([]byte, error) {
	if len(args) > 0 {
		data, err := os.ReadFile(args[0])
		if err != nil {
			if os.IsNotExist(err) {
				return nil, fmt.Errorf("ファイルが見つかりません: %s", args[0])
			}
			return nil, fmt.Errorf("ファイルを読み込めません: %s\n  原因: %w", args[0], err)
		}
		return data, nil
	}

	stat, _ := os.Stdin.Stat()
	if (stat.Mode() & os.ModeCharDevice) != 0 {
		return nil, fmt.Errorf("使い方: syllabus-cli convert <input-file>\n" +
			"  ファイル指定:  syllabus-cli convert raw.json -o data.json\n" +
			"  パイプ入力:    cat raw.json | syllabus-cli convert -o data.json")
	}

	data, err := io.ReadAll(os.Stdin)
	if err != nil {
		return nil, fmt.Errorf("標準入力の読み込みに失敗しました: %w", err)
	}
	return data, nil
}

// parseInput tries to parse the raw JSON bytes as either a wrapped response or bare array.
func parseInput(data []byte) ([]model.RawCourse, error) {
	data = []byte(strings.TrimSpace(string(data)))
	if len(data) == 0 {
		return nil, fmt.Errorf("入力が空です。JSONデータを含むファイルを指定してください")
	}

	// Try as wrapped response first
	var resp model.RawResponse
	if err := json.Unmarshal(data, &resp); err == nil && resp.SelectKogiDtoList != nil {
		return resp.SelectKogiDtoList, nil
	}

	// Try as bare array
	var courses []model.RawCourse
	if err := json.Unmarshal(data, &courses); err == nil {
		return courses, nil
	}

	// Both failed — provide helpful diagnostics
	firstChar := string(data[0])
	switch {
	case firstChar == "{":
		return nil, fmt.Errorf("JSONオブジェクトとして解析しましたが、selectKogiDtoListキーが見つかりません。\n" +
			"  KULASのAPIレスポンス全体をコピーしているか確認してください。\n" +
			"  期待される形式: {\"selectKogiDtoList\": [...]}")
	case firstChar == "[":
		return nil, fmt.Errorf("JSON配列として解析しましたが、講義データとして認識できません。\n" +
			"  配列内の各要素にkogiCd, kogiNmなどのフィールドが必要です")
	case firstChar == "<":
		return nil, fmt.Errorf("HTMLが入力されました。JSONデータを指定してください。\n" +
			"  ブラウザのDevTools → Networkタブ → findPageリクエスト → Responseタブからコピーしてください")
	default:
		return nil, fmt.Errorf("入力がJSON形式ではありません (先頭: %q...)。\n"+
			"  KULASのAPIレスポンスのJSONをコピーしたファイルを指定してください",
			truncate(string(data), 50))
	}
}

func marshalOutput(data model.ProcessedData, compact bool) ([]byte, error) {
	var output []byte
	var err error
	if compact {
		output, err = json.Marshal(data)
	} else {
		output, err = json.MarshalIndent(data, "", "  ")
	}
	if err != nil {
		return nil, fmt.Errorf("JSON出力の生成に失敗しました: %w", err)
	}
	return output, nil
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen]
}
