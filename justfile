set shell := ["bash", "-cu"]

# Default: list available recipes
default:
    @just --list --unsorted

# === ビルド・実行 ===

# Go バイナリをビルド
build:
    go build -o bin/syllabus-cli ./cmd/syllabus-cli

# raw/ から web/public/data.json を生成
convert: build
    ./bin/syllabus-cli convert raw/*.json --v2 --compact -o web/public/data.json

# 生 JSON を inspect (使い方: just inspect raw/講義データ.json)
inspect file:
    go run ./cmd/syllabus-cli inspect {{file}}

# Web 側 dev server
dev:
    cd web && bun install && bun run dev

# Web 側 production build
web-build: convert
    cd web && bun install --frozen-lockfile && bun run build

# === テスト ===

# 全テスト (Go + Web)
test: test-go test-web

# Go の test を -race 付きで
test-go:
    go test -race ./...

# Web 側のテスト + check
test-web:
    cd web && bun install --frozen-lockfile && bun run check && bun run test

# === フォーマット・リント ===

# 全部 format (Go + YAML + JSON など、まとめて自動修正)
fmt:
    gofumpt -w .
    -typos --write-changes 2>/dev/null || echo "(typos: install with 'cargo install typos-cli' or use just install-tools)"

# 全 linter (CI と同じ)
lint: lint-go lint-typos lint-actions lint-md

# Go の lint
lint-go:
    golangci-lint run

# typos check
lint-typos:
    typos

# GitHub Actions workflow lint
lint-actions:
    actionlint

# Markdown lint
lint-md:
    markdownlint-cli2 "**/*.md" "#node_modules" "#web/dist"

# CI と等価のチェックを全部 (lefthook pre-push と同じ範囲)
check: lint test

# === セットアップ ===

# 開発ツール一括 install (mise 経由を推奨)
install-tools:
    @if command -v mise > /dev/null; then \
      mise install; \
    else \
      echo "→ mise が入っていません: https://mise.jdx.dev/ を install してから再実行"; \
      echo "  (fallback) go install mvdan.cc/gofumpt@latest"; \
      echo "  (fallback) go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@latest"; \
      echo "  (fallback) cargo install typos-cli"; \
      echo "  (fallback) bun install -g markdownlint-cli2"; \
      exit 1; \
    fi

# git hooks (pre-commit / pre-push) を install
install-hooks:
    lefthook install
