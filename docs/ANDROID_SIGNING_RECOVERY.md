# v0.0.1 Android 签名连续性调查

当前新环境实测原公开 APK：

- SHA-256：`5100cdd29b6751e3ffd1d6536f5c125af6888f679dc524cdfbb6048afe3195d8`。
- apksigner 验证 v2 签名通过，证书 SHA-256：`9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1`。
- `CN=Android Debug, O=Android, C=US`，RSA 2048。
- 包名 `io.local.deepseekcompanion`，versionName `0.0.1`，versionCode `1001`，ARM64。

仓库基线只有 Windows Actions 工作流；旧 Android 脚本使用 `--debug`，历史报告显示沿用了旧本地开发 keystore。本机可访问的 /workspace、/home/agent、/opt 未找到 `.keystore` 或 `.jks`。这不能证明旧云机器上的文件已丢失。证书与 APK 中只有公钥，不能据此恢复签名私钥。

GitHub Actions Secrets 元数据目前查询被当前云代理拒绝（api.github.com 未获运行时网络许可），尚不能确认 Secrets 是否已有签名绑定。没有读取、导出或公开任何 Secret 值。

恢复顺序：

1. 在仓库 Settings → Secrets and variables → Actions 检查已有签名条目，或授予当前环境 api.github.com 网络访问后由代理列出名称；仅名称不等于私钥可读。
2. 若旧 Codex 任务环境仍可打开，检查当时 `ANDROID_USER_HOME` 下的 debug.keystore，或默认 `~/.android/debug.keystore`、备份/构建目录。不要在公开聊天、Git 仓库或 Actions Artifact 中传送文件或密码。
3. 用 keytool 导出公开证书计算 SHA-256，与上方实际指纹核对。找到匹配密钥后安全保存到 Actions Secrets：`ANDROID_KEYSTORE_B64`、`ANDROID_KEYSTORE_PASSWORD`、`ANDROID_KEY_ALIAS`、`ANDROID_KEY_PASSWORD`。Base64 不是加密，必须作为 Secret 保存。
4. 发布工作流只在临时受限文件中恢复 keystore，先验证证书，再构建、apksigner 验签并清理临时文件；指纹不符立即失败。

没有找到匹配私钥前，原签名是否可恢复仍是未确定状态；正式 v0.0.2 覆盖升级与发布被阻止。不生成替代密钥，不改变 applicationId，不要求卸载或清除数据。仅在确实无法恢复且用户另行同意后，才能设计独立测试包身份和安全数据迁移，不能将独立包描述为直接覆盖升级。

## 本次 CI 修复复核（2026-11-14）

在 b7330d9 修复任务中再次下载实际公开 v0.0.1 APK，使用官方 Build Tools 36 的 apksigner 与 aapt2 重验；上方 APK 哈希、唯一签名证书与应用身份全部一致，原始输出保存于 validation-phase2-ci-fix/v0.0.1-apksigner.txt。e65b3ed 双端 CI 已成功，但 Android 仍为未签名编译检查 APK。列出四项 Secrets 名称的请求仍被 api.github.com 代理拒绝，不能据此判断 Secrets 缺失或旧私钥丢失；未触发原签名 Release。详情见 PHASE2_CI_FIX_REPORT.md。
