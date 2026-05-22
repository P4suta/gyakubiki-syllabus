package fetch

import (
	"fmt"
	"log/slog"
	"net/http"
	"net/http/httputil"
	"os"
	"path/filepath"
	"sync/atomic"
	"time"
)

// DumpTransport は http.RoundTripper を wrap し、各リクエスト/レスポンスを
// timestamped ファイルとして Dir 配下に保存する。--debug-dump-dir で有効化。
// SignalR の WebSocket frame も同じ Dir に dump される (signalr.go 側)。
type DumpTransport struct {
	Base http.RoundTripper
	Dir  string
	seq  atomic.Int64
}

// RoundTrip は req/resp を dump してから委譲する。dump 自体が失敗しても通信は止めない (best-effort)。
func (t *DumpTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	n := t.seq.Add(1)
	base := filepath.Join(t.Dir, fmt.Sprintf("http_%04d", n))

	if reqBytes, err := httputil.DumpRequestOut(req, true); err == nil {
		_ = os.WriteFile(base+"_req.txt", reqBytes, 0o600)
	}

	resp, rtErr := t.Base.RoundTrip(req)
	if rtErr != nil {
		_ = os.WriteFile(base+"_err.txt", []byte(rtErr.Error()), 0o600)
		return resp, rtErr //nolint:wrapcheck // 透過的 wrap、エラーはそのまま返す
	}

	if respBytes, err := httputil.DumpResponse(resp, true); err == nil {
		_ = os.WriteFile(base+"_resp.txt", respBytes, 0o600)
	}
	return resp, nil
}

// InitDumpDir は base/YYYYMMDD-HHMMSS/ を作成し、そのパスを返す。
// base が空のときは ("", nil) を返す (dump 無効化)。
func InitDumpDir(base string) (string, error) {
	if base == "" {
		return "", nil
	}
	dir := filepath.Join(base, time.Now().UTC().Format("20060102-150405"))
	if err := os.MkdirAll(dir, 0o750); err != nil {
		return "", fmt.Errorf("debug dump dir 作成に失敗 %q: %w", dir, err)
	}
	slog.Info("debug dump dir created", "path", dir)
	return dir, nil
}
