# 0.0.2 扫码修订与持久签名交付

2026-11-15，已按用户最新授权更新原 [v0.0.2 Pre-release](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.2) 的安装附件，不新建版本、不移动历史 Tag、不开始阶段 3。Windows 和 Android 当前安装包均由同一源码构建，版本为 0.0.2 / Android versionCode 1002。

## 直接安装

- [Windows 中文 EXE](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_windows-x64-setup.exe)
- [Android ARM64 扫码测试 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk)
- [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SHA256SUMS.txt)
- [BUILD-RESULTS.json](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/BUILD-RESULTS.json)
- [Windows 安装验证](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/BUILD-RESULTS-WINDOWS.json) · [自动化测试记录](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/AUTOMATED-TEST-RESULTS.txt)
- [公开签名身份](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SIGNING-IDENTITIES.json) · [签名管理说明](TEST_SIGNING_SETUP.md)

不需要下载压缩包。Windows 先关闭旧程序，运行新版安装 EXE 后重新打开。手机安装后请打开 **“DeepSeek娘 扫码测试版”**；旧应用不会因下载附件更新而自动改变。

Android 新身份 `io.local.deepseekcompanion.test2` 与旧 `.test`、正式 `io.local.deepseekcompanion` 并存，以保留旧安装和数据；不能覆盖旧签名 APK。不要卸载旧应用或清理数据。旧手机尚未同步的资料不会自动导入新沙箱；新手机端可扫码同步电脑已有资料，API Key 在各设备独立配置。旧手机资料迁移需要另行验收，不能把从电脑同步宣称为旧手机数据库自动迁移。

## 本次交付内容

- 电脑局域网同步页只保留“连接手机”主入口，点击弹出短期二维码；手机只有“扫一扫”，直接调用相机，识码后使用现有安全协议自动配对。
- 移除文件/图片选择、手动配对码/IP、旧版兼容界面入口；保留手动同步；本机诊断信息放在底部。
- 手机采用独立底部导航与紧凑聊天，原生 WindowInsets 适配状态栏、导航栏、刘海和键盘；保留原品牌图标。
- Windows NSIS 安装向导为简体中文，保留原安装身份、数据目录及数据库迁移。
- 两端安装资产使用新测试签名，并先持久保存到 GitHub Secrets。签名、真实私钥证明、实际安装包验签及 Windows 安装启动均完成后才更新 Release。

本次发布步骤没有重新开发同步业务或更改数据库结构；既有 TLS 1.3、设备认证、一次性配对、增量队列、冲突和重连逻辑及全部原测试保留。

## 源码与运行记录

