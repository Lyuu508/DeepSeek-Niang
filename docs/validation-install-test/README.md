# 安装包构建验证证据

日期 2026-11-12，本地版本 0.1.0-dev.1。两端安装包已生成；不是 Windows/Android 运行验收。
windows-build.txt 为完整 NSIS 成功构建记录，Windows 编译使用官方 Tauri cross compile 流程；不存在本地 Windows 主机。android-build.txt 包含真实 Rust、Kotlin/Java 与 Gradle APK 构建。Java 构建代理配置行已移除，其余仅清除终端颜色控制符。
windows-results.json 记录安装器和应用哈希，归档内容与应用仅有 Tauri 预期的 NSIS 类型标记差异；android-results.json 与 android-signature.txt/包清单记录真实签名、ABI、API 级别。
policy-tests.txt 是本次 3 项策略测试；48 项 Rust 与 13 项 UI Mock 为 validation-phase1 中保留的既有结果，本轮业务 Rust/React 未修改，未将旧测试冒充新实机测试。
所有 KeyStore/凭据管理器设备调用、真实 DeepSeek、安装/重启/键盘/性能均待用户实际测试。没有用户 Key、聊天库、签名私钥或代理凭据。
