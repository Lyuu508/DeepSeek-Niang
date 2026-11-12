# Android 安装与 Windows 本机构建 APK

当前源码版本 0.0.1，正式原始PNG已于2026-11-13导入并生成图标；以下为本次构建/安装步骤，实际产物与测试范围见RELEASE_0.0.1。旧dev.1 APK不作为本版附件。目标Android 10/API 29+、ARM64。开发 APK 使用开发签名，不能视为已正式发布或已经真机测试；实际状态以随包 BUILD-RESULTS.json 为准。

## 安装实际 APK

先校验 SHA-256，再将实际 APK 复制到自己的手机。按系统提示仅为所用文件管理器授予安装来源权限，安装完成后可撤销。不要关闭系统防护。
若已配置 Android Platform-Tools、开启并授权 USB 调试，可用 `adb install -r <实际APK路径>`；运行 `adb devices` 确认自己的设备。不要为覆盖签名不匹配的旧包而直接卸载，以免丢失本地数据。
安装后先运行设置中的安全存储自检，再由用户在手机本地输入自己的 DeepSeek Key，测试中文/键盘/流式/停止，新增及切换人格、独立记忆/用量、重启恢复上次人格；每人格仍是同一永久空间。手机不依赖电脑开机。本阶段不做 LAN 同步。

## 在 Windows 安装构建组件

先按 INSTALL_WINDOWS 安装 Node/Rust 与 MSVC。通过官方 Android Studio https://developer.android.com/studio 安装 SDK 组件；建议工程路径简短，如 C:\Projects\deepseek-ai-companion。

SDK Manager 中启用：

- Android SDK Platform 36。
- Android SDK Build-Tools 36.0.0 或由生成工程要求的兼容版本。
- Android SDK Platform-Tools。
- Android SDK Command-line Tools。
- NDK (Side by side)，优先 28.2.13676358/r28c；记录实际选中版本。
- 完整 JDK 21（或经生成工程验证兼容的 JDK 17+）；可用 Android Studio 配套 JBR，必须包含 bin\javac.exe，JRE 不够。

用户在 Android Studio 阅读并接受 SDK/NDK 许可；脚本不代替用户交互安装 SDK。当前 PowerShell 会话设置实际路径，例如：

```powershell
$env:JAVA_HOME = 'C:\Program Files\Android\Android Studio\jbr'
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:NDK_HOME = Join-Path $env:ANDROID_HOME 'ndk\28.2.13676358'
$env:Path = "$env:JAVA_HOME\bin;$env:ANDROID_HOME\platform-tools;$env:ANDROID_HOME\cmdline-tools\latest\bin;$env:Path"
java -version
javac -version
sdkmanager --list_installed
.\scripts\build-android.ps1 -CheckOnly
.\scripts\build-android.ps1
```

将示例替换为自己实际安装的路径。CheckOnly 是检查，不代表已经编译 APK。
先按 INSTALL_WINDOWS 中原图步骤制作/检查正式图标。脚本缺图会在安装依赖之前停止。手动命令：

```powershell
node scripts/check-branding.mjs
rustup target add aarch64-linux-android
npm ci --ignore-scripts
cargo test --workspace --locked
npm run tauri -- android init --ci
py -3 scripts/prepare_android.py
npm run tauri -- android build --apk --debug --target aarch64
```

辅助脚本在真实生成工程中统一应用名称 DeepSeek娘，保留 applicationId，复制已校验的launcher/adaptive/启动/通知资源及主题，配置 INTERNET、键盘 adjustResize、禁自动备份/数据迁移及生产禁明文；Keystore Kotlin 源码位于 plugins\companion-keystore\android，不只保存在忽略的 gen 目录。
实际 APK 在 `apps\companion\src-tauri\gen\android\app\build\outputs\apk\` 下寻找；保留成功日志和哈希。不将浏览器界面、Rust 交叉检查或失败构建称为 APK。

遇到 Java 版本错误，应让 JAVA_HOME 与 Gradle 所用 JDK 一致；遇到 NDK 缺失，检查 NDK_HOME 下是否有 toolchains\llvm；遇到证书/代理错误，配置受信任 CA，不禁用 TLS 校验。真实密钥只在手机设置页输入。

本次从正式PNG重新构建完整APK并核验资源；真实设备运行仍需独立验收。实际打包日志与签名以本次BUILD-RESULTS为准。若已有dev.1安装，使用原签名覆盖；签名与Key不可放源码ZIP。原签名缺失时不能靠换applicationId或卸载来冒充升级成功。
