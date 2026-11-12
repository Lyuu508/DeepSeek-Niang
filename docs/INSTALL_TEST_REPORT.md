# 阶段 1 安装测试交付

日期：2026-11-12；保留原有工程和本地版本 **0.1.0-dev.1**，没有重新建项目，没有进入阶段 2，也没有远程 GitHub 操作。

## 实际安装文件

| 平台 | 实际产物 | 构建与静态核验 | 设备运行 |
|---|---|---|---|
| Windows 10/11 x64 | DeepSeek-Companion_0.1.0-dev.1_x64-setup.exe，3,749,417 字节 | Tauri 官方 Linux→Windows MSVC/NSIS 流程成功；PE、归档、应用载荷与哈希检查通过 | 未安装、未运行、未做代码签名 |
| Android 10/API 29+ ARM64 | DeepSeek-Companion_0.1.0-dev.1_arm64-debug.apk，33,163,209 字节 | Rust、Kotlin/Java、Gradle 实际成功；APK v2 签名、ZIP、ELF ABI、manifest 检查通过 | 未在手机/模拟器运行；Android Debug 签名 |

真实 DeepSeek 测试由用户在设备设置中输入自己的 Key 完成。没有要求用户将 Key 发到聊天，没有用 Mock 代替设备或供应商实测。

Windows 安装器 SHA-256：`018132936ab123e0c5557957470df4b00a6cce5915b80a7412f142f1155e81f1`。
Android APK SHA-256：`c2b17159cb6e0589a232813d2cfe0726a65a627ec7dfd94c9c35705bff92d612`。
Android 签名证书 SHA-256：`9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1`。

下载 ZIP 各自包含安装器/APK、SHA256SUMS.txt、BUILD-RESULTS.json 和对应安装说明。交付经用户明确授权保存为 Google Drive 私有文件，并读取权限确认仅所有者访问；实际链接在本次聊天，不将私人 Drive 文件 ID 写入源码。没有公开分享或 Release。

## 构建过程和验证范围

当前只有 Linux 主机。没有假称拥有 Windows 主机；使用 [Tauri 官方 Windows 交叉编译流程](https://v2.tauri.app/distribute/windows-installer/) 完成 NSIS。此前 CRT 标志冲突和只读/NSIS 数据路径问题均已解决，最终 Tauri 命令退出 0。NSIS 安装器 stub 是 x86、实际应用是 x64；归档内应用与编译产物仅有 Tauri 预期的 TYPE_VAR_UNK→TYPE_VAR_NSS 标记差异，逐字节比较通过。静态导入表只见 Windows 系统 API，WebView2 Runtime 仍是运行前置条件。

Android 已实际安装 Temurin JDK 21.0.13+9、SDK 36、NDK 28.2.13676358/r28c，完成初始化、manifest 准备、Rust ARM64、Kotlin/Java、Gradle 和 APK 打包。Gradle 9.6.1、AGP 9.3.1、Kotlin 2.2.10；AGP/Java 兼容性弃用警告保留在记录中，不据警告声称失败或隐去失败。

APK applicationId 为 io.local.deepseekcompanion，versionName 0.1.0-dev.1，minSdk 29，target/compileSdk 36，仅包含 arm64-v8a 原生库，INTERNET 权限与正确主 Activity。官方 apksigner 验证 v2 签名通过；这是开发签名，非正式发布签名。开发签名身份在本地另行保留，不进入源码/下载包；更换构建机默认 debug key 后不能直接覆盖同包名，应先规划数据保留与签名一致性，不能用直接卸载掩盖问题。

本轮改动是 Kotlin API 导入和构建/准备脚本、文档，业务 Rust/React 未改。新增 SDK 策略随现有 3 项 Python 策略测试通过；两平台实际构建亦完成前端类型检查/打包。此前 48 项 Rust 和 13 项 UI Mock 结果保留在 validation-phase1，不将旧测试冒充本次设备测试。原 0001 迁移、21 项核心测试与任务/关系/视频算法按字节保留。

## 依赖与脚本安全复查

- 先读取项目约束、直接依赖/锁文件、原有 Windows/Android 构建脚本与 Tauri build hooks；npm 保持 ignore-scripts，不执行陌生仓库启动器、未知 EXE 或远程 PowerShell。
- Windows 工具采用 Debian LLVM/LLD 19.1.7 和 NSIS 3.11。Debian InRelease 使用已安装发行版 keyring 验签，Packages 与软件包哈希匹配；仅解包到可写工作目录，未以管理员安装。
- cargo-xwin 0.23.1 的 crates.io 源码 checksum 与 PyPI 工具 wheel 哈希核对，检查 MIT 许可、源码的命令/下载路径，使用默认 clang-cl 和微软 SDK/CRT manifest；没有执行其备用第三方 sysroot 路径。Tauri NSIS 工具插件由官方流程下载并校验 hash。
- Temurin JDK 从官方 Adoptium 元数据给出的地址下载并校验 SHA-256；Android command-line tools 根据 Google 官方仓库元数据校验，SDK/NDK 从官方源安装。查看官方 launcher、Gradle wrapper 和生成的安装器脚本；安装器为 currentUser，未加入自定义提权、注册表保活或外部脚本。
- 构建网络保持 TLS 验证，Java 构建仅使用工作区私有 trust store 加入平台提供的 CA，没有禁用证书验证或打包该配置。
- 空间有限时仅清理可恢复构建缓存，并裁剪临时 SDK 的非 ARM64/调试分析工具；保留编译器依赖检查和成功输出。真实安装文件在清理前复制并核验。

这是有限的静态/来源/构建核验，未作全供应链逐行审计、RustSec 或 Windows 杀毒扫描，不声称绝对无漏洞。未将第三方候选桌宠、未知二进制或未授权角色素材加入安装包。

## 文件修改和后续测试

本轮主要改动：

- plugins/companion-keystore/android/.../CompanionKeystorePlugin.kt：修正 JSObject 为 app.tauri.plugin.JSObject，实际 Kotlin 编译通过。
- scripts/prepare_android.py 与 tests/test_build_policy.py：统一已验证 API 36，覆盖 SDK 模板变化；保留网络/键盘/备份策略。
- scripts/build-windows.ps1、build-android.ps1、build-android.sh：无实际产物时明确失败并打印真实文件哈希。
- README、安装/构建/安全/开发计划/验收文档与 validation-install-test 证据；版本号未变。

完整变更列表见 FILES.md。本轮没有修改 DB 迁移或聊天/同步/关系/任务/视频的业务 Rust 文件。

用户先安装 Windows，检查窗口与历史、运行安全存储自检、在本机配置 Key、测试中文流式/停止/重新生成/关闭重开。再测试手机的 Keystore、键盘、断网和重启历史。记录脱敏错误码、系统版本和包哈希；不发送密钥或私人数据库。

**GitHub 未更新，本次修改仅保留在本地。**
