package main

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"strconv"
	"time"

	"github.com/livec/gyakubiki-syllabus/internal/fetch"
	"github.com/spf13/cobra"
)

const tokenEnvVar = "KULAS_API_TOKEN" //nolint:gosec // 環境変数名であって秘匿値ではない

func newFetchCommand() *cobra.Command {
	var (
		outDir       string
		yearFlag     string
		token        string
		minTotal     int
		dryRun       bool
		debugDumpDir string
	)

	cmd := &cobra.Command{
		Use:   "fetch",
		Short: "KULAS から月次でシラバスを取得し raw/ を更新する",
		Long: `KULAS の findPage API をページネーションで全件叩いて、
raw/ ディレクトリの 講義データ*.json を更新します。

token 取得方針 (優先順):
  1. --token フラグ
  2. 環境変数 ` + tokenEnvVar + `
  3. SignalR WebSocket で動的取得 (上記が両方空のとき)`,
		Args: cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			ctx, cancel := context.WithTimeout(cmd.Context(), 10*time.Minute)
			defer cancel()

			kaikoNendo := yearFlag
			if kaikoNendo == "" {
				kaikoNendo = strconv.Itoa(currentKaikoNendo(time.Now()))
			}

			dumpDir, err := fetch.InitDumpDir(debugDumpDir)
			if err != nil {
				return fmt.Errorf("--debug-dump-dir 初期化に失敗: %w", err)
			}

			provider, providerName := buildTokenProvider(token, dumpDir)

			slog.Info("fetch start",
				"kaikoNendo", kaikoNendo,
				"out_dir", outDir,
				"min_total", minTotal,
				"dry_run", dryRun,
				"token_provider", providerName,
				"debug_dump_dir", dumpDir,
			)
			client, err := fetch.NewClient(ctx, kaikoNendo, provider, fetch.WithDumpDir(dumpDir))
			if err != nil {
				slog.Error("client init failed", "error", err.Error())
				return fmt.Errorf("client 初期化に失敗 (TLS/session/token のいずれか — 直前の slog Error 参照): %w", err)
			}

			slog.Info("client ready, fetching pages")
			result, err := fetch.All(ctx, fetch.Options{
				OutDir:   outDir,
				MinTotal: minTotal,
				DryRun:   dryRun,
			}, client)
			if err != nil {
				slog.Error("page fetch failed", "error", err.Error())
				return fmt.Errorf("page 取得に失敗: %w", err)
			}

			printFetchReport(result, dryRun)
			return nil
		},
	}

	cmd.Flags().StringVar(&outDir, "out-dir", "raw", "raw JSON の出力ディレクトリ")
	cmd.Flags().StringVar(&yearFlag, "year", "", "kaikoNendo (空なら現在年度を自動計算)")
	cmd.Flags().StringVar(&token, "token", "", "KULAS API token (空なら環境変数 "+tokenEnvVar+" → SignalR の順で取得)")
	cmd.Flags().IntVar(&minTotal, "min-total", 1500, "page 1 の total 件数の最小ガード (これを下回ると fail)")
	cmd.Flags().BoolVar(&dryRun, "dry-run", false, "取得して件数のみ報告、ファイル書き込みなし")
	cmd.Flags().StringVar(&debugDumpDir, "debug-dump-dir", "",
		"指定すると <dir>/<timestamp>/ に全 HTTP req/resp と SignalR frame を保存 (root cause 分析用)")

	return cmd
}

// currentKaikoNendo returns the academic year for the given date.
// In Japan academic years start in April, so Jan–Mar belongs to the previous year.
func currentKaikoNendo(now time.Time) int {
	year := now.Year()
	if now.Month() < time.April {
		year--
	}
	return year
}

// buildTokenProvider は flag / env / SignalR の優先順で token provider を選ぶ。
// 第2戻り値は slog 用の provider name (例: "flag", "env:KULAS_API_TOKEN", "signalr")。
func buildTokenProvider(flagToken, dumpDir string) (fetch.TokenProvider, string) {
	if flagToken != "" {
		return fetch.StaticTokenProvider{Token: flagToken}, "flag"
	}
	if env := os.Getenv(tokenEnvVar); env != "" {
		return fetch.StaticTokenProvider{Token: env}, "env:" + tokenEnvVar
	}
	cfg := fetch.DefaultSignalRConfig()
	cfg.DumpDir = dumpDir
	return fetch.SignalRTokenProvider{Config: cfg}, "signalr"
}

func printFetchReport(result *fetch.Result, dryRun bool) {
	slog.Info("fetch summary",
		"total_courses", result.Total,
		"max_page_no", result.MaxPageNo,
		"pages", len(result.Pages),
		"dry_run", dryRun,
	)
	changedCount := 0
	for _, p := range result.Pages {
		if !dryRun && p.Changed {
			changedCount++
		}
		slog.Info("page",
			"no", p.PageNo,
			"items", p.ListLen,
			"file", p.FileName,
			"changed", !dryRun && p.Changed,
		)
	}
	if len(result.Cleaned) > 0 {
		slog.Info("stale files removed", "files", result.Cleaned)
	}
	if !dryRun {
		slog.Info("write summary", "files_changed", changedCount)
	}

	// GitHub Actions step summary (optional)
	writeStepSummary(result, dryRun, changedCount)
}

// writeStepSummary は $GITHUB_STEP_SUMMARY が設定されているとき markdown を追記する。
// CLI ローカル実行や $GITHUB_STEP_SUMMARY 未設定環境では何もしない。
func writeStepSummary(result *fetch.Result, dryRun bool, changedCount int) {
	path := os.Getenv("GITHUB_STEP_SUMMARY")
	if path == "" {
		return
	}
	f, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o600) //nolint:gosec // path は GitHub Actions が制御
	if err != nil {
		slog.Warn("GITHUB_STEP_SUMMARY を開けませんでした", "path", path, "error", err.Error())
		return
	}
	defer func() { _ = f.Close() }()

	mode := "通常実行"
	if dryRun {
		mode = "dry-run"
	}
	_, _ = fmt.Fprintf(f, "## Fetch syllabus result (%s)\n\n", mode)
	_, _ = fmt.Fprintf(f, "- 取得件数: **%d** / 全 %d ページ\n", result.Total, result.MaxPageNo)
	if !dryRun {
		_, _ = fmt.Fprintf(f, "- 変更ファイル: %d\n", changedCount)
	}
	_, _ = fmt.Fprintf(f, "\n| page | 件数 | ファイル | 変更 |\n|---|---|---|---|\n")
	for _, p := range result.Pages {
		changed := "—"
		if !dryRun && p.Changed {
			changed = "✓"
		}
		_, _ = fmt.Fprintf(f, "| %d | %d | `%s` | %s |\n", p.PageNo, p.ListLen, p.FileName, changed)
	}
	if len(result.Cleaned) > 0 {
		_, _ = fmt.Fprintf(f, "\n古いファイルを削除: %v\n", result.Cleaned)
	}
}
