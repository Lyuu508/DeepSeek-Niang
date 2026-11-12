# 阶段 7：语音、固定音色与歌声

状态：独立接口计划，没有安装候选模型、复制真人声音、调用新增付费服务或实现录音权限。

## 四类可替换 Provider

```ts
type VoiceScope = { profileId: string; personaId: string; voiceId: string };
interface SpeechProvider {
  synthesize(input: { scope: VoiceScope; text: string; emotion?: string; speed: number }, signal: AbortSignal): AsyncIterable<{ pcm: Uint8Array; sampleRate: number; visemes?: unknown[] }>;
  transcribe?(audio: Uint8Array, signal: AbortSignal): Promise<{ text: string; confidence?: number }>;
}
interface SingingProvider {
  render(input: { scope: VoiceScope; lyrics: string; notes: { pitchMidi: number; startMs: number; durationMs: number; syllable: string }[]; tempo: number }, signal: AbortSignal): Promise<{ vocalPath: string; alignment: unknown[] }>;
}
interface MusicGenerationProvider {
  compose(input: { scope: VoiceScope; style: string; lengthMs: number; tempo?: number; lyrics?: string }, signal: AbortSignal): Promise<{ accompanimentPath: string; melody?: unknown[]; rights: string }>;
}
interface VoiceConversionProvider {
  convert(input: { scope: VoiceScope; sourceAudioPath: string; authorizedReferenceId: string }, signal: AbortSignal): Promise<{ audioPath: string; modelVersion: string }>;
}
```

STT 也可分为独立 RecognitionProvider，不强制所有 TTS 后端具备识别。普通说话、音符级歌声、伴奏/旋律生成、音色转换分开建任务和预算。TTS 拉长或升降音高不能冒充音符级唱歌；同角色使用同一 voice profile 的说话/歌声适配，但不同模型无法保证绝对同音色，须试听验收。

## 候选与授权

研究 Qwen3-TTS、CosyVoice（日常语音/授权参考），DiffSinger（音符级歌声），ACE-Step（音乐生成），RVC（音色转换）。实现前确认实际版本、权重/代码/商用许可、数据集/音色许可、流式能力、中文韵律、硬件、模型大小和依赖安全；名字出现在计划不代表已验证可用。不自动安装大型模型或新程序，不调用未配置的付费 API。

声音素材登记权利人、授权范围/证明、期限、来源哈希、可否克隆和可否分发。未经允许不复制真人音色。撤销授权使对应 profile 不可再生成，清理受控缓存并保留必要的授权审计元数据；用户原始录音属于私密数据，不进入代码/同步或公开模型包。

## 运行、成本与设备

优先本地 Windows/GPU 生成；手机可按能力选用已有本地轻模型或用户单独配置的 Provider。NAS 将来作为同一任务执行节点，无需改变人物身份。每种 Provider 公开本地/云端、模型校验和、硬件、费用估计、最长输入和支持音色；云端传输录音/文本前显示范围，默认关闭。

支持指定音色、克隆参考、情绪/语速、流式播放、音量/静音、取消和切换设备。音频焦点/蓝牙路由/电话中断、录音权限拒绝和后台限制使用平台合法能力。音频队列按人格和任务 ID 隔离，切换人格明确停止或继续播放来源提示，不将 A 的声音记为 B 的活动。

真实 PCM/对齐时间轴驱动桌宠口型；暂停/停止播放即暂停嘴型，唱歌按音符/音素对齐。长音频任务缓存包含文本/歌词/音符、音色、模型/参数版本和授权引用，防止重复付费；设时长、并发、每日费用和本地资源上限，不能将 unknown 费用当0。

## 数据与验收

未来迁移新增 voice_profiles、voice_authorizations、speech_tasks、singing_scores、audio_results/usage 分类，统一 profile/persona/task 归属、检查点和来源；密钥仍在系统存储，不放 voice profile JSON。客观音频路径为节点本地路径，LAN 同步只传允许的引用/摘要，不接受远程任意文件路径。

验收：有许可参考音色的中文对话/情绪/流式、静音/停止、手机权限拒绝、焦点中断、口型跟随真实播放；以明确歌词+音符乐谱验证音高、节奏、音节对齐及说话/歌声音色一致性；原创歌曲的伴奏/旋律与歌声分轨可检查。记录硬件、延迟、资源和真实费用，未经试听及授权检查不标完成。
