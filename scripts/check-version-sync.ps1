param(
    [string]$ExpectedVersion = ""
)

$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$expected = if ([string]::IsNullOrWhiteSpace($ExpectedVersion)) {
    (Get-Content -LiteralPath (Join-Path $repositoryRoot "package.json") -Raw | ConvertFrom-Json).version
} else {
    $ExpectedVersion
}

function Assert-JsonVersion([string]$RelativePath) {
    $path = Join-Path $repositoryRoot $RelativePath
    $value = (Get-Content -LiteralPath $path -Raw | ConvertFrom-Json).version
    if ($value -ne $expected) {
        throw "$RelativePath declares version $value; expected $expected."
    }
}

Assert-JsonVersion "package.json"
Assert-JsonVersion "apps\desktop\package.json"
Assert-JsonVersion "apps\desktop\src-tauri\tauri.conf.json"

$cargo = Get-Content -LiteralPath (Join-Path $repositoryRoot "Cargo.toml") -Raw
$python = Get-Content -LiteralPath (Join-Path $repositoryRoot "pyproject.toml") -Raw
$worker = Get-Content -LiteralPath (Join-Path $repositoryRoot "worker\llm_wiki_engine\runtime.py") -Raw
if ($cargo -notmatch "(?m)^version = `"$([regex]::Escape($expected))`"$") {
    throw "Cargo.toml is not synchronized to $expected."
}
if ($python -notmatch "(?m)^version = `"$([regex]::Escape($expected))`"$") {
    throw "pyproject.toml is not synchronized to $expected."
}
if ($worker -notmatch "(?m)^WORKER_VERSION = `"$([regex]::Escape($expected))`"$") {
    throw "The Python worker is not synchronized to $expected."
}

Write-Host "All application components declare version $expected."
