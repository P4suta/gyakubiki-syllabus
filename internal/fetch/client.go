// Package fetch retrieves KULAS syllabus data via the findPage API.
package fetch

import (
	"bytes"
	"context"
	"embed"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"net/http/cookiejar"
	"text/template"
	"time"
)

//go:embed findpage_body.tmpl.json
var bodyTemplateFS embed.FS

const (
	defaultSearchPageURL = "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/main/ja/Simple/1900/3000120/wsl/SyllabusKensaku"
	defaultFindPageURL   = "https://kulas.kochi-u.ac.jp/cpsmart/public/wsl/WebRoot/SystemD.Lead.Gkm.Com.KogiKensaku.App.KogiKensakuWebApi/findPage"
	defaultUserAgent     = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36"
	requestTimeout       = 120 * time.Second
)

// Client wraps an HTTP client with KULAS session state and the body template.
//
// 通常の lifecycle: NewClient() → FetchPage() を繰り返し → Client が GC される。
// セッション (CPSMART_PUBLIC_AUTH cookie + entryContext.token) は NewClient
// 内の establishSession で一度に確立し、以降のリクエストで再利用する。
type Client struct {
	httpClient    *http.Client
	bodyTmpl      *template.Template
	token         string
	kaikoNendo    string
	searchPageURL string
	findPageURL   string
	dumpDir       string // empty = no dump
}

// ClientOption configures a Client (used by tests to swap URLs).
type ClientOption func(*Client)

// WithSearchPageURL overrides the search page URL (for tests).
func WithSearchPageURL(u string) ClientOption {
	return func(c *Client) { c.searchPageURL = u }
}

// WithFindPageURL overrides the findPage URL (for tests).
func WithFindPageURL(u string) ClientOption {
	return func(c *Client) { c.findPageURL = u }
}

// WithDumpDir enables HTTP req/res dumping to the given directory.
// Empty path disables dumping. Used by --debug-dump-dir for root cause analysis.
func WithDumpDir(dir string) ClientOption {
	return func(c *Client) { c.dumpDir = dir }
}

// NewClient builds a Client and establishes a KULAS session.
//
// tokenOverride: 通常は空文字列で OK (HTML から抽出した token を使う)。
// 空でないとき、HTML 抽出済の token を上書きする。`--token` flag や
// `KULAS_API_TOKEN` env からの fallback / 検証用途。
func NewClient(ctx context.Context, kaikoNendo, tokenOverride string, opts ...ClientOption) (*Client, error) {
	tmpl, err := template.ParseFS(bodyTemplateFS, "findpage_body.tmpl.json")
	if err != nil {
		return nil, fmt.Errorf("body テンプレートのパースに失敗: %w", err)
	}

	jar, err := cookiejar.New(nil)
	if err != nil {
		return nil, fmt.Errorf("cookie jar の作成に失敗: %w", err)
	}

	c := &Client{
		bodyTmpl:      tmpl,
		kaikoNendo:    kaikoNendo,
		searchPageURL: defaultSearchPageURL,
		findPageURL:   defaultFindPageURL,
	}
	for _, opt := range opts {
		opt(c)
	}

	var rt http.RoundTripper = &http.Transport{
		TLSClientConfig: newKulasTLSConfig(),
	}
	if c.dumpDir != "" {
		rt = &DumpTransport{Base: rt, Dir: c.dumpDir}
	}
	c.httpClient = &http.Client{
		Jar:       jar,
		Timeout:   requestTimeout,
		Transport: rt,
	}

	if err := c.establishSession(ctx); err != nil {
		return nil, fmt.Errorf("KULAS セッションの確立に失敗: %w", err)
	}

	if tokenOverride != "" {
		slog.Info("token override applied", "source", "flag_or_env", "prev_token_prefix", tokenPrefix(c.token))
		c.token = tokenOverride
	}

	return c, nil
}

