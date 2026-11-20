import { invoke } from '@tauri-apps/api/core';

export interface VoicePreferences {
  voice_id: string;
  speed_percent: number;
  volume_percent: number;
  muted: boolean;
  follow_mood: boolean;
}

export interface VoiceOption { id: string; name: string; language: string }
export interface Take { wav: string }
export interface Guide {
  audio: Take;
  notes: { midi: number; start_ms: number; duration_ms: number; vowel: string }[];
}

type RecordAction = { kind: 'check' | 'begin' | 'status' | 'finish' | 'cancel' }
  | { kind: 'transcribe'; pcm: string };

export const voiceAPI = {
  prefs: (personaId: string, update?: VoicePreferences) =>
    invoke<VoicePreferences>('voice_preferences', { personaId, update }),
  voices: () => invoke<VoiceOption[]>('voice_voices'),
  speak: (personaId: string, session: string, text: string) =>
    invoke<Take>('voice_speak', { personaId, session, text }),
  guide: (personaId: string) => invoke<Guide>('voice_guide', { personaId }),
  record: (personaId: string, session: string, action: RecordAction) =>
    invoke<string | null>('voice_record', { personaId, session, action }),
  focus: (session: string, kind: 'acquire' | 'status' | 'release') =>
    invoke<boolean>('voice_focus', { session, action: { kind } }),
  meter: (personaId: string, level: number) => invoke<void>('voice_meter', { personaId, level }),
  selectModel: () => invoke<string | null>('video_select_whisper'),
};

const errors: Record<string, string> = {
  VOICE_BUSY: '本机语音正在处理上一项请求，请稍后重试。',
  VOICE_CANCELLED: '语音操作已停止。',
  VOICE_FOCUS_LOST: '声音播放已停止，其他应用正在使用音频。',
  VOICE_INVALID_AUDIO: '本机语音引擎返回的音频无法播放。',
  VOICE_INVALID_SETTINGS: '语音设置的数值超出范围。',
  VOICE_MODEL_REQUIRED: '请在语音设置中选择本地 Whisper 模型。',
  VOICE_NO_OFFLINE_VOICE: '系统没有可用的离线音色。请在系统语音设置中添加。',
  VOICE_NO_ON_DEVICE_RECOGNIZER: '语音输入需要 Android 12 或更新版本及系统提供的离线识别。',
  VOICE_NO_SPEECH: '没有识别到文字，请重试。',
  VOICE_PERMISSION_DENIED: '请在系统设置中允许应用使用麦克风。',
  VOICE_PERSONA_CHANGED: '已切换人格。本次语音已结束。',
  VOICE_RECORDING_LENGTH: '请至少说半秒，单次录音最长 30 秒。',
  VOICE_RECOGNITION_FAILED: '本机转写失败，请检查语音资源后重试。',
  VOICE_RECOGNIZER_LANGUAGE: '设备尚未提供普通话离线识别，请检查系统语音语言包。',
  VOICE_STORAGE: '读取或保存本机语音设置时出错。',
  VOICE_SYNTHESIS_FAILED: '系统未能生成音频，请检查本机音色后重试。',
  VOICE_TEXT_LENGTH: '这段文字无法朗读。',
  VOICE_TIMEOUT: '本机语音处理超时，请重试。',
  VOICE_UNAVAILABLE: '当前设备的语音服务不可用。',
  VOICE_VOICE_REMOVED: '已选的音色在本机不可用，请到语音设置中重新选择。',
  VOICE_WHISPER_MISSING: '请配置本机 whisper-cli，并让应用可通过 PATH 找到它。',
};

export function voiceError(error: unknown): string {
  if (error instanceof DOMException && ['NotAllowedError', 'SecurityError'].includes(error.name)) {
    return errors.VOICE_PERMISSION_DENIED;
  }
  if (error instanceof DOMException && ['NotFoundError', 'NotReadableError'].includes(error.name)) {
    return '无法使用麦克风，请检查设备连接和系统音频设置。';
  }
  return typeof error === 'string' && Object.hasOwn(errors, error)
    ? errors[error] : '无法完成本次语音操作，请检查设备上的语音设置。';
}
