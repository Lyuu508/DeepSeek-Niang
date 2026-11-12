# 后续模块共用的 Provider 与成本合同

状态：设计合同，不是已上线运行的注册器。本次继续使用现有官方 DeepSeek 客户端，未切换服务商、下载模型或启用新增收费接口。阶段2～9的细节见各自独立文件。

## 共用请求边界

```ts
type RequestScope = { profileId: string; personaId: string; taskId: string; requestId: string };
interface ProviderDescriptor {
  id: string; kind: 'chat' | 'video' | 'speech' | 'singing' | 'music' | 'voice_conversion' | 'embedding';
  execution: 'local' | 'cloud'; modelId: string; revision: string;
  capabilities: string[]; maxInput: Record<string, number>;
  price: { currency: string; version: string; units: unknown } | null;
}
interface ProviderContext {
  scope: RequestScope; executorId: string; fence?: number;
  permissionSnapshotId: string; budgetReservationId?: string;
  allowedSourceIds: string[]; signal: AbortSignal;
}
interface ProviderRegistry {
  list(kind: ProviderDescriptor['kind']): Promise<ProviderDescriptor[]>;
  configure(providerId: string, enabled: boolean, credentialReference?: string): Promise<void>;
  resolve(kind: ProviderDescriptor['kind'], context: ProviderContext): Promise<unknown>;
}
interface ChatProvider {
  describe(): ProviderDescriptor;
  stream(context: ProviderContext, prompt: { role: string; content: string }[]): AsyncIterable<{
    textDelta?: string; usage?: { input: number; output: number }; finished?: boolean; errorCode?: string;
  }>;
}
```

注册器只返回已配置、获授权且本节点有能力运行的适配器，不按“云端报错”静默换供应商或发送更多私密数据。DeepSeek负责人格对话；视觉/语音工具只是能力，人物身份不随工具变化。所有新网络目的地使用显式许可/固定HTTPS白名单，不接受网页/模型输出提供的任意URL作为执行命令。

## 存储、权限与预算

未来以新迁移保存 provider配置（非秘密）、模型/许可/校验和、版本、节点能力、模块启用状态、权限快照、请求/任务审计和 api_usage_records 的persona/模块分类。credentialReference 指向操作系统/服务安全存储；Key、Cookie、录音授权原件和节点模型绝对路径不进入人格同步事件。

调用前验证 profile/persona/source 授权、实际能力、输入上限与取消信号；需要任务独占时核对fence。云端调用先预留最坏可配置成本，再根据实际返回量结算。价格/Token未知时标unknown，不能记为0或绕过硬预算；本地模型也有限制GPU/CPU/内存/温度/时长/磁盘的配额。

按人格、日期（含用户时区）、设备、模块统计Token和费用，价格保留币种/快照版本；只记Provider/模型、请求ID、耗时、用量、脱敏错误，不记录正文、音频、认证头或密钥。关闭模块后不创建新的模型请求，免打扰只影响通知时不能误当成已关闭生成。

## 生命周期和验收

每个适配器支持取消、明确超时、受限重试、可诊断失败；有外部副作用的操作不自动重试，输出按task/request/source幂等键提交。缓存键包括输入内容指纹、模型/配置/提示模板版本和授权范围，不能跨人格复用角色感想或泄漏上下文。

Provider切换需重新验证能力/许可/格式，保留旧结果的原模型元数据，不伪造同等质量。大型模型安装、新第三方程序、新付费API先由用户确认；当前计划列出候选不等于安装授权。验收包含禁用后零调用、敏感范围限制、网络目的地/重定向拒绝、预算并发预留/未知量/失败结算、缓存失效、跨人格隔离及任务迟到结果拒绝。
