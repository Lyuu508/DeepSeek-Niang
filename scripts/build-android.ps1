[CmdletBinding()]
param([switch]$CheckOnly)
$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command failed with exit code $LASTEXITCODE" }
}
foreach ($Tool in @('node','npm','cargo','rustup')) { if (-not (Get-Command $Tool -ErrorAction SilentlyContinue)) { throw "Missing prerequisite: $Tool" } }
if (-not $env:JAVA_HOME -or -not (Test-Path (Join-Path $env:JAVA_HOME 'bin/javac.exe'))) { throw 'JAVA_HOME must point to a complete JDK, not a JRE.' }
if (-not $env:ANDROID_HOME -or -not (Test-Path $env:ANDROID_HOME)) { throw 'Install Android SDK (platform 36 and build/platform tools), set ANDROID_HOME and accept licenses in Android Studio.' }
if (-not $env:NDK_HOME -or -not (Test-Path (Join-Path $env:NDK_HOME 'toolchains/llvm'))) { throw 'Install an official side-by-side Android NDK and set NDK_HOME to that exact version.' }
if ($CheckOnly) { Write-Host 'SDK/JDK paths exist. This is not an APK build or device verification.'; exit 0 }
Push-Location -LiteralPath $ProjectRoot
try {
    Invoke-Checked 'node' @('scripts/check-branding.mjs')
    Invoke-Checked 'rustup' @('target','add','aarch64-linux-android')
    Invoke-Checked 'npm' @('ci','--ignore-scripts')
    Invoke-Checked 'cargo' @('test','--workspace','--locked')
    Invoke-Checked 'npm' @('run','tauri','--','android','init','--ci')
    if (Get-Command py -ErrorAction SilentlyContinue) { Invoke-Checked 'py' @('-3','scripts/prepare_android.py') }
    elseif (Get-Command python -ErrorAction SilentlyContinue) { Invoke-Checked 'python' @('scripts/prepare_android.py') }
    else { throw 'Python 3 is required only for the manifest preparation script; it is not an app runtime dependency.' }
    Invoke-Checked 'npm' @('run','tauri','--','android','build','--apk','--debug','--target','aarch64')
    $Packages = @(Get-ChildItem 'apps/companion/src-tauri/gen/android/app/build/outputs/apk' -Filter '*.apk' -Recurse)
    if ($Packages.Count -eq 0) { throw 'Build returned without an APK. No installation success is claimed.' }
    $Packages | ForEach-Object { Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName }
    Write-Host 'APK files exist. Installation, Keystore and real API tests still require an Android device.'
} finally { Pop-Location }
