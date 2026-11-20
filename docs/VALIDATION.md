# 验证范围与待核实问题

## v0.0.9 发布证据

两端使用提交 `7728043708e0d35267e348be533064abe3718faa`。Windows EXE 已在 GitHub Windows runner 安装并启动，安装器、应用图标与原签名通过检查。Android ARM64 APK 验证了遮罩资源、包名、1009 版本号与 v0.0.8 test2 证书。公开下载已核对 25 项附件的 SHA-256。[构建报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.9/BUILD-RESULTS.json) · [同源 CI](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44716167090) · [签名发布](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44718040176)

Linux 162 项、Windows 163 项 Rust 测试通过。另有 25 项 Python 检查、5 项同步模型测试和 81 项界面夹具检查。SQLite 升级、故障回滚和双副本回环 mTLS 测试通过，覆盖离线修改、恢复和永久删除。[自动化日志](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.9/AUTOMATED-TEST-RESULTS.txt) · [迁移说明](V009_RECYCLE_MIGRATION.md)

375×812 浏览器中可见聊天区从 447px 增至 650px。截图中的 Android 输入法通过视口和原生事件模拟。Android 真机安装、家庭网络及真实 DeepSeek 流式对话待验收。[截图与测量](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.9/UI-EVIDENCE.zip) · [实机验收清单](V009_UI.md#实机验收)

首次签后截图在应用窗口开启后采集，聊天区仍为空白。另在 Windows runner 下载并安装具有相同 SHA-256 的公开 EXE，等待后取得包含默认人格、欢迎页、输入区和插件入口的原生截图。复测的调试接口未暴露 WebView，截图证据以实际桌面采集为准。[Windows 原生截图](validation-v009/windows-published-native.png) · [复测步骤](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44719233553)

## v0.0.8 发布证据

双端安装包从 `6caba0d1259db64b9c3e77e713c5279293b5a0bb` 构建。Windows EXE 在 Windows runner 安装并启动。Android ARM64 APK 核对了语音桥接、麦克风权限、包 ID 与 v0.0.7 test2 签名。两包已从公开下载地址回下载并验证 SHA-256。安装后的 Windows 程序通过 WinRT 系统语音生成了 WAV。 [构建报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.8/BUILD-RESULTS.json) · [同源 CI](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44695203939) · [签名发布](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44696382185)

Linux 154 项、Windows 155 项 Rust 测试通过，另有 24 项 Python 检查与 5 项同步模型测试。5 项语音浏览器夹具检查覆盖合成麦克风、音频播放、草稿、权限拒绝和焦点丢失。真实麦克风、系统普通话识别、耳机表现、Android 安装后运行及真实 DeepSeek 对话仍待用户设备验收。[语音验收清单](phases/07-voice-singing.md)

## v0.0.7 发布证据

双端安装包从 `88ec37ef54551c7a05268a0cc54d3eda43d90e2a` 构建并沿用此前的测试签名。已从 GitHub 回下载两包，HTTP 均为 200，文件大小与 SHA-256 匹配。Windows 签名包在 GitHub Windows runner 安装并启动，已安装主程序包含视频中心原生命令。[构建报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.7/BUILD-RESULTS.json) · [同源 CI](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44689094089) · [签名发布](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44690576994)

Linux 151 项、Windows 152 项 Rust 测试通过。生成的 16 秒编码视频用于核对首尾画面、连续音轨及文件变更。另用实际媒体与模型接口夹具走通导入、3 段分析、汇总、临时文件清理和显式保存记忆。浏览器夹具验证了视频中心入口和操作状态。

实际 Qwen3-VL 推理和 Whisper 转写质量、用户设备运行及 B站授权媒体接入待验证。[模型和实机验收范围](phases/06-video-understanding.md)

## v0.0.6 发布证据

Windows EXE 和 Android APK 来自同一提交 `5e80ce8bb1333ade52ee64a4efe33569aabbc253`。两个公开下载均返回 HTTP 200，文件大小与 SHA-256 和发布报告一致。[同源 CI](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44232561567) · [签名发布](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44233536958) · [构建报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.6/BUILD-RESULTS.json)

GitHub Windows runner 实际安装并启动主客户端，确认已安装文件包含桌宠入口和动画。原生启动截图显示聊天首页和桌宠按钮。Linux 146 项及 Windows 147 项 Rust 测试通过，另有 3 项走动算法及 7 项桌宠浏览器夹具检查。[原生启动截图](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.6/windows-installed-window.png)

Android ARM64 APK 的签名与 v0.0.5 test2 一致。Windows 用户设备上的透明点击、多屏、走动与覆盖安装以及 Android 实机运行仍待体验验证。[桌宠验收](phases/05-desktop-pet.md)

## v0.0.5 发布证据

发布源码：`9248ffa1a91a5636bc5c25202a11cc1de50f0820`，Windows 与 Android 报告一致。查看[构建报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.5/BUILD-RESULTS.json)、[同源 CI](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43817794325)和[签名发布运行](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43819179372)。本文描述已发布版本，不把本次文档分支的检查当成重新构建安装包。

| 层次 | 已有证据 | 不能推出的结论 |
| --- | --- | --- |
| 核心逻辑与 SQLite | Linux 142 项、Windows 143 项 Rust 测试；迁移、隔离、任务、恢复等回归 | 用户真实旧库一定无问题 |
| 脚本与界面构建 | 24 项 Python、5 项 Node；前端、Clippy、schema 和品牌检查 | 所有设备上的完整运行体验通过 |
| 同步 | 真实回环 TLS 1.3／双向认证与数据库交换 | 家庭路由器、Wi-Fi、Windows 防火墙或手机后台通过 |
| 模型客户端 | 回环 HTTP／SSE 提供者夹具 | 已使用真实 DeepSeek 或核对真实账单 |
| 浏览器 UI | 普通／测试包各 32 项显式 UI 夹具 | 已在 Windows WebView 或 Android 真机操作通过 |
| Windows 安装 | 中文安装控件、安装结果、进程和原生窗口出现 | 完整内容渲染、真实 API 或用户旧安装升级通过 |
| Android 包 | ARM64、版本、身份、官方签名校验及 v0.0.4 同证书 | 真机安装、覆盖升级、Keystore、相机和通知已验收 |

完整测试文本与截图仍在同版 Release，历史资料保持原位置。[阶段 4 实现报告](PHASE4_REPORT.md)描述测试范围；[实机清单](PHASE4_ACCEPTANCE.md)全部是待用户实际执行事项，不是已通过记录。

## 待核实问题

### Windows 空白窗口截图

v0.0.5 的[发布截图](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.5/windows-installed-window.png)只有空白主窗口和标题栏。`.github/scripts/windows-package-smoke.ps1` 检查安装退出码、进程存活和窗口句柄，然后截图，没有断言前端内容加载。现有证据不足以判断截图过早还是内容加载存在问题，也不能证明完整界面正常。

尚无已验证的通用修复，需在独立代码／平台任务中复现；本轮未改代码或降低测试要求。用户若持续白屏，保留数据并提供系统、版本、WebView2 状态及脱敏错误信息，见 [FAQ](FAQ.md#windows-打开后持续白屏怎么办)。

### 旧界面提示与可选模型分享

部分界面帮助文字仍概括为“本地规则、不额外调用模型”，而 v0.0.5 已有独立模型主动分享开关；部分生活来源记忆的显示标签也未区分 `life` 来源。这些文字需要单独代码修订。当前文档按实际开关解释：普通日程、结构化日记及规则处理不调用模型；允许模型分享和聊天引用资料会产生用量。

### 仍待实际设备验收

真实 DeepSeek 及账单、Android 覆盖安装与安全存储、家庭局域网、相机／键盘／系统栏、通知和长期任务恢复均需用户设备证据。本次没有这些设备、真实 Key 或私人旧库；未将其标成通过。

## 首页截图来源

首页两张图片来自云环境对 v0.0.5 源码执行 `npm run test:ui` 后保留的浏览器截图，不是原生运行截图。使用当前 UI 的合成状态，不包含真实 Key 或对话；模拟提示原样保留，未裁掉或替换。来源、文件摘要见[图片说明](images/README.md)。

## 历史验证记录

以下原文描述阶段 0～3 的历史环境、计数、网络阻碍与候选状态，不代表当前状态。历史失败和测试资料仍保留，不因本次首页整理而删除。

<details>
<summary>历史开发与发布说明（原文保留，不代表当前状态）</summary>

# 阶段3追加修复验证入口

候选0.0.4/1004的修复、测试性质与限制见 [追加报告](PHASE3_FOLLOWUP_REPORT.md)、[扫码根因](PHASE3_SCAN_REPAIR.md)、[本地证据](validation-phase3-followup/README.md)、[实机清单](PHASE3_FOLLOWUP_ACCEPTANCE.md)。当前本地115 Rust、24 Python、5 Node、普通/隔离UI各28项；原21项内核和全部原用例保留，无忽略。实际回环TLS不等于家庭局域网，UI/HTTP提供者是明确夹具，无真实DeepSeek或手机通道。

双端完整原生/签名/安装结果只认同源候选BUILD-RESULTS及Actions，不在执行前宣称成功。版本选择未确认前不公开新版本，原v3仍保留。下面为已公开v3及更早历史记录。

---

# v0.0.3 验证入口

当前阶段3报告：PHASE3_REPORT.md；扫码身份缺陷：PHASE2_PERSONA_IDENTITY_FIX.md；主人实机：PHASE3_ACCEPTANCE.md。本地合成证据：validation-phase3/README.md。原21项内核测试和全部原有用例保留。

本地103 Rust、24 Python、5 Node通过，普通/隔离UI夹具各26项，Clippy/前端/SQLite检查通过。普通/隔离UI最终结果、Windows原生测试/中文安装启动、Android完整APK、签名与公开回下载结果由同源CI及Release `BUILD-RESULTS.json`、日志提供，不用UI夹具代替原生。回环TLS是实际加密套接字，但不等于用户家庭网络；HTTP/SSE提供者是夹具，没有真实DeepSeek API Key。Linux GUI检查缺GTK开发包受阻，未虚报通过。

Secrets元数据HTTP403不表示丢失；CI只记录存在状态，签名任务必须实际恢复原test2私钥并比较官方apksigner证书。无用户数据库/设备，真实重复条数、覆盖升级、家庭网线+Wi-Fi、相机/系统栏/键盘和真实API待验收。以下为历史验证记录，不能作为当前版本全体验收完成证明。

---

# 阶段 0 实际验证记录

日期：2026-11-12。本地版本：`0.1.0-dev.0`。以下仅记录确实执行的检查，不将计划、fixture 或文档支持当成真机结果。

## 环境

| 项目 | 实际情况 |
|---|---|
| 执行系统 | Debian 13 / Linux x86_64 |
| Node / npm | 24.22.1 / 11.9.0 |
| Rust / Cargo | 安装后均为 1.99.1；官方 rustup-init SHA-256 核对通过，安装在工作目录 |
| Python | 3.12.14 |
| SQLite | Rust bundled 3.54.1；Python 运行时 3.53.1，分别实测 |
| Java | OpenJDK 21.0.13 JRE；没有 javac，不能视为 Android 完整 JDK |
| Android | adb、sdkmanager、SDK、NDK 不存在，未连接手机 |
| Windows | 没有 Windows 主机、MSVC/WebView2 验证条件，未执行 Windows 构建 |
| Linux GUI | 缺 GLib/GTK/WebKit 开发依赖，原生桌面检查受阻 |
| 凭据 | 未配置/读取用户 DeepSeek 或多模态 Key，未执行模型 API 请求 |

安全环境检查报告见 [environment.json](validation/environment.json)，不含凭据值。

## 已通过

| 检查 | 实际结果 | 范围 |
|---|---|---|
| `cargo test --workspace --locked` | **21 passed, 0 failed** | 应用实际 Rust/SQLite 内核与明确测试夹具；不含网络或真实模型 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过，无 warning | 共享内核及 CLI；不含 Tauri GUI crate |
| `cargo check -p companion-core --no-default-features --locked` | 通过 | 原生默认核心不包含 lab 导入/伪配对入口 |
| `cargo fmt --all` 与 native manifest format | 完成 | 代码格式化，不等于 native 编译成功 |
| `cargo run --locked -p companion-node -- lab-sync` | 两个真实 SQLite 文件各 2 条消息，收敛，完整性均 ok | 临时数据、进程内交换；network_tested=false |
| CLI `check` 新建诊断夹具 DB | schema 1，integrity ok，SQLite 3.54.1 | 临时夹具位于项目外工作目录，未进入交付 |
| `python3 scripts/check_schema.py` | **33/33** 要求表存在；外键/范围/中文 FTS/完整性通过 | 初始迁移共 42 张声明表，含 FTS 表，排除内部 shadow tables |
| `npm ci --ignore-scripts --no-audit` | 从锁文件干净安装成功 | 安装生命周期脚本保持禁用 |
| `npm run build` | TypeScript 检查及 Vite 生产构建成功 | 主 JS 229.95 kB / gzip 72.82 kB；CSS 7.71 kB / gzip 2.36 kB |
| Chromium/Playwright 界面检查 | 8 项通过 | 浏览器模式；1440px 与 390px、导航、主题、无原生写入、同步提示、无 pageerror/外部请求 |
| Tauri CLI `info` | 正常读取配置和前端框架；报告系统缺依赖 | 不是安装包成功记录 |
| Tauri CLI 图标生成 | 由本项目原创 SVG 生成 PNG/ICO/Android 图标 | 通用 DS 应用标识，不是 DeepSeek 娘角色素材 |
| 原生 manifest metadata / lock resolution | 通过，独立 native Cargo.lock 固定依赖图 | 依赖解析不等于编译或逐行审计 |
| npm audit | 当前数据库报告 **0 已知漏洞** | 不证明绝对安全；未完成 RustSec/原生依赖全量审计 |
| `.gitignore` | DB/WAL、.env、私钥、备份、导出与本机配置被忽略；.env.example 可保留 | 在新建的本地 Git 仓库执行 check-ignore |

核心测试原始结果：[core-tests.txt](validation/core-tests.txt)。实验输出：[sqlite-lab.json](validation/sqlite-lab.json)。其他摘要：[schema.json](validation/schema.json)、[browser.json](validation/browser.json)、[frontend-build.txt](validation/frontend-build.txt)。

## 测试确实覆盖的故障

- 两端同时离线新增消息，恢复合并保留两端；事件重发不重复。
- seq=2 先到不会推进连续检查点；分页交换大于单批数据后收敛。
- 并发记忆修改确定性选主；delete 比 create 更早到达、迟到编辑及消息删除都不复活原 ID。
- checksum 篡改、未知来源、错误档案、同原序号异内容被拒绝；批内已处理的好事件在冲突时一起回滚。
- 中文 FTS 随修订/删除更新，短词与通配符转义有效；重启保留身份、内容、序号和完整性。
- 任务 dedupe key 重用不同输入被拒绝；单协调节点只签发一个有效租约。
- 暂停、恢复、取消、失败/次数上限；过期和权威重启进入 uncertain；旧 fence/boot/负时钟不能提交。
- 关系普通问答不固定加分；证据去重、冷却、来源验证、稳定重放和范围保护。
- 分段覆盖完整尾段；重叠不重复计数；分析进度使用画面/音频交集，不以播放进度替代；不确定区间阻止完成。

## 原生构建阻塞

实际执行：

```bash
cargo check --manifest-path apps/companion/src-tauri/Cargo.toml --locked
```

退出码 **101**，失败于 `glib-sys` 的系统依赖探测：

```text
Package glib-2.0 was not found in the pkg-config search path.
The system library glib-2.0 required by crate glib-sys was not found.
```

Tauri `info` 同时报告 webkit2gtk-4.1、rsvg2 未安装。因此原生 crate 没有完成编译，IPC/窗口尚未运行验证；这不能证明剩余原生源码没有兼容问题。
当前目标是 Windows/Android，未为 Linux GUI 安装整套系统依赖。Windows 和 Android 仍须在阶段 1 用其实际工具链执行构建与真机验收。
没有自动审批拒绝；阻塞来自实际缺少的系统/平台构建条件。

## 未完成，不能宣称通过

Windows 安装包、Android APK、系统密钥存储、真实 DeepSeek SSE、设备发现/配对/mTLS、真实 LAN 同步、Android 后台、角色素材授权确认、透明桌宠、AI 关系事件抽取、本地语义模型、生活/主动消息、真实视频/多模态/B站媒体访问、加密备份恢复及 NAS daemon 均未完成验收。
本次不把 SQLite 合并实验称为“电脑手机已经同步”，不把覆盖算法称为“完整视频已理解”，不把前端生产构建称为原生应用已安装。

## 本地与 GitHub

交付目录限制：聊天预设导出路径 `/codex/browser/projectless/0003-deepseek-ai-companion-v3-0-windows-android-ai-na/output` 在本执行环境不存在，普通写入得到只读文件系统错误，经过工具授权后的写入仍因操作系统目录权限失败。这不是自动审批拒绝。
源码包和完整项目副本改存可写工作区 `/workspace/output`；未上传任何外部服务，也不提供指向不存在导出文件的下载链接。

已初始化一个本地 Git 仓库；没有创建提交、没有配置 remote、没有推送、远程 PR、tag、Release 或发布工作流。
源码归档排除 `.git`、node_modules、target、dist、缓存、DB、密钥、备份和候选仓库快照；文档保留必要审查 hash 与公开来源链接。

**GitHub 未更新，本次修改仅保留在本地。**

</details>
