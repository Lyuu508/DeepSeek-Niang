当前开发迁移版本为 v4（0.0.2 尚未发布）。新增生产 LAN 日志、待发队列、字段寄存器、冲突、墓碑、可信证书与共享档案；五张原型表扩展人格自然键。独占迁移连接按 SQLite 重建流程处理外键，提交前检查所有约束与行数、恢复 FK 强制执行，失败整体回滚并保留 Online Backup。详细说明见 [PHASE2_MIGRATION](PHASE2_MIGRATION.md)。下面保留历史版本表结构说明。

# SQLite 数据与迁移设计

执行源：原样保留 `0001_initial.sql`、`0002_chat.sql`，新增 `0003_personas.sql`，当前 user_version=3。所有数据在设备本地，数据库不通过网络共享。预留表已有真实 DDL，但对应业务不一定实现。下方原始表清单的阶段列沿用阶段0计划，最新阶段2～9对应关系以 DEVELOPMENT_PLAN.md 为准。

## v3 人格扩展

新增 personas、conversation_personas、persona_chat_preferences、persona_states、persona_selection 五表；十三张原表增加 persona_id/归属约束和索引。默认人格沿用 profile ID，新人格独立 UUID，与配对档案身份分离。每个人格唯一主空间，原 conversation/message 行不删除不改写，通过映射组成永久时间线。近期上下文、手动记忆/FTS、配置与用量按人格筛选，最后选择为本设备数据。

v1/v2 升级前 Online Backup，全部待执行迁移一个事务，验证保留行数/外键/跨档案消息归属再提交。备份失败/迁移失败不覆盖原数据，重复打开不重复生成。详情及真实夹具结果见 [PERSONA_MIGRATION](PERSONA_MIGRATION.md)。原型 profile 唯一键仍保留兼容性，关系/兴趣/生活等引擎上线前必须按对应计划扩展人格唯一键及事件类型。

## 存储不变量

- UUID v4 字符串主键；同一角色档案 ID 在加入同一数据集后保持一致。
- 时间为 UTC epoch ms，用于展示/审计。冲突依据逻辑版本，不依据本机时间。
- 本地写事件与更新视图同事务，`foreign_keys=ON`、`journal_mode=WAL`、`synchronous=FULL`、busy timeout 5 秒。
- 数据访问经过 Rust 仓储和参数化 SQL，不向 React 暴露任意 SQL 或完整文件路径读写。
- 五维关系使用千分值整数 `[0,100000]`；置信度/权重使用 `[0,1000]`，不接受 NaN、无限大或任意 LLM 总分。
- 主动消息、生活事件和视频任务有发生/触发/版本唯一键。运行状态和可重建缓存不代替原始事件。

## 表与键

