> 历史记录：本页描述 v0.0.2 中文修订时点。以下原安装器与六项证据后来改为 `ARCHIVE-` 名称保留，下载链接已指向对应原字节；原摘要与当时结论不变。当前安装请看 [Windows 说明](INSTALL_WINDOWS.md)。

# 阶段 2：扫码与中文安装修订

## 交付范围

从 main `0b2d7131d85aea48e4a73d563f2980825534ee4b` 继续，版本保持 0.0.2 / 1002，不开始阶段 3。用户希望改善扫码和安装语言，又明确选择“保持当前测试身份，等待同一签名可用”。因此 Windows 中文安装器单独验证并增加到现有 Release；原 Tag、成套附件、Android APK 与原校验/报告不覆盖。Windows 修订与旧手机来自不同源码，分别列出证据。

原成套包源码仍为 `1c86678963cb83819e7a72cada6d279fcac9736c`，原 Android 文件仍为 `DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk`，SHA-256 为 `cb8cb0c4daef387aad9ed3d6ebdc9f0c934b6f9dcb81762e499a0c1b9a306dec`。它是独立 `.test` 身份，不是 v0.0.1 的原身份升级。等待的是这份测试 APK 的相同签名，不能用新测试密钥或正式版密钥冒充。

## 已实现

- 电脑“显示扫码连接码”一次操作启用服务并明确授权一部手机；二维码包含运行中网卡的私有 IPv4 与端口，支持网线电脑与同一路由器 Wi-Fi 手机。手机在应用内实时识码，自动验证并连接，无须逐项输入端口、地址或再次勾选指纹。
- 手机扫描即同意双向同步人格、聊天与记忆。电脑出码和手机扫码分别构成明确授权；保留 TLS 1.3、二维码绑定服务器证书、双私钥证明、32 字节随机令牌、两分钟过期、五次尝试和首台身份绑定。已消费二维码可重试相同设备丢失的回复，但不能撤销后重新授权。
- 先验证全部地址，仅连接二维码中的最多八个私有地址，不扫描网段、不接公网 DNS。失败不显示同步成功；仍以实际增量交换及 ACK 标记完成。
- 旧版兼容码保留原四字段和原双方确认流程。当前已发布的旧手机请选择“旧版手机：显示兼容配对码”，其单步扫码功能尚未交付。
- 相机画面在本机由 jsQR 识别，不录音、不上传；拒绝权限时给出中文提示，可选择照片；关闭、识别成功、卸载组件或权限晚到均停止相机轨道。Android 增加相机权限与可选硬件特性。
- NSIS 只配置简体中文且不弹语言选择，WiX 预设中文。Windows runner 启动实际安装向导并读取“下一步/取消”等控件，之后执行独立静默安装和原生窗口启动。

没有修改数据库结构、迁移、正式 applicationId、数据路径、图标原图或现有持久身份。新增 if-addrs 是原锁文件已有的 mDNS 间接依赖，不升级其版本。

## 自动化验证

本地 Rust 全工作区 77 项通过，Clippy 无警告；新增四项扫码/安全回归使用真实 TCP/TLS 与 SQLite，覆盖双向消息、过期、取消、错误次数、混合地址拒绝、旧码不能伪造预授权、丢包重试及撤销。它们运行于单机回环，不是实机验收。

Node 同步模型五项通过，其中从 Rust 产生的中文实际 QR SVG 解码；浏览器标准界面与测试横幅分别二十项，明确为 UI 夹具，新增拒绝相机与延迟授权后关闭轨道回归。Python 策略十七项；版本、DDL、品牌六十九项哈希、TypeScript/前端、格式和工作流检查均须通过。最终 CI 与原生安装证据在公开修订后追加，不预先标记原生成功。

本地 Android ARM64 release 原生构建已生成真实未签名 APK，实际检查为 0.0.2 / 1002、`io.local.deepseekcompanion`、仅 ARM64、不可安装、签名不存在。该本地构建来自尚未提交的工作区，不作正式源码证明；最终 CI 使用固定提交并检查工作区干净。本次不生成/迁移签名，不将未签名包发布为用户安装包。

## 待用户验收与外部依赖

