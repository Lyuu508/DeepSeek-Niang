# 当前进度

阶段 2 正在开发，目标 v0.0.2，尚未发布。用户已授权本阶段开发、GitHub 更新、双端构建与测试发布；停止在阶段 2，不开始阶段 3。
生产同步实现及验证见 [PHASE2_REPORT](docs/PHASE2_REPORT.md)，家庭网线 PC + Wi-Fi 手机实机验收见 [PHASE2_ACCEPTANCE](docs/PHASE2_ACCEPTANCE.md)。
用户已确认原 Keystore 在另一账号的旧任务中。原签名恢复/迁移与 Secrets 作为独立待办，本次不查找、不恢复，也不阻挡代码开发、Windows EXE 或隔离 Android 测试包。正式 applicationId 与旧版数据保持不变。每阶段双端包都必须来自同一 SHA；长期规则见 [VERSIONING](docs/VERSIONING.md)。

以下保留 0.0.1 发布时的阶段历史，不能据此将阶段 2 或后续阶段勾选为完成。

# 分阶段开发计划

当前版本 **0.0.1 测试版已发布**（继承0.1.0-dev.2）。本次仅在既有工程完成永久聊天/人格迁移、应用改名、品牌资源接线和阶段文档。正式海滩PNG已从2026-11-13用户ZIP导入并生成资源；本次重建实际Windows/Android安装包，实际构建/实机范围见RELEASE_0.0.1。阶段0/1原代码、DDL及历史验证保留。

按用户最新指令采用下表编号，取代原始0～10计划。旧报告中的阶段号属于历史语境，不表示当前已完成。

各阶段共用 [Provider/权限/成本合同](docs/phases/PROVIDER_CONTRACTS.md)，与现有DeepSeek、SecretStore和任务接口逐步适配；本次只定义计划，不启用新的服务。

| 阶段 | 独立文档 | 出口验收 |
|---|---|---|
| 0 基础工程 | [历史验证](docs/VALIDATION.md) | 原始 DDL、事件/同步/任务/关系/视频原型及21项测试保留 |
| 1 双端聊天与本次重构 | [历史](docs/PHASE1_REPORT.md)、[本次](docs/PERSONA_BRANDING_REPORT.md) | 永久空间/旧库迁移、真实双端安装/Key/API/重启、正式素材完整 |
| 2 LAN双向同步 | [02-lan-sync](docs/phases/02-lan-sync.md) | 网线PC+Wi-Fi手机、认证加密、增量/并发/删除/断线/撤销 |
| 3 关系与记忆 | [03-relationship-memory](docs/phases/03-relationship-memory.md) | 证据/规则、无固定刷分、人格检索/修订删除、Token预算 |
| 4 生活与主动分享 | [04-life-proactive](docs/phases/04-life-proactive.md) | 连续计划、真实/虚拟/过期、租约恢复、冷却/免打扰/开关 |
| 5 Windows桌宠 | [05-desktop-pet](docs/phases/05-desktop-pet.md) | 透明窗口、授权动画、共享人格/活动、DPI/性能 |
| 6 B站与视频 | [06-video-understanding](docs/phases/06-video-understanding.md) | 授权文件、本地全时间线、视觉/音频覆盖与缺口、合法媒体 |
| 7 语音与歌声 | [07-voice-singing](docs/phases/07-voice-singing.md) | 授权音色、独立Speech/Singing/Music/Conversion、试听 |
| 8 NAS | [08-nas-migration](docs/phases/08-nas-migration.md) | 数据保留、配对、权威交接、备份/回滚、离线聊天 |
| 9 发布准备与优化 | [09-release-optimization](docs/phases/09-release-optimization.md) | 真实EXE/MSI/APK、实机/升级、费用/性能/备份/安全 |

## 本次检查点

