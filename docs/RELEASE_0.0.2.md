这是0.0.2测试版。

- [直接下载 Windows EXE 安装程序](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_windows-x64-setup.exe)
- [直接下载 Android 独立测试 APK](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk)
- [SHA-256 校验文件](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SHA256SUMS.txt)

直接下载 Assets 中的 Windows EXE 或 Android APK 即可安装，不需要下载、解压 Actions 压缩包。SHA256SUMS.txt、BUILD-RESULTS.json、测试报告和完整构建日志一并提供。

**Android 为独立测试版**：applicationId 为 `io.local.deepseekcompanion.test`，启动名称“DeepSeek娘 测试版”，使用开发测试签名。它可以与 v0.0.1 同时安装，不覆盖原应用，不自动导入、删除旧版聊天或人格数据，也不是原签名连续升级 APK。原始签名恢复与 Secrets 迁移仍为独立待办。

Windows 和 Android 均来自源码 `1c86678963cb83819e7a72cada6d279fcac9736c`，版本为 0.0.2 / Android versionCode 1002。该源码的 Linux 测试、Windows 原生测试/EXE安装启动、Android ARM64构建/真实签名与身份检查均已通过；发布前再次检查同源码报告、哈希与实际 APK 签名。Windows 安装器暂无 Authenticode 签名。

本版本实现人工配对后的局域网双向增量同步、TLS 1.3、身份验证、离线补传和冲突/删除处理。自动化测试不替代真实 Windows 网线 + Android Wi-Fi 家庭网络验收；真实双端、系统凭据/生命周期和用户旧数据验收仍由你安装后进行。使用合成资料，按 INSTALL-AND-ACCEPTANCE.md 测试；不上传真实 Key、数据库、聊天或配对码。

本次仅公开阶段2测试版，不表示实机验收完成，也不开始阶段3。开发测试签名可能随构建环境改变，不承诺测试包未来连续覆盖升级；不要为安装测试包卸载正式应用或清理其数据。
