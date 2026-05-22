package fetch

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"

	"github.com/gorilla/websocket"
)

// SignalRConfig holds the URLs needed to establish a SignalR connection to KULAS.
type SignalRConfig struct {
	NegotiateURL string // https://.../dashboard/signalr/negotiate
	ConnectURL   string // wss://.../dashboard/signalr/connect
	StartURL     string // https://.../dashboard/signalr/start
	HubName      string // "roothub" for KULAS
	WaitTimeout  time.Duration
	DumpDir      string // empty = no frame dump
}

// DefaultSignalRConfig returns the production KULAS SignalR config.
func DefaultSignalRConfig() SignalRConfig {
	return SignalRConfig{
		NegotiateURL: "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/signalr/negotiate",
		ConnectURL:   "wss://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/signalr/connect",
		StartURL:     "https://kulas.kochi-u.ac.jp/cpsmart/public/dashboard/signalr/start",
		HubName:      "roothub",
		WaitTimeout:  15 * time.Second,
	}
}

type negotiateResponse struct {
	URL              string  `json:"Url"`
	ConnectionToken  string  `json:"ConnectionToken"`
	ConnectionID     string  `json:"ConnectionId"`
	KeepAliveTimeout float64 `json:"KeepAliveTimeout"`
	TryWebSockets    bool    `json:"TryWebSockets"`
	ProtocolVersion  string  `json:"ProtocolVersion"`
}

var tokenRegex = regexp.MustCompile(`"token"\s*:\s*"([a-f0-9]{64})"`)

// fetchTokenViaSignalR walks the SignalR negotiate → connect → start handshake,
// then reads WebSocket frames until one of them contains an entryContext token.
func fetchTokenViaSignalR(ctx context.Context, httpClient *http.Client, cfg SignalRConfig) (string, error) {
	logger := slog.With("op", "signalr_token", "hub", cfg.HubName)
	connData := fmt.Sprintf(`[{"name":"%s"}]`, cfg.HubName)

	logger.Info("SignalR handshake start", "negotiate_url", cfg.NegotiateURL)

	neg, err := signalrNegotiate(ctx, httpClient, cfg.NegotiateURL, connData)
	if err != nil {
		return "", err
	}
	if !neg.TryWebSockets {
		return "", fmt.Errorf("SignalR サーバが WebSocket をサポートしていません (ProtocolVersion=%s)", neg.ProtocolVersion)
	}
	logger.Info("negotiate done",
		"connection_id", neg.ConnectionID,
		"protocol", neg.ProtocolVersion,
		"keepalive_sec", neg.KeepAliveTimeout,
	)

	conn, err := signalrConnect(ctx, httpClient, cfg.ConnectURL, neg.ConnectionToken, connData)
	if err != nil {
		return "", err
	}
	defer func() { _ = conn.Close() }()
	logger.Info("websocket connected", "url", cfg.ConnectURL)

	if err := signalrStart(ctx, httpClient, cfg.StartURL, neg.ConnectionToken, connData); err != nil {
		return "", err
	}
	logger.Info("start signaled, awaiting token frames", "timeout", cfg.WaitTimeout, "dump_dir", cfg.DumpDir)

	return readUntilToken(conn, cfg.WaitTimeout, cfg.DumpDir)
}

func signalrNegotiate(ctx context.Context, httpClient *http.Client, base, connData string) (*negotiateResponse, error) {
	u := fmt.Sprintf("%s?clientProtocol=2.0&connectionData=%s&_=%d",
		base, url.QueryEscape(connData), time.Now().UnixMilli())
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", defaultUserAgent)
	req.Header.Set("Accept", "*/*")
	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("SignalR negotiate HTTP に失敗: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("SignalR negotiate が HTTP %d を返しました", resp.StatusCode)
	}
	var neg negotiateResponse
	if err := json.NewDecoder(resp.Body).Decode(&neg); err != nil {
		return nil, fmt.Errorf("SignalR negotiate レスポンスのパースに失敗: %w", err)
	}
	return &neg, nil
}

