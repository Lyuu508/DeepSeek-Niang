[CmdletBinding()]
param([switch]$CheckOnly, [switch]$TestPackage, [switch]$UnsignedTestCI)
$ErrorActionPreference = 'Stop'
if ($TestPackage -and $UnsignedTestCI) { throw 'Select one Android distribution mode' }
$ProjectRoot = Split-Path -Parent $PSScriptRoot
function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command failed with exit code $LASTEXITCODE" }
}
function Invoke-Python([string[]]$Arguments) {
    if (Get-Command py -ErrorAction SilentlyContinue) { Invoke-Checked 'py' (@('-3') + $Arguments) }
    elseif (Get-Command python -ErrorAction SilentlyContinue) { Invoke-Checked 'python' $Arguments }
    else { throw 'Python 3 is required for packaging policy and verification.' }
}
foreach ($Tool in @('node','npm','cargo','rustup')) { if (-not (Get-Command $Tool -ErrorAction SilentlyContinue)) { throw "Missing prerequisite: $Tool" } }
if (-not $env:JAVA_HOME -or -not (Test-Path (Join-Path $env:JAVA_HOME 'bin/javac.exe'))) { throw 'JAVA_HOME must point to a complete JDK, not a JRE.' }
if (-not $env:ANDROID_HOME -or -not (Test-Path $env:ANDROID_HOME)) { throw 'Install Android SDK (platform 36 and build/platform tools), set ANDROID_HOME and accept licenses in Android Studio.' }
if (-not $env:NDK_HOME -or -not (Test-Path (Join-Path $env:NDK_HOME 'toolchains/llvm'))) { throw 'Install an official side-by-side Android NDK and set NDK_HOME to that exact version.' }
if ($CheckOnly) { Write-Host 'SDK/JDK paths exist. This is not an APK build or device verification.'; exit 0 }
Push-Location -LiteralPath $ProjectRoot
$PreviousTestPackage = $env:VITE_COMPANION_TEST_PACKAGE
$PreviousDevDebug = $env:CARGO_PROFILE_DEV_DEBUG
$PreviousDevStrip = $env:CARGO_PROFILE_DEV_STRIP
try {
    $env:VITE_COMPANION_TEST_PACKAGE = $null
    if ($TestPackage -or $UnsignedTestCI) {
        $env:VITE_COMPANION_TEST_PACKAGE = '1'
        $env:CARGO_PROFILE_DEV_DEBUG = '0'
        $env:CARGO_PROFILE_DEV_STRIP = 'debuginfo'
    }
    else { Invoke-Python @('scripts/android_signing.py') }
    Invoke-Checked 'node' @('scripts/check-branding.mjs')
    Invoke-Checked 'rustup' @('target','add','aarch64-linux-android')
    Invoke-Checked 'npm' @('ci','--ignore-scripts')
    Invoke-Checked 'cargo' @('test','--workspace','--locked')
    Invoke-Checked 'npm' @('run','tauri','--','android','init','--ci')
    if ($UnsignedTestCI) { Invoke-Python @('scripts/prepare_android.py','--unsigned-test-ci') }
    else { Invoke-Python @('scripts/prepare_android.py') }
    if ($TestPackage) {
        Invoke-Python @('scripts/android_test_signing.py')
        Invoke-Checked 'npm' @('run','tauri','--','android','build','--apk','--debug','--target','aarch64')
        Invoke-Python @('scripts/verify_android.py','--test-package')
    } elseif ($UnsignedTestCI) {
        Invoke-Checked 'npm' @('run','tauri','--','android','build','--apk','--debug','--target','aarch64')
        Invoke-Python @('scripts/verify_android.py','--unsigned-test-ci')
    } else {
        Invoke-Python @('scripts/android_signing.py','--configure')
        Invoke-Checked 'npm' @('run','tauri','--','android','build','--apk','--target','aarch64')
        Invoke-Python @('scripts/verify_android.py')
    }
    $Variant = if ($TestPackage -or $UnsignedTestCI) { 'debug' } else { 'release' }
    $Packages = @(Get-ChildItem 'apps/companion/src-tauri/gen/android/app/build/outputs/apk' -Filter '*.apk' -Recurse | Where-Object { $_.Directory.Name -eq $Variant })
    if ($Packages.Count -ne 1) { throw 'Expected exactly one APK for the requested variant. No installation success is claimed.' }
    $Packages | ForEach-Object { Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName }
    Write-Host 'APK files exist. Installation, Keystore and real API tests still require an Android device.'
} finally {
    $env:VITE_COMPANION_TEST_PACKAGE = $PreviousTestPackage
    $env:CARGO_PROFILE_DEV_DEBUG = $PreviousDevDebug
    $env:CARGO_PROFILE_DEV_STRIP = $PreviousDevStrip
    Pop-Location
}
