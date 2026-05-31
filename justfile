set shell := ["bash", "-cu"]

# Default: list available recipes
default:
    @just --list --unsorted

# === ビルド・実行 ===

# Go バイナリをビルド
build:
    go build -o bin/syllabus-cli ./cmd/syllabus-cli

# raw/ から web/public/data.json を生成 (Rust pipeline)
convert:
    cargo run --release -q -p syllabus-cli -- convert raw/*.json --v2 --compact -o web/public/data.json

# 生 JSON を inspect (使い方: just inspect raw/講義データ.json)
inspect file:
    go run ./cmd/syllabus-cli inspect {{file}}

# Rust core を WASM にビルドし web/src/wasm へ出力 (web ビルド/dev の前段依存)
# 注意: wasm-pack の --out-dir は *crate ルート* (crates/wasm) 相対なので、
# リポジトリ直下の web/src/wasm に出すには ../../ が必須 (engine.ts の import 先)。
wasm-build:
    wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus

# Web 側 dev server (WASM を先にビルド)
dev: wasm-build
    cd web && bun install && bun run dev

# Web 側 production build (data.json + WASM を先に用意)
web-build: convert wasm-build
    cd web && bun install --frozen-lockfile && bun run build

# === テスト ===

# 全テスト (Rust + Go + Web)
test: test-rust test-go test-web

# Rust core の test (パリティの正解オラクル)
test-rust:
    cargo test

# Go の test を -race 付きで
test-go:
    go test -race ./...

# Web 側のテスト + check (WASM を先にビルド)
test-web: wasm-build
    cd web && bun install --frozen-lockfile && bun run check && bun run test

# 移行期のパリティゲート: Go と Rust の convert 出力が byte 一致するか検証。
# generatedAt は wall-clock で必ず違うので比較前にブランク化する。
parity: build
    cargo build --release -p syllabus-cli
    ./bin/syllabus-cli convert raw/*.json --v2 --compact -o /tmp/parity-go.json
    "${CARGO_TARGET_DIR:-target}/release/syllabus-cli" convert raw/*.json --v2 --compact -o /tmp/parity-rs.json
    sed -E 's/"generatedAt":"[^"]*"/"generatedAt":""/' /tmp/parity-go.json > /tmp/parity-go.norm
    sed -E 's/"generatedAt":"[^"]*"/"generatedAt":""/' /tmp/parity-rs.json > /tmp/parity-rs.norm
    cmp /tmp/parity-go.norm /tmp/parity-rs.norm && echo "✅ Go と Rust の convert は byte 一致"

# === フォーマット・リント ===

# 全部 format (Rust + Go + YAML + JSON など、まとめて自動修正)
fmt:
    cargo fmt
    gofumpt -w .
    -typos --write-changes 2>/dev/null || echo "(typos: install with 'cargo install typos-cli' or use just install-tools)"

# 全 linter (CI と同じ)
lint: lint-rust lint-go lint-typos lint-actions lint-md

# Rust の lint (clippy を警告=エラー扱い + fmt 検査)
lint-rust:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

# Go の lint
lint-go:
    golangci-lint run

# typos check
lint-typos:
    typos

# GitHub Actions workflow lint
lint-actions:
    actionlint

# Markdown lint (web/ は別管理なので除外)
lint-md:
    markdownlint-cli2 "**/*.md" "#node_modules" "#web/**"

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
