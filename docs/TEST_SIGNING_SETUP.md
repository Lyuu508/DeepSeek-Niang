# 新测试签名的一次性安全配置

用户已授权新 0.0.2 测试签名，替代此前等待旧测试私钥的选择。新 Android 安装身份为 `io.local.deepseekcompanion.test2`、名称“DeepSeek娘 扫码测试版”，保留旧应用及其聊天；不能覆盖旧 `.test` 或正式版。Windows 使用独立、自签名的测试代码证书，不是公共 CA 信任发布者，系统安全提示可能仍存在。

## 已完成状态

2026-11-15，管理员已配置一次性 `SIGNING_SECRETS_TOKEN`。两端私钥已完整写入下方七项仓库加密 Secrets，并在多个新的 GitHub-hosted Windows runner 中成功恢复、完成真实私钥证明、签名和验签。原 0.0.2 下载资产已更新；[发布运行 43697671242](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43697671242) 成功，[公开证书指纹](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.2/SIGNING-IDENTITIES.json) 可核验。这里不展示任何秘密值。

一次性 PAT 现在可在账号的 Developer settings 撤销，并在仓库 Actions Secrets 中删除 **仅 `SIGNING_SECRETS_TOKEN`**；保留下方七项持久签名配置。不要删除/重建签名 bundle。其他协作者更新同一仓库时不需要再生成或传递私钥。

## 首次管理员配置（历史步骤，已完成）

本任务初始连接拥有仓库 push 权限，但 Secrets 公钥/列表 API 实际返回 HTTP 403，不能直接保存私钥。以下最小权限步骤已通过管理员配置与工作流执行完成，不需要再次配置 Token。

1. 由 `Lyuu508/DeepSeek-Niang` 仓库管理员在 GitHub 创建短期 Fine-grained personal access token：Resource owner 选择仓库拥有者，仅选择本仓库，Repository permissions 只授予 **Secrets: Read and write**（Metadata 为默认读取），建议有效期一天。
2. 在仓库 [Settings → Secrets and variables → Actions](https://github.com/Lyuu508/DeepSeek-Niang/settings/secrets/actions) 新建 Repository secret：`SIGNING_SECRETS_TOKEN`。值仅填到 GitHub 的 Secret 输入框，不发聊天、代码、Issue 或日志。
3. 告知本任务“已配置 SIGNING_SECRETS_TOKEN”。已授权工作流会在通过的固定源码 CI 产物上生成/保存新签名、签名两端、验签和 Windows 安装启动检查，然后替换原 0.0.2 的直接下载附件。

## 工作流如何保存

`refresh-v0.0.2.yml` 只处理固定 0.0.2 请求；不自动发布后续版本。它在同一 GitHub-hosted Windows runner 中操作 Android PKCS12 与 Windows PFX，不通过公开附件或普通仓库文件传输私钥。

先以单次 Secrets API 写入完整的 `TEST_RELEASE_SIGNING_B64` 加密仓库 Secret，包含两端私钥、密码、alias 和身份元数据；这保证后续单项写入中断时仍可恢复同一身份。兼容配置保存成功后才将该 bundle 的完成标志设为 true，并允许发布。全部私密文件只在 RUNNER_TEMP，最后清理。

保存的配置名：

- `TEST_RELEASE_SIGNING_B64`：完整、持久的签名 bundle；优先用于后续自动恢复。
- `ANDROID_TEST_KEYSTORE_B64`、`ANDROID_TEST_KEYSTORE_PASSWORD`、`ANDROID_TEST_KEY_ALIAS`、`ANDROID_TEST_KEY_PASSWORD`。
- `WINDOWS_TEST_PFX_B64`、`WINDOWS_TEST_PFX_PASSWORD`。

原身份的 `ANDROID_KEYSTORE_*` / `ANDROID_KEY_*` Secrets 不覆盖，新测试签名也不用于正式 `io.local.deepseekcompanion` 升级。私钥、密码、bundle 的 Base64 不进入普通文件、构建附件或发布附件。公开附件只包含证书、指纹和构建/验证报告。

## 其他账号以后如何更新

为其他账号授予同一仓库适当的协作者和 Actions 运行权限；使用同一工作流读取加密 Secrets，即可复用已有签名，无需知道或复制私钥/密码。完整 bundle 存在时不再需要短期 `SIGNING_SECRETS_TOKEN`，首次成功后可以删除该短期 Token/Secret。

工作流会与公开 `SIGNING-IDENTITIES.json` 中的真实证书指纹比较；发现身份已公开但 bundle 丢失时拒绝重新生成，避免再次破坏覆盖更新。后续 `.test2` APK 必须保持该 applicationId 和证书。新的版本仍需用户明确授权，按项目版本序列递增。

GitHub Secrets API 不能读回秘密值。换仓库不是换账号；若未来要迁移到另一个仓库，应另行安排受保护的备份/迁移，不能仅复制公开证书来恢复私钥。
