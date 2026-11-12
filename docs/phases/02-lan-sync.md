# 阶段 2：Windows + Android 局域网双向同步

状态：接口与实施计划，未开始联网实现。以用户本次阶段编号为准。前置门槛：阶段 1 两端聊天、系统密钥存储及升级验收；本次人格重构的 schema v3 测试通过。

## 数据与设备边界

Windows 网线与 Android Wi-Fi 连接同一路由器即可；发现与信任分离。Windows 可监听家中 LAN，手机可主动连接；Windows 关机后手机继续使用自己的 SQLite 和 DeepSeek Key。没有云数据库、公网入口或常在线中转。

`profile_id` 表示配对后共享的数据档案；`persona_id` 表示人格，二者不能混用。v3 默认人格 ID 等于原 profile ID，以保留旧事件归属；新建人格有独立 UUID。首次配对两个已有独立档案时，必须展示预检和导入映射，保留源档案命名空间，不能覆盖任一库或任意重写已签名事件。可选择导入备份成为同一档案；冲突人格由用户确认合并或分别保留。设备 ID、证书、私钥和本机最后选中的人格不复制。

## 模块合同（计划）

```ts
type Scope = { profileId: string; personaId: string };
interface DiscoveryProvider {
  discover(signal: AbortSignal): AsyncIterable<{ address: string; port: number; protocol: number }>;
}
interface PairingProvider {
  begin(): Promise<{ pairingId: string; expiresAt: number; qrPayload: string }>;
  confirm(pairingId: string, peerFingerprint: string, comparisonCode: string): Promise<void>;
  revoke(deviceId: string): Promise<void>;
}
interface SyncTransport {
  connect(trustedDeviceId: string, address: string, signal: AbortSignal): Promise<void>;
  exchange(request: { checkpoints: Record<string, number>; maxEvents: number }): Promise<unknown>;
}
interface SyncCoordinator {
  syncNow(deviceId: string): Promise<string>; // persisted task ID
  status(): Promise<{ lastSuccessAt: number | null; pending: number; errorCode?: string }>;
}
```

Windows 使用 mDNS、Android 使用 NSD 适配；协议服务类型拟定 `_deepseek-niang._tcp.local`，只广播随机实例 ID/端口/协议号，不广播昵称、Key、二维码 token 或聊天。提供手动 IP 和防火墙、同子网、访客网络隔离诊断。发现只提供地址，不能授予读取权限。

二维码携带短时会话 ID、临时高熵密钥材料/公钥指纹，TLS 通道内验证双方持有者；人工核对短码。短码不直接派生长期传输密钥。复用经评估的 TLS 1.3 双向认证、固定已配对证书/公钥；限速、过期、单次使用、防重放。配对私钥用系统安全存储，撤销立即拒绝连接。具体库版本与算法在实现前复核，不自造密码协议。

## 增量、冲突和删除

保留原有 `sync_events`、连续 origin_seq 检查点、Lamport 逻辑版本、墓碑和事务实验；当前 lab feature 的 SHA-256 只检查完整性，不能认证发送者，禁止直接接网络。

实施事件协议 v2：显式档案、人格、实体类型、实体 ID、操作、因果版本、来源设备序号、内容哈希与签名；版本协商后逐实体授权。新增人格创建/设定/归属映射、关系事件、生活事件与用量记录的同步投影。v1 原始信封保持不变，适配层将缺少 persona 的旧事件确定性映射到默认人格。新版本事件不交给旧 v1 解析器猜测处理。

- 消息追加合并、全局 UUID 去重；草稿/处理中状态需明确生命周期，不能把临时草稿当成完成回复。回复变体按原始请求/用户回合保留。
- 记忆修订使用逻辑版本与来源设备作为稳定比较键；删除墓碑优先且保留压缩水位，旧设备回连不得复活已删内容。冲突内容允许用户检查。
- 关系只同步原始证据事件，按相同规则版本重放；不把两端的分数相加。
- 人设字段/整版本的并发修改采用确定性合并，并保留冲突副本，禁止依赖设备墙钟唯一判定。
- 活动/主动消息采用稳定 occurrence/task ID、检查点、租约 fence 和幂等输出键。网络分区期间不能保证全局 exactly-once；需要协调权威的任务暂停或执行可安全补偿的步骤。
- 每批限制大小/事件数，验证后同一 SQLite 事务写事件和投影，再推进连续检查点；缺口重传、重复批次幂等、断电不跨越未提交数据。

## 数据库计划与验证

在新迁移中补充可信设备证书、人格事件映射、同步会话/失败诊断和完整性摘要。保留 v3 所有消息 ID 与历史会话映射。当前 `character_interests`、`life_events`、`task_queue` 的唯一键仍基于 profile；启用多人格生活前迁移为包含 persona 的唯一键或显式人格前缀。用量 request_key 全局唯一，不能二次累计。API Key、Cookie、设备限制、通知权限和配对私钥永不进入同步。

验收至少覆盖：网线 PC + Wi-Fi 手机双向新增、并发写入、重复/乱序/缺失批次、篡改/重放/陌生证书/撤销、记忆删除和旧版本离线回连、导入两个独立档案、人设冲突、两个不同人格隔离、PC 关闭后手机聊天和补传。检查点与合并摘要需一致；失败可用脱敏错误码诊断。

Android 前台实时、网络恢复重连、系统允许的后台工作和打开后补传；强制停止后不能保证即时通信。未完成真机测试与安全传输前，不勾选本阶段完成。
