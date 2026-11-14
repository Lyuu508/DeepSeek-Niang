# b7330d9 CI 修复本地证据

本目录仅包含合成测试日志和公开 APK 证书信息，不含用户数据库、Secrets 或私钥。nonblocking-before.txt 是新增真实 TLS 回归在原服务代码上的确定失败；nonblocking-after.txt 是修复后通过。rust-tests.txt 为最终工作区 73 项测试，UI 为 18 项明确 Mock。actionlint 1.7.12、cargo fmt 和 git diff --check 退出码为 0（无输出）。Windows Hosted 与本次 Android 产物未在这些日志中执行，不计通过。见 ../PHASE2_CI_FIX_REPORT.md。
