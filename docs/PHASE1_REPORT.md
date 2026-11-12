# 阶段 1 实际开发报告

> 本文保留阶段 1 首次实现时的验证记录。后续已成功生成 Windows NSIS 与 Android APK，最新结果见 [安装测试交付](INSTALL_TEST_REPORT.md)；设备运行、系统凭据与真实 API 验收仍待完成。

日期：2026-11-12。本地开发版本 **0.1.0-dev.1**，继承 `/workspace/output/deepseek-ai-companion` 与原始阶段 0 ZIP。阶段 1 的代码实现已落地，**双端安装运行和真实 DeepSeek API 的出口验收尚未完成**。

## 代码完成范围

- `companion-chat`：固定官方 HTTPS 地址的 reqwest/rustls 客户端，增量 UTF-8/SSE、真正的流式回调、停止、连接/读取/整体超时、静态脱敏错误、一次显式 429 重试和实际返回用量；无真实请求结果不明确时的自动重发。阶段 1 不开工具调用或思考链。
- `companion-secrets`：无 Serialize/明文 Debug 的 Secret，Windows Generic Credential Manager；Android 私有插件桥接 Keystore AES-256-GCM，密文 AtomicFile 存于 noBackupFilesDir。可配置、替换、删除、查询是否存在及自检，JS 无读取 Key 命令，无明文 SQLite/配置文件回退。
- 聊天仓储：多会话、原有 UUID/设备/逻辑序号事件、原子写用户消息和生成请求、逐段落盘草稿、最终回复与用量事务、幂等请求、最新用户轮重新生成、多版本选择、异常退出恢复与文件锁。
- Tauri IPC 与 React：同一套 Windows/Android 业务代码；蓝白/深色、中文/IME、流式状态、停止/重试、历史变体、自动滚动、多会话、可编辑人设/称呼、上下文范围和 Token 统计、移动布局与可视视口适配。WebView 重建后读取活动草稿快照并可停止。角色仅用明确标记的 DS 占位标识。
- 关系和短期情绪只预留 `ContextInjection` 可信状态接口，默认不注入虚构数值；没有按消息加好感。长期记忆管理原型仍保留，但本阶段不向模型自动发送记忆。

## 真实 API 接入

