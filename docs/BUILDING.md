# 阶段 1 本机构建与设备验收

版本 `0.0.1`。本轮已生成并发布Windows x64 EXE与Android ARM64 APK；[实际下载与验证范围](RELEASE_0.0.1.md)。Windows在原生runner通过58项测试、静默安装和窗口启动，Android通过完整构建与签名/资源检查、未真机运行。指定原始海滩PNG已由用户ZIP导入；旧dev.1记录只是历史证据。用户设备升级、Android Keystore与真实API仍待本机验收。

先按 [品牌资源说明](../assets/branding/README.md) 导入同一原图、检查头像/ICO/adaptive安全区域，再执行本页脚本。脚本不会以旧DS图标替代。`npm run build`仅前端构建，不等于原生安装包。

## Windows 10/11 x64

先从官方渠道安装 Node 22.12+、Rust stable（至少 1.99）、Visual Studio 2022 Build Tools 的「使用 C++ 的桌面开发」（MSVC x64、Windows SDK、链接工具）以及 WebView2。Python 3 仅为开发脚本使用。
参考 [Tauri 前置条件](https://v2.tauri.app/start/prerequisites/)。在可找到 MSVC 的终端解压源码并运行：

```powershell
.\scripts\build-windows.ps1 -CheckOnly
.\scripts\build-windows.ps1
```

CheckOnly 仅检查命令可用性，不代表 MSVC/凭据存储已经验证。构建脚本先验证品牌，再顺序执行：

```powershell
node scripts/check-branding.mjs
rustup target add x86_64-pc-windows-msvc
npm ci --ignore-scripts
cargo test --workspace --locked
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis
```

Windows 上的 `cargo test` 会额外运行真实 Credential Manager 写入/读回/删除测试，使用单独随机凭据名与非真实 Key，不触碰用户凭据。系统限制导致失败时不能跳过后称为通过。

默认安装器在 `apps/companion/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`；设置 `CARGO_TARGET_DIR` 时查该目录的对应 target/release/bundle。脚本只对真实找到的 EXE 打印 SHA-256。当前不配置代码签名，未签名开发安装器不等于正式发布版。

开发窗口：`npm run tauri -- dev`。在本机设置页运行安全存储自检，输入自己的 Key，再按验收表测试。不要在 Linux 上把 `--target windows` 的中途构建称为 Windows 原生验收。

## Android API 29+、ARM64

通过 Android Studio 官方 SDK Manager 安装完整 JDK（建议完整 JDK 21，和旧dev.1实际构建一致；兼容性仍以生成工程AGP要求为准）、SDK Platform 36、对应 Build-Tools、Platform-Tools、Command-line Tools、当前 Tauri 支持的 side-by-side NDK，并自行阅读接受 SDK 许可。JRE 不含 `javac`，不能替代 JDK。

设置 `JAVA_HOME` 到 JDK 根目录，`ANDROID_HOME` 到 SDK 根目录，`NDK_HOME` 到该 SDK 下实际安装的 `ndk/<版本>` 目录。将所选版本记录进设备验收结果，不能仅从目录存在推断版本兼容。
参考 [Tauri Android prerequisites](https://v2.tauri.app/start/prerequisites/#android)。

Windows：

```powershell
.\scripts\build-android.ps1 -CheckOnly
.\scripts\build-android.ps1
```

Linux/macOS：

```bash
sh scripts/build-android.sh --check-only
sh scripts/build-android.sh
```

也可按顺序手动运行：

```bash
node scripts/check-branding.mjs
rustup target add aarch64-linux-android
npm ci --ignore-scripts
cargo test --workspace --locked
npm run tauri -- android init --ci
python3 scripts/prepare_android.py
npm run tauri -- android build --apk --debug --target aarch64
```

Windows 的 Python 命令可改为 `py -3`。辅助脚本仅处理真实生成的 manifest：INTERNET、adjustResize、关闭自动云备份/数据迁移、生产 cleartext=false。应用窗口设置 useHttpsScheme=true，以免默认的 HTTP 资源协议与生产禁明文策略冲突；IPC CSP 同时允许本地 HTTPS 协议。`gen/android` 被忽略，Keystore Kotlin 源码独立保存在 `plugins/companion-keystore/android`，不会因重新初始化而丢失。

这是开发签名 APK，通常位于 `apps/companion/src-tauri/gen/android/app/build/outputs/apk/` 下；按实际构建输出查找。使用 Android Studio 或已授权的 `adb install -r <实际APK>` 安装。模拟器若非 ARM64，应重新评估对应 target，不以此脚本的 ARM64 包承诺兼容。

源码包不分发 gen 目录的缓存工程；初始化需要访问官方 Gradle/Android/Maven 依赖。prepare_android.py 将模板默认 API 固定到已验证的 36，与 Tauri Android 库和私有插件一致。本轮0.0.1的 Kotlin/Java、Rust ARM64、Gradle APK 已完整构建，签名/版本/ABI和图标资源通过检查；versionCode=1001，沿用旧dev.1开发签名。Keystore/键盘/生命周期仍待手机验证。本次实际工具链为 Temurin JDK 21.0.13、SDK/Build-Tools 36、NDK r28c、Gradle 9.6.1、AGP 9.3.1、Kotlin 2.2.10。

## 双端逐项验收

1. 在设备本地设置页点击安全存储自检，记录成功后端；输入自己的 Key，退出重启应显示已配置。替换和删除后状态应改变。不得录屏展示密钥。
2. 默认人格永久空间发送中文，新增B并切换，验证流式/多轮/错误及上下文/记忆/用量隔离；关闭“发送当前人格的近期上下文”检查行为。退出重启恢复最后人格，每人格不增加新会话。
3. 生成中停止：不再增长，已收正文标为中断；重新生成后同一用户消息只有一条，旧回复仍能查看。
4. 流式中杀进程、再打开：保留草稿和明确的中断状态，不自动重发。切换网络和无网后手动重试，不产生重复用户消息。
5. 应用不依赖另一设备；本阶段两端历史尚不互通。验证键盘弹出/收起、旋转、小屏和长文本。
6. 记录设备/OS、JDK/SDK/NDK/MSVC、实际模型、包 SHA-256、各用例观察及脱敏错误码。真实返回的 Token 才计为实测，不按字符数推算。

需要密钥仅在设备本地输入；不需要上传聊天数据库。未验证项在验收矩阵继续保持待验收。

旧库v1/v2首次启动会在原私有目录生成一致性 .pre-v3-UUID.bak，再事务迁移；本次仅合成旧库实测。用户数据不进源码，实机升级前保留备份，失败别删库。回滚方法/限制见 PERSONA_MIGRATION.md。
