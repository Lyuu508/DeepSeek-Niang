# v0.1.0 测试与发布前验收

GitHub 基线 `cd446774b239ceac8d09eb6b7d6020ececd9eab3`，v0.0.10 发布源码 `b4f56f55b4cbf567c0b57194a0936cc7c8eeb664`。GitHub 的正式 Release 查询返回 404，最新公开内容是 v0.0.10 预发布。本轮开发和构建在本地分支，正式分享和推送等用户验收同意。

| 平台或身份 | 已发布参照 | v0.1.0 期望 |
| --- | --- | --- |
| Windows | `DeepSeek娘`，Tauri 标识 `io.local.deepseekcompanion`，v0.0.10 中文 NSIS | 同名同标识，x64 NSIS。应用内更新需要系统能够验证的 Authenticode 安装器 |
| 原 Android | v0.0.1，`io.local.deepseekcompanion`，证书 `9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1` | 同一个包名和证书，显示名 DeepSeek娘，versionCode 1011 |
| Android 测试 | v0.0.10，`io.local.deepseekcompanion.test2`，证书 `fefb57342751c7bd5ef8271e4a8b09f8702c0285e0120e8407b557d20dbbf153` | 保留测试安装与其私有数据。正式应用无需卸载测试版 |

两份已发布 Android APK 已逐文件与 GitHub SHA-256 比对，并使用 Build-Tools 36.0.0 的 `apksigner` 和 `aapt2` 复核。Secrets 元数据 HTTP 403。v0.0.10 的 [签名检查](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/44778883247) 只报告存在状态，`ANDROID_KEYSTORE_B64`、`ANDROID_KEYSTORE_PASSWORD`、`ANDROID_KEY_ALIAS`、`ANDROID_KEY_PASSWORD` 在该次运行中均未提供，`TEST_RELEASE_SIGNING_B64` 可用且对应另一证书。当前编译环境没有匹配的原私钥，因此正式身份的 APK 仅可编译检查。正式测试包需要原签名接入可以私下构建和验签的环境。验收后可在已授权的 GitHub Actions 中由 `scripts/android_signing.py` 使用四项既有配置和原证书核验。

## 视频接口和授权

请求形状对照 [Qwen 官方视频示例](https://github.com/QwenLM/Qwen3-VL/blob/main/cookbooks/video_understanding.ipynb) 和 [DashScope Python SDK](https://pypi.org/project/dashscope/2.1.1/) 中的 `resources/_oss_upload.py` 及 `legacy_1x/aigc/multimodal_conversation.py`。临时上传授权走 `/api/v1/uploads?action=getPolicy`，文件使用独立 OSS multipart 请求。服务端识别接口为 `/api/v1/services/aigc/multimodal-generation/generation`，使用 `X-DashScope-OssResourceResolve` 解析 `oss://` 素材。API 地址限定官方区域，与公有仓库更新请求分开。凭据分别放入本机安全存储。

12 MiB 以内 MP4 可用原片上传。其他由本机 WebView 支持的格式、较大的 MP4 或原片调用遭拒时，可以选择 4–12 帧的兼容模式。原片授权显示可能含音轨。输出只使用视觉证据，抽样帧可能漏掉动作。接口与权限随百炼账号和地区变化，真实联调必须用用户已授权的模型与非私密视频。阶段测试以回环 HTTP 验证工序、凭据边界和 DeepSeek 失败重试，不代表百炼已经在线识别过用户视频。真实成本查看两家提供商的账单，视频单独调用尚未纳入原聊天 Token 面板。

## 用户安装验收

1. 先闭合旧程序及桌宠，备份 Windows 本地数据。安装 EXE 后检查版本、人设、原聊天和记忆，确认窗口及安装向导使用 DeepSeek娘名称。检查三栏宽度、导航渐变、焦点、状态与设置、聊天图片和文本附件。
2. 获得原证书签名的 APK 后，先核对包名和证书。覆盖原 `io.local.deepseekcompanion` 或与 `.test2` 并存。已有 `.test2` 的用户可以先通过信任的 Windows 主机同步支持同步的数据，再在正式应用重新配对。测试应用私有的设置和未同步记录继续留在旧应用。验证通过前保留两个安装和备份。
3. 检查竖屏、刘海、系统栏、IME、系统文件选择、相机扫码、回收站确认、关系与日常。两端各发一条新消息，以断网重连证明收发和旧数据都正常。检查旧插件清单、授权和日常设置。
4. 保存各自的模型 Key。确认千问固定图连接测试，分别以短 MP4 和抽样帧运行一次。核对 DeepSeek 最终回答与时间点。在 DeepSeek 失败时只重试回答，并在百炼账单核对没有重复识别。验证错误后选中的视频仍可使用。
5. 在关于页点击检查更新。GitHub 首次正式发布前预期显示暂无正式发布。之后从较旧的、具备更新功能的版本验证下载、外部签名、显式安装、旧数据保留与失败回退。Windows 还需实际核对可信签名，Android 在系统页面手动授权安装并确认，拒绝更新时原应用应仍可用。

## 发布授权

只有用户明确表示“测试通过，同意发布正式版。”后才推送、创建 v0.1.0 Tag 与正式 Release、上传同源安装包和证据。当前 `release.yml` 是历史预发布入口。正式分发时需要显式设置稳定发行状态、核对客户端期待的包文件名和 SHA256SUMS，并验证实际公开下载。任何签名身份冲突应先停在正式包验证步骤，保留已有安装和数据。
