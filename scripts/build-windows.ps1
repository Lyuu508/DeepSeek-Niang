[CmdletBinding()]
param([switch]$CheckOnly)
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Run this script on Windows 10/11 x64 with MSVC C++ Build Tools and WebView2 installed.' }
$ProjectRoot = Split-Path -Parent $PSScriptRoot
function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command failed with exit code $LASTEXITCODE" }
}
foreach ($Tool in @('node','npm','cargo','rustup')) { if (-not (Get-Command $Tool -ErrorAction SilentlyContinue)) { throw "Missing prerequisite: $Tool" } }
Invoke-Checked 'node' @('--version')
Invoke-Checked 'cargo' @('--version')
if ($CheckOnly) { Write-Host 'Command checks complete. MSVC/WebView2 and real credential storage still require native build/runtime checks.'; exit 0 }
Push-Location -LiteralPath $ProjectRoot
try {
    Invoke-Checked 'node' @('scripts/check-branding.mjs')
    Invoke-Checked 'rustup' @('target','add','x86_64-pc-windows-msvc')
    Invoke-Checked 'npm' @('ci','--ignore-scripts')
    # On Windows this also runs the actual Windows Credential Manager round-trip test.
    Invoke-Checked 'cargo' @('test','--workspace','--locked')
    Invoke-Checked 'npm' @('run','tauri','--','build','--target','x86_64-pc-windows-msvc','--bundles','nsis')
    $TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $ProjectRoot 'apps/companion/src-tauri/target' }
    $BundleRoot = Join-Path $TargetRoot 'x86_64-pc-windows-msvc/release/bundle'
    $Installers = @(Get-ChildItem -LiteralPath $BundleRoot -Filter '*.exe' -Recurse)
    if ($Installers.Count -eq 0) { throw 'Build returned without an EXE installer. No installation success is claimed.' }
    $Installers | ForEach-Object { Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName }
    Write-Host 'Installer files exist. Install and test on Windows separately; this script does not mark runtime or real API tests as passed.'
} finally { Pop-Location }
