# ============================================================================
# App - Eigenständiges Build-Skript
# ============================================================================
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$appName = Split-Path $root -Leaf

Write-Host "== Baue $appName (Release) ==" -ForegroundColor Cyan
Push-Location $root
cargo build --release --color never
if ($LASTEXITCODE -ne 0) {
    Write-Host "FEHLGESCHLAGEN beim Kompilieren von $appName!" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

Write-Host "FERTIG! $appName wurde erfolgreich gebaut." -ForegroundColor Green