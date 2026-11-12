# DeepSeek娘 0.0.1 测试版安装与验证

更新：2026-11-13。用户明确授权本次重新构建Windows EXE和Android APK并上传公开仓库 [Lyuu508/DeepSeek-Niang](https://github.com/Lyuu508/DeepSeek-Niang) 的测试Release。发布标题为“0.0.1测试版”，正文只写“这是0.0.1测试版”；不自动发布以后的版本。

## 正式资源与版本

原始海滩PNG从用户ZIP导入，1254×1254、1724316字节，SHA-256为 `070adedf4a8896745ea8fb7755b3cbe4b673bcb51d1edc88cc76e46ea937aff1`。master原字节保留；生成69个头像、Windows多尺寸ICO/PNG和Android各密度launcher/adaptive/启动/通知资源。资源清单ready，打包前校验全部哈希。

版本0.0.1，SQLite仍为v3，Android versionCode为1001，高于旧开发APK的1000。applicationId/bundle identifier仍为io.local.deepseekcompanion，数据目录、凭据空间和Android开发签名不变。不同签名不能直接覆盖安装，不建议卸载清数据。

## 安装文件

在 [Releases](https://github.com/Lyuu508/DeepSeek-Niang/releases) 中选择Windows x64安装EXE或Android ARM64 APK。仅在本次构建实际成功并核验后添加附件，不把旧dev.1包改名。Windows为未购买Authenticode证书的测试安装器，Android为开发签名测试包。

先核对SHA-256。Windows运行setup.exe，Android复制APK到手机后按系统提示安装，或对已授权设备执行adb install -r。组件/升级与数据保护见 [Windows](INSTALL_WINDOWS.md)、[Android](INSTALL_ANDROID.md) 和 [BUILDING](BUILDING.md)。

## 构建与真实性

Windows手动构建定义位于.github/workflows/windows-test-package.yml：只由workflow_dispatch触发，token仅contents:read，官方Action固定完整提交SHA。任务运行锁定依赖下的核心测试，原生MSVC/NSIS打包；随后在临时Windows runner执行静默安装、进程启动及可能的窗口截图，实际结果在BUILD-RESULTS-WINDOWS.json中记录。没有自动Release步骤，也不上传用户密钥。

Android使用现有JDK21、SDK36、NDKr28c和Gradle环境，在当前Linux主机实际构建ARM64调试APK，保留原开发签名。核验ZIP完整性、签名、ABI、versionName/versionCode、名称/权限/禁云备份及图标资源；不会把Rust检查当作APK成功。

.gitattributes将源码、SQL和生成的XML固定LF；图片为二进制不转换，避免Windows检出改变迁移/资源校验和。开发依赖锁定，npm生命周期脚本关闭，第三方构建步骤先核对清单和来源。

## 用户设备验收

自动化Windows runner测试不等于用户设备全部验收。Android真机、不同Windows/DPI/路由器、Keystore/Credential Manager和真实DeepSeek效果按实际记录标注；API Key由用户在本机设置，不发送到聊天。

进入永久人格空间测试中文/流式/停止/重生成、新增及切换人格、记忆/Token隔离；关闭重开恢复上次人格与历史。实际API只发送当前人格设定及有限近期回合。阶段2～9仍只有计划，本版不声称已实现LAN同步、桌宠、自动生活/关系、视频或语音。
