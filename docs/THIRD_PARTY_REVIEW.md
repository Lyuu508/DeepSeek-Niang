# 第三方静态初审与接入计划

范围：2026-11-12 读取 3 个候选仓库公开信息，固定提交并获取指定文本文件。**没有执行其安装/构建脚本，没有运行二进制，没有复制任何角色素材到应用。**
这不是完整安全审计，也不能证明无病毒。`review-evidence.json` 记录 16 个本地 UTF-8 文本快照的 SHA-256（不是 Git blob SHA）。许可证/清单作全文核对，较长源码基于定向检索和关键片段检查，未逐行审计。

## 候选结果

| 仓库 / 固定 commit | 许可与初步观察 | 决定 |
|---|---|---|
| C2006L/dafeiyu `7ca2bb30f977e01c52198e27cff1e9264c92c755` | 根 LICENSE 为 MIT；README 描述 vendored dsh-pet 与社区二创，尚未形成逐项素材授权链。批处理会 npm install、prepare、下载 Electron、执行 PowerShell 并按特征停止旧进程；README 提到镜像下载 | 不运行启动器，不整体移植；后续只评估有来源的动画与交互思路 |
| c-dom/deepseek-pet `eb2ef68fa648df3f1b5246fff861655a780e6daf` | LICENSE 的 MIT 覆盖代码，明确排除 web 下角色图像/精灵等素材。Python 依赖为宽版本范围，包括 Qt、PyAutoGUI、psutil、rembg、PyInstaller；backend 默认 loopback WebSocket，片段中无客户端认证；main 中有 Windows DPAPI 保护 Key 的代码 | 素材先不复用；不能把 loopback 广播直接搬成 LAN 服务；Qt/Python 壳与 Tauri 无直接复用收益 |
| Andersen216/dsh-whale-girl-live2d `f977bef508690e75ba6535cd8f4ce18e7f6ba9b0` | NOTICE/PROVENANCE 区分 MIT 代码、CC BY-NC-SA 4.0 模型和专有 Cubism Core；说明三位创作者来源。现有实现面向 DSH 宿主，Windows 壳有调用 dsh 插件安装命令的路径，lib 会访问 DeepSeek balance；带本地 token/配置文件逻辑 | 作为素材与状态映射候选；需核验上游授权链和 Live2D 分发条款。当前不接入 DSH 依赖、不复用安装路径 |

源码当前检查的网络请求/进程调用本身不构成恶意证据；它们扩大权限和运行依赖，需要与新项目最小能力要求重新评估。
仓库更新时间不等于持续维护承诺；本次只记录了 GitHub pushed_at，不以 stars/README 判定可靠。

## 已获取文件范围

- dafeiyu：LICENSE、README、启动批处理、嵌套 package.json、prepare.js。
- deepseek-pet：requirements、install.ps1、backend_server.py、main.py、build_app.py；LICENSE 另从固定 commit 完整读取。
- live2d：NOTICE、PROVENANCE、根 package.json、Windows package.json/main.js、lib/index.js。

未获取完整素材、可执行发行包；未对 lockfile、所有脚本、Actions、上游依赖与实际网络流量作全量审查。
为避免分发不明权利内容，交付包仅包含本报告与 hash 清单，不携带候选仓库源码/素材快照。

