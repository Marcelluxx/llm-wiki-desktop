param(
    [string]$OutputRoot = ""
)

$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $repositoryRoot ".runtime"
}
$runtimeRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$repositoryPrefix = $repositoryRoot.TrimEnd('\') + '\'
if (-not $runtimeRoot.StartsWith($repositoryPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "The release runtime must be created inside the repository."
}

$pythonCommand = Get-Command python -ErrorAction Stop
$uvCommand = Get-Command uv -ErrorAction Stop
if ([string]::IsNullOrWhiteSpace($env:JAVA_HOME)) {
    throw "JAVA_HOME is required. Configure a portable Java 21 runtime before packaging."
}
$javaSource = [System.IO.Path]::GetFullPath($env:JAVA_HOME)
if (-not (Test-Path -LiteralPath (Join-Path $javaSource "bin\java.exe") -PathType Leaf)) {
    throw "JAVA_HOME does not contain bin\java.exe."
}

$pythonSource = (& $pythonCommand.Source -c "import sys; print(sys.base_prefix)").Trim()
if (-not (Test-Path -LiteralPath (Join-Path $pythonSource "python.exe") -PathType Leaf)) {
    throw "The selected Python installation is not a complete Windows runtime."
}

if (Test-Path -LiteralPath $runtimeRoot) {
    $resolvedRuntime = (Resolve-Path -LiteralPath $runtimeRoot).Path
    if (-not $resolvedRuntime.StartsWith($repositoryPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to replace a runtime outside the repository."
    }
    Remove-Item -LiteralPath $resolvedRuntime -Recurse -Force
}

$pythonTarget = Join-Path $runtimeRoot "python"
$javaTarget = Join-Path $runtimeRoot "java"
New-Item -ItemType Directory -Force -Path $pythonTarget, $javaTarget | Out-Null

Write-Host "Copying the private Python runtime..."
Copy-Item -Path (Join-Path $pythonSource "*") -Destination $pythonTarget -Recurse -Force

$requirements = Join-Path $runtimeRoot "requirements.lock.txt"
Push-Location $repositoryRoot
try {
    & $uvCommand.Source export --quiet --locked --no-dev --no-emit-project --format requirements.txt --output-file $requirements
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $packagedPython = Join-Path $pythonTarget "python.exe"
    & $uvCommand.Source pip install --python $packagedPython --system --break-system-packages --require-hashes --requirements $requirements
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & $uvCommand.Source pip install --python $packagedPython --system --break-system-packages --no-deps .
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

# NSIS cannot reliably materialize the very deep license paths shipped by a
# few Python wheels (notably PyTorch). Keep a complete, compact license
# inventory in the private runtime and remove only the duplicated wheel
# subdirectories.  Nothing importable is removed: these folders contain
# metadata/licenses only.
$licenseInventory = Join-Path $runtimeRoot "licenses"
New-Item -ItemType Directory -Force -Path $licenseInventory | Out-Null
$licenseRecords = @()
$sitePackages = Join-Path $pythonTarget "Lib\site-packages"
if (Test-Path -LiteralPath $sitePackages -PathType Container) {
    Get-ChildItem -LiteralPath $sitePackages -Directory -Filter "*.dist-info" | ForEach-Object {
        $sourceLicense = Join-Path $_.FullName "licenses"
        if (Test-Path -LiteralPath $sourceLicense -PathType Container) {
            $safePackageName = ($_.Name -replace "[^A-Za-z0-9._-]", "_")
            $destination = Join-Path $licenseInventory $safePackageName
            New-Item -ItemType Directory -Force -Path $destination | Out-Null
            $licenseIndex = 0
            Get-ChildItem -LiteralPath $sourceLicense -File -Recurse | ForEach-Object {
                $licenseIndex++
                $safeFileName = ($_.Name -replace "[^A-Za-z0-9._-]", "_")
                if ($safeFileName.Length -gt 80) {
                    $safeFileName = $safeFileName.Substring(0, 80)
                }
                $shortName = "{0:D5}-{1}" -f $licenseIndex, $safeFileName
                Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $destination $shortName) -Force
            }
            $licenseRecords += [ordered]@{
                package = $_.Name
                files = $licenseIndex
            }
            Remove-Item -LiteralPath $sourceLicense -Recurse -Force
        }
    }
}
$licenseRecords | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $licenseInventory "licenses-manifest.json") -Encoding utf8

Write-Host "Copying the private Java runtime..."
Copy-Item -Path (Join-Path $javaSource "*") -Destination $javaTarget -Recurse -Force

$packagedPython = Join-Path $pythonTarget "python.exe"
$packagedJava = Join-Path $javaTarget "bin\java.exe"
& $packagedPython -c "import llm_wiki_engine, opendataloader_pdf; print('Packaged Python worker ready')"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& $packagedJava -version
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$manifest = [ordered]@{
    applicationVersion = "0.8.2"
    architecture = "x86_64"
    pythonVersion = (& $packagedPython --version 2>&1).ToString()
    javaVersion = (& $packagedJava -version 2>&1 | Select-Object -First 1).ToString()
    pythonSha256 = (Get-FileHash -LiteralPath $packagedPython -Algorithm SHA256).Hash.ToLowerInvariant()
    javaSha256 = (Get-FileHash -LiteralPath $packagedJava -Algorithm SHA256).Hash.ToLowerInvariant()
}
$manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runtimeRoot "runtime-manifest.json") -Encoding utf8

Write-Host "Release runtime ready at $runtimeRoot"
