# 阶段 6：B站浏览与完整视频理解

## v0.0.7 · 本地文件分析

Windows 主客户端提供「视频中心」。程序从本机文件逐段提取真实画面和音轨，使用本机 Ollama 的 Qwen3-VL 识别抽样画面、使用本机 whisper-cli 转写语音，按实际覆盖保存分段结果、合并摘要和时间戳。桌宠和聊天上下文可读取真实执行阶段。聊天上下文中的活动状态只包含阶段和覆盖数字。

### 准备和使用

1. 让本机 PATH 能找到 **FFmpeg 和 ffprobe**，启动本机 **Ollama**，准备应用所选的 **Qwen3-VL 2B、4B 或 8B**。导入时检查本机模型列表与图像能力，并记录模型摘要和工具版本。[FFmpeg 下载](https://ffmpeg.org/download.html) · [Ollama 下载](https://ollama.com/download) · [Qwen3-VL 模型](https://ollama.com/library/qwen3-vl)
2. 带音轨的视频需要 PATH 中的 **whisper-cli**。在视频中心选择已有的 Whisper 多语种 GGML `.bin`，例如 `ggml-base.bin`。无音轨的视频只使用画面模型。[whisper.cpp 使用说明](https://github.com/ggml-org/whisper.cpp/tree/v1.8.4)
3. 打开 Windows 客户端侧栏「视频中心」，确认有权分析视频，再选择 MP4、MOV、M4V、MKV 或 WebM。导入完成后点「开始分析」。视频中心会显示采样、转写、整理和失败原因。
4. 暂停时程序保留已写入 SQLite 的片段。恢复前核对原文件和模型；原文件仍需可读。可打开时间线查看真实处理的片段、不确定项、关键时间点和转写。
5. 在完成后点击「保存到长期记忆」，摘要写到当前人格。摘要默认不加入 DeepSeek 对话请求，已设置的局域网记忆同步可传递这条摘要。Android 可查看该记忆，Windows 执行视频分析。

### 当前限制与数据

- 每个文件最多 256 MiB 和 5 分钟。滚动 24 小时最多导入 3 部、总时长 10 分钟，同时执行 1 个任务。原始媒体进入应用工作目录的临时副本会校验摘要。正常停止时释放临时文件，下次启动清理上次中断留下的工作目录。
- 按 8 秒规划片段并重叠 0.5 秒。基础采样 1 帧/秒，较明显的场景变化可从 2 帧/秒候选中补帧。进度取样本覆盖区间的并集。画面精度和可能遗漏的快速动作取决于采样及模型。
- 音轨按时间段解码为 16 kHz 单声道后转写。可识别的语音带时间戳。音乐、音效和其它非语音声音的语义识别尚未接入。播放状态保持「未播放」。
- schema 9 保留旧表和旧数据。新增 `video_local_media` 保存本机设备绑定、路径、大小和音轨信息。分段结果与检查点在同一事务提交，换人格时结果隔离，客观片段按文件、区间、工具及模型指纹复用。
- Ollama 仅使用本机回环地址。程序使用已安装的工具和模型。用户素材未通过程序上传到远程模型服务。B站媒体直连、自动发现、收藏及评论接入仍待合法授权接口。[哔哩哔哩开放平台](https://open.bilibili.com/doc/)

### 验收范围

使用 FFmpeg 生成的 16 秒真实编码文件验证首尾不同画面、全时间线分段、音轨 PCM 读取和原文件变化校验。用回环 HTTP 夹具验证 Ollama 图像能力查询、输入图像和时间戳解析。SQLite 测试验证暂停与恢复、租约拒绝过期写入、缺失音频不标满进度和显式保存记忆。本机实际 Qwen3-VL 推理、Whisper 识别效果及用户 Windows 设备验收仍待测试。

## 后续接入设计

## Provider 与默认本地处理

```ts
interface VideoUnderstandingProvider {
  capabilities(): Promise<{ local: boolean; video: boolean; audio: boolean; maxSegmentMs: number; sampling: string; modelId: string; revision: string }>;
  analyze(segment: { taskId: string; personaId: string; mediaHash: string; startMs: number; endMs: number; frames: unknown[]; audioPath?: string; captions?: unknown[] }, signal: AbortSignal): Promise<{
    visualCoverage: [number, number][]; audioCoverage: [number, number][];
    events: unknown[]; summary: string; uncertainties: unknown[]; modelUsage?: unknown;
  }>;
}
interface MediaSourceProvider {
  discover(query: string, signal: AbortSignal): Promise<unknown[]>;
  authorize(sourceId: string): Promise<{ permitted: boolean; mediaReference?: string; restriction?: string }>;
}
```

先实现用户合法提供的文件，以真实授权视频验收；再接 B站经许可的搜索/推荐/元数据和完整媒体来源。无完整媒体访问权限则返回 restricted，仍可展示视频链接，不声称看完。播放器打开网页、解码媒体、取得字幕、AI 理解是不同状态。收藏等账号操作需明确授权并使用允许的接口；评论只生成草稿，确认特定草稿后才可发布，接口不支持则复制草稿并打开视频页面。不绕验证码、DRM、反爬、登录和风控。

默认链路为 FFmpeg/ffprobe 探测与解码、OpenCV 场景变化/关键帧、Faster-Whisper 等本地语音识别、本地视觉模型（候选 Qwen3-VL）和时间对齐融合。**本地 Qwen 模型不是千问云 API**。DeepSeek 保持人格对话大脑；本地分析结果仅在用户允许范围内用于总结和感想。可选云多模态 Provider 默认关闭，需核对当时实际输入能力、时长/帧/音频/文件限制、价格/延迟并经配置授权。

模型名称只是研究候选，不表示兼容性已验证。实施前在目标 Windows/GPU/Android/NAS 硬件核对模型许可证、权重大小、量化/显存/内存、运行库和实际音视频能力；不得未经用户确认下载大型权重或安装新的第三方程序。Android 只承担能力允许的轻任务，不假定所有手机能持续运行长视频模型。

## 全时间线流程

1. 校验合法来源、文件类型/大小/哈希、真实时长、音视频轨及缓存命中。解码器作为受限子进程，固定参数、禁止把媒体内容当 shell 参数代码，限制文件/网络协议和资源。
2. 建立 [0,duration) 的分段清单、可恢复检查点及模型/采样配置版本；长视频分段有少量重叠，合并时用时间戳去重。
3. 全时间线按顺序分析视觉与音频；视觉分段持续采样、场景变化/快速动作加密；音频连续切片识别并对齐字幕/合法弹幕。不能仅处理开头或封面。
4. 关联人物、动作、剧情、台词、笑点与事件时间点，记录冲突/无法解码/无法识别区间。字幕是辅助轨，不代替视觉和音频覆盖。
5. 合并分段结果、跨段人物/剧情线及不确定性；只根据真实结果生成记忆、兴趣变化和可分享候选，附来源/UP 主/链接/时间戳。

区分“计划覆盖”“已处理视觉区间”“音频识别区间”和“理解可信度”。抽帧处理一个区间不表示逐帧理解；记录 FPS/采样时间和遗漏风险。完整任务要求没有未处理计划区间；仍须显示实际无法理解的片段，不能以100%调度完成掩盖内容失败。

## 双轨状态与资源

video_progress 分别保存播放位置、解码/预处理进度、视觉分析覆盖、音频分析覆盖、合并完成状态；播放100%不推出分析100%。UI 状态至少含播放、解码、分析、暂停、整理、完成、失败、受限，显示执行设备、更新时间、真实覆盖和缺口。没有播放器运行则播放进度为未知，不假造视频正在播放。

缓存键包含媒体哈希、分段区间、视觉/音频模型版本、预处理/采样配置和提示模板；同文件跨人格可以复用客观分析缓存，但感想、兴趣和记忆属于各人格，权限仍隔离。缓存失效/删除不破坏来源元数据；限制磁盘和保留天数。

默认并发1，设置每日视频数量/总时长/单视频时长、CPU/GPU内存、磁盘、手机Wi-Fi/电量/温度门槛；在线提供商有独立每日预算和任务预留上限，未知价格先停止而不是无限调用。支持暂停/恢复/取消和用户优先级。租约 fence 保证旧节点不能提交新结果；不确定任务按片段缓存对账。

## 数据与出口验收

以新迁移扩展 video_tasks、sources、progress、timeline_events、analysis_results、video_memories 的 persona 归属和覆盖区间、模型版本、不确定性、媒体授权状态与缓存引用；不得删除现有表。来源/客观缓存和角色化结果明确分开。stage2 同步只传允许的元数据/结果，原媒体默认不全量同步。

真实测试至少有：前后半段不同剧情、纯音乐/无字幕、快动作、长于模型限制、破损中段、无音轨、重复文件、暂停退出恢复、多人格复用客观结果但记忆不串、无权限来源。逐段人工核对画面/语音对齐、首尾QA、时间戳和缺口；报告实际采样、覆盖、耗时、硬件、资源和费用。未获合法B站媒体或模型不能处理时保持受限/失败，不勾选完整理解。
