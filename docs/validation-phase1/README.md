# 阶段 1 验证证据

2026-11-12，当前本地源码 0.1.0-dev.1。summary.json 列出结果与未验证边界。

- rust-tests.txt：真实 SQLite 与本地 Mock HTTP 的 48 项测试，含阶段 0 原始 21 项。
- browser.txt/results.json 和 *-mock.png：明确浏览器 Mock 验收，不是真实 API、原生 SQLite 或 OS 凭据存储。截图里的消息与 Token 均为测试夹具。
- policy-tests.txt：3 项 Python 构建/导出策略测试，不是 Android 编译。
- windows-vault-check.txt/android-*-check.txt：只对 Rust 目标代码 cargo check，不运行 OS API、不编译 Kotlin。
- windows-build-attempt.txt/android-init-attempt.txt：较早同阶段源码 dev.0 的实际原生构建尝试，失败于缺 MSVC lib.exe/SDK，之后 dev.1 未宣称生成安装包。保留原始版本文字，不改写失败日志。
- environment.json、schema.json、sync-lab.json：实际只读环境、DDL 和原有两 SQLite 文件协议实验。lab transport 仍是进程内，未验证 LAN。
- npm-audit-*.json 与 dependency-review.json：当次依赖元数据检查的有限范围，非安全保证。

没有真实用户 Key、对话、数据库或安装包。敏感凭据不通过日志收集，当前没有执行真实 DeepSeek 请求。

source-clean-install.txt/source-clean-frontend-build.txt：将源码 ZIP 解压到独立目录，重新 npm ci --ignore-scripts 后前端构建通过；之后归档只补充本验证文档，应用和依赖输入未变。tauri-info.txt 读取最终原生配置，仍明确报告缺少 Linux WebKit/rsvg，不是原生 GUI 运行证明。
