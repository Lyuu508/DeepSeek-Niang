# 首页整理前的历史说明

以下保留 v0.0.5 发布提交中的原首页内容，仅校正迁移后的相对链接。旧候选状态与下载入口属于历史，不作为当前安装推荐；新用户请看[项目首页](../../README.md)。

> [v0.0.5 安装与发布页](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.5) · v0.0.5 阶段4：在既有生活/任务表上增加连续计划、恢复、结构化日记、兴趣和主动分享。Android 保持 `.test2` / 1005；双端必须同时更新。详见 [阶段4报告](../PHASE4_REPORT.md) 和 [升级验收](../PHASE4_ACCEPTANCE.md)。本地规则不调用模型，主动模型分享需另行开启；实体设备与真实 API 结果单独验收。以下历史说明按原版本保留。

# DeepSeek娘

DeepSeek-Niang AI Companion 是 Windows + Android 本地优先陪伴应用，沿用 Tauri 2、React、TypeScript、Rust、SQLite 与 DeepSeek API。默认一个人格一个永久聊天空间，不使用云数据库。

## 阶段3追加修复候选 0.0.4 / Android1004

针对主人真实反馈继续修复：旧版已签名待发队列缺少新增列，导致首次同步整批拒绝及默认人格未归一；手机主动双向同步后电脑反向发现失败也会错误显示未连接。已新增真实旧队列/TLS回归并修复，家庭网络仍待实机验收。

手机扫描电脑新二维码时可安全配置电脑已保存的 API Key（证书认证后的独立短期传递，二维码/普通同步不含Key）。新增完整旧聊的本地全文检索，与结构化记忆共用有限上下文预算，不额外调用模型。电脑按主人示意调整为顶部横栏、左侧实际人格、中央聊天和右侧状态/设置。

[追加报告](../PHASE3_FOLLOWUP_REPORT.md) · [连接/重复人格根因](../PHASE3_SCAN_REPAIR.md) · [实机验收](../PHASE3_FOLLOWUP_ACCEPTANCE.md) · [自动化证据](../validation-phase3-followup/README.md)。当前新版本公开选择待确认，候选工作流默认草稿，无新公开Release/Tag；下方0.0.3链接仍是已公开历史安装包，不包含本次追加修复。现有.test2与持久签名保持、旧数据不删除。

## v0.0.3 已公开测试版 / Android 1003

本版修复扫码后默认人格重复，加入五维关系、持续情绪、分层记忆与虚拟生活基础。[阶段 3 报告](../PHASE3_REPORT.md) · [人格身份修复](../PHASE2_PERSONA_IDENTITY_FIX.md) · [安装与实机清单](../PHASE3_ACCEPTANCE.md)。下列链接由同源构建、签名和回下载检查通过的发布工作流生成；构建事实以 Release 的 BUILD-RESULTS 为准。

- [Windows 中文 EXE 安装程序](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.3/DeepSeek-Niang_0.0.3_windows-x64-setup.exe)
- [Android ARM64 扫码测试 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.3/DeepSeek-Niang_0.0.3_android-arm64-isolated-test.apk)
- [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.3/SHA256SUMS.txt) · [BUILD-RESULTS.json](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.3/BUILD-RESULTS.json) · [发布页面](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.3)
- [测试报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.3/AUTOMATED-TEST-RESULTS.txt) · [签名证明](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.3/ANDROID-SIGNING-REFERENCES.json)

无需解压。关闭电脑上的旧程序后安装 EXE；手机打开 **“DeepSeek娘 扫码测试版”**。APK 保持当前 0.0.2 `.test2` 的 applicationId 和已保存签名，1003 高于 1002，满足该测试版的签名/版本升级条件；实际手机覆盖仍待验收。它与原 v0.0.1 正式版、早期 `.test` 身份不同，不能覆盖或自动读取它们的私有数据。请保留旧应用及数据，不卸载、不清数据。[签名复用与旧身份迁移说明](../TEST_SIGNING_SETUP.md)。Windows 沿用自签名测试证书，尚非公共 CA 信任发布者。

