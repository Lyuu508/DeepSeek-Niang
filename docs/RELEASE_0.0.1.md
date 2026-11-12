# DeepSeek娘 0.0.1 测试版安装与验证

已发布：[0.0.1测试版](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.1)，发布时间2026-11-12 17:11:17 UTC（北京时间11月13日）。用户明确授权本次重新构建Windows EXE和Android APK并上传公开仓库 [Lyuu508/DeepSeek-Niang](https://github.com/Lyuu508/DeepSeek-Niang)。发布标题为“0.0.1测试版”，正文严格为“这是0.0.1测试版”；没有自动发布以后的版本。

## 正式资源与版本

原始海滩PNG从用户ZIP导入，1254×1254、1724316字节，SHA-256为 `070adedf4a8896745ea8fb7755b3cbe4b673bcb51d1edc88cc76e46ea937aff1`。master原字节保留；生成69个头像、Windows多尺寸ICO/PNG和Android各密度launcher/adaptive/启动/通知资源。资源清单ready，打包前校验全部哈希。

版本0.0.1，SQLite仍为v3，Android versionCode为1001，高于旧开发APK的1000。applicationId/bundle identifier仍为io.local.deepseekcompanion，数据目录、凭据空间和Android开发签名不变。不同签名不能直接覆盖安装，不建议卸载清数据。

## 安装文件

以下均为实际附件，已通过公开链接完整下载并核验长度和SHA-256；两包都从源码提交 `8ae10e7c62602fde124989800c6e16d5a951dcb0` 重新生成，tag固定于该提交，没有把旧dev.1包改名。

- [Windows x64 安装 EXE](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_windows-x64-setup.exe)（5,826,955字节）。
- [Android 10+ ARM64 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/DeepSeek-Niang_0.0.1_android-arm64.apk)（35,511,972字节）。
- [SHA256SUMS.txt](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/SHA256SUMS.txt) 与 [BUILD-RESULTS.json](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.1/BUILD-RESULTS.json)。

Windows安装器未做Authenticode签名；Android为沿用原开发证书的调试包。

先核对SHA-256。Windows运行setup.exe，Android复制APK到手机后按系统提示安装，或对已授权设备执行adb install -r。组件/升级与数据保护见 [Windows](INSTALL_WINDOWS.md)、[Android](INSTALL_ANDROID.md) 和 [BUILDING](BUILDING.md)。

## 构建与真实性

Windows手动构建定义位于.github/workflows/windows-test-package.yml：只由workflow_dispatch触发，token仅contents:read，官方Action固定完整提交SHA。[实际构建run 43262321945](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43262321945)成功，锁定依赖下58项Rust测试通过，包含原21项内核不变量与真实Windows Credential Manager测试。原生MSVC/NSIS打包后，临时Windows runner静默安装退出码0，安装后进程与窗口启动通过；已目视确认[真实窗口截图](validation-release-0.0.1/windows-installed-window.png)中的正式头像/图标。没有输入用户Key或真实聊天。

Android使用现有JDK21、SDK36、NDKr28c和Gradle环境，在当前Linux主机实际构建ARM64调试APK，保留原开发签名。核验ZIP完整性、签名、ABI、versionName/versionCode、名称/权限/禁云备份及图标资源；不会把Rust检查当作APK成功。

.gitattributes将源码、SQL和生成的XML固定LF；图片为二进制不转换，避免Windows检出改变迁移/资源校验和。开发依赖锁定，npm生命周期脚本关闭，第三方构建步骤先核对清单和来源。

当前连接直接上传Release附件曾返回401；改用固定草稿ID、源码提交与四个附件SHA-256的一次性手动GitHub任务，已[上传成功](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43266632511)，随后才手动公开发布。一次性任务已禁用并从main移除，常规Windows构建仍只读，不会自动发布。签名私钥始终留在本地。

| 检查 | 实际结果与范围 |
|---|---|
| Windows | 原生58项测试、编译、NSIS静默安装与窗口启动通过；未验证用户旧版覆盖升级 |
| Android | 完整APK构建、ZIP/签名/ARM64/API29+/版本与图标资源检查通过；未真机运行或执行Keystore自检 |
| 共享前端与数据库 | 类型/生产构建、SQLite schema v3检查、6项构建/导出策略测试通过 |
| 浏览器交互 | 16项明确Mock测试通过；不作为真实API/原生存储证据 |
| 公开下载 | 四个附件HTTP200，完整下载字节数与SHA-256一致 |
| DeepSeek真实请求 | 未测试；用户在各端设置中本地配置Key |

完整机器可读记录见 [validation-release-0.0.1](validation-release-0.0.1/README.md)。发布后的main仅补充交付文档和移除临时上传任务，不移动0.0.1 tag，也不改变两个安装包。

## 用户设备验收

自动化Windows runner测试不等于用户设备全部验收。Android真机、不同Windows/DPI、旧库升级、设备系统存储自检和真实DeepSeek效果仍需本机验证；API Key由用户在本机设置，不发送到聊天。

进入永久人格空间测试中文/流式/停止/重生成、新增及切换人格、记忆/Token隔离；关闭重开恢复上次人格与历史。实际API只发送当前人格设定及有限近期回合。阶段2～9仍只有计划，本版不声称已实现LAN同步、桌宠、自动生活/关系、视频或语音。
