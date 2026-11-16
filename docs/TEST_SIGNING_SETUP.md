# 测试签名与升级兼容

当前公开版本为 v0.0.5 / Android 1005。[签名身份](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.5/SIGNING-IDENTITIES.json)与[新旧 APK 对照](https://github.com/Lyuu508/DeepSeek-Niang/releases/download/v0.0.5/ANDROID-SIGNING-REFERENCES.json)均为公开资料，不包含私钥。

| 平台 | 当前测试身份 |
| --- | --- |
| Android | `io.local.deepseekcompanion.test2`，证书 SHA-256 `fefb57342751c7bd5ef8271e4a8b09f8702c0285e0120e8407b557d20dbbf153` |
| Windows | 自签名测试证书 SHA-256 `069f0150a9698a8178bdaf7dfea13f5509185aa69b9ba940f5473edbafa8af62`，不是公共 CA 信任发布者 |

本版已核对实际 v0.0.4 与 v0.0.5 APK 同证书、版本代码递增；满足该测试身份更新条件，但真实手机覆盖升级未验收。`upgrade_compatible_with_test2_v0_0_4` 与原身份兼容是不同范围，不能把报告中历史通用 `upgrade_compatible: false` 解释为所有 test2 更新都不可行。

v0.0.1 原身份及早期 `.test` 与本版不同，不能覆盖或自动读取其私有数据。保留旧应用、不卸载、不清数据，完整跨身份迁移尚未实现。普通用户看[升级说明](USAGE.md#升级与数据保护)，无需生成、导入或接触签名私钥。

协作者继续复用仓库已有加密签名 bundle；缺失或不匹配必须停止，不能重建替代身份。旧版本公开 APK、身份文件和证书对照是验证依赖，不因附件整理而改名或删除。本次未读取 Secrets 元数据或值，也未执行签名、发布或证书信任操作。以下“一次性配置”是已完成历史，不是后续更新必须重做的步骤。

<details>
<summary>历史开发与发布说明（原文保留，不代表当前状态）</summary>

# 当前阶段3追加候选与后续协作者复用

本版 `release-companion-test.yml` 仅恢复**既有**完整 `TEST_RELEASE_SIGNING_B64`，签名配置缺失/不完整/证书不符时失败，禁止生成替代私钥。无需一次性 `SIGNING_SECRETS_TOKEN`，不再执行首次初始化。具有同仓库Actions权限的其他账号直接运行授权版本工作流，Secret归仓库而非ChatGPT账号；无需在聊天传递任何值。

当前身份：Android `io.local.deepseekcompanion.test2`，证书 `fefb57342751c7bd5ef8271e4a8b09f8702c0285e0120e8407b557d20dbbf153`；Windows自签名测试证书SHA256 `069f0150a9698a8178bdaf7dfea13f5509185aa69b9ba940f5473edbafa8af62`。v3与当前0.0.4/1004候选保持它们，Android1004满足现有test2v3的同签名/版本升级条件，实际手机升级待验收。工作流必须真实私钥证明、旧APK官方验签、新APK再验签，不仅检查Secret名称。

原v1正式证书 `9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1` 与test2不同。本次不改变原正式applicationId/Secrets，不从旧任务找文件，不声称升级正式版；跨身份备份/迁移边界见 PHASE3_ACCEPTANCE.md。不要卸载或清除旧版数据。候选先用真实v0.0.3 APK和官方apksigner复核旧证书，再签名验证新包；默认草稿，确认版本后才公开。下面首次初始化内容是已完成的历史步骤，不应为后续版本重新生成签名。

---

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

</details>
