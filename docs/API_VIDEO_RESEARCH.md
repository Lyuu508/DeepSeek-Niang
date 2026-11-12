# 官方接口研究与验证边界

核对日期：2026-11-12。当前没有用户 API Key、授权测试视频或真机，因此下面是**官方文档能力核对**；不是供应商可用性、质量、费用或延迟的实际测试报告。

## DeepSeek 文本接入

官方基地址为 `https://api.deepseek.com`，文本流式接口 `POST /chat/completions`；模型 ID 从设置读取。当前官方入门示例使用 `deepseek-flash`，同时列出 `deepseek-v4-pro`，所以不把早期 `deepseek-chat` 名称硬编码为永远可用。
请求包含 messages、model、stream；使用 SSE 按事件边界和 UTF-8 增量解析，不能假定 HTTP 每个 chunk 恰好是一条消息。`data: [DONE]` 表示该流结束，usage 从供应商实际末尾统计读取。
工具调用支持情况与 thinking/reasoning 字段按具体模型和 API 版本验证；阶段 1 默认只接文本对话，随后才开放白名单工具。
来源：[官方入门](https://api-docs.deepseek.com/)、[Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/)、[工具调用](https://api-docs.deepseek.com/guides/tool_calls/)。

实现要求：

- Rust HTTPS 适配器持有原生安全存储中的 Key，React 不使用浏览器 SDK 携带 Key 请求模型。
- 固定官方 HTTPS 目的地；更换自定义提供商应作为显式配置，不能让模型输入改变 base URL 或把认证头跟随重定向发到其他主机。
- 持久化生成 ID、请求生命周期和分支；已发送但响应不确定时保留 unknown，用户决定重试，不承诺远端 exactly-once。
- 支持取消、连接/读取超时、429 退避；身份或余额错误明确提示；重试次数和累计时间受限。
- chat prompt 只选当前分支、允许的记忆和实际状态，不上传整个 SQLite DB。
- 用量缺失保存 null/unknown；成本以价格快照估算并标记 estimated，不把估算当账单。

阶段 0 仅有 provider/stream ports。阶段 1 已实现 companion-chat HTTPS/SSE 适配器，固定上述官方地址，stream_options.include_usage=true、thinking.type=disabled。已有本地 HTTP Mock 协议测试，仍未请求真实 DeepSeek，没有真实 API 费用。其余多模态和 B站研究继续作为后续阶段快照，不在阶段 1 接入。

## Android 安全存储和后台适配

Keystore 保存加密密钥，API Key 是被 AES-GCM 加密的应用私有密文；每次写入生成新的安全 nonce，封装由平台成熟实现提供。
恢复备份或重装后 Keystore 密钥可能不可用，提示用户重新输入 API Key；不把硬件不可导出身份私钥混进跨设备备份。
来源：[Android Keystore](https://developer.android.com/privacy-and-security/keystore)。
WorkManager 用于符合系统条件的可延迟补同步，不当作精确定时器或永不停止的 Agent。
前台 APP 会话负责实时同步；系统停止后暂停真实任务，重开恢复。权限、通知与电池条件逐项判断。
来源：[WorkManager 官方指南](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started)。

## 多模态模型候选快照

| 候选 | 官方文档视频/音频限制摘要 | 当前决定 |
|---|---|---|
| Qwen3.8-Omni-Flash | 视频最长 2 小时、URL 文件上限 2 GB；音频最长 3 小时；视频采样受帧数/像素限制 | 优先做阶段 6 实验，未实测 |
| Qwen3-Omni-Flash | 视频最长 150 秒、URL 文件上限 256 MB；音频最长 20 分钟；图像序列 2–128 帧 | 短段备选，不能按音频时长推断视频时长 |

文档中的 Base64 音视频编码字符串要求小于 10 MB；本地输入必须根据实际字节、token、帧和时长共同决定段长。较新系列支持更大帧数，示例默认采样和上限不意味着每帧都会进入模型。
来源：[Qwen-Omni 官方能力及限制](https://www.alibabacloud.com/help/en/model-studio/qwen-omni)。

价格快照示例：官方价表当前列 `qwen3.8-omni-flash` 北京区域普通输入/文本输出为每百万 token **US$0.113 / US$0.382**；新加坡为 **US$0.15 / US$0.47**，缓存价另列。区域、工作空间、模式和实际账单都需在接入时复核。
不能从视频分钟数直接承诺固定费用；采样、分辨率、音频、输出长度和重试都会改变 token 用量。
来源：[官方价表](https://www.alibabacloud.com/help/en/model-studio/model-pricing)。

当前尚未测量首包延迟、全片时延或内容质量。阶段 6 必须记录 region、模型实际版本、输入编码、采样策略、请求字节数、分段范围、总 token、真实耗时与错误率。
API 文档可变，因此 adapter 使用可版本化 capability snapshot；不能把本报告参数写成永久常量。上线前用真实授权文件验证音轨是否确实被模型消费。

## 完整理解实验设计

1. 使用用户拍摄/自制或明确授权的视频，至少有前、中、后不同事件；有快速动作、静音段、字幕和尾声。
2. probe 时长/时间基/音轨；全程解码验证，记录 corrupt/missing 范围；不让缺帧自动变成“已看懂”。
3. 在模型限制内切段并保留少量重叠；按实际文件字节和像素/token 限制进一步拆段，不只按最大时长切。
4. 单段返回可验证时间戳、画面证据、听到的对话/声音与不确定内容；字幕/弹幕只作辅助来源。
5. 合并按原片时间基去重，保存两轨实际成功处理的区间和采样清单；聚合摘要有来源回指。
6. 询问前半段与后半段的独立问题，核对正确事件、人物、音频与时间。覆盖达标与回答准确率分别报告。
7. 中断、暂停、续传、同一段重试、预算到限、外部超时结果不明都做故障测试。

当前 `video.rs` 只验证分段计划和区间覆盖。测试中的毫秒区间是明确的算法夹具，不是真实视频、模型或播放结果。
静音/无音轨应由 probe 产生明确的“不适用”证据；不能伪造音频分析成功。模型无法处理的片段保留 gap/uncertain，不能以 100% 进度掩盖。

## B站接口核对与限制

已查看官方开放平台入口及文档索引，公开说明主要涉及稿件分发和授权数据能力；文档站当前为动态页面，读取不到完整接口正文。
本次**未取得能够授权此应用访问任意完整媒体的接口证据，也没有开发者应用和账户授权**。这不等于证明不存在任何合作接口，但不能据此实现或宣称已可完整观看 B站。
来源：[B站开放平台](https://open.bilibili.com/)、[官方文档入口](https://open.bilibili.com/doc/)。

阶段 7 先核对实际开发者权限与条款；分别验证视频发现、元数据、完整媒体、字幕/弹幕、收藏或评论的授权范围。
完整媒体不可得时只做可授权元数据、链接和评论草稿；打开页面不是绕过媒体授权的理由。
不采用逆向非公开接口、登录 Cookie 抓取、DRM 绕过、验证码代过或反爬规避。公开评论必须用户确认且接口允许，否则只提供草稿和打开页面。

## 用户后续需要提供的内容

阶段 1：在本机安全设置页输入 DeepSeek Key，以及 Windows/Android 环境检测结果和真机运行条件。
阶段 6：有合法处理权的测试视频、多模态提供商凭据、发送范围与金额预算。
阶段 3：若用户已有授权角色文件，可提供素材和许可信息；这不阻塞当前架构和核心开发。
当前不需要购买 NAS，不需要云数据库，也不需要把 Key 发到聊天里。