依据：[dafeiyu 固定版本](https://github.com/C2006L/dafeiyu/tree/7ca2bb30f977e01c52198e27cff1e9264c92c755)、[deepseek-pet 许可](https://github.com/c-dom/deepseek-pet/blob/eb2ef68fa648df3f1b5246fff861655a780e6daf/LICENSE)、[Live2D NOTICE](https://github.com/Andersen216/dsh-whale-girl-live2d/blob/f977bef508690e75ba6535cd8f4ce18e7f6ba9b0/NOTICE.md)、[Live2D PROVENANCE](https://github.com/Andersen216/dsh-whale-girl-live2d/blob/f977bef508690e75ba6535cd8f4ce18e7f6ba9b0/PROVENANCE.md)。

## 正式复用前必须执行

1. 以固定 commit 取得源码清单，识别 submodule、Git LFS、构建生成文件与预编译内容；核查下载来源。
2. 阅读 package/Cargo/Python 依赖与锁文件，审查 preinstall/install/postinstall/prepare/build、PowerShell/bat/sh、GitHub Actions 权限与发布动作。
3. 检索并逐项评审动态执行、网络目的地、文件访问、注册表、提权、进程控制、自动更新、远程 WebView 和本地端口鉴权。
4. 针对所需素材逐文件记录原作者、角色衍生许可、模型绑定、动作、字体/音频、商用/再分发/改编/署名条件；仓库 MIT 不能替代美术许可。
5. Live2D Core 单独核对 SDK 发布条款、应用用途及需附带 notice，核验供应链 hash；不把仓库里的压缩 JS 直接视为官方可信构件。
6. 只移植最低必要代码/资源，替换 shell 安装器为应用自身受控适配；在隔离环境、无用户 Key/数据库条件下构建和观测网络。
7. 检查 Windows 透明/拖动/缩放/托盘、资源占用；确认不会启动第二套聊天服务或另存一份人格记忆。

## 本项目新增依赖审查

基础 Rust 依赖：rusqlite/SQLite、serde/json、sha2、uuid、thiserror，均固定版本并生成 Cargo.lock。相关构建依赖含 C 编译工具、SQLite 绑定与 proc-macro，构建脚本不是无权限数据。
Rust 工具链从官方 static.rust-lang.org 下载；rustup-init 对照官方 SHA-256 验证后安装到工作目录，未修改 HOME 或用户全局 PATH。
npm 直接依赖为 React、Tauri 官方 API/CLI、TypeScript、Vite 与类型包；先查看 registry 版本/脚本元数据，再以 `--ignore-scripts` 安装，package-lock 固定完整性。
npm audit 的当次结果单独记录在 VALIDATION；零已知告警不等于全部供应链经逐行审计。RustSec 全量扫描与原生平台依赖审查尚未执行，列为后续安全门槛。
未引入云数据库 SDK、自动发布动作、第三方桌宠依赖或未知可执行安装包。

## 阶段 1 新增依赖与原生代码复查

本次没有重新下载/执行三个桌宠候选项目的代码或素材。新增网络/凭据依赖固定版本并锁定：reqwest 0.12.28（MIT/Apache-2.0）、tokio 1.54.1（MIT）、tokio-util 0.7.20（MIT）、futures-util 0.3.34（MIT/Apache-2.0）、zeroize 1.9.1（MIT/Apache-2.0）、windows-sys 0.61.2（MIT/Apache-2.0）。选择 reqwest 的 rustls-tls-webpki-roots 特性，不依赖 Android 未初始化的平台证书 verifier，不引入 OpenSSL 或自制 TLS。

检查直接清单/锁文件、registry 版本与许可元数据、reqwest TLS 选择代码和构建依赖；TLS 栈包含 ring 的 C/汇编构建流程，Windows 跨构建失败于缺少 MSVC lib.exe；不能绕过为不验证证书。Tokio/rustls 等传递依赖并未逐行审计，RustSec 扫描仍未执行。

新私有 Android 插件仅依赖已有 Tauri Android 模块，Rust tauri-plugin 构建 helper 固定 2.7.2；Kotlin 使用系统 AndroidKeyStore、Cipher、AtomicFile，不引入未知加密二进制。Windows 直接绑定 Microsoft Credential API，检查长度、生命周期和返回码。FFI/Keystore 的设备运行验证仍待完成。

浏览器测试新增 Playwright 1.65.0（Apache-2.0），仅开发依赖；npm 使用 --ignore-scripts，不执行浏览器自动下载脚本，本次调用环境既有 Chromium。生产 bundle 检查不包含测试夹具，未新增运行时 CDN、云数据库或发布依赖。
当次 npm audit production/all 均为 0 已知告警，原始 JSON 保存在 validation-phase1；这是时间点结果，不能作为全面无漏洞保证。源码导出扫描和测试均不含真实 Key、账号或用户聊天。

参考：[Microsoft CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew)、[CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw)、[Android Keystore](https://developer.android.com/privacy-and-security/keystore)、[Tauri mobile plugin](https://v2.tauri.app/develop/plugins/develop-mobile/)。

## dev.2 人格/品牌改造复查（2026-11-12）

未增加外部npm/Cargo依赖或改其锁定版本；只更新本项目自有包版本。外部锁条目按 dev.1 源码包逐项核对，结果在 validation-persona/dependency-preservation.json。新增脚本为本工程本地代码，未执行候选桌宠脚本、未知EXE、远程PowerShell、管理员操作或新安装程序。

已有Pillow13.0.0仅用于本地确定性裁切/缩放及临时纯色夹具测试；未下载新库/模型，未把夹具当角色。原始PNG缺失时校验非零退出，Tauri原生dev/build和本机脚本不以旧DS图标交付新版。当前只做资源脚本/哈希/安全区算法测试，实际人物裁切与系统显示仍须原图及真机。

PowerShell脚本静态检查沿用已审阅的官方工具命令，当前环境未在Windows执行；Android准备脚本只改生成工程的名称/资源/网络/备份/键盘策略，applicationId/签名不变。没有GitHub工作流或自动发布。静态检查不构成绝对安全证明。

## 0.0.1正式图标与Windows构建（2026-11-13）

原始PNG仅从用户ZIP提取单个PNG，限制大小、拒绝符号链接、不执行归档内容。Pillow只缩放/裁切；master原字节校验。没有引入角色/声音/视觉模型或新API。

Windows构建为手动workflow_dispatch，contents:read，没有push/tag触发或发布操作。固定GitHub官方actions/checkout v7.0.1、setup-node v7.1.0、upload-artifact v7.0.2的完整提交（见workflow）。已静态核对其action.yml、package.json与运行入口，都是Node24分发代码，不执行其开发期npm脚本；checkout不保留凭据，禁自动包缓存。有限静态复查不能证明绝对无风险。

Windows smoke脚本只允许在GITHUB_ACTIONS临时runner中执行，安装本次编译的NSIS，启动安装后的程序并记录实际窗口/进程状态；只上传安装器、编译程序、测试结果和应用截图，不上传数据库、用户Key或签名私钥。Android签名仍只留本地，未上传CI。
