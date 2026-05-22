package main

import (
	"fmt"
	"io"
	"log/slog"
	"os"
	"runtime/debug"
	"strings"
)

// LogConfig は CLI 全体の構造化ログ設定。
type LogConfig struct {
	Level     string // debug | info | warn | error
	Format    string // text | json
	Output    io.Writer
	AddSource bool // ログにファイル名と行番号を含めるか (debug 向け)
}

// setupLogger は slog.SetDefault を初期化する。
// PersistentPreRunE から各 subcommand 実行前に呼び出される。
func setupLogger(cfg LogConfig) error {
	level, err := parseLogLevel(cfg.Level)
	if err != nil {
		return err
	}

	out := cfg.Output
	if out == nil {
		out = os.Stderr
	}

	opts := &slog.HandlerOptions{
		Level:     level,
		AddSource: cfg.AddSource || level == slog.LevelDebug,
	}

	var handler slog.Handler
	switch strings.ToLower(cfg.Format) {
	case "json":
		handler = slog.NewJSONHandler(out, opts)
	case "", "text":
		handler = slog.NewTextHandler(out, opts)
	default:
		return fmt.Errorf("無効な --log-format: %q (text | json)", cfg.Format)
	}

	slog.SetDefault(slog.New(handler))
	slog.Debug("logger initialized", "level", level.String(), "format", cfg.Format)
	return nil
}

func parseLogLevel(s string) (slog.Level, error) {
	switch strings.ToLower(s) {
	case "", "info":
		return slog.LevelInfo, nil
	case "debug":
		return slog.LevelDebug, nil
	case "warn", "warning":
		return slog.LevelWarn, nil
	case "error":
		return slog.LevelError, nil
	}
	return 0, fmt.Errorf("無効な --log-level: %q (debug | info | warn | error)", s)
}

// recoverPanic は main() から defer で呼び出し、panic 時に stack trace を構造化ログに出して exit 2。
func recoverPanic() {
	if r := recover(); r != nil {
		slog.Error("panic recovered",
			"value", fmt.Sprintf("%v", r),
			"stack", string(debug.Stack()),
		)
		os.Exit(2)
	}
}
