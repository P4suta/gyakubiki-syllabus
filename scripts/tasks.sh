#!/bin/sh
set -eu

task=${1:-}
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command is not installed: %s\n' "$1" >&2
    exit 1
  fi
}

build_dataset() {
  set -- raw/*.json
  if [ ! -e "$1" ]; then
    printf 'No raw/*.json inputs found\n' >&2
    exit 1
  fi
  source_commit=$(git rev-parse HEAD)
  cargo run --locked --release -q -p syllabus-cli -- \
    build-dataset "$@" --details-dir raw-details --output web/public \
    --source-commit "$source_commit"
}

run_check() {
  cargo fmt --all -- --check
  cargo clippy --locked --workspace --all-targets -- -D warnings
  cargo test --locked --workspace --all-targets
  cargo llvm-cov nextest --locked --workspace --exclude syllabus-wasm --no-report
  cargo llvm-cov report --fail-under-lines 80
  RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
  cargo run --locked -q -p syllabus-cli -- gen-field-docs --check
  cargo run --locked -q -p syllabus-cli -- gen-palette --check
  wasm-pack test --node crates/wasm
  wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus
  cargo audit --deny warnings
  cargo deny check
  cargo cyclonedx --format json --all
  typos
  actionlint
  markdownlint-cli2 '**/*.md' '!web/**' '!node_modules/**' '!target/**'
  git diff --check
  (
    cd web
    bun install --frozen-lockfile
    bun run check
    bun run lint
    bun run test:cov
    bun audit
    bun run sbom
  )
}

case "$task" in
  setup)
    require_command mise
    mise install
    (
      cd web
      bun install --frozen-lockfile
      bun run playwright:install chromium firefox webkit
    )
    ;;
  doctor)
    for command_name in cargo bun wasm-pack git; do
      require_command "$command_name"
    done
    cargo --version
    bun --version
    wasm-pack --version
    git --version
    cargo llvm-cov --version
    cargo nextest --version
    cargo audit --version
    cargo deny --version
    cargo cyclonedx --version
    typos --version
    actionlint --version
    markdownlint-cli2 --version
    printf 'doctor: all required commands are available\n'
    ;;
  build-dataset)
    build_dataset
    ;;
  check)
    run_check
    ;;
  release-check)
    run_check
    build_dataset
    wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus --release
    (
      cd web
      bun install --frozen-lockfile
      GITHUB_PAGES=true bun run build
      bun run test:production-html
      bun scripts/check-artifact-budget.ts dist
      bun scripts/check-search-performance.ts public
      bun run test:e2e
      bun run test:lighthouse
    )
    ;;
  *)
    printf 'Usage: %s {setup|doctor|check|build-dataset|release-check}\n' "$0" >&2
    exit 2
    ;;
esac