- 当前云环境没有已授权的物理 Windows/Android 双端，用户已选择下载后自行验收。家庭网线+Wi-Fi、相机从屏幕识码、Android 权限/后台恢复、断网补传、冲突、重启和真实旧库保留均未标记通过。
- 新扫码手机 APK 的交付等待当前 `.test` 签名；正式身份的原 Keystore 恢复与 Secrets 是另一独立待办，本次不寻找或迁移。
- 访客网络、AP 隔离、不同 VLAN、防火墙未允许应用可能阻止通信；扫码不绕过这些限制。恢复网络并打开应用后才可重连，不能保证强制停止后的后台实时同步。
- Windows 安装器没有 Authenticode 签名。Hosted runner 的安装/启动不等于用户旧安装覆盖升级验收，安装时保留数据目录，勿卸载清数据。

## 发布机制

`publish-windows-zh.yml` 只响应本次固定请求文件：要求指定 main 源码仍在历史中、CI 核心测试和 Windows 成功，下载该运行的真实 Windows 附件。发布器验证 EXE、哈希、源码、干净工作区、原生安装/启动与中文控件证据，再增加 `-setup-zh.exe`、`WINDOWS-ZH-SHA256SUMS.txt`、`WINDOWS-ZH-BUILD-RESULTS.json` 和语言报告，回读下载校验。原 Android、所有旧资产 ID/哈希、Tag 和 Release 身份必须保持。不会自动发布下一版本，也不会将新电脑包与旧手机包描述为同源配套。

## 最终交付结果

- 安装器源码：[7e1d1ceb97d9213c1d307353518b7a86d23f1e1a](https://github.com/Lyuu508/DeepSeek-Niang/commit/7e1d1ceb97d9213c1d307353518b7a86d23f1e1a)。[CI 43580981180](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43580981180) Linux、Windows、Android 均成功。Linux 实际 77 Rust、17 Python、5 Node、两组各 20 UI 夹具；版本、DDL、Clippy、品牌及前端通过。Windows 执行原生 Rust（含系统凭据）测试、真实中文向导、静默安装退出码 0 和窗口启动；未统计为物理双端验收。
- [发布运行 43581796297](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43581796297) 成功。此前一次上传前的校验失败是发布器仅接受旧 Node TAP 文本，而 Node 24 输出 spec 格式；修正后以真实报告总数验证，通过新增解析回归。本地后续 Python 共 18 项；这不改写构建源码原 CI 的 17 项结果。
- [Windows 中文 EXE](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009186-DeepSeek-Niang_0.0.2_windows-x64-setup-zh.exe)：6,484,690 字节，SHA-256 `c20e8de48c0636bf1b29b7c7a6a68a2c1ddd8635c347fcaebcb2d80a10fdaaf0`。实际语言报告包含“下一步(&N) >”“取消(&C)”与“欢迎使用 DeepSeek娘 安装程序”。
- [校验清单](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009455-WINDOWS-ZH-SHA256SUMS.txt)、[构建/安装报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009319-WINDOWS-ZH-BUILD-RESULTS.json)、[语言控件报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009408-WINDOWS-ZH-INSTALLER-LANGUAGE.json)、[自动化报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009257-WINDOWS-ZH-AUTOMATED-TESTS.json)、[测试日志](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009515-WINDOWS-ZH-TESTS.log)、[PowerShell transcript](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/ARCHIVE-9bc43c4d-747009366-WINDOWS-ZH-BUILD.log) 已上传。新 EXE 与六项证据均完整公开下载 HTTP 200，逐文件大小和 GitHub digest 一致；[独立回读证据](validation-phase2-scan-zh/public-download-verification.json)。
- transcript 没有收录所有原生命令输出，完整 Windows 命令输出保留在上述 CI 的 Windows job。当前云环境下载 Actions ZIP 日志受到 `results-receiver.actions.githubusercontent.com` 的策略限制，未修改白名单或尝试绕过；这不影响已验证的公开 EXE/报告下载。
- [现有 Release](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.2) ID 445111880 不变，正文顶部指向中文安装器；原十三项资产 ID/哈希和原 Tag 指向 `1c86678` 均保持。Android 为原 `.test` 开发签名文件，未重新构建为可安装更新，未替换签名或正式身份。
- Android 同源码 CI 成功生成真实 ARM64 未签名 release 检查包，工作区干净、版本 0.0.2 / 1002；该附件仅在 Actions 作为编译证据，不能安装，也未作为手机新版本公开。新版扫码的物理手机验证仍等待同签名 APK 与用户设备验收。

修改涉及同步核心/真实 TLS 测试、Tauri 命令、实时扫码组件与同步页面、Android 权限准备、Windows 安装语言、CI/独立发布校验、三份已有锁文件以及说明/验收/版本记录；没有增加 DDL 或修改已有数据库。下一步停在阶段 2，等待当前测试签名与用户验收，不启动新版本。
