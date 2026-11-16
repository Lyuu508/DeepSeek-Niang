# 仓库展示与发布信息审查

## 本轮范围

以公开 v0.0.5、源码 `9248ffa1a91a5636bc5c25202a11cc1de50f0820` 为审查基线。已结合前端入口、Rust 聊天／记忆／关系／生活实现、同步与安全存储、测试、发布脚本、公开 Release 与附件核实；用户批准先通过文档分支和 PR 整理展示。About、Topics 与 Release 文字待 PR 审核后再更新，当前文件中的文本是候选，不是已执行操作。

本轮不修改业务代码、迁移、版本号、锁文件、安装身份、密钥、原始素材、发布工作流或现有附件，不选择许可证，不合并 main。

## 功能与证据分层

| 状态 | 内容 |
| --- | --- |
| 已实现、可测试 | 独立人格永久聊天，流式／停止／重生成；分层规则记忆与旧聊片段检索；五维关系与持续情绪；LAN 扫码、签名增量同步和一次 Key 配置；Windows 在线虚拟生活、多日计划、日记、兴趣、主动分享、预算和通知 |
| 自动化覆盖、实机未验收 | SQLite 升级／回滚、隔离、回环 TLS、模型 HTTP 夹具、来源去重、任务恢复、浏览器 UI；不能写成真实用户体验已通过 |
| 待补充验收 | 真实 DeepSeek、家庭网络、用户旧库、手机覆盖升级／Key 存储／相机／键盘／后台／通知、Windows 完整内容加载 |
| 后续功能 | 桌宠、真实视频理解、语音与歌声、NAS 生产节点、完整跨 Android 身份迁移及加密备份工具；未作为已发布能力 |

