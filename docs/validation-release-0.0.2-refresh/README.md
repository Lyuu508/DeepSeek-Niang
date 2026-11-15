# 0.0.2 扫码签名修订证据

这些文件只包含公开发布元数据、公开证书指纹、校验摘要与自动化结果，不含密码、Token、私钥、用户数据库或聊天。

- `public-verification.json`：16 项当前 Release 附件实际回下载 HTTP 状态、大小、SHA-256；对照 GitHub API 摘要与 SHA256SUMS，记录同源 SHA、发布运行及历史 Tag 不变。
- `archive-verification.json`：原二十项附件 ID/摘要与归档清单逐项一致。该文件检查 GitHub API 摘要，不把它描述为再次下载了所有历史大包。
- `BUILD-RESULTS.json`、`BUILD-RESULTS-WINDOWS.json`、`SIGNING-IDENTITIES.json`：实际下载的当前公开报告，保留原字节。
- `android-independent.json`、`apksigner.txt`：本云环境对已公开 APK 使用官方 Android Build Tools 的独立复验。独立 APK 检查器不能查询 GitHub Secret，故其通用 `test_signer_persistence_guaranteed` 默认仍是 false；持久化成功由发布工作流的真实 bundle 保存/跨 Runner 恢复，以及公开总报告证明，不能只凭 APK 推断。
- `ci-results.json`、`publish-results.json`：GitHub API 实际作业/步骤状态，不是模拟测试输出。

安装包源码为 `9bc43c4dde86a92153273be8df048b10624475ae`。Windows 安装报告记录的 43697030853 是产生该签后验证记录的运行；最终 43697671242 复用同一已校验字节，完成全部公开下载验证与 Release 正文更新。版本、源码和摘要未混用。

完整结论与实机待办见 [交付报告](../RELEASE_0.0.2_REFRESH.md)。
