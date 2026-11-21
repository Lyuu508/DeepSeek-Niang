import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Icon } from '../OceanDesign';
import { voiceError } from './api';
import type { VoiceController } from './useVoice';

type Settings = { provider: 'open_ai' | 'compatible'; endpoint: string; model: string; voice_id: string; speaker_id: string };
type Binding = { enabled: boolean; voice_id: string; auto: boolean };
type Status = { settings: Settings; binding: Binding; configured: boolean };
const official = 'https://api.openai.com/v1/audio/speech';

export function CloudVoiceSettings({ persona, voice }: { persona: string; voice: VoiceController }) {
  const [value, setValue] = useState<Status>();
  const [secret, setSecret] = useState('');
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState('');
  const [saved, setSaved] = useState(false);

  async function load() {
    setProblem('');
    try { setValue(await invoke<Status>('cloud_voice_status', { persona })); }
    catch (error) { setProblem(voiceError(error)); }
  }
  async function save(remove = false) {
    if (!value || busy) return;
    setBusy(true);
    setProblem('');
    setSaved(false);
    try {
      setValue(await invoke<Status>('cloud_voice_save', {
        persona, settings: value.settings, binding: remove ? { ...value.binding, enabled: false, auto: false } : value.binding,
        key: remove ? null : secret.trim() || null, deleteKey: remove,
      }));
      setSaved(true);
    } catch (error) { setProblem(voiceError(error)); }
    finally { setSecret(''); setBusy(false); }
  }
  function settings(update: Partial<Settings>) {
    setValue(current => current && { ...current, settings: { ...current.settings, ...update } });
    setSaved(false);
  }
  function binding(update: Partial<Binding>) {
    setValue(current => current && { ...current, binding: { ...current.binding, ...update } });
    setSaved(false);
  }

  return <details className="settings-card cloud-voice-settings" onToggle={event => {
    if (event.currentTarget.open && !value) void load();
  }}>
    <summary><Icon name="voice"/>第三方音色与朗读</summary>
    {value && <form onSubmit={event => { event.preventDefault(); void save(); }}>
      <p className="field-help">当前人格启用云朗读后，待朗读文本会发送到所选语音服务。语音输入继续使用设备的转写。</p>
      <label>服务商<select value={value.settings.provider} disabled={busy} onChange={event => {
        const provider = event.target.value as Settings['provider'];
        settings({ provider, ...(provider === 'open_ai' ? { endpoint: official, speaker_id: '' } : {}) });
      }}><option value="open_ai">OpenAI Speech</option><option value="compatible">自定义 OpenAI 兼容语音</option></select></label>
      <div className="two-fields">
        <label>HTTPS 语音接口<input type="url" value={value.settings.endpoint} readOnly={value.settings.provider === 'open_ai'}
          required maxLength={512} onChange={event => settings({ endpoint: event.target.value })}/></label>
        <label>语音模型<input value={value.settings.model} required maxLength={160} onChange={event => settings({ model: event.target.value })}/></label>
        <label>默认 Voice ID<input value={value.settings.voice_id} required maxLength={160} onChange={event => settings({ voice_id: event.target.value })}/></label>
        {value.settings.provider === 'compatible' && <label>Speaker ID（兼容端的语音标识）<input value={value.settings.speaker_id} maxLength={160}
          onChange={event => settings({ speaker_id: event.target.value })}/></label>}
        <label>当前人格的 Voice ID<input value={value.binding.voice_id} maxLength={160}
          onChange={event => binding({ voice_id: event.target.value })} placeholder="留空使用默认音色"/></label>
        <label>语音 API Key<input type="password" value={secret} autoComplete="off" maxLength={2048}
          placeholder={value.configured ? '已保存在系统凭据中' : '输入服务商密钥'} onChange={event => setSecret(event.target.value)}/></label>
      </div>
      <label className="check-row"><input type="checkbox" checked={value.binding.enabled}
        onChange={event => binding({ enabled: event.target.checked })}/><span>为当前人格使用云端朗读</span></label>
      <label className="check-row"><input type="checkbox" checked={value.binding.auto} disabled={!value.binding.enabled}
        onChange={event => binding({ auto: event.target.checked })}/><span>收到新回复后自动朗读</span></label>
      <p className="field-help">音频格式为 WAV，语速使用上方设置。已合成音色可用服务商提供的 Voice ID 绑定。</p>
      <div className="button-row">
        <button className="primary" disabled={busy}>保存第三方语音配置</button>
        <button type="button" disabled={busy || !saved || !value.binding.enabled || voice.phase !== 'idle'}
          onClick={() => void voice.play('你好，主人。我们可以继续聊天了。')}>连接测试与试听</button>
        {value.configured && <button type="button" disabled={busy} onClick={() => void save(true)}>删除语音密钥</button>}
      </div>
      {saved && <p role="status">已保存语音配置。</p>}
    </form>}
    {problem && <p className="inline-error" role="alert">{problem}</p>}
  </details>;
}
