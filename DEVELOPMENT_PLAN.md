# 分阶段开发计划

当前版本 **0.0.1 测试版准备**（继承0.1.0-dev.2）。本次仅在既有工程完成永久聊天/人格迁移、应用改名、品牌资源接线和阶段文档。正式海滩 PNG 文件仍缺失，图标生成/替换和新版原生包不能验收。阶段0/1原代码、DDL及历史验证保留。

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
- [ ] 同一原始海滩PNG，生成/目视检查头像、ICO、adaptive、启动和通知图标。
- [ ] dev.2 Windows安装器和Android APK，旧dev.1包只作历史结果。
- [ ] 用户旧库升级、双端安装/覆盖升级/重启、系统凭据与真实DeepSeek验收。

未发现用户业务库，本次迁移仅在真实SQLite合成旧库验证，不能声称已升级用户设备。实机验收后由用户逐阶段决定后续实现。新DDL使用增量迁移，不删库掩盖问题；不自动安装未经确认的新程序/模型或调用新付费服务。

用户已明确授权本次上传公开仓库Lyuu508/DeepSeek-Niang及0.0.1测试发布；实际状态见docs/RELEASE_0.0.1.md，后续阶段仍不自动发布。
