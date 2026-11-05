<#
RecordCraft - End-User Release Packager (Windows x64)

Creates a ZIP for end users:
  - recordcraft.exe
  - assets (if present)
  - optional VC++ Redistributable + Install.ps1

Output:
  .\dist\RecordCraft-windows-x64.zip
#>

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$NoVCRedist
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$distRoot = Join-Path $root 'dist'
$stageRoot = Join-Path $distRoot '_stage'
$appFolderName = 'RecordCraft'
$stage = Join-Path $stageRoot $appFolderName

$releaseDir = Join-Path $root 'target\release'
$exe = Join-Path $releaseDir 'recordcraft.exe'

function Ensure-Dir([string]$p) { New-Item -ItemType Directory -Force $p | Out-Null }

function Get-VcRedistPath {
    $cacheDir = Join-Path $distRoot '_cache'
    Ensure-Dir $cacheDir
    $cached = Join-Path $cacheDir 'vc_redist.x64.exe'
    if (Test-Path $cached) { return $cached }
    Write-Host '== Download VC++ Redistributable (x64) ==' -ForegroundColor Cyan
    $url = 'https://aka.ms/vs/17/release/vc_redist.x64.exe'
    Invoke-WebRequest -Uri $url -OutFile $cached -UseBasicParsing
    return $cached
}

function Write-InstallScript([string]$destPath) {
    $lines = @(
        '[CmdletBinding()]',
        'param(',
        "  [string]`$InstallDir = (Join-Path `$env:LOCALAPPDATA 'Programs\\ArtCraft\\RecordCraft')",
        ')',
        '',
        'Set-StrictMode -Version Latest',
        "`$ErrorActionPreference = 'Stop'",
        '',
        'function Test-VcRuntime {',
        "  `$keys = @(",
        "    'HKLM:\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64',",
        "    'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64'",
        '  )',
        '  foreach ($k in $keys) {',
        '    try {',
        '      $p = Get-ItemProperty -Path $k -ErrorAction Stop',
        '      if ($p.Installed -eq 1) { return $true }',
        '    } catch {}',
        '  }',
        '  return $false',
        '}',
        '',
        "Write-Host '== RecordCraft Installer ==' -ForegroundColor Cyan",
        '',
        'if (-not (Test-VcRuntime)) {',
        "  `$redist = Join-Path `$PSScriptRoot 'vc_redist.x64.exe'",
        '  if (Test-Path $redist) {',
        "    Write-Host 'Installing Microsoft Visual C++ Redistributable (2015-2022) ...' -ForegroundColor Yellow",
        "    Start-Process -FilePath `$redist -ArgumentList '/install','/passive','/norestart' -Wait",
        '  } else {',
        "    Write-Host 'WARNING: VC++ runtime not detected and vc_redist.x64.exe not found next to this installer.' -ForegroundColor Yellow",
        "    Write-Host 'Download: https://aka.ms/vs/17/release/vc_redist.x64.exe' -ForegroundColor Yellow",
        '  }',
        '}',
        '',
        'New-Item -ItemType Directory -Force $InstallDir | Out-Null',
        'Write-Host ("Copying files to: {0}" -f $InstallDir) -ForegroundColor Cyan',
        'Copy-Item -LiteralPath (Join-Path $PSScriptRoot ''*'') -Destination $InstallDir -Recurse -Force',
        '',
        "Write-Host ''",
        "Write-Host 'Done. Start RecordCraft via:' -ForegroundColor Green",
        "Write-Host (Join-Path `$InstallDir 'recordcraft.exe') -ForegroundColor Green"
    )
    Set-Content -LiteralPath $destPath -Value ($lines -join "`r`n") -Encoding UTF8
}

function Write-Readme([string]$destPath) {
    $lines = @(
        'RecordCraft (Windows) - Quick Install',
        '=====================================',
        '',
        '1) Unzip this folder anywhere (e.g. C:\ArtCraft\RecordCraft).',
        '2) Run Install.ps1 (recommended). It installs the VC++ runtime if needed and copies files to LocalAppData.',
        '3) Start RecordCraft via recordcraft.exe.'
    )
    Set-Content -LiteralPath $destPath -Value ($lines -join "`r`n") -Encoding UTF8
}

Write-Host '== RecordCraft Release Pack ==' -ForegroundColor Cyan

Ensure-Dir $distRoot
Ensure-Dir $stageRoot
if (Test-Path $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
Ensure-Dir $stage

if (-not $SkipBuild) {
    Write-Host '== Build (cargo build --release) ==' -ForegroundColor Cyan
    Push-Location $root
    try {
        cargo build --release --color never
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
    } finally { Pop-Location }
}

if (-not (Test-Path $exe)) {
    throw "Missing built exe: $exe (build first or omit -SkipBuild)"
}

Write-Host '== Stage exe ==' -ForegroundColor Cyan
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'recordcraft.exe') -Force

Write-Host '== Stage assets (optional) ==' -ForegroundColor Cyan
if (Test-Path (Join-Path $root 'assets')) {
    Copy-Item -LiteralPath (Join-Path $root 'assets') -Destination (Join-Path $stage 'assets') -Recurse -Force
}

Write-Host '== Stage docs/licenses (optional) ==' -ForegroundColor Cyan
foreach ($pat in @('LICENSE*','NOTICE*','THIRD_PARTY_NOTICES*','README*')) {
    Get-ChildItem -LiteralPath $root -File -Filter $pat -ErrorAction SilentlyContinue |
        ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $stage -Force }
}

Write-InstallScript -destPath (Join-Path $stage 'Install.ps1')
Write-Readme -destPath (Join-Path $stage 'README.txt')

if (-not $NoVCRedist) {
    $redist = Get-VcRedistPath
    Copy-Item -LiteralPath $redist -Destination (Join-Path $stage 'vc_redist.x64.exe') -Force
}

$zip = Join-Path $distRoot 'RecordCraft-windows-x64.zip'
if (Test-Path $zip) { Remove-Item -LiteralPath $zip -Force }

Write-Host "== Create ZIP: $zip ==" -ForegroundColor Cyan
Compress-Archive -LiteralPath $stage -DestinationPath $zip -Force

Write-Host ''
Write-Host 'DONE.' -ForegroundColor Green
Write-Host "ZIP: $zip" -ForegroundColor Green