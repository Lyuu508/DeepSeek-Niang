# DeepSeek娘 0.0.2 测试版 · 阶段 2

本页在构建前作为发布说明准备；实际发布与包哈希以成功 Release 附件 `BUILD-RESULTS.json`、`SHA256SUMS.txt` 为准。源码准备或单端编译通过不代表已发布。

新增人工配对后的 Windows/Android 局域网双向增量同步：TLS 1.3、证书绑定、来源签名、永久人格 UUID、离线补传、删除标记、字段修订与独立同步管理页。Windows 网线与 Android Wi-Fi 可通过同一可互通家庭网络连接，无云数据库或聊天中转服务器。

Windows 安装 EXE 与 Android ARM64 APK 必须来自同一个源提交。Android 保持 `io.local.deepseekcompanion`，versionCode 1002，强制核对 v0.0.1 原签名。Windows 安装器暂无 Authenticode 签名。

自动化范围：核心回归、两个模拟设备的真实 SQLite/TLS、签名/撤销/配对、队列/分页/去重/删除/冲突、v3→v4 备份回滚、二维码解码、TypeScript 与显式 UI Mock。具体运行结果见阶段报告与 CI；没有宣称 Windows/Android 用户设备、真实 DeepSeek 或家庭网线 + Wi-Fi 通过。

安装与验收见附件 INSTALL-AND-ACCEPTANCE.md。保留旧数据，直接覆盖升级前核对签名与哈希。每台设备单独配置模型 Key；不分享 API Key、私钥或账号凭据。Android 强制停止后不保证后台同步，重新打开恢复网络后补传。

已知范围：使用私有 IPv4，不支持公网或 IPv6 手动地址；直接配对来源，不为陌生第三设备转发授权；两端已有同名人格保留为两个 UUID，不擅自混合聊天；关系/生活/任务自主引擎仍未启用；逻辑删除不承诺清除历史日志与备份。首次配对的指纹与实际家庭网络必须由用户验证。