| 项目 | 实际值 |
|---|---|
| 两端完整构建源码 | [`9bc43c4dde86a92153273be8df048b10624475ae`](https://github.com/Lyuu508/DeepSeek-Niang/commit/9bc43c4dde86a92153273be8df048b10624475ae) |
| 同源 Linux/Windows/Android CI | [43693955496，全部成功](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43693955496) |
| 最终签名与发布验证 | [43697671242，成功](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43697671242) |
| 实际签后 Windows 安装报告运行 | 43697030853；最后发布重用相同已验证安装字节 |
| Release | ID 445111880，v0.0.2，Pre-release |
| 保留的历史 Tag 指向 | `1c86678963cb83819e7a72cada6d279fcac9736c` |

之后的提交只修复签名/发布工具和补充交付文档，没有把不同源码的安装包混为一套。GitHub 页面自动生成的 Source code ZIP 对应历史 Tag，当前安装包源码以本报告的完整提交及 BUILD-RESULTS 为准。

旧二十项附件已改名为 `ARCHIVE-` 开头；原资产 ID 与 GitHub SHA-256 摘要逐项一致，保留原始字节。旧版本 Release 未覆盖。当前 `setup.exe` 与 `setup-zh.exe` 为相同字节的中文安装器别名。

## 自动化与原生检查

| 检查范围 | 实际结果 |
|---|---|
| Linux Rust | 77 通过，0 失败，0 忽略 |
| Windows 原生 Rust | 78 通过，0 失败，0 忽略；多出 Windows 系统凭据往返检查 |
| Python 策略/打包测试 | 21 通过 |
| Node 测试 | 5 通过 |
| 浏览器 UI 夹具 | 普通与独立测试模式各 23 项通过；不是手机实机验收 |
| TypeScript/前端、Clippy、DDL/SQLite、版本、69 项品牌资源 | 通过同源 CI 检查 |
| Windows 真实 EXE | 原生构建成功，中文向导控件验证、签后安装退出码 0、启动窗口标题“DeepSeek娘”通过 |
| Android ARM64 | APK 构建成功，官方 zipalign/apksigner、身份/版本/ABI/禁明文与备份策略检查通过 |
| 公开下载 | 16 项当前附件均实际 HTTP 200，逐项字节数/SHA-256 与 GitHub API 摘要一致；校验文件三项通过 |

构建与测试完整记录作为独立 Release 附件提供：[windows-build.log](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/windows-build.log)、[android-build.log](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/android-build.log)、[Android 工具链](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/android-toolchain.log)、[签后 Windows 安装](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/signed-windows-install.log)。早期发布工具遇到临时证书交互、安装进程日志句柄和覆盖附件缓存问题，均已修复；最终工作流成功，没有以跳过验签或删测试消除错误。

## 签名与后续账号

Android 官方 apksigner 对当前实际公开 APK 的独立复验：成功，v3 签名，单一 RSA 3072 签名者。证书 SHA-256：

```text
fefb57342751c7bd5ef8271e4a8b09f8702c0285e0120e8407b557d20dbbf153
```

Windows 新测试证书 SHA-256：

```text
069f0150a9698a8178bdaf7dfea13f5509185aa69b9ba940f5473edbafa8af62
```

Windows Authenticode 内容与签名在一次性 Hosted runner 验证，签后安装启动通过。它是自签名测试证书，**不是公共 CA 信任发行者**，不能保证消除 SmartScreen/未知发布者提示；没有向用户电脑安装根证书或关闭防护。

下列七项仓库加密 Secrets 已成功配置，且完整 bundle 已在多个新 Runner 成功恢复。本报告只列名称，不含任何秘密值：

- `TEST_RELEASE_SIGNING_B64`
- `ANDROID_TEST_KEYSTORE_B64`
- `ANDROID_TEST_KEYSTORE_PASSWORD`
- `ANDROID_TEST_KEY_ALIAS`
- `ANDROID_TEST_KEY_PASSWORD`
- `WINDOWS_TEST_PFX_B64`
- `WINDOWS_TEST_PFX_PASSWORD`

后续其他账号应获得同一仓库协作者与 Actions 权限，通过现有工作流复用签名，无需读取私钥。一次性 `SIGNING_SECRETS_TOKEN` 完成使命后可撤销，并只删除这一项 Secret；保留上述七项。未来版本仍需用户授权，保持 `.test2` 和这次证书，不重新生成签名。GitHub 不能读回 Secret 值，换仓库需要另行安排受保护迁移。

原正式版证书和 applicationId 未替换，原签名连续升级包仍为独立待办。私钥、密码、Keystore/PFX 和 bundle 未进入普通源码或下载附件。

## 安装包 SHA-256

```text
3e9edfe37a28415d24c59e43b3c1a951447a79a9b4ca7bdb6d72d68abb686bfb  DeepSeek-Niang_0.0.2_windows-x64-setup.exe
3e9edfe37a28415d24c59e43b3c1a951447a79a9b4ca7bdb6d72d68abb686bfb  DeepSeek-Niang_0.0.2_windows-x64-setup-zh.exe
2da68eee6e3d784a44dd4a12aef51e74d56cc4b6d822d17f629514bd48bb06e3  DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk
```

## 待用户实机验收

没有连接到用户的 Windows + Android 设备，本次不能声称实机通过。建议使用合成聊天资料，保留原应用及数据：

1. 电脑安装并启动新版；手机打开“DeepSeek娘 扫码测试版”，确认底部导航、状态栏/刘海、键盘开合与横屏没有遮挡。
2. 电脑接同一路由器的网线，手机连该路由器 Wi-Fi；电脑点“连接手机”，手机点“扫一扫”，允许相机并对准二维码，确认实际配对和可信状态。拒绝相机权限、关闭扫码、过期二维码也需检查。
3. 用合成聊天/人格/记忆核对双向增量同步；手动同步能显示实际结果，不把连接成功当数据已同步。
4. 断网分别添加资料，恢复网络/重启后核对补传、重复去重、并发冲突、逻辑删除和撤销设备。
5. 核对原 Windows 库、旧手机应用的聊天/人格/记忆仍保留；验证新 `.test2` 重启后持久化。旧手机未同步资料与原正式签名升级仍需另行处理。
6. 仅在设备本地设置自己的 API Key，验证真实 DeepSeek；不把 Key、数据库、聊天或二维码发到公开 Issue。

云端真实 TLS/SQLite 集成测试与 UI 夹具分别提供协议/界面证据，均不能替代上述家庭网络、相机和系统生命周期验收。交付后停留阶段 2，等待用户验收。

公开附件摘要、同源/签名与归档核验的脱敏证据见 [validation-release-0.0.2-refresh](validation-release-0.0.2-refresh/README.md)。
