# 阶段 7：语音、固定音色与歌声

## 在 v0.0.8 试用

1. 打开主客户端的「设置 → 语音与麦克风」。选择离线朗读音色、语速、音量和默认静音，再保存语音设置。偏好关联本机与当前人格。
2. 点击「试听朗读」或「试听原创元音唱段」。聊天中带正文的助手回复也有「朗读」按钮。长文按句分段，每次最多朗读前 1680 字。
3. 在聊天中点击「语音输入」，允许应用访问麦克风，讲话后点击「完成录音」。识别后的文字追加到当前草稿，由你编辑和发送。
4. 播放时可暂停、继续、静音或停止。切换人格或页面时结束当前操作。Windows 桌宠在播放有声内容时显示随音量变化的指示。

Windows 朗读通过系统安装的 SpeechSynthesizer 语音包生成。转写需要 `PATH` 上已安装的 `whisper-cli` 和本机 Whisper 多语种 `.bin` 模型。在「语音与麦克风」或「视频中心」选择模型，选择在本次运行中有效。麦克风录音上限为 30 秒。[Windows SpeechSynthesizer](https://learn.microsoft.com/uwp/api/windows.media.speechsynthesis.speechsynthesizer)

Android 朗读通过系统 `TextToSpeech` 提供的离线音色生成。语音输入需要 Android 12 或更新版本及系统支持的设备端普通话识别。音频焦点被其他应用接管时，当前播放停止。应用退到后台时，录音和播放结束。[Android TextToSpeech](https://developer.android.com/reference/android/speech/tts/TextToSpeech) · [Android SpeechRecognizer](https://developer.android.com/reference/android/speech/SpeechRecognizer)

应用处理录音、WAV 和播放音量时使用内存或本机临时文件。识别和朗读结束会清理临时文件。用户发送草稿后，文本按聊天设置发给 DeepSeek。现有 SQLite 中只新增设备范围的语音偏好记录，schema 保持 9。

原创唱段是 3.8 秒的七音符元音合成。保存的系统音色用于朗读。带歌词的自然歌声、统一朗读与歌唱音色、授权音色和伴奏分轨仍按下方计划评估。

## 验收

- 双端核对系统可选音色、切换人格后分别保存偏好，以及重新打开应用后恢复。试用过程中可让设备离线。
- Windows 检查朗读、暂停、静音、桌宠发声提示及麦克风权限。用本机 Whisper 测试短句、取消、错误模型和 30 秒限制。
- Android 验证拒绝授权、授予授权、草稿确认、切后台结束、打断音频焦点。没有设备端普通话时检查中文提示。
- 试听唱段七个音符和停顿，比较系统语音清晰度与情绪调整。自动化测试覆盖音高、时序和浏览器播放流程，耳机、麦克风和授权参考音色留给实机验收。

## 后续接口计划

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