func signalrConnect(ctx context.Context, httpClient *http.Client, base, connToken, connData string) (*websocket.Conn, error) {
	u := fmt.Sprintf("%s?transport=webSockets&clientProtocol=2.0&connectionToken=%s&connectionData=%s&tid=0",
		base, url.QueryEscape(connToken), url.QueryEscape(connData))

	header := http.Header{}
	header.Set("User-Agent", defaultUserAgent)
	header.Set("Origin", "https://kulas.kochi-u.ac.jp")
	if httpClient.Jar != nil {
		negURL, _ := url.Parse(strings.Replace(base, "wss://", "https://", 1))
		for _, ck := range httpClient.Jar.Cookies(negURL) {
			header.Add("Cookie", ck.Name+"="+ck.Value)
		}
	}

	dialer := *websocket.DefaultDialer
	dialer.TLSClientConfig = newKulasTLSConfig()
	conn, resp, err := dialer.DialContext(ctx, u, header)
	if err != nil {
		extra := ""
		if resp != nil {
			body, _ := io.ReadAll(resp.Body)
			_ = resp.Body.Close()
			extra = fmt.Sprintf(" (HTTP %d: %s)", resp.StatusCode, string(body))
		}
		return nil, fmt.Errorf("SignalR WebSocket 接続に失敗%s: %w", extra, err)
	}
	if resp != nil {
		_ = resp.Body.Close()
	}
	return conn, nil
}

func signalrStart(ctx context.Context, httpClient *http.Client, base, connToken, connData string) error {
	u := fmt.Sprintf("%s?transport=webSockets&clientProtocol=2.0&connectionToken=%s&connectionData=%s&_=%d",
		base, url.QueryEscape(connToken), url.QueryEscape(connData), time.Now().UnixMilli())
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return err
	}
	req.Header.Set("User-Agent", defaultUserAgent)
	req.Header.Set("Accept", "*/*")
	resp, err := httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("SignalR start HTTP に失敗: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()
	_, _ = io.Copy(io.Discard, resp.Body)
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("SignalR start が HTTP %d を返しました", resp.StatusCode)
	}
	return nil
}

func readUntilToken(conn *websocket.Conn, waitTimeout time.Duration, dumpDir string) (string, error) {
	logger := slog.With("op", "signalr_recv")
	deadline := time.Now().Add(waitTimeout)
	if err := conn.SetReadDeadline(deadline); err != nil {
		return "", fmt.Errorf("WebSocket read deadline 設定に失敗: %w", err)
	}

	var (
		frameCount int
		lastFrame  string
		start      = time.Now()
	)
	const previewMax = 300

	for {
		msgType, msg, err := conn.ReadMessage()
		if err != nil {
			elapsed := time.Since(start)
			logger.Error("recv error before token",
				"frames_received", frameCount,
				"elapsed_ms", elapsed.Milliseconds(),
				"last_frame_preview", lastFrame,
				"error", err.Error(),
			)
			return "", fmt.Errorf("SignalR メッセージ受信中にエラー (token 未取得、frames=%d, last=%q): %w",
				frameCount, lastFrame, err)
		}
		frameCount++
		dumpFrame(dumpDir, frameCount, msg)
		preview := string(msg)
		if len(preview) > previewMax {
			preview = preview[:previewMax] + "..."
		}
		lastFrame = preview

		logger.Debug("frame received",
			"#", frameCount,
			"type", websocketMessageTypeName(msgType),
			"bytes", len(msg),
			"preview", preview,
		)

		if m := tokenRegex.FindSubmatch(msg); m != nil {
			logger.Info("token extracted from SignalR frame",
				"frame_no", frameCount,
				"elapsed_ms", time.Since(start).Milliseconds(),
			)
			return string(m[1]), nil
		}
		if time.Now().After(deadline) {
			logger.Error("token wait timeout",
				"frames_received", frameCount,
				"elapsed_ms", time.Since(start).Milliseconds(),
				"last_frame_preview", lastFrame,
			)
			return "", fmt.Errorf("SignalR から token を %s 以内に受信できませんでした (受信 %d frames、最後の frame: %q)",
				waitTimeout, frameCount, lastFrame)
		}
	}
}

func websocketMessageTypeName(t int) string {
	switch t {
	case websocket.TextMessage:
		return "text"
	case websocket.BinaryMessage:
		return "binary"
	case websocket.CloseMessage:
		return "close"
	case websocket.PingMessage:
		return "ping"
	case websocket.PongMessage:
		return "pong"
	default:
		return fmt.Sprintf("type_%d", t)
	}
}
