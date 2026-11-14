# 阶段 2 隔离测试交付本地证据

这些结果来自 7acd099 之后的修改工作区，不冒充最终已提交 CI 安装包。android-local-build.txt 是首次失败的完整脱敏日志（开发测试签名文件不存在），后续日志记录修复、去除调试符号和重新打包后通过。android-local-report.json 为真实 .test APK 验签/身份/ABI/安全 Manifest 检查，production-regression 为仍保留原 ID 的未签名 release 回归。正式原签名未恢复或迁移。UI 两次各18项是明确的浏览器夹具，Rust73含真实TCP/TLS自动化但不等于Windows+Android物理双端。用户已确认由其收到包后进行真实设备验收。没有保存私钥、开发 Keystore、API Key、用户数据库或聊天。最终同源码双端 CI 产物、SHA256SUMS 与完整构建日志在对应 Actions Artifacts 中。