## 已公开v0.0.3的能力与限制（历史）

- **聊天与同步**：人格独立永久时间线、流式/停止/重生成、系统安全存储。Windows 出码、Android 实时相机扫一扫，TLS 1.3 双向认证、证书固定、增量/去重/冲突/断线补传；同一路由器下网线电脑与 Wi-Fi 手机无需都用无线连接。
- **同一人格**：手机采用已认证 Windows 主机默认人格的原 ID、资料、头像、人设和设置；自动默认记录以可审计别名关联，旧消息和记忆 ID 保留，手动创建的人格不合并。已配对双端更新后可自动修复，无需清库。v3两端要求schema6，本次候选两端要求schema7。
- **关系与情绪**：熟悉、信任、好感、亲密、舒适分别计算；有来源、变化原因和历史，问候不固定加分。关系阶段和未过期情绪参与真实聊天请求上下文，保留人格边界。11 种情绪可持续并自然回落。
- **长期记忆**：近期上下文与情景/语义/关系/知识记忆；本地批量规则选择明确有价值的内容，去重、重要性、保护、冲突确认、编辑/删除、来源时间和预算检索。不代表通用语义理解或向量检索。
- **虚拟生活基础**：Windows 应用在线时运行虚拟活动、日常台账、下一计划；手机查看同步状态。主动消息默认关闭，支持冷却、每日上限、安静时段。看视频/听音乐是明确标识的虚拟活动，没有实际播放、浏览或理解媒体。
- **成本**：每人格/每天按聊天、记忆、生活任务统计实际 API 用量；本地规则整理与生活调度不额外调用模型。未知 Token 不伪造，输入按条数/字节预算控制，不发送全部聊天历史。

关系/记忆/情绪/生活已有真实 Rust/SQLite 与本机 TLS 自动化测试；浏览器测试明确使用 UI 夹具。家庭局域网、相机、系统栏/键盘、用户旧库和真实 DeepSeek 效果 **尚待主人实机验收**。本任务没有用户设备通道、真实 API Key 或实际用户数据库，不能声称已检测用户手机里的重复条数。测试范围见 [验证说明](../VALIDATION.md)。

## 本地开发与构建

Rust 1.99+、Node 22.12+、Python 3。依赖锁定；云环境可用 `python3 scripts/cloud_env.py cargo …` 激活已准备工具链。

```bash
npm ci --ignore-scripts
npm run build
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 scripts/check_schema.py
python3 scripts/version.py
python3 -m unittest discover -s tests -p 'test_*.py'
node --test tests/sync-model.test.mjs
node scripts/check-branding.mjs
npm run test:ui
```

UI 测试需要 Chromium。普通浏览器仅预览，DEV 且显式 `?test=mock` 才使用带标记的内存夹具；它不验证 SQLite/系统凭据，生产包不包含夹具。用户原始海滩 PNG 与 69 项品牌派生资源原样保留，不重新设计。

[构建说明](../BUILDING.md) · [SQLite 与备份](../DATABASE.md) · [开发计划](../../DEVELOPMENT_PLAN.md) · [版本规则](../VERSIONING.md) · [变更记录](../../CHANGELOG.md)。当前候选工作流仅恢复仓库现有签名，缺失或不符则停止签名，不生成替代密钥；两端独立构建，统一核验后默认保留草稿，确认版本后才公开。完成本版后停止，等待验收，不自动开始阶段 4。

## 历史交付

[v0.0.2 扫码与持久签名交付](../RELEASE_0.0.2_REFRESH.md) · [v0.0.2 发布页](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.2) · [v0.0.1 发布页](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.1) · [阶段 1](../PHASE1_REPORT.md) · [品牌与人格重构](../PERSONA_BRANDING_REPORT.md)。旧版本、Tag、历史记录和旧安装数据保留。
