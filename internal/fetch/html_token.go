package fetch

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"regexp"
)

// cpSmartVueStartupRe matches the inline script in the search page HTML:
//
//	cpSmartVueStartup('dash-app-main', '<version>', true|false, '<base64-json>')
//
// The 4th positional argument is a base64-encoded JSON literal containing the
// entryContext object (token / userId / kinoId / etc.). KULAS の Razor view
// が server-side で render するので、HTML 取得時点で常に有効な token が手に入る。
var cpSmartVueStartupRe = regexp.MustCompile(
	`cpSmartVueStartup\(\s*'dash-app-main'\s*,\s*'[^']+'\s*,\s*\w+\s*,\s*'([A-Za-z0-9+/=]+)'`,
)

// ErrTokenNotFound は HTML から token が抽出できなかった場合に返される sentinel error。
// KULAS の HTML 構造が変わった (cpSmartVueStartup の呼び出し方や引数順が変更) と
// 推測されるシグナル。
var ErrTokenNotFound = errors.New("entryContext token が HTML から抽出できませんでした")

// entryContext は cpSmartVueStartup 第 4 引数 (base64-JSON) のうち本プロジェクトが
// 使うフィールドのみを定義した部分構造体。他のフィールドは無視。
type entryContext struct {
	Token string `json:"token"`
}

// extractTokenFromHTML はシラバス検索ページの HTML から findPage で使う token
// (`tempData.entryContext.token`, 64-char hex) を抽出する。
//
// 手順:
//  1. inline script cpSmartVueStartup('dash-app-main', ..., '<base64>') から base64 を取り出す
//  2. base64 デコードして JSON を得る
//  3. JSON.token を返す
//
// 失敗時はどのステップでこけたかが分かる error を返す。
func extractTokenFromHTML(html []byte) (string, error) {
	m := cpSmartVueStartupRe.FindSubmatch(html)
	if m == nil {
		return "", fmt.Errorf("%w: cpSmartVueStartup('dash-app-main', ...) inline script が見つかりません (HTML 構造変更の可能性)", ErrTokenNotFound)
	}
	decoded, err := base64.StdEncoding.DecodeString(string(m[1]))
	if err != nil {
		return "", fmt.Errorf("entryContext の base64 デコードに失敗: %w", err)
	}
	var ec entryContext
	if err := json.Unmarshal(decoded, &ec); err != nil {
		return "", fmt.Errorf("entryContext の JSON パースに失敗: %w", err)
	}
	if ec.Token == "" {
		return "", fmt.Errorf("%w: entryContext.token が空", ErrTokenNotFound)
	}
	return ec.Token, nil
}