没有把“待验收”写成“正在开发”，也没有因表结构、算法或计划文档存在而声称业务已实现。Windows 空白截图和旧 UI 文字的证据及限制见[验证说明](VALIDATION.md#待核实问题)，需另行代码／平台任务调查。

## 发现与处理

| 问题 | 本轮处理 | 保留与风险 |
| --- | --- | --- |
| 首页偏历史修复、旧版下载 | 新首页先给产品定位与 v0.0.5 下载，再列功能、安装、变化和限制 | 原首页存入[历史文档](history/README_BEFORE_PRODUCT_REVIEW.md) |
| About 把同步与虚拟生活写作未来计划，公开页无 Topics | 提供下方最终候选 | 尚不修改远端元信息 |
| 安装文档停留旧身份与无 LAN 状态 | 提供当前安装步骤及数据保护入口 | 原文折叠保留，避免抹去历史构建与签名说明 |
| 多份文档叠加不同“当前”状态 | 补 v0.0.5 当前摘要，将旧文折叠标明历史 | 不移动脚本依赖的原路径 |
| 旧中文修订附件 6 个证据链接 404，EXE 同名链接已指向刷新后的包 | 将 7 个历史链接指向原字节的 `ARCHIVE-` 文件 | 不改历史摘要或原附件 |
| Release 技术附件混排 | 提供用户优先的[发布说明草稿](release-notes/v0.0.5.md)和附件分类 | 本轮不删除、改名或替换附件 |
| 首页可用素材主要是 UI 测试图 | 原样纳入现存 v0.0.5 浏览器截图，明确标注 | 不伪装原生、实机或真实 API 效果 |
| 缺少项目许可证 | 明示状态 | 仍需维护者选择；不能由依赖许可证推定 |

## About 候选

面向 Windows 与 Android 的本地优先 AI 陪伴软件：自定义人格、持续聊天、长期记忆、关系与情绪、局域网同步及虚拟生活。使用自备 DeepSeek API Key，目前处于测试阶段。

## Topics 候选

```text
ai-companion
deepseek
windows
android
local-first
lan-sync
tauri
react
typescript
rust
sqlite
```

不使用暗示已发布桌宠、语音、视频理解或离线模型的标签，也不在许可证未明确时新增开源许可徽章。

## 全部公开版本附件盘点

下表计上传附件，不含 GitHub 自动生成的两个源码归档。每版均为 Pre-release；审查时 `/releases/latest` 返回版本列表，因此首页使用明确 v0.0.5 链接。

| 版本 | 上传数 | 核实情况与建议 |
| --- | --- | --- |
| v0.0.1 | 4 | 两个包、摘要和构建报告；保留历史原身份 APK，签名门禁会读取 |
| v0.0.2 | 41 | 16 个当前、25 个归档；当前两个 Windows EXE 摘要相同，但保留旧 URL；Tag 源码不等于后续刷新包源码 |
| v0.0.3 | 23 | 保留公开安装及签名证据；长篇发布故障说明可通过历史入口访问，尚未改远端正文 |
| v0.0.4 | 23 | 报告仍有发布前候选措辞，作为当时快照保留；旧 APK／身份文件是后续签名依据 |
| v0.0.5 | 25 | 页面显示 27 项，额外 2 项为自动源码归档；已完整下载两包并核对清单摘要 |

## v0.0.5 附件分类

| 类别 | 文件 | 处理建议 |
| --- | --- | --- |
| A：用户安装及校验 | `DeepSeek-Niang_0.0.5_windows-x64-setup.exe`、`DeepSeek-Niang_0.0.5_android-arm64-isolated-test.apk`、`SHA256SUMS.txt` | README／Release 顶部直接链接 |
| B：构建与签名证据 | `BUILD-RESULTS.json`、`BUILD-RESULTS-WINDOWS.json`、`SIGNING-IDENTITIES.json`、`ANDROID-SIGNING-REFERENCES.json`、`android-reverified.json`、`apksigner-0.0.5.txt`、`WINDOWS-TEST-SIGNER.cer` | 保留稳定路径，技术入口链接 |
| B：测试与安装证据 | `AUTOMATED-TEST-RESULTS.txt`、`UI-TEST-RESULTS.json`、`signed-windows-install.log`、`windows-installer-language.json`、`windows-installed-window.png` | 保留；空白截图不用于宣传 |
| C：可减少重复展示的文档副本 | `CHANGELOG.md`、`DATABASE-MIGRATION.md`、`INSTALL-AND-ACCEPTANCE.md`、`PHASE4-REPORT.md`、`SECURITY-REVIEW.md`、`SIGNING-SETUP.md`、`TOKEN-COSTS.md` | 后续可用固定源码版本文档作阅读入口，发布时快照继续保全 |
| C：可合并归档的日志 | `windows-build.log`、`android-build.log`、`android-toolchain.log` | 后续可纳入完整证据包，不直接删除 |

本次推荐保留全部 25 个文件。C 类不是无用或已批准删除；GitHub 原生 Assets 是扁平列表，正文分类不能真正隐藏附件。

## 后续工作流方案与风险

如另行批准，可让新版本直接提供 EXE、APK、校验和、构建摘要、签名摘要及一个完整发布证据 ZIP。ZIP 保存原文件名、版本、源码 SHA、每文件摘要和原始报告／日志／公钥证书；保留长期可下载位置并验证完整性，不上传私钥。

- `.github/scripts/publish-companion-test.py` 按明确文件名复制资料，并严格核对附件集合、摘要和大小；必须同步设计打包、门禁与下载校验，不能只删上传步骤。
- `.github/scripts/sign-test-package.py` 读取历史 APK 与身份文件验证签名；移动、改名或删除可能破坏后续发布。v0.0.1 与 v0.0.4 的相关文件尤其应保留。
- CI／签名／发布证据 Artifacts 主要保留 30 天，另有旧工作流仅 7 天，不能成为唯一长期档案。
- 已发布包与摘要不重写，Tag 不移动；文字编辑不通过重新运行发布器进行，因为它拒绝已发布版本复用。
- 文档 PR 会按现有配置触发 CI 测试及原生编译，不自动公开 Release。本轮不为减少检查改工作流。

## 本轮检查与执行状态

本轮已完成以下文档检查：

- 19 份变更或新增 Markdown 的 258 个链接／图片引用检查，本地文件与章节锚点无缺失。
- 39 个公开 GitHub 链接均返回 HTTP 200，包括当前安装入口及修复后的历史归档；历史管理员 Secrets 设置页匿名访问返回 404，需要有权限账号登录，不能据此判断设置不存在。本轮未访问 Secrets API。
- 原中文修订 EXE 的归档文件实际下载为 6,484,690 字节，SHA-256 为 `c20e8de48c0636bf1b29b7c7a6a68a2c1ddd8635c347fcaebcb2d80a10fdaaf0`，与原报告相符；没有把刷新后同名文件当成旧文件。
- 两张截图与原输出逐字节一致，原图及 69 项品牌资源校验通过；版本检查仍为 0.0.5／1005。
- 十份更新的说明文档保留原历史正文，仅 Windows 历史安装文档修正两个归档链接；旧首页另存并检查相对链接。Git 差异仅含 19 份 Markdown 和两张 UI 截图，无应用、迁移、身份、锁文件、原始素材或工作流改动。

已有软件测试数只引用 v0.0.5 发布证据，不把文档检查描述为重新完成真实设备验收。没有为文档改动重建、签署或发布安装包。

候选 About／Topics 和 Release 文字须在文档 PR 审核后执行；当前所有安装包、Release、Tag、工作流和项目许可证状态保持原状。
