# Android 安装说明

当前下载：**v0.0.5 测试版，Android 10+、ARM64**。[下载 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.5/DeepSeek-Niang_0.0.5_android-arm64-isolated-test.apk)。普通用户不需要 Android Studio、SDK 或 USB 调试。

## 先确认安装身份

本版应用名为“DeepSeek娘 扫码测试版”，安装身份 `io.local.deepseekcompanion.test2`，versionCode 为 1005。已用官方工具核对与 v0.0.4 test2 同证书，满足该测试身份更新条件；真实手机覆盖升级仍待验收。

**不能覆盖 v0.0.1 原身份或早期 `.test`，也不能自动读取其私有数据。** 更早的 test2 仍需按实际升级跨度验证。保留旧应用及数据；提示签名冲突时停止，不通过卸载或清数据强行安装。详见[升级与数据保护](USAGE.md#升级与数据保护)。

## 安装步骤

1. 下载 APK，对照同版 [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.5/SHA256SUMS.txt) 校验文件。也可先在电脑计算 SHA-256，再传到手机。
2. 按 Android 提示，仅为所用文件管理器或浏览器临时允许安装来源；安装完成后可撤销权限，不关闭系统防护。
3. 打开正确的“扫码测试版”。在设置中验证安全存储，再在手机本地配置自己的 DeepSeek API Key，或按下文通过可信电脑授权配置。
4. 检查旧人格、聊天和记忆是否保留；有异常先保留现状并反馈，不重复卸载、降级或重建数据。

## 连接电脑与独立使用

电脑与手机都更新到 v0.0.5，连接可互访局域网。电脑点击“连接手机”，手机进入连接页点击“扫一扫”，需要相机权限；相机识别在本机进行。扫码也会授权电脑将已有 Key 一次写入本次认证手机安全存储，重新扫码可能更新手机已有配置。二维码本身不含 Key，普通同步不传 Key。完整边界见[连接说明](USAGE.md#连接电脑与手机)。

手机 Key 配置成功后可以独立联网聊天，不要求电脑开机。手机查看已同步生活记录，但不独立执行 Windows 生活任务。应用被停止或系统限制时，不保证实时同步或系统通知，没有云推送。

## 数据与权限

Key 由 Android Keystore 保护的私有存储保存，业务数据库和迁移备份未做数据库级加密。自动云备份被禁用，目前没有完整一键加密导出和跨安装身份迁移工具；系统不允许导出时保留原应用，不绕过沙箱。升级前保全方式和恢复限制见[使用说明](USAGE.md#升级与数据保护)。

APK 构建、签名及清单校验不能替代真实相机、键盘、Keystore、覆盖升级或通知体验。当前范围见[验证说明](VALIDATION.md)。自行编译请看[开发与构建](BUILDING.md)，下方为历史构建与安装记录。

<details>
<summary>历史开发与发布说明（原文保留，不代表当前状态）</summary>

# Android 安装与 Windows 本机构建 APK

当前版本 0.0.1，[下载本次APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_android-arm64.apk)。正式原始PNG已导入，已完成完整构建和签名/版本/ABI/图标资源核验，实际范围见RELEASE_0.0.1。旧dev.1 APK不作为本版附件。目标Android 10/API 29+、ARM64；versionCode=1001，高于旧包1000，签名证书一致。已作为GitHub预发布提供下载，仍是开发签名调试包，尚未真机安装运行；以随包 BUILD-RESULTS.json 为准。

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

</details>
