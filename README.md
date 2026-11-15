# DeepSeek娘

0.0.2 扫码修订已替换原 Release 的 EXE/APK，直接下载即可安装。两端来自同一源码 `9bc43c4d`，使用已持久保存到 GitHub 加密 Secrets 的新测试签名。[完整交付与验收报告](docs/RELEASE_0.0.2_REFRESH.md) · [其他账号后续复用签名](docs/TEST_SIGNING_SETUP.md)。

## 0.0.2 测试版直接下载

- [Windows 中文 EXE 安装程序](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_windows-x64-setup.exe)
- [Android 扫码测试 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk)
- [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SHA256SUMS.txt) · [BUILD-RESULTS.json](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/BUILD-RESULTS.json) · [GitHub Release](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.2)
- [公开签名身份](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SIGNING-IDENTITIES.json) · [安装与验收](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/INSTALL-AND-ACCEPTANCE.md)

电脑关闭旧程序后运行新版 EXE，再重新打开。手机安装后请打开 **“DeepSeek娘 扫码测试版”**；其 applicationId 为 `io.local.deepseekcompanion.test2`，与旧 `.test` 和正式版并存，旧应用不会自动变成新版。不要卸载旧应用或清数据，旧手机未同步的聊天不会自动导入新身份；新手机端可扫码同步电脑数据。Windows 使用自签名测试证书，尚非公共 CA 信任发布者。真实双端与家庭局域网仍待用户验收；原正式签名升级保持独立待办。

以下保留项目与历史版本说明；最新公开版本为 0.0.2 测试版。

