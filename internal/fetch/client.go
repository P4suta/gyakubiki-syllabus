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

// TokenProvider abstracts how the entryContext token is obtained.
// Static deployments use StaticTokenProvider; production uses SignalRTokenProvider
// which establishes a WebSocket connection and reads the token from a push message.
type TokenProvider interface {
	GetToken(ctx context.Context, httpClient *http.Client) (string, error)
}

// StaticTokenProvider returns a pre-supplied token. Useful for tests or
// short-term fallback when the dynamic route is unavailable.
type StaticTokenProvider struct {
	Token string
}

// GetToken returns the static token.
func (s StaticTokenProvider) GetToken(_ context.Context, _ *http.Client) (string, error) {
	if s.Token == "" {
		return "", fmt.Errorf("StaticTokenProvider に token が設定されていません")
	}
	return s.Token, nil
}

// SignalRTokenProvider obtains the token by listening to the KULAS SignalR hub.
type SignalRTokenProvider struct {
	Config SignalRConfig
}

// GetToken connects to the SignalR hub and reads the token from a push message.
func (s SignalRTokenProvider) GetToken(ctx context.Context, httpClient *http.Client) (string, error) {
	return fetchTokenViaSignalR(ctx, httpClient, s.Config)
}

// Client wraps an HTTP client with KULAS session state and the body template.
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

// WithDumpDir enables HTTP req/res + SignalR frame dumping to the given directory.
// Empty path disables dumping. Used by --debug-dump-dir for root cause analysis.
func WithDumpDir(dir string) ClientOption {
	return func(c *Client) { c.dumpDir = dir }
}

// NewClient builds a Client, establishes a KULAS session, and obtains the token.
func NewClient(ctx context.Context, kaikoNendo string, tokenProvider TokenProvider, opts ...ClientOption) (*Client, error) {
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

	if tokenProvider == nil {
		return nil, fmt.Errorf("TokenProvider が指定されていません")
	}
	token, err := tokenProvider.GetToken(ctx, c.httpClient)
	if err != nil {
		return nil, fmt.Errorf("token の取得に失敗: %w", err)
	}
	c.token = token

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
			"  ※ token または body フォーマットが API と不整合の可能性があります",
			resp.StatusCode, pageNo, preview)
	}
	return body, nil
}

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
		return fmt.Errorf("検索ページ GET に失敗 (TLS chain や ネットワーク疎通を確認): %w", err)
	}
	defer func() { _ = resp.Body.Close() }()
	n, _ := io.Copy(io.Discard, resp.Body)
	elapsed := time.Since(start)

	logger.Info("response received",
		"status", resp.StatusCode,
		"resp_bytes", n,
		"elapsed_ms", elapsed.Milliseconds(),
	)

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("検索ページが HTTP %d を返しました (URL: %s)", resp.StatusCode, c.searchPageURL)
	}

	cookieNames := make([]string, 0, 4)
	for _, ck := range c.httpClient.Jar.Cookies(req.URL) {
		cookieNames = append(cookieNames, ck.Name)
		if ck.Name == "CPSMART_PUBLIC_AUTH" {
			logger.Info("session cookie acquired", "cookies", cookieNames)
			return nil
		}
	}
	logger.Error("CPSMART_PUBLIC_AUTH cookie missing", "received_cookies", cookieNames)
	return fmt.Errorf("CPSMART_PUBLIC_AUTH cookie が取得できませんでした (受信 cookie: %v)", cookieNames)
}