| 表 | 主键 / 关键约束与用途 | 阶段 |
|---|---|---|
| local_profiles | id；本地角色档案 | 0 |
| devices | id；profile FK，local/trusted/revoked；公开身份及 secret_ref | 0 实验，2 配对 |
| conversations | id；profile FK | 0 基础，1 多对话 |
| messages | id；conversation/profile/origin/source_event FK；source_event 唯一 | 0 基础，1 生成分支 |
| character_profiles | id；profile 唯一；基础人格 JSON、revision | 3 |
| relationship_states | profile PK；五维范围、rule_version、source_digest | 4 派生缓存 |
| relationship_events | id；profile+evidence_hash+rule_version 唯一；逻辑顺序索引 | 4 |
| character_moods | id；profile FK；强度、有效时间、source_event | 4 |
| memories | id；profile FK；正文、重要性、敏感级别、逻辑版本、删除标记 | 0 基础，4 完整 |
| memory_relations | from+to+relation 联合 PK；memory FK，禁止自关联 | 4 |
| character_interests | id；profile+topic 唯一；有来源的权重 | 5 |
| daily_activities | id；plan/task FK；real/virtual、时间、进度 | 5 |
| life_events | id；profile+occurrence_key 唯一；activity FK | 5 |
| life_plans | id；连续计划 JSON、revision、next_step | 5 |
| proactive_messages | id；profile+trigger_key 唯一；证据、内容、候选/投递状态 | 8 |
| proactive_settings | profile PK；模式、时区、免打扰、冷却、通知授权 | 8 |
| video_tasks | task_queue.id PK/FK；source+analysis_version 唯一；模型能力快照 | 6 |
| video_sources | id；profile+source_kind+external_video_id 唯一；媒体 hash、权限证据、时长 | 6 |
| video_progress | task PK；播放位置、解码位置、两种覆盖、不确定范围、fence | 6 |
| video_timeline_events | id；task FK；时间范围、描述、置信度与证据 | 6 |
| video_analysis_results | id；task/segment FK；修订、模型、prompt 版本、内容 hash | 6 |
| video_memories | id；source/memory/task FK；task+memory 唯一；感想/分享候选 | 6 |
| meme_knowledge | id；profile+phrase 唯一；来源/解释/置信度 | 7 |
| media_interests | interest+source 联合 PK；来源评价 | 7 |
| external_accounts | id；profile+provider+external_user_id 唯一；只存 credential_ref/scopes | 7+ |
| application_settings | profile+key+device 联合 PK；profile/device scope | 1+ |
| task_queue | id；profile+dedupe_key 唯一；固定 authority、状态、owner、boot/fence/lease、次数 | 0 台账 |
| task_checkpoints | task+step 联合 PK；fence、JSON、hash | 0 |
| activity_states | id；task/activity 至少一个来源；executor、TTL、fence、进度与错误 | 5 |
| sync_events | event_id PK；origin+seq 唯一；envelope、hash、可选原始签名 | 0 实验 |
| sync_checkpoints | origin PK；连续 seq，不是已见最大 seq | 0 |
| data_tombstones | profile+entity_type+entity_id 联合 PK；删除事件 FK | 0 |
| api_usage_records | id；request_key 唯一；provider/model/token/cost；measured/estimated/unknown | 1+ |

额外表：`schema_migrations`（迁移 hash）、`local_clock`（每本机流时钟）、`sync_sessions`（会话诊断）、`memory_fts`（FTS5）、`memory_embeddings`（可重建向量）、`task_logs`（审计）、`budget_reservations`（预算预留）、`notification_deliveries`（设备投递幂等）、`video_segments`（范围/输入 hash/request key）。
FTS5 创建的内部影子表不作为业务表计算。基础迁移只为 sync_events 开放 message/memory 类型；其他模块上线必须新增迁移和真实事件映射器，不直接绕过约束。

## 关系与索引

主要路径：profile → conversations → messages；profile → memories → memory_relations/embeddings；profile → task_queue → checkpoints/video_tasks → segments/results/timeline。
基础索引支持 `(conversation_id,created_lamport,origin,id)` 历史分页、按 profile 的可见记忆、按设备原序号导出、按权威节点状态优先级取任务、视频时间线和最新活动。
索引不是无上限缓存：消息分页、记忆候选上限、事件分批、媒体附件独立缓存清理。性能基线在后续使用大规模合成夹具测试，不借用户数据做压力测试。

## 记忆更新和删除

物化主版本元组 `(lamport,origin,event_id)` 大者胜；全部修订保留在 sync_events，后续提供冲突历史界面。
墓碑一旦存在，同 ID put 都不会重建可见内容；恢复另建 ID。投影清空正文、清除 FTS 和向量；原始事件旧正文仍可能保留，阶段 0 不是彻底擦除实现。
FTS INSERT/UPDATE/DELETE 触发器随事务更新；中文短关键词使用转义 LIKE，长关键词做 trigram 候选检索。
语义向量记录模型 checksum、维度、正文 checksum；必须先过滤已删除或禁止发送的记忆，再构建 prompt。

## 迁移策略

