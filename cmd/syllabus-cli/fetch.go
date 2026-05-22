package main

import (
	"context"
	"fmt"
	"os"
	"strconv"
	"time"

	"github.com/livec/gyakubiki-syllabus/internal/fetch"
	"github.com/spf13/cobra"
)

const tokenEnvVar = "KULAS_API_TOKEN"

func newFetchCommand() *cobra.Command {
	var (
		outDir   string
		yearFlag string
		token    string
		minTotal int
		dryRun   bool
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

			provider := buildTokenProvider(token)

			fmt.Fprintf(os.Stderr, "✓ KULAS セッション確立中 (kaikoNendo=%s)...\n", kaikoNendo)
			client, err := fetch.NewClient(ctx, kaikoNendo, provider)
			if err != nil {
				return err
			}

			fmt.Fprintln(os.Stderr, "✓ findPage 取得開始")
			result, err := fetch.FetchAll(ctx, fetch.Options{
				OutDir:   outDir,
				MinTotal: minTotal,
				DryRun:   dryRun,
			}, client)
			if err != nil {
				return err
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

func buildTokenProvider(flagToken string) fetch.TokenProvider {
	if flagToken != "" {
		return fetch.StaticTokenProvider{Token: flagToken}
	}
	if env := os.Getenv(tokenEnvVar); env != "" {
		return fetch.StaticTokenProvider{Token: env}
	}
	return fetch.SignalRTokenProvider{Config: fetch.DefaultSignalRConfig()}
}

func printFetchReport(result *fetch.Result, dryRun bool) {
	fmt.Fprintln(os.Stderr)
	fmt.Fprintf(os.Stderr, "✓ 取得完了: %d 件 / 全 %d ページ\n", result.Total, result.MaxPageNo)
	for _, p := range result.Pages {
		marker := ""
		if !dryRun && p.Changed {
			marker = " (changed)"
		}
		fmt.Fprintf(os.Stderr, "  page %d: %d 件 → %s%s\n", p.PageNo, p.ListLen, p.FileName, marker)
	}
	if len(result.Cleaned) > 0 {
		fmt.Fprintf(os.Stderr, "  古いファイルを削除: %v\n", result.Cleaned)
	}
}
