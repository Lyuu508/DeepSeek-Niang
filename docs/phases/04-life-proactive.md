# 阶段 4：虚拟生活与主动分享

状态：开发计划。当前任务租约/暂停恢复为实验基础，persona_states 的生活状态默认未启用；没有后台定时调用模型或虚构联网经历。

## 执行合同

每个人格拥有计划、兴趣、活动历史和日记。长计划保存下一步骤而非每日重新随机；阅读、休息、练习等明确标注虚拟事件，视频获取、媒体分析等必须有真实执行证据。

```ts
type TaskScope = { profileId: string; personaId: string };
interface TaskRunner {
  capabilities(): Promise<{ nodeId: string; kinds: string[]; resources: unknown }>;
  run(input: { scope: TaskScope; taskId: string; fence: number; checkpoint: unknown }, signal: AbortSignal): AsyncIterable<unknown>;
  pause(taskId: string): Promise<void>;
  cancel(taskId: string): Promise<void>;
}
interface ActivitySnapshot {
  scope: TaskScope; activityId: string; taskId?: string;
  executionKind: 'real' | 'virtual'; state: string;
  startedAt: number; updatedAt: number; executorId?: string;
  progress: number | null; sourceIds: string[]; nextStep?: string;
  stale: boolean; errorCode?: string;
}
interface ProactiveEngine {
  propose(scope: TaskScope, sourceEventId: string): Promise<string | null>;
  deliver(candidateId: string, deviceId: string): Promise<void>;
}
```

TaskScheduler 领取任务、租约续期、fence 验证和写检查点；TaskRunner 只执行自身支持的步骤。Windows 优先媒体重任务，Android 前台按电量/Wi-Fi/温度/内存能力限制；未来 NAS 也是同一接口。原型租约只在同一权威节点有效，阶段 2 完成权威协商前不让两端各自声称唯一执行者。

状态机：queued → running → completed；running 可 paused/cancelled/failed/uncertain，重试基于检查点与幂等输出键。领取使用 SQLite 条件更新事务；提交结果必须同时验证 taskId/fence/authorityEpoch。墙钟不作为租约有效性唯一依据；权威重启使旧租约失效，离线不确定任务先 reconcile，禁止自动双执行有外部副作用的步骤。

## 连续生活和状态真实性

保存日程、开始/结束、进度、来源、执行节点、当前情绪、下一步及错误。进度只来自完成的可量化工作；无法计量时用不确定进度，不随机填百分比。状态更新有限频率，避免每秒写库或每分钟调 DeepSeek。

两端都停止时没有真实网络任务。下次启动只能推进明确的虚拟休息/日程事件，来源标为 virtual/catch_up；未完成网络任务显示暂停/待恢复。远端快照包含更新时间与最后同步时间，超时标记过期；问“在做什么”先读真实快照，再允许模型概述，有依据的视频/活动才能说已看过/已完成。

## 主动分享

真实视频结果、学习/创作成果、显式约定的后续关心和共同记忆可产生候选。候选去重键包含 persona、来源事件、触发类型和规则版本。先本地规则筛选，再在预算允许时生成自然语言；生活状态不需要频繁调用模型。不同人格不能读取对方计划或触发来源。

默认自然模式，支持关闭、安静、活跃、自定义；免打扰按用户时区正确处理跨午夜和时区变化。通知开关与内容生成开关区分；按人格冷却、相似候选去重、每日数量/预算上限、重要性排序。关闭后取消待生成任务，待发送候选允许查看/清除。跨设备生成只一次，通知送达/已读与内容生成分开记录，避免双端重复骚扰。

Windows 应用运行时本地通知/气泡；Android 用前台界面和系统允许的后台任务，不保证强停后送达，没有云推送。迟到消息保留事件发生时间，不能伪装刚完成。

## 数据迁移与验收

扩展 life_plans、daily_activities、life_events、proactive_messages、activity_states 的人格唯一键；保留全部 v3 行和 task_checkpoints。activity 与 plan/task 的 persona 一致性必须加外键或触发器验证。关系变化消费已去重的生活证据，不直接加分。

验收：三日连续计划、中途退出恢复、同任务双节点领取与 fence 拒绝、权威重启、不确定任务对账、取消/重试无重复成果、过期状态和虚拟补推清楚；有依据分享、无关事件不生成、免打扰跨午夜、关闭后无模型调用、预算耗尽暂停、两端不重复生成，Android 被停止后不声称继续联网。