- [x] 保留迁移0001/0002、原21项测试、关系/任务/视频源文件。
- [x] schema v3：五张人格表、十三张旧表归属、事务迁移与升级前备份。
- [x] 永久空间、新增/切换/编辑、上次选择持久化、上下文/记忆/用量隔离。
- [x] 移除传统会话入口，移动抽屉/桌面资料栏、流式切换/慢读取保护。
- [x] 显示名称DeepSeek娘；identifier、应用目录和凭据命名空间保持。
- [x] 原图保全/图标制作/平台接线/打包校验脚本，缺图明确停止。
- [x] 57 Rust、16 UI Mock、6策略测试；前端/类型/Clippy/DDL通过。平台检查见实际报告。
- [x] 阶段2～9八份计划及接口，没有安装未来模型或实现后续阶段。
- [x] 原始海滩PNG已导入、校验并生成69项派生资源，头像/ICO/adaptive及安装后Windows窗口已目视检查；Android桌面显示仍待真机验证。
- [x] 从同一8ae10e7源码重建0.0.1 Windows x64 NSIS EXE与Android ARM64 APK，已发布并回读下载/哈希；旧dev.1包只作历史结果。
- [x] Windows runner原生58项测试、真实凭据往返、静默安装和启动窗口通过；Android APK签名/版本/ABI/图标清单通过。
- [ ] 用户旧库升级、双端安装/覆盖升级/重启、系统凭据与真实DeepSeek验收。

未发现用户业务库，本次迁移仅在真实SQLite合成旧库验证，不能声称已升级用户设备。实机验收后由用户逐阶段决定后续实现。新DDL使用增量迁移，不删库掩盖问题；不自动安装未经确认的新程序/模型或调用新付费服务。

用户已明确授权本次上传公开仓库Lyuu508/DeepSeek-Niang及0.0.1测试发布；实际状态见docs/RELEASE_0.0.1.md，后续阶段仍不自动发布。

## 阶段 2 CI 修复（2026-11-14）

从 b7330d9 修复 Windows accepted socket 模式与 Android SDK 命令路径，保留全部原有测试，新增真实 TLS 非阻塞回归；本地 73 项 Rust 与其余常规检查通过。原 APK 已重新实际验签。e65b3ed 的 Run 43554759767 三个 Hosted 作业均成功，Windows 含原生测试与安装 smoke，Android 为未签名 ARM64 APK；本机另有完整未签名 APK 与脱敏日志。当前 GitHub API 已连通，Secrets 接口受授权权限限制，构建附件另受 Azure 存储域名限制；原签名升级包、远程附件复核与发布尚待完成，不开始阶段 3。见 [CI 修复报告](docs/PHASE2_CI_FIX_REPORT.md)。

## 接管后执行顺序（2026-11-14）

以最新 main 9cc9a16 为接管基线，不使用旧环境代码覆盖。当前下一个未完成环节是阶段 2 的签名身份/Secrets、正式升级 APK、构建附件复核与发布，随后执行家庭网线 PC + Wi-Fi Android 实机验收。关系与记忆的产品实现属于阶段 3，不因阶段 2 签名受阻而擅自占用 0.0.3 或启动后续阶段。独立完成的常规测试、品牌核验和文档校正见 [接管报告](docs/TAKEOVER_2026-11-14.md)。

## 阶段 2 独立测试交付（最新指令）

用户已确认 Keystore 在另一个账号的旧任务中，当前不再查找、猜测路径、迁移签名或配置 Secrets。这些是独立待办，不阻挡阶段 2 的代码修复、测试、Windows 构建或隔离 Android 测试包。

本次增加 .test applicationId 的可安装开发测试包、明确名称/横幅、正式升级包拒收保护、双端独立 SHA256SUMS/日志和 CI 测试报告。生产同步与旧数据库迁移保持现有实现；新增单元测试验证测试身份不改正式身份。实际构建与测试结果见 [测试交付报告](docs/PHASE2_TEST_DELIVERY.md)。

当前可完成自动化和原生构建，当前云机器没有可用家庭 Windows + Android 设备组合，真实网线+Wi-Fi 配对、双方确认、聊天/记忆双向、冲突、离线补传和数据保留仍须用户设备验收。不能用 loopback 或 UI Mock 勾选实机完成。阶段 2 收尾后等待用户验收，不开始阶段 3、不启动 0.0.3。
