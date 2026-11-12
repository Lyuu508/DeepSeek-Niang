# Windows 首次安装与本机构建

目标：Windows 10/11 x64，本地开发版本 0.0.1。正式海滩PNG已于2026-11-13导入并校验；本页为本次重新构建后的安装与本机构建说明。旧dev.1安装器只代表旧版，不能验证本次人格重构，不进入阶段2。安装包的实际构建/运行状态以随包 BUILD-RESULTS.json 为准；构建成功不代表已经在 Windows 实机运行。

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

改名后 NSIS 安装路径/卸载条目可能不同，覆盖升级需实际检查旧版识别和用户数据保留；目前未在Windows测试，不承诺安装器无缝覆盖。不要为解决问题先卸载/清空聊天。数据库升级及回滚见 PERSONA_MIGRATION.md。
