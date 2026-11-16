# v0.0.5 依赖与构建审查

本次在已审查的 v0.0.4 main 上继续开发，原图和旧迁移不变，没有执行候选桌宠仓库、远程安装脚本或未知 EXE。npm 始终 `ci/install --ignore-scripts`，锁文件提交，Hosted Cargo/Gradle/MSVC 用原工作流。GitHub 官方 Actions 仍固定完整提交 SHA，发布为手动明确授权的 v0.0.5，不新增未来自动发布。

新增依赖仅 Tauri 官方 `tauri-plugin-notification` / `@tauri-apps/plugin-notification` 2.8.1，来自 crates.io/npm 官方包，声明 MIT/Apache-2.0、Rust>=1.90/Tauri2.12/Android compileSdk36，与本项目匹配。读取发行归档的 Cargo/package、build.rs、Android Gradle、manifest、插件入口；npm 没有安装生命周期脚本，Rust build.rs 使用标准 tauri-plugin 构建注册。锁定其传递依赖。npm production audit 实测0漏洞，不代表对所有源码和平台的绝对安全证明。

Android manifest 明确移除依赖库携带的 RECEIVE_BOOT_COMPLETED、WAKE_LOCK、定时发布及开机恢复接收器；构建后 verify_android 从实际二进制 manifest 断言这些能力不存在。仅保留用户同意的即时通知和现有网络/扫码权限。前端权限仅增加通知状态查询、申请及通知发送，不开放计划通知、shell、任意文件系统或远程窗口。

签名沿用 TEST_RELEASE_SIGNING_B64；Windows 路径强制 `--restore-only`，Android 强制 existing_bundle；值仅 Hosted 进程内和受限临时签名文件（Android0600），随后删除，不进入 Git/日志/构件。原来首建脚本的生成分支不会被本次工作流调用。恢复/私钥校验/公钥 SHA 或实际旧新 APK/apksigner 不一致即阻止发布。没有查询 Secrets 元数据，没有重新要求用户迁移旧 Keystore。Windows 公钥 SHA-256 `069f0150a9698a8178bdaf7dfea13f5509185aa69b9ba940f5473edbafa8af62`，Android `.test2` 公钥 SHA-256 `fefb57342751c7bd5ef8271e4a8b09f8702c0285e0120e8407b557d20dbbf153`，均由签名脚本钉住。

新增 SQLite 事务与外键限定活动、任务、计划、人格、来源和主机；主动消息/生活记忆稳定 ID 去重。生产同步仍先验证原始来源签名和 mTLS，新增列仅投影固定旧默认值，不能放开任意字段。回执只能本人设备写入；生活成果与兴趣来源验证绑定主机。执行授权不来自远端同步的租约。

API Key 仍使用 DPAPI/Android Keystore，模型生成无 Key 不建立收费任务，不输出密钥。默认保留关闭/模型显式同意；相关记忆受已有 API 隐私许可过滤。未知 POST 保留额度不自动重放。系统通知正文为通用提示，不展示私人聊天；系统 API 成功不等于已读。

发布门禁检查原生安装包/测试来源同 SHA、官方签名和历史证书、完整性和禁止私人文件模式；保留旧版本资产。141项 Rust、原24项 Python、5项 Node、schema、Clippy和品牌/UI门禁不降低。机器级代码签名、实体设备与真实 API 均按实际证据记录。
