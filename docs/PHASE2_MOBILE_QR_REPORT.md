# 0.0.2 手机与单一扫码修订

基线 main `665312434f65446079f2fdc4ebc81ad05284585e`。本次按用户图示完成手机端及电脑配对入口，停留在阶段 2 / 0.0.2 / Android 1002。正式 `io.local.deepseekcompanion`、测试 `io.local.deepseekcompanion.test`、用户品牌素材、SQLite 结构/迁移和安全同步协议均保持。

## 实现范围

- 电脑只保留“连接手机”这一种首配入口。点击后弹出原生 HTML dialog 展示一次性二维码，手机连接、过期、关闭或离开页面会收起；Escape 可取消，焦点返回原按钮。二维码包含局域网地址与固定证书，保留现有双私钥证明、TLS 1.3、过期/一次性/撤销校验。
- Android 首配入口只有“扫一扫”，直接 getUserMedia 请求后置相机。识码后自动调用安全配对，无文件选择、手动填码/IP 或兼容入口；权限拒绝给出中文重试路径，关闭/识码/晚到授权均释放镜头。双方均明确同意同步人格/聊天/记忆，API Key 与私钥留在本机。
- 保留手动同步和已连接设备管理、断线进度/错误及并发修订；本机信息始终是最后一个区块。协议级兼容与原测试保留，界面删除旧入口不等于删除安全回归。
- 手机独立四项底部导航、头像切换人格、紧凑侧栏、聊天输入和卡片、44px 以上主要点击区域。按 Android 原生平台渲染，横屏不会变回桌面侧栏。切页保持草稿、人格与历史。
- 状态栏遮挡源于生成的 MainActivity 调用了 enableEdgeToEdge 却没有处理 WindowInsets。本次以受版本管理的 Activity 处理 systemBars、displayCutout 和 IME 边距，并消费已处理的 insets；WebView 的整块可用区域（含弹窗/侧栏）落在安全边界内。网页补充 viewport-fit 与 visualViewport/window resize，键盘与旋转后重新计算高度。

## 已执行验证与边界

本地两组各 23 项浏览器检查通过，明确是 UI 夹具：保留原 20 项场景并适配已被替换的入口，新增真实 Rust 测试 QR 的合成摄像头帧识别、底部导航/草稿保留、窄屏/横屏输入区边界检查。模拟后端仍拒绝配对，未伪造 TLS/同步成功。Rust 原 77 项真实 TCP/TLS/SQLite 自动化保留；它们是单机测试，不是物理 Windows + Android。

本地原生 release ARM64 已编译包含新 MainActivity 的代码；本次另提供 `.test` 未签名候选模式，保持当前测试安装身份、不生成密钥。Python 增加无签名策略可反复执行与恢复的回归；签名状态由官方 apksigner 实际检查，不能仅从文件名推断。最终固定 SHA CI/原生结果在完成后追加。

云环境没有用户的物理 Windows/Android。通知栏/刘海/手势导航、系统键盘、相机硬件、家庭网线 PC+Wi-Fi、后台恢复及旧库覆盖保留均待用户设备验收；浏览器截图没有真实通知栏，不能作为已修复所有机型的证明。

## 签名与发布条件

用户已明确选择“保持当前测试身份，等待同一签名可用”，本次不恢复/迁移 Keystore，不生成替代签名，不变更 applicationId，不要求卸载。实际已发布 0.0.2 APK SHA-256 为 `cb8cb0c4daef387aad9ed3d6ebdc9f0c934b6f9dcb81762e499a0c1b9a306dec`，官方 apksigner 再次验签有效，其真实测试证书 SHA-256 为 `4a092d7012af2d1063f6420cf2d3ba77d0cf53882118926a8fc5e1723cc460c0`。这与 v0.0.1 正式身份的证书不同。当前本地构建目录里的已有开发签名不匹配，未用于本次候选包。

当前 gh 授权读取仓库 Secrets 元数据返回 403（Resource not accessible by integration）；这不证明 Secrets 丢失。[CI 43597397423](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43597397423) 只传四个存在布尔值，确认该 main 运行的 `ANDROID_KEYSTORE_B64`、`ANDROID_KEYSTORE_PASSWORD`、`ANDROID_KEY_ALIAS`、`ANDROID_KEY_PASSWORD` 均不可用，未读取或输出密钥值。仓库 Environments 读取成功，列表为空。配置存在也不证明与上述测试证书匹配。[仅含状态的证据](validation-phase2-mobile-qr/signing-presence.json)。

