# 阶段 8：可选 NAS 常驻节点

状态：迁移计划。现在不购买/部署 NAS；Windows 和 Android 仍各自独立聊天与本地持久化。现有 companion-node 是无 UI 实验入口，尚不是生产同步服务器。

## 复用现有模块

NAS 运行同一 Rust 核心、SQLite、人格/关系/记忆处理、生活/主动调度、视频/音频 Providers 和同步服务；Tauri UI、Credential Manager/Android Keystore 不进入 NAS 服务。系统密钥通过 SecretStore 的 Linux/服务安全后端适配，需当时单独验证，不提供明文普通表回退。

```ts
interface ExecutionNode {
  describe(): Promise<{ nodeId: string; kind: 'windows' | 'android' | 'nas'; capabilities: string[]; models: unknown[] }>;
  health(): Promise<{ available: boolean; updatedAt: number; resources: unknown }>;
}
interface AuthorityTransfer {
  prepare(targetNodeId: string, expectedEpoch: number): Promise<{ transferId: string; checkpointDigest: string }>;
  commit(transferId: string, verifiedDigest: string): Promise<{ authorityEpoch: number }>;
  rollback(transferId: string): Promise<void>;
}
```

选择部署方式取决于实际 x86/ARM、Linux、容器和 GPU/NPU。Docker 只是可选进程管理，不作为当下依赖。不在客户端硬编码 NAS IP、模型路径或 Windows 专属媒体处理命令；Providers 和 TaskRunner 根据节点能力选择。

## 迁移步骤

1. 在源节点停止领取可产生外部副作用的新任务，完成或暂停已有任务并保存检查点；导出 SQLite 一致性快照、迁移版本、人格/消息/事件/任务摘要与必要配置。
2. 对独立历史备份加密、校验并保留。NAS 在临时目录验证版本、文件大小、外键、完整性、行数和哈希，再原子导入；不直接共享/挂载一个活动 WAL SQLite 供多设备写。
3. NAS 创建自己的设备身份并通过阶段 2 的配对验证；导入业务数据，不能复制源设备私钥、local device ID、Key、通知权限或本机最后选中人格。需要重置 local-only 设备字段的迁移适配和事件水位预检。
4. 配对 Windows/Android，双向补齐事件。保留 profile、persona、消息/记忆/关系事件 UUID 和逻辑版本；不能重建一个空角色或把分数置零。
5. 通过 authorityEpoch 和 fence 原子交接任务权威；旧节点领取被拒绝，未完成任务标 uncertain 并根据媒体/音频片段输出键核对后恢复。单个“NAS优先”布尔标记不能防重复。
6. 验证摘要一致、常驻任务、客户端断线回退和恢复后补传；再选择 NAS 为首选执行节点。旧数据备份留存到验收完成。

迁移必须可中止和回滚：失败不覆盖唯一备份，旧节点恢复时推进权威 epoch，不能恢复过期租约继续提交。若原权威无法联系，采用显式安全接管/等待，而非网络分区两端自选自己为唯一权威。

## 离线与权限

NAS 关闭时 Windows/Android 继续本地聊天，使用各设备自己的 DeepSeek Key；常驻任务按可验证的租约规则回退或显示待恢复。NAS 快照超时就显示最后同步时间，手机无法连到家中 LAN 时不假装实时；当前方案不承诺公网远程访问。

服务仅监听经配置的 LAN 地址，配对、mTLS、撤销与事件校验沿用阶段 2，无裸数据库端口。容器/服务使用普通账户、最小挂载、只读模型目录和受限媒体缓存；不挂整个用户 HOME，不配置陌生远程脚本。模型安装/新费用/大磁盘使用仍需用户确认。

## 出口验收

真实 NAS/迷你主机执行断电/重启、导入失败、备份恢复/回滚、任务权威交接、旧节点迟到提交、网络分区、双端离线聊天和重连补传；人格/历史/关系/记忆/视频/任务记录摘要一致。记录硬件和模型能力，弱硬件不勉强承担视频/歌声大模型。没有实际硬件与迁移测试时只交付计划，不标“无缝迁移已完成”。
