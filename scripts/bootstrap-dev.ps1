$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$toolRoot = Join-Path $repositoryRoot ".tools"
$rustupHome = Join-Path $toolRoot "rustup"
$cargoHome = Join-Path $toolRoot "cargo"
$rustupInstaller = Join-Path $toolRoot "rustup-init.exe"

New-Item -ItemType Directory -Force -Path $toolRoot | Out-Null
$env:RUSTUP_HOME = $rustupHome
$env:CARGO_HOME = $cargoHome
$env:PATH = "$(Join-Path $cargoHome 'bin');$env:PATH"

if (-not (Test-Path -LiteralPath $rustupInstaller)) {
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInstaller
}

& $rustupInstaller -y --no-modify-path --profile minimal --default-toolchain 1.98.0-x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

rustup component add clippy rustfmt --toolchain 1.98.0-x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Push-Location $repositoryRoot
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    uv sync --locked --all-groups
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    if ($null -eq (Get-Command nvidia-smi -ErrorAction SilentlyContinue)) {
        Write-Host "No NVIDIA GPU detected. Skipping the optional CUDA download."
    }
    else {
        Write-Host "NVIDIA GPU detected. CUDA remains optional and can be enabled from Settings > Performance."
    }
}
finally {
    Pop-Location
}

Write-Host "Development environment ready. Run scripts/quality.ps1 to validate it."
