# 开发与构建

v0.1.1 验收预发布提供 Windows 和 Android 安装包。普通用户可从项目首页下载安装并按说明配置。[下载安装](../README.md#下载安装) · [使用说明](USAGE.md)

## 开发环境

使用仓库现有检出，先阅读 [AGENTS.md](../AGENTS.md) 和 [SECURITY.md](../SECURITY.md)。Node.js 至少 22.12（CI 为 24），Rust 1.99.0、Clippy、Python 3、C/C++ 编译工具；使用已提交的 npm／Cargo 锁文件。UI 测试需要 Chromium。每个云任务已有隔离环境，除非用户明确要求，不另建 Git worktree。

在仓库根目录执行：

```bash
npm ci --ignore-scripts
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run build
python3 scripts/check_schema.py
python3 scripts/version.py
python3 -m unittest discover -s tests -p 'test_*.py'
node --test tests/sync-model.test.mjs
node scripts/check-branding.mjs
npm run test:ui
```

**先完成 Rust 测试，再执行 Node 同步及 UI 测试。** Rust 测试产生 `.cache/phase2-qr.json` 和 `.cache/phase2-quick-qr.json`，后两者读取这些夹具；缺文件不能判为二维码功能失败。UI 是显式夹具，不验证真实 DeepSeek、系统 Key 存储或手机驱动。测试包横幅模式可另运行 `VITE_COMPANION_TEST_PACKAGE=1 npm run test:ui`。

## 启动前端与原生开发窗口

```bash
npm run dev --workspace @deepseek/companion -- --host 127.0.0.1
```

默认端口 1420；可在末尾加 `--port 1422` 等空闲端口。根脚本的参数转发不能替代上面的 workspace 形式。普通浏览器只有预览能力，开发环境显式 `?test=mock` 才使用标记过的内存夹具，不要填真实 Key。

Windows 原生开发使用 `npm run tauri -- dev`，需要 MSVC、Windows SDK 与 WebView2；Linux 浏览器预览不代表 Windows 原生启动。若云环境已准备 `/workspace/.cache` 工具链，可在仓库根目录使用 `python3 scripts/cloud_env.py cargo …` 激活；该帮助脚本不表示 Android 工具链也已经安装。

## 原生构建与发布边界

Windows 需要 Visual Studio C++ Build Tools、Windows SDK、WebView2 和 Rust MSVC target。Android 当前 CI 使用完整 JDK 21、SDK Platform 36、Build-Tools 36.0.0、NDK 28.2.13676358 及 ARM64 target；用户需自行接受相关许可。具体构建选项以 `scripts/build-windows.ps1`、`scripts/build-android.sh` 和 `.github/workflows/native-packages.yml` 为准；本地开发包不能冒充已发布 test2 更新包，不调用历史生成签名分支。

v0.0.5 已发布，**不要重跑发布器来修改说明，也不要复用版本号或移动 Tag**。现有 `release-companion-test.yml` 固定该版，恢复原测试签名、核对 v0.0.4 APK、同源门禁及摘要后才允许显式发布；不是可自动发布任意下一版的通用工作流。普通 CI 不公开 Release。

签名连续性见[签名说明](TEST_SIGNING_SETUP.md)，附件整理影响见[展示与发布信息审查](PRODUCT_PRESENTATION_REVIEW.md)。本次文档整理没有修改任何脚本或工作流。下方历史命令和工具链背景保留供追溯，其中旧版本、安装身份及授权状态不适用于当前发布。

<details>
<summary>历史开发与发布说明（原文保留，不代表当前状态）</summary>

# 阶段3追加候选0.0.4 / Android1004

沿用现有CI的独立Windows/Android构建与现有持久test2签名；平台、applicationId不变。先CI全绿，再手动运行 `release-companion-test.yml`，指定完整 main SHA、同源CI ID、authorized_version=0.0.4。新增 `publish_after_checks` 默认 false，只上传已验证同源安装包和日志到草稿，不创建Tag/公开Release。版本选择待主人确认；不得自动传true。

签名脚本强制恢复 `TEST_RELEASE_SIGNING_B64`，用真实v0.0.3 APK官方apksigner确认当前签名/ID，再签新1004包，同时验证真实v1证书不同。Windows签后仍检查中文向导和安装启动。全部附件上传后要求GitHub服务器计算摘要与本机已验签字节一致；明确授权公开后还验证公开HTTP回下载。草稿下载使用已授权人工账号独立复核，不把不稳定GITHUB_TOKEN草稿下载当产品测试。

0001～0006不改，升级schema7前一致备份，见 [数据库](DATABASE.md)。实际新增检查见 [追加报告](PHASE3_FOLLOWUP_REPORT.md)。以下v3命令/运行状态保留为历史；现有平台前置条件继续适用。

---

# v0.0.3 当前构建与发布流程

目标版本 0.0.3 / Android1003。先运行 `CI`：Linux 全量检查，Windows 原生 cargo test、真实中文 NSIS EXE 与安装启动，Android JDK21、Command-line Tools、Platform36、Build-Tools36.0.0、NDK28.2.13676358 和 ARM64 APK。两端 jobs 独立，不因另一端失败取消成功产物；普通 CI 的 unsigned-ci APK 仅编译检查，不可安装且不能冒充签名升级包。

通过后执行已授权的 `release-companion-test.yml`，输入 `source_sha`（main 当前完整 SHA）、`ci_run_id`（该 SHA 的成功 CI）、`authorized_version=0.0.3`。它拒绝已公开版本复用和 main 漂移，恢复既有 `TEST_RELEASE_SIGNING_B64`，**不生成新私钥**。Windows/Android 独立签署与验证，再统一生成 SHA256SUMS/BUILD-RESULTS、测试和构建日志，draft 附件回下载校验后公开新 Pre-release，随后复验实际公开链接。保留原 v1/v2 Release 及 Tag。

Android 正式配置仍为 `io.local.deepseekcompanion`；本次测试包保持现有 `io.local.deepseekcompanion.test2` 与已发布0.0.2证书。工作流使用 protected bundle 内的 Android PKCS12 与 Windows PFX；兼容单项 Secret 名称见 TEST_SIGNING_SETUP.md。CI 仅输出存在布尔值，不输出密码/Base64。实际旧v1/v2APK由官方apksigner对比后再签新APK，当前test2与正式v1不同，不能宣称正式升级。签后Windows临时证书信任只发生在可销毁Hosted runner，不操作用户机器信任库；自签名不是公共CA。

完整本地/Hosted/实机范围与依赖限制见 PHASE3_REPORT.md；用户验收见 PHASE3_ACCEPTANCE.md。以下是历史构建命令与阶段1说明，当时“未签名/双端不互通/尚未发布”等状态不能作为当前状态。

---

# 当前独立测试包构建方式

最新 CI 的 `android_compile_only` 现在构建 **`.test` 未签名更新候选**：`sh scripts/build-android.sh --unsigned-test-ci`，Windows 对应 `-UnsignedTestCI`。它沿用已发布测试包的 applicationId、0.0.2/1002 与测试标记，通过 Gradle 显式 `signingConfig = null`，不创建任何密钥；实际 apksigner 必须确认没有可验证签名。该候选不可安装，等待相同 `.test` 私钥签署才可替换现有 APK。普通 `--unsigned-ci` 仍是原身份 release 编译模式，两者不能混淆。

`prepare_android.py` 将受版本管理的 `apps/companion/android/MainActivity.kt` 写入生成工程：原生 WindowInsets 处理系统状态栏、导航栏、刘海及输入法；传给 WebView 的边距已消费，防止 CSS 重复偏移。CSS env() 只作为支持浏览器的回退。原生构建不等于实际手机状态栏/键盘验收，仍由用户设备验证。Windows 完整原生命令输出保存在 `windows-native-build.log`，原 transcript 仅作补充。

已发布的 0.0.2 为 `.test` applicationId、开发测试签名和“DeepSeek娘 测试版”名称。用户现在选择等待相同测试签名以保留升级与数据连续性，因此普通 CI 改为 `--unsigned-ci` 原生编译检查，不生成新的测试签名或替换手机 APK。该检查包不能安装、不能用于升级。Windows EXE 独立构建，NSIS 强制 `SimpChinese`、WiX 配置 `zh-CN`；runner 读取实际安装向导的中文控件后，再静默安装并启动窗口。

- Linux/macOS：`sh scripts/build-android.sh --test-package`。
- Windows：`.\scripts\build-android.ps1 -TestPackage`。
- 正式升级：原签名配置可用时，无参数脚本执行原 Keystore 校验、release 构建和原证书验签；不使用 debug 签名。

上面的 `--test-package` 是显式开发工具，不代表本次授权重新生成已发布测试包的签名。新版扫码源码可先编译验证，手机交付等待已发布 `.test` APK 的同一签名；不得用正式 v0.0.1 签名替换 `.test` 签名冒充覆盖升级。中文 Windows 修订通过 `publish-windows-zh.yml` 校验指定 CI 的核心测试和 Windows 成功后，给现有 Release 增加独立附件；最初成套包、Tag、APK 与校验文件均保留。

Android debug 变体始终增加 `.test`，release 的固定 applicationId/namespace 不变；资源覆盖只在 debug 目录，正式图标原图和 release 名称不变。开发测试 Keystore 只生成在被忽略的 Android gen/.test-signing 中，既不是原签名，也不用于正式升级；不会进入源码、构建附件或普通文件上传。不同 Runner 不保证同一测试签名，不要求卸载正式版。

只核对当前构建变体的 APK，保留另一变体已有输出，避免误把上一次 APK 当新结果。`verify_android.py --test-package` 检查真实包 ID、名称、版本、ABI 和真实签名；正式收集器同时拒绝测试包 identity/distribution，禁止进入正式 Release。详见 [交付报告](PHASE2_TEST_DELIVERY.md)。

# 历史双端构建与验证范围（未签名检查包）

当前开发目标 0.0.2 / Android versionCode 1002；已公开发布版本仍是 0.0.1。最新核验的 main 9cc9a16 在 CI 43555778492 中完成 Linux 测试、Windows EXE 构建/安装启动和 Android ARM64 未签名 APK 构建。普通 CI 不读取原签名，未签名 APK 不能安装或覆盖升级；它不等于独立签名测试包。

Windows 和 Android 是 native-packages.yml 中两个独立 job，分别保留成功产物及失败日志，Android 签名问题不取消 Windows 构建。release.yml 固定完整 source_sha，等待测试与双端原签名验证全部成功后才公开同版本 Pre-release。原签名不可用时保留独立构建结果，不生成替代密钥、不公开不完整升级版本。

当前签名/网络阻碍与最新证据见 [接管报告](TAKEOVER_2026-11-14.md)；平台前置条件和构建命令沿用下文。后续历史验收中的“尚不互通”等描述仅适用于原 0.0.1，不适用于当前阶段 2。

阶段 2 起通过 CI 与手动授权 Release 流程构建。Windows 复用 build-windows.ps1，Android 复用 build-android.sh；两端使用同一 source_sha。Android Release 强制已有签名 Secrets，CI 的 --unsigned-ci 产物只供编译检查，不能安装或冒充覆盖升级包。原签名调查见 [ANDROID_SIGNING_RECOVERY](ANDROID_SIGNING_RECOVERY.md)。以下保留本机构建的系统前置条件。

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

</details>
