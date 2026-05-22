package fetch

import (
	"crypto/tls"
	"crypto/x509"
	_ "embed"
)

// kulasCAPEM は KULAS の中間 CA (NII Open Domain CA - G7 RSA) を埋め込む。
// KULAS のサーバが TLS handshake で中間 CA を配信しないため、Go の crypto/tls
// が chain を補完できず "x509: certificate signed by unknown authority" になる。
// この PEM を RootCAs に積めば、root (Security Communication RootCA2) は
// システム pool に既にあるので chain が完成する。
//
// 証明書 rotate (有効期限: 2029-05-29) で fetch が unknown authority で
// fail し始めたら docs/kulas-api-spec.md の手順で再取得する。
//
//go:embed kulas_ca.pem
var kulasCAPEM []byte

// newKulasTLSConfig はシステム CA pool に KULAS 中間 CA を追加した TLS 設定を返す。
// HTTP client と WebSocket dialer で共通利用する。
func newKulasTLSConfig() *tls.Config {
	pool, err := x509.SystemCertPool()
	if err != nil || pool == nil {
		pool = x509.NewCertPool()
	}
	pool.AppendCertsFromPEM(kulasCAPEM)
	return &tls.Config{
		RootCAs:    pool,
		MinVersion: tls.VersionTLS12,
	}
}
