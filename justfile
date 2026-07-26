set shell := ["sh", "-eu", "-c"]
set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

task-runner := if os_family() == "windows" { "./scripts/tasks.ps1" } else { "sh ./scripts/tasks.sh" }

default:
    just --list --unsorted

# Install every version-pinned development dependency.
setup:
    {{task-runner}} setup

# Report missing prerequisites without modifying the workspace.
doctor:
    {{task-runner}} doctor

# Build and atomically publish the manifest-selected production dataset.
build-dataset:
    {{task-runner}} build-dataset

# Run the required local Rust/WASM/Web checks.
check:
    {{task-runner}} check

# Produce and validate the exact GitHub Pages artifact.
release-check:
    {{task-runner}} release-check

dev:
    wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus
    cd web && bun install --frozen-lockfile && bun run dev

e2e:
    cd web && bun install --frozen-lockfile && bun run playwright:install chromium && bun run test:e2e

test-wasm:
    wasm-pack test --node crates/wasm

cov:
    cargo llvm-cov nextest --locked --workspace --exclude syllabus-wasm --summary-only

install-hooks:
    lefthook install

gen-field-docs:
    cargo run --locked -q -p syllabus-cli -- gen-field-docs

fetch-details *ARGS:
    cargo run --locked --release -q -p syllabus-cli -- fetch-details {{ARGS}}

fuzz TARGET *ARGS:
    cd crates/cli/fuzz && cargo +nightly fuzz run {{TARGET}} -- -max_total_time=60 {{ARGS}}

mutants *ARGS:
    cargo mutants --timeout 60 {{ARGS}}

stryker:
    cd web && bun run mutation