新版为电脑一个“连接手机”按钮弹出二维码、手机一个“扫一扫”直接调用相机，移除旧版/文件/手动输入入口，保留手动同步，本机信息放在底部。手机使用独立底部导航、紧凑聊天及原生系统栏/刘海/键盘边距。源码 [`9bc43c4d`](https://github.com/Lyuu508/DeepSeek-Niang/commit/9bc43c4dde86a92153273be8df048b10624475ae) 的 [同源 CI](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43693955496) 与 [签名发布](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43697671242) 已全部成功，16 项当前附件已实际回下载 HTTP 200/哈希校验，APK 另经官方 apksigner 复验。实机相机与系统栏效果尚未验收。[界面预览（UI 夹具）](docs/validation-phase2-mobile-qr/README.md)。

旧附件已改为 `ARCHIVE-` 开头保留原字节，v0.0.2 Tag 没有改写。GitHub 自动生成的 Source code ZIP 属于历史 Tag；本次安装包源码以上方完整提交与构建报告为准。[早期手机修订记录](docs/PHASE2_MOBILE_QR_REPORT.md) · [早期 Windows 修订记录](docs/PHASE2_SCAN_ZH_REPORT.md)。

内部英文名称：DeepSeek-Niang AI Companion。初次阶段 2 测试交付见 [历史测试报告](docs/PHASE2_TEST_DELIVERY.md)；接管时的状态快照见 [项目状态报告](docs/TAKEOVER_2026-11-14.md)。

> 项目愿景（保留仓库初始说明）：使用GPT制作的电脑跟手机数据互通的养一只有自己生活能刷b站主动聊天还有好感度系统的大肥鱼开源软件。

实际已实现能力与后续计划以下文为准。

Windows + Android 本地优先陪伴应用，0.0.1 历史测试版已发布，继承本地开发版0.1.0-dev.2。继承原阶段 0/1，改为“一人格一个永久聊天空间”：新增/切换/编辑人格，独立上下文、记忆、Token 统计，重启恢复上次选择。保留 DeepSeek 流式聊天、停止、重生成和双端系统密钥存储。

**0.0.1 测试版**使用用户提供的正式海滩PNG，原图按字节保留，Windows/Android图标和聊天头像已生成。安装文件通过 [GitHub Releases](https://github.com/Lyuu508/DeepSeek-Niang/releases) 交付，分别选择Windows EXE或Android ARM64 APK；实际构建及设备验证范围见附件说明与 [安装测试说明](docs/RELEASE_0.0.1.md)。

- [Windows 10/11 x64 安装 EXE](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_windows-x64-setup.exe)
- [Android 10+ ARM64 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_android-arm64.apk)
- [0.0.1测试版发布页](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.1) · [SHA-256](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/SHA256SUMS.txt)

两个安装包均已从公开链接完整下载并校验哈希。Windows 原生构建、58项测试、静默安装及窗口启动通过；Android 完整构建和签名/资源检查通过，尚未真机运行。Windows 安装器未做 Authenticode 签名，APK 使用原开发签名；真实 API 请在各设备设置中输入自己的 Key 验收。

0.0.1 的历史记录见 [人格/品牌报告](docs/PERSONA_BRANDING_REPORT.md)、[迁移与回滚](docs/PERSONA_MIGRATION.md)、[品牌素材状态](assets/branding/README.md)。当时阶段 2～9 仅有[独立计划](DEVELOPMENT_PLAN.md)；当前阶段 2 实现与限制以下文为准。

## 阶段 2 开发状态（目标 0.0.2）

以 main `22096939c20134708c6dc1a4476468a7bbe96827` 为基线重新实现局域网同步，没有使用旧云端未提交代码。
已实现生产同步协议、Windows mDNS/Android NSD、一次性二维码、TLS 1.3 与系统安全存储、来源签名、事务队列与同步页面。当前同源 `9bc43c4d` 双端包已公开：电脑出码授权，手机相机识码后自动配对、双向增量同步，Windows 中文安装。签名、原生构建与 Hosted Windows 安装启动已验证；原正式身份升级和家庭实机验收保持待办。
见 [阶段报告](docs/PHASE2_REPORT.md)、[实机验收](docs/PHASE2_ACCEPTANCE.md)、[迁移恢复](docs/PHASE2_MIGRATION.md)、[签名调查](docs/ANDROID_SIGNING_RECOVERY.md)、[版本规则](docs/VERSIONING.md)。

## 本地开发

Rust 1.99+、Node 22.12+、npm；Python 3 用于辅助。依赖锁定，不自动安装新程序/模型或调用新增付费 API。

```bash
npm ci --ignore-scripts
npm run build
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check_schema.py
python3 -m unittest discover -s tests -p 'test_*.py'
npm run test:ui
```

UI 测试需要已安装 Chromium，可用 COMPANION_CHROMIUM_EXECUTABLE 指定路径。普通浏览器只预览，DEV 且显式 `?test=mock` 才使用有横幅的内存夹具；它不验证 SQLite/系统凭据，生产打包不包含夹具。

仓库已包含正式原图及派生资源。重新制作图标时使用已审核的Pillow；日常构建直接校验资源即可：

```powershell
python scripts/prepare_branding.py --source "C:\你的目录\原图.png"
node scripts/check-branding.mjs
.\scripts\build-windows.ps1
# 或按 Android 前置条件配置后：
.\scripts\build-android.ps1
```

原图 master 按字节保留，派生文件有哈希；不重绘人物。[Windows 安装](docs/INSTALL_WINDOWS.md)、[Android 安装](docs/INSTALL_ANDROID.md) 和 [BUILDING](docs/BUILDING.md) 含组件与升级检查。固定 identifier/applicationId/数据目录；Android 覆盖安装必须沿用原签名。

## 聊天与隐私

默认 DeepSeek娘，打开进入上次人格；每个人格唯一主聊天入口。旧会话原始行保留，通过归属映射按时间组成永久时间线；升级前一致备份，事务失败回滚。新人格有独立配置与关系/情绪/生活/兴趣容器；自动引擎尚未启用，界面显示未评估/尚未记录。

原生应用「设置 → DeepSeek API」输入本机 Key，保存后清空输入。本设备人格共用该凭据，用量按人格统计。Windows Credential Manager、Android Keystore AES-GCM 加密私有文件，无明文业务表回退，不跨设备同步。Windows runner已通过真实 Credential Manager 写入/读回/删除测试，Android Keystore仍待真机验证。不要把 Key 发到聊天或写入源码。

Rust 请求固定官方 `https://api.deepseek.com/chat/completions`，沿用 `deepseek-flash` / `deepseek-v4-pro` 文本配置。只发送当前人格设定、本次输入及可关闭的有限近期回合；其他人格、长期记忆、整个数据库不发送，没有云数据库。真实 API 由用户在设备本地配置 Key 测试。

Windows Enter 发送、Shift+Enter 换行；手机 Enter 换行、按钮发送。生成中切换时回复/用量仍归原人格，本阶段一次一个请求。保留历史变体、中断恢复和阶段 0 的事件、检查点、墓碑、关系/任务/视频原型。

## 保存源码与记录

`python3 scripts/export_source.py` 生成源码 ZIP/SHA-256，包含 assets、八份计划、逐文件哈希，排除数据、密钥、备份、缓存和安装包。见 [文件清单](docs/FILES.md)、[取回方式](docs/SOURCE_EXPORT.md)。容器内部路径不等于下载附件。

历史 [阶段1报告](docs/PHASE1_REPORT.md)、[dev.1安装构建](docs/INSTALL_TEST_REPORT.md)、阶段0文档保留，以各自版本为准。[数据库设计](docs/DATABASE.md)、[安全边界](SECURITY.md)、[变更记录](CHANGELOG.md) 记录限制。当前阶段 2 已实现需显式启用的 LAN 同步；多维关系和长期记忆自动引擎、虚拟生活调度、动态桌宠、Bilibili 视频理解及语音仍未实现，不能把原型或模拟状态当作已交付功能。NAS 为后续可选节点。

阶段 2 的 v0.0.2 同源双端测试修订已按用户授权发布并验证下载；本次使用持久新测试签名，未发布原正式身份升级包。等待用户验收，不自动开始下一阶段或下一个版本。已发布历史为 v0.0.1 与 v0.0.2 独立测试版。

## 阶段 2 独立测试包

原正式签名恢复保持独立待办。普通 CI 只编译、验证未签名测试候选；当前发布请求明确选择 `.test2`，发布流程从 `TEST_RELEASE_SIGNING_B64` 恢复持久私钥后签名。正式身份 `io.local.deepseekcompanion` 保持不变。当前可安装测试包不能覆盖 v0.0.1 或旧 `.test`，本机 SQLite 与系统凭据属于独立应用沙箱；不会自动导入旧版数据。未来 `.test2` 更新必须复用本次签名，保持身份和数据兼容。

本地测试入口：Linux/macOS `sh scripts/build-android.sh --test-package`；Windows `.\scripts\build-android.ps1 -TestPackage`。它们不替代仓库持久签名发布流程，不能用本地临时签名覆盖公开 `.test2`。正式签名构建仍需独立的原签名配置；无参数构建不会擅自生成正式密钥。仅有 unsigned-ci 成功不等于可安装包交付。最新证据见 [扫码签名交付报告](docs/RELEASE_0.0.2_REFRESH.md)。

Windows 和 Android 各自上传安装包、SHA256SUMS、构建报告与日志；Linux 上传自动化测试报告。构建证据保留在 Actions Artifacts；按用户最新授权，安装包已作为独立文件公开到 v0.0.2 Release，不需要解压后安装。物理双端与家庭网线+Wi-Fi 的验收按 [实机清单](docs/PHASE2_ACCEPTANCE.md) 执行，未执行的项目仍为待验收。
