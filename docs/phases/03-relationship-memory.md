# 阶段3当前实现：v0.0.3

本次用户已授权并实现关系/情绪/分层规则记忆/检索管理/成本，以及前移的虚拟生活基础。实际范围、测试与限制以 [PHASE3_REPORT](../PHASE3_REPORT.md) 为准，实机/真实API仍待验收。确定性本地规则取代每条消息多次分析API；embedding、通用自动摘要、桌宠/真实视频/语音未实现，不因本次接口规划声称完成。

下文保留最初设计供后续评估，接口示意与研究方向不等于全部已交付；与本版不同之处以报告和实际代码为准。

---

# 阶段 3：多维关系与长期记忆

状态：开发计划，当前只保留阶段 0 规则原型、v3 人格状态容器及手动记忆。没有自动抽取、自动好感增长或在线 embedding。

## 状态与证据

每个人格独立维护喜欢、信任、熟悉、亲密、舒适、主动意愿；短期情绪另有来源、强度、开始/到期时间。新状态为“未评估”，不往提示词硬写固定高好感。阶段 0 的五维整数规则保留，熟悉维度与本次阶段名称须通过版本迁移增加。

AI 只提议有聊天/共同活动 ID 的结构化关系候选；验证器核对来源、人格、证据范围、置信度、用户关闭选项。确定性规则引擎负责去重、冷却、单次/每日上限、重复行为边际递减和数值范围。普通问答可为零变化；刷屏不能无限刷分。跨端重放同一事件只算一次。

```ts
type CharacterScope = { profileId: string; personaId: string };
interface RelationshipEvidence {
  scope: CharacterScope; eventId: string; sourceIds: string[];
  kind: string; confidence: number; explanation: string;
}
interface RelationshipEngine {
  evaluate(candidate: RelationshipEvidence): Promise<{ accepted: boolean; ruleVersion: number }>;
  replay(scope: CharacterScope, ruleVersion: number): Promise<unknown>;
  context(scope: CharacterScope): Promise<{ relation: string | null; mood: string | null }>;
}
interface MemoryProvider {
  retrieve(scope: CharacterScope, query: string, budgetTokens: number): Promise<unknown[]>;
  revise(scope: CharacterScope, memoryId: string, text: string, expectedVersion: string): Promise<void>;
  remove(scope: CharacterScope, memoryId: string): Promise<void>;
}
interface EmbeddingProvider {
  describe(): { local: boolean; modelId: string; checksum: string; dimensions: number };
  embed(texts: string[], signal: AbortSignal): Promise<Float32Array[]>;
}
```

关系阶段采用多维条件、最低有效共同事件数和进入/退出滞回，平滑影响称呼、主动性、分享和亲密表达。阶段只是生成的约束，不是固定台词。规则升级保留原事件并可重新计算、对比摘要；生成器不能直接改 SQLite 分数。不得用内疚、威胁、失联惩罚强迫互动。

## 记忆流水线与成本

记忆包括用户偏好/习惯/约定、共同经历、人格自己的兴趣/学习/视频/网络梗。记录类别、来源、时间、可信度、重要性、隐私级别、版本与失效状态。区分“用户明确说过”与“模型推断”，冲突时询问/展示修订，不把猜测当事实。

本地规则先筛选显式“请记住”和实际任务成果；批量、可关闭的摘要任务只处理新增未摘要范围。去重按规范化内容指纹和近似候选，更新保留来源链。删除同时更新墓碑、FTS、向量及摘要引用；历史事件原文与独立备份的清理另有保留策略，不能把逻辑删除称为彻底擦除。

当前 SQLite FTS5 trigram 支持中文片段；短关键词用受限 LIKE。先过滤 profile/persona/权限，再检索，不能在检索后才剔除他人人格数据。研究可维护的 SQLite 向量方案或小规模本地向量索引，模型/向量维度/校验和必须入库，换模型重建索引。在线 embedding 默认关闭，启用时明确哪些文本离开设备并单独获得配置授权。

上下文组合：稳定人设 → 经验证关系/未过期情绪 → 当前人格近期有限回合 → 预算内记忆片段 → 本次输入。摘要具有覆盖消息范围和摘要版本；不能每次反复发送永久时间线或整个数据库。沿用当前 include_history 和轮数设置，增加上下文 Token 上限、检索条数和模块预算。

## 存储和验收

以新迁移扩展 persona_states 投影版本、证据事件、记忆来源/摘要覆盖和嵌入元数据；persona_id 不可随编辑改变。同步采用阶段 2 的版本事件，关系状态由事件重放，记忆修改/删除带逻辑版本。保留现有 v1 关系/记忆记录并迁移映射，不重置数据。

验收：普通问答零固定分、同来源重复无增量、刷屏递减、冷却/上下限、误会与和解、规则升级回放、两端状态摘要一致；人格 A/B 的记忆检索和上下文完全隔离；修订/删除后不被引用，离线设备回连不复活；检索本地运行、有限 Token、敏感内容控制有效。不同关系下模型表达自然有差异，但真实 DeepSeek 效果需用户在设备配置 Key 验证。
