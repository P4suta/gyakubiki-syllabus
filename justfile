set shell := ["bash", "-cu"]

# List recipes.
default:
    @just --list --unsorted

# === Build / run ===

# raw/ (+ raw-details/ when present) → web/public/data.json + details/.
convert:
    cargo run --release -q -p syllabus-cli -- convert raw/*.json --compact --details-dir raw-details -o web/public/data.json

# Generate FIELD_SPEC artifacts (TS for the frontend, docs/syllabus-fields.md).
gen-field-docs:
    cargo run -q -p syllabus-cli -- gen-field-docs

# Rust core → WASM into web/src/wasm (--out-dir is crate-root relative, hence ../../).
wasm-build:
    wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus

# Dev server (builds WASM first).
dev: wasm-build
    cd web && bun install && bun run dev

# Synthesize a KULAS-free dummy dataset into dev-data/ (--count/--seed to vary).
gen-sample *ARGS:
    cargo run -q -p syllabus-cli -- gen-sample {{ARGS}}

# Build data.json + details from the dummy dataset, then run dev (no KULAS access).
dev-sample: wasm-build gen-sample
    cargo run -q -p syllabus-cli -- convert dev-data/sample-raw.json --compact --details-dir dev-data/sample-details --details-out web/public/details -o web/public/data.json
    cd web && bun install && bun run dev

# Production web build.
web-build: convert wasm-build
    cd web && bun install --frozen-lockfile && bun run build

# Crawl KULAS detail pages into raw-details/ (Actions runs this; pass args for local dry runs).
fetch-details *ARGS:
    cargo run --release -q -p syllabus-cli -- fetch-details {{ARGS}}

# === Test ===

test: test-rust test-wasm test-web

# Rust tests via nextest (the workspace has no doctests, so nextest is complete).
test-rust:
    cargo nextest run

# WASM boundary tests (JsValue shapes) in Node — needs wasm-pack.
test-wasm:
    wasm-pack test --node crates/wasm

test-web: wasm-build
    cd web && bun install --frozen-lockfile && bun run check && bun run test

# Full E2E (Playwright) against the sample dataset — heavy; CI runs it too. The
# visual specs self-skip off CI. Rewrites web/public with the sample dataset;
# run `just convert` afterwards to restore your real data for `just dev`.
e2e: wasm-build gen-sample
    cargo run --release -q -p syllabus-cli -- convert dev-data/sample-raw.json --compact --details-dir dev-data/sample-details --details-out web/public/details -o web/public/data.json
    cd web && bun install --frozen-lockfile && bunx playwright install chromium && bun run test:e2e

# Native line coverage via nextest (excludes the wasm-bindgen surface).
cov:
    cargo llvm-cov nextest --workspace --exclude syllabus-wasm --summary-only

# Continuous fuzzing (nightly + cargo-fuzz). Pass a target, e.g. `just fuzz fuzz_parse_jikanwari`.
fuzz TARGET *ARGS:
    cd crates/cli/fuzz && cargo +nightly fuzz run {{TARGET}} -- -max_total_time=60 {{ARGS}}

# Mutation testing (weekly in CI, slow): do the tests actually kill injected bugs?
# Scope (core + tested cli logic) lives in .cargo/mutants.toml. Pass `--in-diff <file>`
# for a fast changed-lines-only run.
mutants *ARGS:
    cargo mutants --timeout 60 {{ARGS}}

# Mutation testing for the web's pure lib modules (see web/stryker.conf.json).
stryker:
    cd web && bunx stryker run

# === Format / lint ===

fmt:
    cargo fmt
    -typos --write-changes 2>/dev/null || echo "(typos: cargo install typos-cli, or just install-tools)"

lint: lint-rust lint-typos lint-actions lint-md

lint-rust:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

lint-typos:
    typos

lint-actions:
    actionlint

lint-md:
    markdownlint-cli2 "**/*.md" "#node_modules" "#web/**"

# CI-equivalent checks (matches lefthook pre-push).
check: lint test

# === Setup ===

install-tools:
    @if command -v mise > /dev/null; then \
      mise install; \
    else \
      echo "→ mise not found: install from https://mise.jdx.dev/ and retry"; \
      echo "  (fallback) cargo install typos-cli"; \
      echo "  (fallback) bun install -g markdownlint-cli2"; \
      exit 1; \
    fi

install-hooks:
    lefthook install
