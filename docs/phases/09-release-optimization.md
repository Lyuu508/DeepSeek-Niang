# 阶段 9：安装包、备份与最终优化

状态：发布准备计划。v0.1.0-dev.1 旧 EXE/APK 已生成但未实机运行；本次 dev.2 的构建状态单独见 PERSONA_BRANDING_REPORT，不能借用旧包结果。

## 发布合同与版本边界

```ts
interface BackupService {
  create(input: { destination: string; encryptionRecipient: string }): Promise<{ version: number; digest: string }>;
  inspect(archivePath: string): Promise<{ compatible: boolean; warnings: string[] }>;
  restore(archivePath: string): Promise<void>;
}
interface BudgetService {
  reserve(input: { personaId: string; module: string; maxCost: string; currency: string }): Promise<string>;
  settle(reservationId: string, actualUsage: unknown): Promise<void>;
}
interface UpdateVerifier {
  inspect(manifest: unknown): Promise<{ version: string; signatureValid: boolean; compatibleSchema: boolean }>;
}
```

SemVer 区分本地开发和 GitHub 正式版本。只有用户明确“更新到 GitHub”或等效授权后执行对应远程操作，提交不自动扩大为 Release。自动更新目前只设计，不配置未经批准的发布工作流、公共更新源或自动上传安装包。

安装、升级和运行分别记录。Windows EXE/MSI 的生成须有实际产物与校验和；Windows 10/11 实机安装、关闭/重启、卸载、升级和 Credential Manager 自检另验。Android APK 要校验 manifest/applicationId、ABI、min/target SDK、签名与包内资源，再在真实设备安装、重启、升级、Keystore 和键盘交互验收。未完成的平台不勾选，通过 Linux 编译不能冒充 Windows 实机结果。

## 品牌、身份和包签名

正式名称始终 DeepSeek娘，原始海滩 PNG 和派生资源哈希可核查；Windows 多 DPI/任务栏/标题栏/安装器/快捷方式，Android 圆形/圆角方形 launcher、adaptive、启动画面、通知图标必须有截图。原图缺失时打包门槛明确失败。

固定 `io.local.deepseekcompanion`、Android applicationId、原生库标识和数据目录，不为改名而迁移。Android 沿用原签名，丢失签名可能不能覆盖安装；私钥和密码不可加入源码 ZIP/GitHub。Windows productName 改名可能影响 NSIS 安装目录/卸载条目，实际升级必须检测旧版、验证同一用户数据仍被使用，不能仅凭 bundle identifier 推断安装器升级兼容。安装器签名须使用用户授权证书，不伪造已签名状态。

## 数据、费用和隐私

历史备份使用 SQLite Online Backup，校验后采用成熟加密格式/维护库，原子写入、版本目录和保留策略。恢复先临时验证迁移版本、外键、完整性、行数与哈希，再保留恢复前副本。同步不能代替历史备份；误删可能被同步传播。备份可能含聊天原文，不能上传源码仓库。

多人格永久时间线增加分页/虚拟列表、最近上下文索引和受限检索；一万/十万消息的启动、滚动、内存和数据库体积实测。UI 改进不删除原记录；上下文只发送预算内当前人格历史/允许记忆，Provider 日志只含模型、模块、耗时、错误码和实际用量，不含正文/Key。

Token/费用按人格、设备、模块、日期统计，记录价格快照/币种/缺失值；当前只有 Token 统计，不能把未知费用显示为免费。每日预算先预留、后结算，失败和未知用量保守处理；视频/语音/主动任务在预算不足时暂停，手动聊天的提示与设置清楚。支持禁用任何非必要 Provider。

卸载明确保留/删除本机数据选项，不能自动删除其他设备副本或独立备份；彻底清理必须覆盖事件原文、FTS、向量、缓存及密钥，并解释跨设备已同步/备份的保留范围。

## 出口验收矩阵

- 自动化：阶段 0 原有合并/去重/墓碑/租约/关系/视频测试保留，新增迁移、隔离、Provider、预算、签名/篡改、UI 和恢复测试通过；锁文件/依赖/安装脚本重新审查。
- 同步：断线、并发、删除、损坏/篡改、旧版本回连、权限撤销和任务重复防护在实机验证。
- 关系/记忆/生活：证据、去重、规则一致、检索权限、过期状态、停止进程不伪造联网、主动开关/免打扰有效。
- 视频/语音：真实授权素材的全时间线覆盖/缺口、缓存与恢复、固定音色/音符级唱歌，报告模型/硬件/延迟/资源/费用。
- 平台：Windows 和 Android 安装/覆盖升级/卸载/重启恢复、系统密钥读写、真实 DeepSeek 流式/停止/多轮/错误；Key 由用户在本机输入，不发到聊天。
- 交付：实际安装文件、源码 ZIP、SHA-256、许可证/素材清单、操作说明、备份/NAS 指南和已知问题。当前工具无附件传输时使用用户明确授权的私有渠道，不能把内部路径冒充下载链接。

全部实测报告完成后才称正式验收；远程发布仍须单独明确授权。
