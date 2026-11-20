# Windows 安装说明

**v0.0.10 测试版**适用于 Windows 10/11 x64。中文 EXE 随主客户端安装桌宠。[下载中文 EXE](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.10/DeepSeek-Niang_0.0.10_windows-x64-setup.exe)

从「设置 → 语音与麦克风」试听朗读和原创唱段。Windows 使用系统安装的语音包。语音输入需要已安装的 whisper-cli 与本机模型。[语音使用与验收](phases/07-voice-singing.md)

## 安装步骤

1. 已有用户先阅读[升级与数据保护](USAGE.md#升级与数据保护)，退出旧应用，保留当前用户应用数据目录的完整备份，不先卸载或删库。
2. 下载 EXE 和同版 [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.10/SHA256SUMS.txt)。在文件目录运行：

   ```powershell
   Get-FileHash -Algorithm SHA256 .\DeepSeek-Niang_0.0.10_windows-x64-setup.exe
   ```

   与清单逐字核对摘要；不一致时停止使用该文件。
3. 运行安装器。当前有自签名测试签名，并非公共 CA 信任发布者；不要关闭系统防护或导入不明根证书。
4. 缺少 WebView2 时，从[微软官方页面](https://developer.microsoft.com/microsoft-edge/webview2/)安装 Evergreen Runtime 后再尝试启动。
5. 打开“设置”验证安全存储，再输入自己的 DeepSeek API Key。聊天需要互联网和提供商额度；密钥由 Windows Credential Manager 保管。
6. 使用手机时同时更新手机到 v0.0.10，再按[连接说明](USAGE.md#连接电脑与手机)扫码。扫码会授权给本次认证手机配置电脑已有 Key，请仅向可信手机展示。

## 显示桌宠

桌宠随这个 EXE 安装。启动应用后，在顶部打开「桌宠」并勾选「显示桌宠」。移开鼠标后角色会在当前屏幕内走动。[桌宠操作](phases/05-desktop-pet.md)

本机视频分析可在侧栏「视频中心」运行。[工具与模型准备](phases/06-video-understanding.md)

## 数据与验证边界

应用通过当前用户的 Tauri 应用数据目录保存 `companion.sqlite3`，Windows 标识为 `io.local.deepseekcompanion`。数据库与备份未做库级加密，迁移前一致性备份不替代独立备份；不要直接复制运行中的 WAL 数据库主文件或上传私有目录。

v0.0.10 已在 GitHub Windows runner 安装、启动并取得实际窗口截图。检查了原有签名和应用图标，确认安装包包含桌宠、视频、语音和新插画。用户电脑上的覆盖升级、桌宠移动和实际聊天列入实机验收。[原生截图](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.10/windows-installed-window.png)

首次使用见[使用说明](USAGE.md)，自行编译见[开发与构建](BUILDING.md)。下方保留 v0.0.1／v0.0.2 历史说明，其中旧签名、下载与“不提供同步”等状态不适用于当前版本。

<details>
<summary>历史开发与发布说明（原文保留，不代表当前状态）</summary>

# 当前 0.0.2 中文修订

中文 Windows 安装器的发布与检查结果见 [扫码与中文修订报告](PHASE2_SCAN_ZH_REPORT.md)。在现有 0.0.2 页面选择带 `windows-x64-setup-zh.exe` 后缀的附件与 `WINDOWS-ZH-SHA256SUMS.txt`，无需解压。原 EXE/APK 和原校验文件保留用于回退；新 Windows 修订的源码见其独立构建报告，与旧手机 APK 不同。

安装前关闭正在运行的应用，保留现有安装与数据目录，勿先卸载或清理聊天。向导应直接显示简体中文；安装后核对原人格、历史与记忆，用户旧机覆盖升级仍须实机验收。Windows 系统自身的 SmartScreen、权限和 WebView2 对话框语言由系统决定，安装器未做 Authenticode 签名，不要关闭系统防护。

现有手机测试包按用户选择保持原文件与签名。新电脑里选择“旧版手机：显示兼容配对码”与旧手机配对，仍需双方确认；电脑出码、手机一扫的新版流程等待相同手机签名后交付。网络条件及实机清单见 [阶段 2 验收](PHASE2_ACCEPTANCE.md)。

# Windows 0.0.1 历史安装与本机构建

目标：Windows 10/11 x64，测试版本 0.0.1。正式海滩PNG已导入并校验，[下载本次安装器](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_windows-x64-setup.exe)。Windows原生runner已通过58项测试、静默安装和安装后窗口启动，并目视确认正式图标/头像；用户设备安装、旧库升级和真实API仍待验收。详细范围见随包 BUILD-RESULTS.json；旧dev.1安装器只代表旧版，本次不进入阶段2。

## 已取得 EXE 安装器时

1. 下载 setup.exe 和对应 SHA256SUMS.txt，PowerShell 运行 `Get-FileHash -Algorithm SHA256 .\<实际安装器文件名>.exe`，与清单比较。
2. 此开发包没有购买代码签名证书，发布者可能显示未知；确认文件来自自己的本次构建且哈希一致，不需要关闭杀毒软件。系统拦截时保留提示信息用于排查，不关闭系统防护。
3. 运行安装器，优先安装到当前账户。若系统缺 WebView2，安装器可能联网取得微软官方 WebView2 Runtime；也可从 https://developer.microsoft.com/microsoft-edge/webview2/ 下载官方 Evergreen Runtime 后重试。
4. 从开始菜单打开 DeepSeek娘（旧dev.1包的显示名称为 DeepSeek AI Companion）。首次没有密钥时可先检查界面和本地数据页面；聊天必须在设置中配置自己的 Key。
5. 在「设置」点击「验证安全存储」，成功后在本机输入 Key。保存后输入框清空，只显示已配置状态。不要将 Key、完整凭据页面或聊天数据库发到聊天中。
6. 在默认 DeepSeek娘永久空间发送中文，验证流式/停止/重新生成。通过“新增人格”创建B，验证聊天/记忆/设置/Token不串线；生成中切换的回复仍归原人格。关闭重启后应恢复上次人格和原时间线。
7. 本设备所有人格共用Key；删除后都不能生成，另一设备Key不受影响。本阶段不提供两端历史同步。

业务数据由 Tauri 保存到当前用户 app data 下的 `io.local.deepseekcompanion` 目录，数据库名 companion.sqlite3；不要为修复问题直接删库。API Key 位于当前 Windows 用户的 Credential Manager，不在业务数据库里。数据库和自动迁移前备份目前未做库级加密，应保留在自己的账户中。

## 需要在自己的 Windows 电脑构建时

从官方渠道安装：

| 组件 | 要求与官方入口 |
|---|---|
| Node.js | 22.12+，建议当前 LTS x64，包含 npm；https://nodejs.org/en/download |
| Rust | 官方 rustup，MSVC stable 工具链至少 1.99；https://www.rust-lang.org/tools/install |
| C++ Build Tools | Visual Studio 2022 Build Tools，「使用 C++ 的桌面开发」、MSVC v143 x64/x86、Windows 10/11 SDK；https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022 |
| WebView2 | Evergreen Runtime；https://developer.microsoft.com/microsoft-edge/webview2/ |
| Python 3 | 用于开发辅助/Android manifest 脚本，Windows EXE 的运行本身不依赖 Python；https://www.python.org/downloads/windows/ |

图标制作另需已审核/用户确认安装的 Pillow。拿到本次指定的原始PNG后先运行（不重绘、不更换原图）：

```powershell
python scripts/prepare_branding.py --source "C:\你的目录\原图.png"
node scripts/check-branding.mjs
```

检查头像/完整图、ICO多尺寸、Android圆形遮罩的脸部和V手势，原图/哈希不符时停止。没有PNG不得临时改回DS图标绕过检查。

安装完重新打开 PowerShell，进入解压后的工程根目录，先检查：

```powershell
node --version
npm --version
rustc --version
cargo --version
rustup target list --installed
.\scripts\build-windows.ps1 -CheckOnly
```

若没有 MSVC，可从开始菜单打开「x64 Native Tools Command Prompt for VS 2022」，再在其中启动 PowerShell。CheckOnly 只查命令，不等于完整编译或系统存储运行通过。

```powershell
.\scripts\build-windows.ps1
```

该脚本先校验品牌素材，再执行官方 Rust target 安装、`npm ci --ignore-scripts`、锁文件下 workspace 测试及 Tauri NSIS 打包；没有推送或发布操作。Windows 上测试还会写入/读取/删除随机名字的测试凭据，不使用真实 Key。
如果本机执行策略禁止 .ps1，不更改全局策略，也可以在工程根目录逐条执行：

```powershell
node scripts/check-branding.mjs
rustup target add x86_64-pc-windows-msvc
npm ci --ignore-scripts
cargo test --workspace --locked
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis
```

默认产物在 `apps\companion\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\`。设置了 CARGO_TARGET_DIR 则从该目录找对应子目录。以实际成功日志及存在的文件为准，不把旧 target 中遗留的 EXE 当作新构建成功。
MSI 需在 Windows 安装相应 WiX/VBScript 前置条件后使用 `--bundles msi`；先使用 EXE 验收，无需两种格式同时生成。

## 测试反馈

记录系统版本、包 SHA-256、能否打开窗口、本地历史能否保留、凭据自检结果以及脱敏错误码即可。提供真实 API 连通测试结果时，不附 Key、Authorization 头或私人数据库。

改名后 NSIS 安装路径/卸载条目可能不同，覆盖升级需实际检查旧版识别和用户数据保留；本次Windows runner仅测试全新安装，没有验证用户旧版覆盖。不承诺安装器无缝覆盖。不要为解决问题先卸载/清空聊天。数据库升级及回滚见 PERSONA_MIGRATION.md。

</details>
