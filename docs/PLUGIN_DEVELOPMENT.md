# v0.0.9 声明式插件

在「插件管理」选择本地 JSON 清单或应用内示例。预览显示来源、作者、许可证、版本、平台、SHA-256 与所需权限。安装或更新后先授权，再选择全局或当前人格的启用状态。可以撤销任一权限。人格配置可继承本机全局配置，也可单独设置。插件清单、权限、审计、设置和提醒存于本机。

## Manifest API 1

| 字段 | 规则 |
| --- | --- |
| id | 小写字母起始，最多 64 个 ASCII 字符。可用数字、点、下划线和连字符 |
| name、description、author、license | 必填纯文本 |
| version | 三段数字，从较小版本更新到较大版本 |
| entry | 带 kind 的固定声明式入口 |
| supportedPlatforms | windows 或 android，列表可同时包含两项 |
| permissions | 精确填写入口要求的能力清单 |
| configSchema | 最多 12 项，包含 key、label、kind、default、choices |
| requiredApiVersion | 1 |
| checksum | 去掉 checksum 字段的 JSON 经 RFC 8785 规范化后计算小写十六进制 SHA-256 |

完整清单限 16 KiB。配置支持 text、number、toggle 和 choice。number 接受 0–1440 的整数。choice 的可选项由 choices 提供，其他类型的 choices 为空数组。SHA-256 用于核对清单完整性，作者与许可证由清单提供。

## 入口与能力

| entry.kind | permissions | 运行范围 |
| --- | --- | --- |
| chat_style | chat_style | preset 可选 warm、brief、playful、story。可配置布尔 concise。为当前人格新回复添加已审查的表达偏好 |
| time_reminder | local_clock、local_reminder | 本机时钟和当前人格的到期提醒。布尔 hour12 和整数 minutes 可设置显示格式和默认时间 |
| memory_note | memory_write | 将确认提交的纯文本保存到当前人格。可配置 prefix |
| info_card | display_card | 在管理页显示 entry.body 的文本 |
| media_note | media_note、memory_write | 用户填写媒体名称、时间点和备注后可保存，后续媒体提供者可在明确能力和接口下扩展 |

运行器是原生的固定解释器，页面以文本渲染内容。可以申领的能力只有上表六项。清单格式拒绝脚本、可执行路径和额外权限。本机时间提醒在客户端运行并选中该人格时检查。恢复该人格或重新启动后，已到期的提醒可以继续显示。

重复安装相同或旧版本会被拒绝。更新保留插件 ID 并重新确认权限和配置。顶层状态和最近操作来源于本机执行结果。审计记录版本校验值、操作、时间和必要的人格关联，不含用户消息、提示词或提醒内容。

两个应用内示例也可以作为 JSON 文件导入。[简洁聊天风格](../apps/companion/src/plugins/examples/chat-style.json) · [本地时间与提醒](../apps/companion/src/plugins/examples/time-reminder.json)

## 自由对话参考核查

dsh-jailbreak 的 package.json 声明 AGPL-3.0-only，运行要求为 Node.js 20 及以上，入口面向 DeepSeek Harness。项目审查仅使用仓库元信息、许可证和入口的宿主接口。客户端独立实现了模式开关、受限风格层和审计记录。自定义模式将用户的风格偏好加入系统上下文，保留原有聊天正文、流式传输、模型接口和使用记录。[参考项目](https://github.com/xqpsec/dsh-jailbreak)
