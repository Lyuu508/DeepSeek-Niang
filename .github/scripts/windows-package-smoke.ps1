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

if ($Config.bundle.windows.nsis.languages.Count -ne 1 -or $Config.bundle.windows.nsis.languages[0] -ne 'SimpChinese') { throw 'Simplified Chinese installer configuration missing' }
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class CompanionInstallerUi {
  public delegate bool EnumProc(IntPtr handle, IntPtr param);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr handle, EnumProc callback, IntPtr param);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr handle, StringBuilder value, int count);
  public static string[] Text(IntPtr handle) {
    var result = new List<string>();
    EnumChildWindows(handle, (child, param) => {
      var text = new StringBuilder(2048); GetWindowText(child, text, text.Capacity);
      if (!String.IsNullOrWhiteSpace(text.ToString())) result.Add(text.ToString());
      return true;
    }, IntPtr.Zero);
    return result.ToArray();
  }
}
'@
$Wizard = $null
$InstallerLabels = @()
try {
    $Wizard = Start-Process -FilePath $Installer -PassThru
    for ($Attempt = 0; $Attempt -lt 40; $Attempt++) {
        Start-Sleep -Milliseconds 500
        $Wizard.Refresh()
        if ($Wizard.HasExited) { throw 'Installer language preview exited before opening its wizard' }
        if ($Wizard.MainWindowHandle -ne 0) {
            $InstallerLabels = @([CompanionInstallerUi]::Text($Wizard.MainWindowHandle))
            if (($InstallerLabels -join ' ') -match '下一步|取消') { break }
        }
    }
    if (($InstallerLabels -join ' ') -notmatch '下一步|取消') { throw 'Native installer did not display Chinese wizard controls' }
    @{ language='zh-CN'; native_wizard_verified=$true; labels=$InstallerLabels; source_commit=(git rev-parse HEAD) } | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Output 'windows-installer-language.json') -Encoding utf8
} finally {
    # Preview only: never click Install. The actual install smoke below is separate.
    if ($Wizard -and -not $Wizard.HasExited) { Stop-Process -Id $Wizard.Id -Force }
}

$InstallFolder = Join-Path $env:RUNNER_TEMP 'deepseek-install-smoke'
$Setup = Start-Process -FilePath $Installer -ArgumentList @('/S', "/D=$InstallFolder") -PassThru
if (-not $Setup.WaitForExit(180000)) { Stop-Process -Id $Setup.Id -Force; throw 'Installer timed out' }
if ($Setup.ExitCode -notin @(0, 3010)) { throw "Installer exited with $($Setup.ExitCode)" }
$InstalledApps = @(Get-ChildItem -LiteralPath $InstallFolder -Filter '*.exe' | Where-Object Name -NotMatch '(?i)uninstall')
if ($InstalledApps.Count -ne 1) { throw 'Installed application not found' }
$DesktopPetBundled = $false
$LocalVideoBundled = $false
$VoiceBundled = $false
if (Test-Path 'apps/companion/pet.html') {
    if ($Config.identifier -ne 'io.local.deepseekcompanion') { throw 'Expected the main Windows installation identity' }
    $InstalledPayload = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($InstalledApps[0].FullName))
    foreach ($Resource in @('pet.html', 'pet_motion', 'pet_action')) {
        if (-not $InstalledPayload.Contains($Resource)) { throw "Missing desktop pet resource in the installed client: $Resource" }
    }
    foreach ($Sprite in @('idle', 'walk', 'wave')) {
        if ($InstalledPayload -notmatch ('assets[/\\]' + $Sprite + '-[\w-]+\.webp')) { throw "Missing sprite from the installed client: $Sprite" }
    }
    $DesktopPetBundled = $true
    if (Test-Path 'apps/companion/src/VideoPanel.tsx') {
        foreach ($Command in @('video_import', 'video_list', 'video_control')) {
            if (-not $InstalledPayload.Contains($Command)) { throw "Missing local video command in the installed client: $Command" }
        }
        $LocalVideoBundled = $true
    }
    if (Test-Path 'apps/companion/src/voice/VoiceControls.tsx') {
        foreach ($Command in @('voice_speak', 'voice_record', 'voice_guide', 'pet_speech_level')) {
            if (-not $InstalledPayload.Contains($Command)) { throw "Missing voice command in the installed client: $Command" }
        }
        $VoiceBundled = $true
    }
}
$VoiceResult = 'unavailable'
if ($VoiceBundled) {
    $Probe = Start-Process -FilePath $InstalledApps[0].FullName -ArgumentList '--self-test-voice' -PassThru
    if ($Probe.WaitForExit(30000)) {
        switch ($Probe.ExitCode) {
            0 { $VoiceResult = 'system_voice_wav_generated' }
            2 { $VoiceResult = 'runner_has_no_usable_system_voice' }
            3 { $VoiceResult = 'runner_system_synthesis_failed' }
            default { throw "Local voice self-test exited with $($Probe.ExitCode)" }
        }
    } else {
        Stop-Process -Id $Probe.Id -Force
        $VoiceResult = 'runner_system_synthesis_timed_out'
    }
}
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
        desktop_pet_bundled = $DesktopPetBundled
        local_video_bundled = $LocalVideoBundled
        voice_bundled = $VoiceBundled
        native_voice_probe = $VoiceResult
        application_identifier = $Config.identifier
        window_title = $WindowTitle
        window_screenshot = $Screenshot
        real_user_device_tested = $false
        real_deepseek_api_tested = $false
        authenticode_signed = $false
        api_key_embedded = $false
        installer_language = 'zh-CN'
        installer_language_verified = $true
        installer_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Installer).Hash.ToLowerInvariant()
        application_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $AppCopy).Hash.ToLowerInvariant()
    }
    $Report | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $Output 'BUILD-RESULTS-WINDOWS.json')
} finally {
    if ($Process -and -not $Process.HasExited) { Stop-Process -Id $Process.Id -Force }
}
