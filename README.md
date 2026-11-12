# DeepSeek娘

> 项目愿景（保留仓库初始说明）：使用GPT制作的电脑跟手机数据互通的养一只有自己生活能刷b站主动聊天还有好感度系统的大肥鱼开源软件。

实际已实现能力与后续计划以下文为准。

Windows + Android 本地优先陪伴应用，当前准备 **0.0.1 测试版**，继承本地开发版0.1.0-dev.2。继承原阶段 0/1，改为“一人格一个永久聊天空间”：新增/切换/编辑人格，独立上下文、记忆、Token 统计，重启恢复上次选择。保留 DeepSeek 流式聊天、停止、重生成和双端系统密钥存储。

**当前提供 0.0.1 测试版源码，EXE/APK 尚未生成。** [公开仓库](https://github.com/Lyuu508/DeepSeek-Niang)由用户创建，本次上传保留其初始化README历史。指定的海滩图目前只有聊天内嵌预览，工具未得到可读取的原始PNG；正式图标/安装包等待原图文件。可将原PNG作为ZIP文件附件，或放到 `assets/branding/deepseek-niang-master.png` 再运行资源脚本。[本次发布状态与安装说明](docs/RELEASE_0.0.1.md)。

见 [本次报告](docs/PERSONA_BRANDING_REPORT.md)、[迁移与回滚](docs/PERSONA_MIGRATION.md)、[品牌素材状态](assets/branding/README.md)。阶段 2～9 本次仅交付[八份独立计划](DEVELOPMENT_PLAN.md)，没有启用后续功能。

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

收到同一原始 PNG 后，使用已审核的本地 Pillow（本轮环境已有）生成资源，再构建：

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

原生应用「设置 → DeepSeek API」输入本机 Key，保存后清空输入。本设备人格共用该凭据，用量按人格统计。Windows Credential Manager、Android Keystore AES-GCM 加密私有文件，无明文业务表回退，不跨设备同步。不要把 Key 发到聊天或写入源码；两端系统自检仍待实机验证。

Rust 请求固定官方 `https://api.deepseek.com/chat/completions`，沿用 `deepseek-flash` / `deepseek-v4-pro` 文本配置。只发送当前人格设定、本次输入及可关闭的有限近期回合；其他人格、长期记忆、整个数据库不发送，没有云数据库。真实 API 由用户在设备本地配置 Key 测试。

Windows Enter 发送、Shift+Enter 换行；手机 Enter 换行、按钮发送。生成中切换时回复/用量仍归原人格，本阶段一次一个请求。保留历史变体、中断恢复和阶段 0 的事件、检查点、墓碑、关系/任务/视频原型。

## 保存源码与记录

`python3 scripts/export_source.py` 生成源码 ZIP/SHA-256，包含 assets、八份计划、逐文件哈希，排除数据、密钥、备份、缓存和安装包。见 [文件清单](docs/FILES.md)、[取回方式](docs/SOURCE_EXPORT.md)。容器内部路径不等于下载附件。

历史 [阶段1报告](docs/PHASE1_REPORT.md)、[dev.1安装构建](docs/INSTALL_TEST_REPORT.md)、阶段0文档保留，以各自版本为准。[数据库设计](docs/DATABASE.md)、[安全边界](SECURITY.md)、[变更记录](CHANGELOG.md) 记录限制。没有启用 LAN、桌宠、自动关系/生活、视频或语音，未来 NAS 为可选节点。

GitHub发布仅限本次明确授权的0.0.1测试版；后续更新仍需用户明确指令。实际上传与安装包状态见本次发布记录。