1. 打开 DB 验证 `user_version`；高于当前可支持版本则拒绝写入。
2. v1 在 `BEGIN IMMEDIATE` 中执行 DDL、记录迁移文件 SHA-256 与版本，再 commit；重开校验 checksum，避免悄悄改旧迁移。
3. 后续发布禁止修改已发布迁移；新增 0002、0003…；需要重建 CHECK/外键表时用影子表迁移并校验行数/引用。
4. 正式升级前使用 Online Backup API 备份；失败回滚并保留旧 DB。阶段 0 仅有初始建库与版本拒绝逻辑，升级备份尚未实现。
5. 内核版本、数据库迁移版本、同步协议、关系规则、模型/prompt、备份格式独立编号；同步前协商，不能把应用版本号当数据库版本。

阶段 1 预计新增本地生成草稿和请求生命周期，以便流式取消/中断持久化。草稿不作为不可变最终消息提前同步；最终 complete/interrupted 变体再产生事件。
阶段 2 预计新增安全身份/档案引导/依赖 inbox/版本化 wire envelope 和投递 ACK。需要兼容读取 v1 实验数据，但发布时不把实验 trust 标记当成有效配对。

## 完整性与备份

启动执行轻量检查，完整 `integrity_check` 在用户诊断/备份/恢复流程运行；外键错误不可静默忽略。
当前诊断 CLI/页面执行完整 SQLite integrity + FK 检查。损坏自动只读和 UI 恢复引导仍为后续功能，当前失败应停止打开。
不能把 `companion.sqlite3` 活动主文件直接复制到手机；同步必须交换事件。独立备份通过 SQLite backup snapshot 取得一致视图。
恢复验证 hash、schema、完整性、外键、压缩展开大小与档案归属；生成新的复制身份，避免 `(origin,seq)` 分叉。
恢复不能把旧墓碑删除后继续冒用原设备序号，也不能用旧设备私钥克隆另一个节点。
加密导出、恢复前副本、历史保留、损坏恢复和用户彻底删除属于阶段 10 及隐私实现门槛。

## 已执行的验证

`scripts/check_schema.py` 独立使用真实 SQLite 建库，检查全部 33 张用户要求表、外键、关系范围、中文 FTS 和完整性。
Rust 测试使用应用实际 bundled SQLite 验证迁移、事务回滚、删除同步、FTS 修订和重启持久化。不同 SQLite 运行时版本分别记录，不混为同一环境。

## 阶段 1 已实现的聊天扩展

| 新表 | 作用与关键约束 |
|---|---|
| chat_requests | 请求 UUID PK、input_hash 幂等、会话/用户 FK、唯一 assistant ID、send/regenerate、生命周期/模型/错误/时间 |
| chat_generations | 草稿/最终变体 UUID PK、request 唯一 FK、原用户 FK、版本 revision、旧回复 FK、内容/状态/模型 |
| chat_turn_choices | 用户消息 PK/FK、选中生成 FK；重新生成不复制用户消息 |
| chat_preferences | profile PK/FK、校验过的 JSON、revision；无 API Key 字段，当前本地设置 |

conversations 新增可空显示时间；messages 新增显示时间、model_id、request_id 与索引，原有逻辑版本/来源/删除/事件字段不丢弃。旧记录不伪造墙上时间。
用户消息+请求+草稿同事务提交；逐段保存草稿；最终回复事件+请求状态+usage 同事务。部分回复中断也保存，空失败只留生成记录。启动恢复进行中草稿为 interrupted/APP_RESTARTED；不得自动重发。当前消息历史按会话完整加载，超大历史分页还未接 UI。

迁移先校验全部历史 checksum、高版本拒绝写入；v1 文件用 Online Backup 保留升级前副本。Store 持有 OS 文件锁，防止第二实例并发执行恢复。完整性检查不等于自动修复。阶段 2 仍需把人物设置、会话元数据和回复选择转为可合并事件，不能直接同步这些本地投影表或覆盖整库。
