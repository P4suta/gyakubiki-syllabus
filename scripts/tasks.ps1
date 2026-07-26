[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("setup", "doctor", "check", "build-dataset", "release-check")]
    [string] $Task
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repoRoot

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Command,
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]] $Arguments
    )
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

function Assert-Command {
    param([Parameter(Mandatory = $true)][string] $Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command is not installed: $Name"
    }
}

function Get-RawFiles {
    $files = Get-ChildItem -LiteralPath (Join-Path $repoRoot "raw") -Filter "*.json" -File |
        Sort-Object -Property Name |
        ForEach-Object { $_.FullName }
    if ($files.Count -eq 0) {
        throw "No raw/*.json inputs found"
    }
    return [string[]] $files
}

function Build-Dataset {
    $commit = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $commit) {
        throw "Could not resolve the source commit"
    }
    $arguments = @(
        "run", "--locked", "--release", "-q", "-p", "syllabus-cli", "--",
        "build-dataset"
    )
    $arguments += Get-RawFiles
    $arguments += @(
        "--details-dir", (Join-Path $repoRoot "raw-details"),
        "--output", (Join-Path $repoRoot "web/public"),
        "--source-commit", $commit
    )
    Invoke-Checked cargo @arguments
}

function Invoke-Rustdoc {
    $previous = $env:RUSTDOCFLAGS
    $env:RUSTDOCFLAGS = "-D warnings"
    try {
        Invoke-Checked cargo doc --locked --workspace --no-deps
    }
    finally {
        $env:RUSTDOCFLAGS = $previous
    }
}

function Invoke-Check {
    Invoke-Checked cargo fmt --all -- --check
    Invoke-Checked cargo clippy --locked --workspace --all-targets -- -D warnings
    Invoke-Checked cargo test --locked --workspace --all-targets
    Invoke-Checked cargo llvm-cov nextest --locked --workspace --exclude syllabus-wasm --no-report
    Invoke-Checked cargo llvm-cov report --fail-under-lines 80
    Invoke-Rustdoc
    Invoke-Checked cargo run --locked -q -p syllabus-cli -- gen-field-docs --check
    Invoke-Checked cargo run --locked -q -p syllabus-cli -- gen-palette --check
    Invoke-Checked wasm-pack test --node crates/wasm
    Invoke-Checked wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus
    Invoke-Checked cargo audit --deny warnings
    Invoke-Checked cargo deny check
    Invoke-Checked cargo cyclonedx --format json --all
    Invoke-Checked typos
    Invoke-Checked actionlint
    Invoke-Checked markdownlint-cli2 "**/*.md" "!web/**" "!node_modules/**" "!target/**"
    Invoke-Checked git diff --check
    Push-Location (Join-Path $repoRoot "web")
    try {
        Invoke-Checked bun install --frozen-lockfile
        Invoke-Checked bun run check
        Invoke-Checked bun run lint
        Invoke-Checked bun run test:cov
        Invoke-Checked bun audit
        Invoke-Checked bun run sbom
    }
    finally {
        Pop-Location
    }
}

switch ($Task) {
    "setup" {
        Assert-Command mise
        Invoke-Checked mise install
        Push-Location (Join-Path $repoRoot "web")
        try {
            Invoke-Checked bun install --frozen-lockfile
            Invoke-Checked bun run playwright:install chromium firefox webkit
        }
        finally {
            Pop-Location
        }
    }
    "doctor" {
        foreach ($command in @("cargo", "bun", "wasm-pack", "git")) {
            Assert-Command $command
        }
        Invoke-Checked cargo --version
        Invoke-Checked bun --version
        Invoke-Checked wasm-pack --version
        Invoke-Checked git --version
        Invoke-Checked cargo llvm-cov --version
        Invoke-Checked cargo nextest --version
        Invoke-Checked cargo audit --version
        Invoke-Checked cargo deny --version
        Invoke-Checked cargo cyclonedx --version
        Invoke-Checked typos --version
        Invoke-Checked actionlint --version
        Invoke-Checked markdownlint-cli2 --version
        Write-Output "doctor: all required commands are available"
    }
    "build-dataset" {
        Build-Dataset
    }
    "check" {
        Invoke-Check
    }
    "release-check" {
        Invoke-Check
        Build-Dataset
        Invoke-Checked wasm-pack build crates/wasm --target web --out-dir ../../web/src/wasm --out-name syllabus --release
        Push-Location (Join-Path $repoRoot "web")
        try {
            Invoke-Checked bun install --frozen-lockfile
            $previous = $env:GITHUB_PAGES
            $env:GITHUB_PAGES = "true"
            try {
                Invoke-Checked bun run build
            }
            finally {
                $env:GITHUB_PAGES = $previous
            }
            Invoke-Checked bun scripts/check-artifact-budget.ts dist
            Invoke-Checked bun scripts/check-search-performance.ts public
            Invoke-Checked bun run test:e2e
            Invoke-Checked bun run test:lighthouse
        }
        finally {
            Pop-Location
        }
    }
}