新 EXE 与手机候选按同一完整 SHA 构建。只有手机实际同签名验证和双端检查均满足后，才替换既有 0.0.2 下载与报告；不再用单独 Windows 发布宣称手机扫码交付。当前既有 Release、原 APK 和数据保持，后续等待同一测试签名或用户新的明确决定。

签名保管者可在仓库 **Settings → Secrets and variables → Actions → New repository secret** 安全配置以上四项，值仅填写到 GitHub 加密 Secrets，不发聊天、不写仓库文件或命令日志。必须是与实际 0.0.2 `.test` APK 相同的私钥；只有原 v0.0.1 正式签名不能满足本次测试包覆盖条件。配置后仍须执行实际证书、私钥可用性与新 APK 验签，才允许替换。若该私钥已不可恢复，继续遵守当前等待选择，不擅自生成替代身份或要求卸载。

## 构建记录与界面预览

源码 `cf3e0f5cbb552c0dd9ab295e1fc3ac3f40df429e` 的上述首轮 CI 已全部成功，Windows 实际中文向导、安装和启动通过，Android 实际 `.test` ARM64 未签名候选通过。后续仅将错误提示改为纯扫码路径；最终交付源码与结果如下。

[手机聊天与连接页面预览](validation-phase2-mobile-qr/README.md) 为明确标注的 UI 夹具截图，可查看单一扫码、底部导航与紧凑聊天；不代表手机实机截图，不验证通知栏、相机硬件或真实局域网。所有物理验收项仍保持待验证。

## 最终同源构建结果

- 最终源码：[c609b8ba366139956aa91562259bdc3e06d459a6](https://github.com/Lyuu508/DeepSeek-Niang/commit/c609b8ba366139956aa91562259bdc3e06d459a6)。[CI 43597948623](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43597948623) 的 Linux `test`、Windows Package、Android Package 和仅检查配置存在状态的 job 均成功；[API 回读的作业/附件记录](validation-phase2-mobile-qr/ci-results.json)。
- Linux：77 Rust、19 Python、5 Node、两组各 23 浏览器 UI 夹具；版本、Clippy、SQLite schema、前端/TypeScript 与 69 项品牌文件哈希通过。没有删除/忽略原测试，实际失败 0；有意移除的手动界面场景改验新入口和相同安全边界。
- Windows：真实 EXE 已生成；原生 cargo test（含随机系统凭据往返）、中文 NSIS 控件、静默安装及窗口启动均成功。保存 `windows-native-build.log`、安装结果、`windows-build.json` 和 `SHA256SUMS.txt`。
- Android：真实 ARM64 `.test` 未签名更新候选 APK 已生成，0.0.2/1002、测试名称、`io.local.deepseekcompanion.test`、实际签名缺失与干净源码检查通过；保存完整构建/SDK日志、`android-build.json`、分发状态和 `SHA256SUMS.txt`。这是可供同签名持有人后续签署的候选，当前不可安装，未使用其他密钥冒充升级。
- 两端构建 job 都检出并验证上述完整源码 SHA；不是把旧 Windows 与新手机混成同源。日志与测试报告分别随 Actions Artifacts 保存 30 天，不能把原生构建通过等同实机验收。
- **本次未替换 GitHub Release 的安装包**：最终 CI 再次确认四项签名配置在该运行不可用，当前本地已有签名与真实旧测试 APK 不匹配。当前既有 20 项 Release 附件的 ID/哈希与 Tag `v0.0.2 → 1c86678` 均保持；未占用新版本，也未更改数据库/迁移或删除用户数据。待当前测试签名可用后继续实际签名验证与同源替换，不用单独发布电脑端代替手机交付。

本次修改涉及 `SyncPanel.tsx`、`PairScanner.tsx`、新增 `Modal.tsx` / `MobileNav.tsx`、主界面/样式/viewport、受版本管理的 Android Activity、Android 准备/构建/验包脚本、CI/日志保留、对应测试与说明。原同步核心、TLS、数据库与品牌原图未重写。

待实机：已签名更新的覆盖与旧库保留、各机型通知栏/刘海/导航栏/键盘、相机硬件扫码及权限、网线 Windows + Wi-Fi 手机自动同步、断线/冲突/重启与手动重试。用户已说明由其在安装包可用后验收；本次没有实机通过记录。
