$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$python = Join-Path $repositoryRoot ".venv\Scripts\python.exe"

if (-not (Test-Path -LiteralPath $python -PathType Leaf)) {
    throw "Run scripts\bootstrap-dev.ps1 before enabling NVIDIA acceleration."
}

$nvidiaSmi = Get-Command nvidia-smi -ErrorAction SilentlyContinue
if ($null -eq $nvidiaSmi) {
    throw "No NVIDIA driver was detected. The CPU OCR runtime remains available."
}

Write-Host "Installing the official PyTorch CUDA 13.0 runtime in the isolated project environment..."
uv pip install `
    --python $python `
    --reinstall-package torch `
    --reinstall-package torchvision `
    --no-deps `
    --default-index https://download.pytorch.org/whl/cu130 `
    "torch==2.13.0+cu130" `
    "torchvision==0.28.0+cu130"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& $python -c "import torch; assert torch.cuda.is_available(), 'CUDA is not available'; print(f'NVIDIA OCR acceleration ready: {torch.cuda.get_device_name(0)}')"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
