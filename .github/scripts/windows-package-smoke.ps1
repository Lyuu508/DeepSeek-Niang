# Only for an ephemeral GitHub runner. Never silently installs on a user's PC.
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This installation smoke test is restricted to GitHub Actions.' }
$Config = Get-Content -Raw 'apps/companion/src-tauri/tauri.conf.json' | ConvertFrom-Json
if ($Config.version -ne ((Get-Content VERSION.json | ConvertFrom-Json).version)) { throw 'Unexpected test version' }
$Target = Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc/release'
$Packages = @(Get-ChildItem (Join-Path $Target 'bundle/nsis') -Filter '*.exe')
if ($Packages.Count -ne 1) { throw 'Expected exactly one fresh NSIS package' }
$Application = Join-Path $Target 'companion-app.exe'
if (-not (Test-Path -LiteralPath $Application)) { throw 'Compiled application missing' }
$ProductVersion = (Get-Item -LiteralPath $Application).VersionInfo.ProductVersion
if ($ProductVersion -notmatch ('^' + [regex]::Escape($Config.version) + '(?:\.0)?$')) { throw "Unexpected embedded version: $ProductVersion" }
$Output = Join-Path $env:RUNNER_TEMP 'deepseek-windows-artifacts'
New-Item -ItemType Directory -Force -Path $Output | Out-Null
$Installer = Join-Path $Output "DeepSeek-Niang_$($Config.version)_windows-x64-setup.exe"
$AppCopy = Join-Path $Output "DeepSeek-Niang_$($Config.version)_windows-x64.exe"
Copy-Item -LiteralPath $Packages[0].FullName -Destination $Installer
Copy-Item -LiteralPath $Application -Destination $AppCopy

$InstallFolder = Join-Path $env:RUNNER_TEMP 'deepseek-install-smoke'
$Setup = Start-Process -FilePath $Installer -ArgumentList @('/S', "/D=$InstallFolder") -PassThru
if (-not $Setup.WaitForExit(180000)) { Stop-Process -Id $Setup.Id -Force; throw 'Installer timed out' }
if ($Setup.ExitCode -notin @(0, 3010)) { throw "Installer exited with $($Setup.ExitCode)" }
$InstalledApps = @(Get-ChildItem -LiteralPath $InstallFolder -Filter '*.exe' | Where-Object Name -NotMatch '(?i)uninstall')
if ($InstalledApps.Count -ne 1) { throw 'Installed application not found' }
$Process = $null
$Screenshot = $false
$WindowTitle = ''
try {
    $Process = Start-Process -FilePath $InstalledApps[0].FullName -PassThru
    for ($Attempt = 0; $Attempt -lt 30; $Attempt++) {
        Start-Sleep -Seconds 1
        $Process.Refresh()
        if ($Process.HasExited) { throw "Installed application exited early: $($Process.ExitCode)" }
        if ($Process.MainWindowHandle -ne 0) { break }
    }
    if ($Process.MainWindowHandle -eq 0) { throw 'Installed app never opened a native window' }
    $WindowTitle = $Process.MainWindowTitle
    try {
      if ($Process.MainWindowHandle -ne 0) {
        Add-Type -AssemblyName System.Drawing
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class CompanionWindowCapture {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out Rect rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr handle);
}
'@
        [CompanionWindowCapture]::SetForegroundWindow($Process.MainWindowHandle) | Out-Null
        Start-Sleep -Seconds 5
        $Bounds = [CompanionWindowCapture+Rect]::new()
        if ([CompanionWindowCapture]::GetWindowRect($Process.MainWindowHandle, [ref]$Bounds)) {
            $Bitmap = [System.Drawing.Bitmap]::new($Bounds.Right-$Bounds.Left, $Bounds.Bottom-$Bounds.Top)
            $Graphics = [System.Drawing.Graphics]::FromImage($Bitmap)
            try {
                $Graphics.CopyFromScreen($Bounds.Left, $Bounds.Top, 0, 0, $Bitmap.Size)
                $Bitmap.Save((Join-Path $Output 'windows-installed-window.png'), [System.Drawing.Imaging.ImageFormat]::Png)
                $Screenshot = $true
            } finally { $Graphics.Dispose(); $Bitmap.Dispose() }
        }
    }
    } catch { Write-Warning "Window screenshot unavailable: $($_.Exception.GetType().Name)"; $Screenshot = $false }
    $Report = [ordered]@{
        version = $Config.version
        build_commit = (git rev-parse HEAD)
        workflow_run = $env:GITHUB_RUN_ID
        host = 'GitHub-hosted Windows runner'
        embedded_product_version = $ProductVersion
        installer_exit_code = $Setup.ExitCode
        installed_on_windows_runner = $true
        installed_process_started = $true
        window_title = $WindowTitle
        window_screenshot = $Screenshot
        real_user_device_tested = $false
        real_deepseek_api_tested = $false
        authenticode_signed = $false
        api_key_embedded = $false
        installer_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Installer).Hash.ToLowerInvariant()
        application_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $AppCopy).Hash.ToLowerInvariant()
    }
    $Report | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Output 'BUILD-RESULTS-WINDOWS.json')
} finally {
    if ($Process -and -not $Process.HasExited) { Stop-Process -Id $Process.Id -Force }
}
