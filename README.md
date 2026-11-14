# DeepSeek娘

## 0.0.2 测试版直接下载

- [Windows EXE 安装程序](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_windows-x64-setup.exe)
- [Android 独立测试 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk)
- [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SHA256SUMS.txt) · [BUILD-RESULTS.json](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/BUILD-RESULTS.json) · [GitHub Release](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.2)

直接下载安装，不需要解压。Android 测试包为 `io.local.deepseekcompanion.test`，可与 v0.0.1 并存，不能覆盖旧正式身份。真实双端与家庭局域网验收仍待用户进行；原签名升级保持独立待办。以后已授权版本均直接提供 Release 安装包附件，Actions 仅保留构建证据，不作为唯一交付。

以下保留项目与历史版本说明；最新公开版本为 0.0.2 测试版。

扫码与中文安装修订正在验证：电脑出码即授权一次配对，新版手机扫码后自动选取二维码中的局域网地址并验证身份。**已发布手机包暂不更新**：按用户选择保持当前测试身份，等待相同签名；旧手机使用“旧版手机：显示兼容配对码”。Windows 中文安装器独立交付，具体范围见 [修订报告](docs/PHASE2_SCAN_ZH_REPORT.md)。

内部英文名称：DeepSeek-Niang AI Companion。最新测试交付见 [阶段 2 测试报告](docs/PHASE2_TEST_DELIVERY.md)；接管时的状态快照见 [项目状态报告](docs/TAKEOVER_2026-11-14.md)。

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
当前新增生产同步协议、Windows mDNS/Android NSD、一次性二维码与双方确认、TLS 1.3 与系统安全存储、来源签名、事务队列与同步页面。
阶段 2 的 LAN 同步实现和双端测试构建已完成：测试源码 `1c86678` 的 [CI 43567948075](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43567948075) 中 Linux 测试、Windows EXE 构建/安装启动和 Android `.test` 独立开发签名 APK 检查全部成功。[测试包、SHA256SUMS 与报告下载](docs/PHASE2_TEST_DELIVERY.md#最终同源-ci-交付结果)；测试 APK 可独立安装，不能覆盖正式版。原签名升级 APK、远程附件独立复核、家庭局域网实机验收与正式发布仍未完成；**0.0.2 独立测试 Pre-release 已公开，原签名升级版尚未提供**，上方 0.0.1 链接是历史包。旧 Run 的未签名检查 APK 仍不可安装，不能与本次隔离测试包混淆。
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

当前已获用户授权继续阶段 2 的 v0.0.2 收尾与双端测试发布；仅在原签名、同源码双端包与必要检查全部满足后发布。不得自动开始下一阶段或下一个版本。已发布历史为 v0.0.1 与 v0.0.2 独立测试版。

## 阶段 2 独立测试包

原签名恢复和 GitHub Secrets 配置现列为独立待办，不再作为日常开发/独立构建的前置条件。当前普通 CI 构建 Android 开发测试签名 APK：applicationId 为 `io.local.deepseekcompanion.test`，启动名称“DeepSeek娘 测试版”，应用内有测试标记；正式身份 `io.local.deepseekcompanion` 保持不变。测试包不能覆盖 v0.0.1，本机 SQLite 与系统凭据属于独立应用沙箱；不会自动导入旧版数据。不同 Runner 的测试签名可能不同，不保证旧测试包可以直接覆盖，不能通过卸载正式版解决。

Linux/macOS：`sh scripts/build-android.sh --test-package`；Windows：`.\scripts\build-android.ps1 -TestPackage`。正式签名构建仍需独立的原签名配置；无参数构建不会擅自生成正式密钥。仅查看旧 Run 中的 unsigned-ci 成功不等于新的可安装测试包已验证。最新构建证据见 [测试交付报告](docs/PHASE2_TEST_DELIVERY.md)。

Windows 和 Android 各自上传安装包、SHA256SUMS、构建报告与日志；Linux 上传自动化测试报告。构建证据保留在 Actions Artifacts；按用户最新授权，安装包已作为独立文件公开到 v0.0.2 Release，不需要解压后安装。物理双端与家庭网线+Wi-Fi 的验收按 [实机清单](docs/PHASE2_ACCEPTANCE.md) 执行，未执行的项目仍为待验收。