2026-11-12 依据官方 [入门](https://api-docs.deepseek.com/) 和 [Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/) 核对：

`POST https://api.deepseek.com/chat/completions`，原生请求携带 Bearer 认证；模型 `deepseek-flash` / `deepseek-v4-pro`。JSON 包含 messages、stream=true、stream_options.include_usage=true、thinking.type=disabled、temperature、max_tokens。

按 SSE 事件而非 HTTP chunk 解析，支持中文跨字节边界，校验 finish_reason、[DONE] 和用量一致性；截断/异常流不会标为成功。拒绝重定向，不把认证头发给其他目的地。最多对明确拒绝的 429 重试一次；网络结果不确定时由用户手动重试。缺失用量记录 unknown，不编造 Token 或费用。

输入 Key 的唯一正式入口为设备应用设置；不从 `.env` 读取。没有用户 Key，本次**没有向 DeepSeek 发送真实聊天请求，也没有真实账单数据**。本地 loopback HTTP 服务用于协议与错误测试，浏览器 Mock 是单独的内存夹具，两者均不代表供应商可用性测试。

## 数据库变更与阶段 0 兼容

`0001_initial.sql` 原样保留，新增 `0002_chat.sql`，user_version=2。向 conversations/messages 增加时间/模型/请求元数据，新增 chat_requests、chat_generations、chat_turn_choices、chat_preferences；约束和索引见 DDL。

已有版本 1 文件升级前，通过 SQLite Online Backup 产生同目录 `*.pre-v2-<UUID>.bak`，然后事务迁移；校验迁移 checksum，拒绝未知高版本或已改旧迁移。备份含私人内容、尚未加密，应留在应用私有目录；它不是用户可管理的加密历史备份功能。

用户消息立即与 sync_events 同事务保存；非空最终/中断回复同样形成原有 v1 消息事件。草稿独立保存，空失败不伪造回复事件。每次生成有独立请求/变体 UUID 和输入哈希；重生成不重复用户消息，重复结束/迟到 token 无效。退出后下次打开将进行中请求标为 APP_RESTARTED/interrupted，不自动收费重发。文件锁避免第二进程把活跃生成误判成崩溃。

逻辑时钟仍用于合并，显示用墙上时间不参与冲突裁决。原有消息去重、检查点、删除传播、关系规则、租约/fence、视频结构、NAS 无 UI 节点全部保留；原始 21 项测试仍通过。首次配对、LAN 通道、会话标题/人物设置/所选回复版本的跨设备同步投影仍属于阶段 2，当前不能声称两端已经同步。

## 本次执行结果

| 检查 | 实际结果与边界 |
|---|---|
| Rust workspace | 48 项通过：原有 21 + 聊天仓储 11 + 网络/服务 13 + 凭据基础 3 |
| 网络层 | 真正本地 HTTP/SSE 测试：请求 JSON、中文流、取消、超时、截断、401/恶意错误体脱敏、429、重定向、用量和服务持久化；供应商为 Mock |
| UI 浏览器 | 13 项 Mock 验收通过，桌面/390px 手机、中文/IME、停止/重生成、历史隔离、设置/清空密钥、错误和主题；不是原生 SQLite/Keystore 测试 |
| Python 策略 | 3 项通过：manifest 准备幂等、源码排除私人文件/符号链接、凭据形状扫描 |
| SQLite DDL | 全部原有 33 张要求表和 4 张聊天新表存在，FK/约束/中文索引/完整性通过 |
| TypeScript/Vite | 类型检查与生产打包通过，生产包不包含 Mock 夹具；ZIP 解压后独立 npm ci / build 也通过 |
| Clippy/fmt | workspace all-targets -D warnings 与格式检查通过 |
| Windows 凭据目标 | x86_64-pc-windows-msvc 的 companion-secrets cargo check 通过；未执行 Windows OS 调用 |
| Android 目标 | aarch64-linux-android 的 secrets 与私有 Keystore Rust 插件 cargo check 通过；未编译 Kotlin/Gradle、未运行设备 |
| Windows 实际打包尝试 | tauri build --target x86_64-pc-windows-msvc --bundles nsis 失败：Linux 缺 MSVC lib.exe 等工具；无安装包 |
| Android 实际初始化尝试 | tauri android init --ci 失败：SDK/NDK 不存在，ANDROID_HOME 未配置；无 APK |
| npm audit | 当次 production/all 均 0 已知告警，见验证 JSON；零已知告警不证明供应链绝对安全 |
| DeepSeek 真实连通 | 未执行，缺用户在设备本地配置的 Key |

当前 Linux：Node 24.22.1、Rust 1.99.1，只有 Java JRE、无完整 JDK/SDK/NDK；缺 GLib/GTK/WebKit 开发包，未在 Linux 运行 Tauri GUI。当前 Linux 凭据后端故意返回不支持，不用明文回退。

命令输出、环境、审核摘要与明确标记的 Mock 截图位于 `docs/validation-phase1/`。原 `docs/VALIDATION.md` 及 `docs/validation/` 为阶段 0 历史记录，不能当作本次结果。

## 未完成与后续顺序

1. 在 Windows 原机运行构建脚本和凭据读写测试，安装启动；在完整 Android SDK/NDK/JDK 环境编译 Kotlin/Gradle、生成并安装 APK。交叉 cargo check 只能证明对应 Rust 代码可编译。
2. 两台设备上实际执行安全存储自检、Key 替换/删除/重启、取消和进程终止恢复；当前方案基于平台官方 API，运行时兼容性尚待验证。
3. 用户在设备设置输入 Key，完成真实 DeepSeek 多轮/流式/Token/错误验收。不得用 Mock 结果填充真实测试记录。
4. 历史暂按会话全部加载、会话列表最多 200 个；超大历史分页、完整导入/导出/加密备份、数据库损坏恢复 UI 待后续补齐。SQLite 本身未加密。尚无 APK/EXE 签名发布流程。
5. 阶段 1 真机出口通过后进入阶段 2：可信设备配对、认证加密通道、LAN 增量同步及人物配置/回复选择的合并适配；保留阶段 0 协议原型的隔离。

修改路径逐项列于 [FILES](FILES.md)。构建见 [BUILDING](BUILDING.md)，源码保存/下载见 [SOURCE_EXPORT](SOURCE_EXPORT.md)。

**GitHub 未更新，本次修改仅保留在本地。** 未执行 push、远程 PR、tag、Release 或自动发布。