// FetchPage fetches one page of findPage results as raw JSON bytes.
func (c *Client) FetchPage(ctx context.Context, pageNo int) ([]byte, error) {
	logger := slog.With("op", "findPage", "page", pageNo)

	var buf bytes.Buffer
	if err := c.bodyTmpl.Execute(&buf, map[string]any{
		"PageNo":     pageNo,
		"KaikoNendo": c.kaikoNendo,
		"Token":      c.token,
	}); err != nil {
		return nil, fmt.Errorf("body テンプレートの render に失敗: %w", err)
	}
	bodySize := buf.Len()

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.findPageURL, &buf)
	if err != nil {
		return nil, fmt.Errorf("リクエスト生成に失敗: %w", err)
	}
	req.Header.Set("User-Agent", defaultUserAgent)
	req.Header.Set("Accept", "*/*")
	req.Header.Set("Accept-Language", "ja")
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Origin", "https://kulas.kochi-u.ac.jp")
	req.Header.Set("Referer", c.searchPageURL)

	logger.Debug("request prepared", "url", c.findPageURL, "method", http.MethodPost, "body_bytes", bodySize)

	start := time.Now()
	resp, err := c.httpClient.Do(req)
	if err != nil {
		logger.Error("HTTP call failed", "error", err.Error(), "elapsed_ms", time.Since(start).Milliseconds())
		return nil, fmt.Errorf("findPage HTTP 呼び出しに失敗 (page %d): %w", pageNo, err)
	}
	defer func() { _ = resp.Body.Close() }()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("findPage レスポンス読み込みに失敗 (page %d): %w", pageNo, err)
	}
	elapsed := time.Since(start)

	logger.Info("response received",
		"status", resp.StatusCode,
		"resp_bytes", len(body),
		"elapsed_ms", elapsed.Milliseconds(),
	)

	if resp.StatusCode != http.StatusOK {
		preview := string(body)
		const maxLen = 500
		if len(preview) > maxLen {
			preview = preview[:maxLen] + "..."
		}
		logger.Error("non-2xx response",
			"status", resp.StatusCode,
			"resp_body_preview", preview,
		)
		return nil, fmt.Errorf("findPage が HTTP %d を返しました (page %d): %s\n"+
			"  ※ token が古い / body フォーマットが API と不整合の可能性があります",
			resp.StatusCode, pageNo, preview)
	}
	return body, nil
}

// establishSession は検索ページを GET して以下 2 つを確立する:
//  1. CPSMART_PUBLIC_AUTH cookie (httpClient.Jar に保存される)
//  2. entryContext.token (HTML 内 cpSmartVueStartup の base64 引数から抽出して c.token にセット)
//
// HTML 取得は 1 回だけで session と token が同時に手に入る。
func (c *Client) establishSession(ctx context.Context) error {
	logger := slog.With("op", "session_establish")

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.searchPageURL, nil)
	if err != nil {
		return fmt.Errorf("リクエスト生成に失敗: %w", err)
	}
	req.Header.Set("User-Agent", defaultUserAgent)
	req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
	req.Header.Set("Accept-Language", "ja")

	logger.Debug("request prepared", "url", c.searchPageURL, "method", http.MethodGet)

	start := time.Now()
	resp, err := c.httpClient.Do(req)
	if err != nil {
		logger.Error("HTTP call failed", "error", err.Error(), "elapsed_ms", time.Since(start).Milliseconds())
		return fmt.Errorf("検索ページ GET に失敗 (TLS chain やネットワーク疎通を確認): %w", err)
	}
	defer func() { _ = resp.Body.Close() }()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("検索ページ HTML の読み込みに失敗: %w", err)
	}
	elapsed := time.Since(start)

	logger.Info("response received",
		"status", resp.StatusCode,
		"resp_bytes", len(body),
		"elapsed_ms", elapsed.Milliseconds(),
	)

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("検索ページが HTTP %d を返しました (URL: %s)", resp.StatusCode, c.searchPageURL)
	}

	cookieNames := make([]string, 0, 4)
	hasAuth := false
	for _, ck := range c.httpClient.Jar.Cookies(req.URL) {
		cookieNames = append(cookieNames, ck.Name)
		if ck.Name == "CPSMART_PUBLIC_AUTH" {
			hasAuth = true
		}
	}
	if !hasAuth {
		logger.Error("CPSMART_PUBLIC_AUTH cookie missing", "received_cookies", cookieNames)
		return fmt.Errorf("CPSMART_PUBLIC_AUTH cookie が取得できませんでした (受信 cookie: %v)", cookieNames)
	}
	logger.Info("session cookie acquired", "cookies", cookieNames)

	token, err := extractTokenFromHTML(body)
	if err != nil {
		return fmt.Errorf("token 抽出に失敗: %w", err)
	}
	c.token = token
	logger.Info("token extracted", "token_prefix", tokenPrefix(token))
	return nil
}

// tokenPrefix returns the first 8 chars of a token for log redaction.
func tokenPrefix(token string) string {
	if len(token) <= 8 {
		return token
	}
	return token[:8] + "..."
}
