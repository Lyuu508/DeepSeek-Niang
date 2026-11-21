# DeepSeek娘 v0.1.0 验收版

Windows 和 Android 共用海洋主题。电脑采用三栏布局，轻量导航、选中高亮、聊天气泡、角色立绘和五维关系面板沿用同一套图标。手机保留底部导航与键盘安全区域，调整人格卡片和回收站的触控体验。原角色素材直接复用，关系数值仍由原规则产生。

视频中心先由 DeepSeek 给出观察计划，经授权使用千问提取画面证据，再由 DeepSeek 回答。视频和抽样帧使用百炼临时私有 OSS。识别记录保存在本机，回答失败后可用已有证据重试。原本地视频记录和长期记忆可以继续查看。

聊天可插入可编辑的 UTF-8 文本附件，以及保存本机图片。DeepSeek 接收发送的文字，图片像素保留在设备上。语音输入继续使用设备原有的转写能力。

关于页支持检查正式 Release、确认下载、校验与安装。Windows 由独立安装进程安装并重启，安装失败时尝试恢复旧程序。Android 核对签名和版本后打开系统安装界面。

Windows EXE 和 Android APK 由源码 `12c84651e6185ba90ef55c3b3cc3877952a8a1df` 构建。Android versionCode 为 1011，发布的验收包使用 `io.local.deepseekcompanion.acceptance` 和独立验收签名，可与旧版并存。正式 Android 应用标识为 `io.local.deepseekcompanion`，升级包须通过原证书校验。双端实机安装和真实 API 联调待验收。[直接下载](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.1.0-acceptance.1) · [构建报告](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.1.0-acceptance.1/BUILD-RESULTS.json) · [验收清单](V010_ACCEPTANCE.md)
