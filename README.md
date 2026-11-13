# DeepSeek娘

> 项目愿景（保留仓库初始说明）：使用GPT制作的电脑跟手机数据互通的养一只有自己生活能刷b站主动聊天还有好感度系统的大肥鱼开源软件。

实际已实现能力与后续计划以下文为准。

Windows + Android 本地优先陪伴应用，当前已发布 **0.0.1 测试版**，继承本地开发版0.1.0-dev.2。继承原阶段 0/1，改为“一人格一个永久聊天空间”：新增/切换/编辑人格，独立上下文、记忆、Token 统计，重启恢复上次选择。保留 DeepSeek 流式聊天、停止、重生成和双端系统密钥存储。

**0.0.1 测试版**使用用户提供的正式海滩PNG，原图按字节保留，Windows/Android图标和聊天头像已生成。安装文件通过 [GitHub Releases](https://github.com/Lyuu508/DeepSeek-Niang/releases) 交付，分别选择Windows EXE或Android ARM64 APK；实际构建及设备验证范围见附件说明与 [安装测试说明](docs/RELEASE_0.0.1.md)。

- [Windows 10/11 x64 安装 EXE](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_windows-x64-setup.exe)
- [Android 10+ ARM64 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_android-arm64.apk)
- [0.0.1测试版发布页](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.1) · [SHA-256](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/SHA256SUMS.txt)

两个安装包均已从公开链接完整下载并校验哈希。Windows 原生构建、58项测试、静默安装及窗口启动通过；Android 完整构建和签名/资源检查通过，尚未真机运行。Windows 安装器未做 Authenticode 签名，APK 使用原开发签名；真实 API 请在各设备设置中输入自己的 Key 验收。

见 [本次报告](docs/PERSONA_BRANDING_REPORT.md)、[迁移与回滚](docs/PERSONA_MIGRATION.md)、[品牌素材状态](assets/branding/README.md)。阶段 2～9 本次仅交付[八份独立计划](DEVELOPMENT_PLAN.md)，没有启用后续功能。

## 阶段 2 开发状态（目标 0.0.2）

以 main `22096939c20134708c6dc1a4476468a7bbe96827` 为基线重新实现局域网同步，没有使用旧云端未提交代码。
当前新增生产同步协议、Windows mDNS/Android NSD、一次性二维码与双方确认、TLS 1.3 与系统安全存储、来源签名、事务队列与同步页面。
阶段仍待原签名恢复、GitHub Actions 双端成功构建及发布；**0.0.2 尚未发布**，上方 0.0.1 链接是历史包。
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

历史 [阶段1报告](docs/PHASE1_REPORT.md)、[dev.1安装构建](docs/INSTALL_TEST_REPORT.md)、阶段0文档保留，以各自版本为准。[数据库设计](docs/DATABASE.md)、[安全边界](SECURITY.md)、[变更记录](CHANGELOG.md) 记录限制。没有启用 LAN、桌宠、自动关系/生活、视频或语音，未来 NAS 为可选节点。

GitHub发布仅限本次明确授权的0.0.1测试版；后续更新仍需用户明确指令。实际上传与安装包状态见本次发布记录。
