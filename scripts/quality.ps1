$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$toolRoot = Join-Path $repositoryRoot ".tools"
$env:RUSTUP_HOME = Join-Path $toolRoot "rustup"
$env:CARGO_HOME = Join-Path $toolRoot "cargo"
$env:PATH = "$(Join-Path $env:CARGO_HOME 'bin');$env:PATH"

Push-Location $repositoryRoot
try {
    npm run check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    uv lock --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    uv run ruff format --check .
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    uv run ruff check .
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    uv run mypy
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    uv run pytest
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
