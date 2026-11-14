# b7330d9 CI 修复本地证据

本目录仅包含合成测试日志和公开 APK 证书信息，不含用户数据库、Secrets 或私钥。nonblocking-before.txt 是新增真实 TLS 回归在原服务代码上的确定失败；nonblocking-after.txt 是修复后通过。rust-tests.txt 为最终工作区 73 项测试，UI 为 18 项明确 Mock。actionlint 1.7.12、cargo fmt 和 git diff --check 退出码为 0（无输出）。Windows Hosted 完整日志尚未下载；后补的本地 Android 构建与公开 CI 状态见下文，不将本地日志冒充 Windows 原生执行记录。见 ../PHASE2_CI_FIX_REPORT.md。

local-android-build.txt 是 e65b3ed 本地完整脱敏 APK 构建日志（退出码 0，移除 ANSI 颜色码）；local-android-build.json 为后续真实包结构检查（未签名）。local-android-installed.txt / local-android-paths.json 为实际 SDK 清单及路径验证。remote-ci-status.json 为 GitHub 公开页面的三个作业成功快照，不包含受限 API 日志。AUTOMATED-TEST-RESULTS.json 汇总范围。原签名 APK、远程完整附件/日志和正式双端 SHA256SUMS 仍未取得。
