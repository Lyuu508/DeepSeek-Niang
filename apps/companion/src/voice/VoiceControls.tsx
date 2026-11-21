import { useEffect, useId, useState } from 'react';
import { voiceAPI, voiceError, type VoiceOption, type VoicePreferences } from './api';
import type { VoiceController } from './useVoice';
import './voice.css';

export function VoiceControls({ voice, recording = false, openSettings }: {
  voice: VoiceController;
  recording?: boolean;
  openSettings?: () => void;
}) {
  if (!voice.enabled) return null;
  const playback = ['loading', 'playing', 'paused'].includes(voice.phase);
  const hint = {
    idle: '', loading: '正在生成音频', playing: '正在播放', paused: '已暂停',
    starting: '正在准备麦克风', recording: '正在录音', transcribing: '正在本机识别',
  }[voice.phase];
  return <div className="voice-controls">
    <div className="voice-actions">
      {recording && voice.phase === 'idle' && <button type="button" onClick={() => void voice.start()}>语音输入</button>}
      {recording && voice.phase === 'recording' && <button className="voice-active" type="button" onClick={() => void voice.finish()}>完成录音</button>}
      {playback && <>
        {voice.phase !== 'loading' && <button type="button" onClick={() => void voice.pause()}>{voice.phase === 'paused' ? '继续播放' : '暂停播放'}</button>}
        <button type="button" aria-pressed={voice.muted} onClick={voice.toggleMute}>{voice.muted ? '取消静音' : '静音'}</button>
      </>}
      {voice.phase !== 'idle' && <button type="button" onClick={voice.stop}>{playback ? '停止播放' : '取消录音'}</button>}
      {openSettings && <button type="button" onClick={openSettings}>语音设置</button>}
      {(hint || voice.message) && <small role="status">{hint || voice.message}</small>}
    </div>
    {voice.problem && <p className="inline-error" role="alert">{voice.problem}</p>}
  </div>;
}

export function VoiceSettings({ voice, platform }: { voice: VoiceController; platform: string }) {
  const voiceId = useId();
  const speedId = useId();
  const volId = useId();
  const [choices, setChoices] = useState<VoiceOption[]>([]);
  const [draft, setDraft] = useState<VoicePreferences>();
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');
  const saved = voice.prefs;
  useEffect(() => { setDraft(saved); }, [saved]);
  useEffect(() => {
    let alive = true;
    void voiceAPI.voices().then(result => { if (alive) setChoices(result); })
      .catch(cause => { if (alive) setError(voiceError(cause)); })
      .finally(() => { if (alive) setLoading(false); });
    return () => { alive = false; };
  }, []);

  async function perform(work: () => Promise<void>, done?: string) {
    setError('');
    setNotice('');
    setBusy(true);
    try { await work(); if (done) setNotice(done); }
    catch (cause) { setError(voiceError(cause)); }
    finally { setBusy(false); }
  }

  return <section className="settings-card voice-settings" aria-label="语音与麦克风">
    <div className="card-heading"><span className="card-icon">♫</span><div><h2>语音与麦克风</h2><p>音色和播放设置随本机的当前人格保存。</p></div></div>
    {draft && <form onSubmit={event => { event.preventDefault(); void perform(() => voice.save(draft), '语音设置已保存。'); }}>
      <label htmlFor={voiceId}>离线朗读音色</label>
      <select id={voiceId} value={draft.voice_id} disabled={loading || busy}
        onChange={event => setDraft({ ...draft, voice_id: event.target.value })}>
        <option value="">自动选择本机音色 · 优先中文</option>
        {draft.voice_id && !choices.some(choice => choice.id === draft.voice_id) && <option value={draft.voice_id}>已选的音色暂不可用</option>}
        {choices.map(choice => <option key={choice.id} value={choice.id}>{choice.name} · {choice.language}</option>)}
      </select>
      <div className="voice-sliders">
        <label htmlFor={speedId}>朗读语速 <output>{draft.speed_percent}%</output>
          <input id={speedId} type="range" min="70" max="130" step="5" value={draft.speed_percent}
            onChange={event => setDraft({ ...draft, speed_percent: event.target.valueAsNumber })}/>
        </label>
        <label htmlFor={volId}>输出音量 <output>{draft.volume_percent}%</output>
          <input id={volId} type="range" min="0" max="100" step="5" value={draft.volume_percent}
            onChange={event => setDraft({ ...draft, volume_percent: event.target.valueAsNumber })}/>
        </label>
      </div>
      <div className="voice-actions">
        <label><input type="checkbox" checked={draft.muted} onChange={event => setDraft({ ...draft, muted: event.target.checked })}/> 默认静音</label>
        <label><input type="checkbox" checked={draft.follow_mood} onChange={event => setDraft({ ...draft, follow_mood: event.target.checked })}/> 随当前情绪调整语气</label>
      </div>
      <button className="primary" disabled={busy || voice.phase !== 'idle'}>保存语音设置</button>
    </form>}
    {loading && <small role="status">正在读取本机音色…</small>}
    {!loading && !choices.length && !error && <p className="field-help">请先在系统中安装离线语音包。</p>}
    <div className="voice-actions">
      <button type="button" disabled={!saved || !choices.length || voice.phase !== 'idle'}
        onClick={() => void voice.play('你好，今天想一起聊些什么？')}>试听朗读</button>
      <button type="button" disabled={!saved || voice.phase !== 'idle'}
        onClick={() => void voice.play()}>试听原创元音唱段</button>
      {platform === 'windows' && <button type="button" disabled={busy || voice.phase !== 'idle'}
        onClick={() => void perform(voice.chooseModel)}>选择 Whisper 模型</button>}
    </div>
    <p className="field-help">试听使用已保存的音量和语速。原创唱段用七个音符合成元音，可以试听音高与节奏。</p>
    {platform === 'windows'
      ? <p className="field-help">录音转写需要本机已有的 whisper-cli 和多语种模型。{voice.model ? `本次选择 ${voice.model}。` : '仅用于麦克风转写。'}</p>
      : <p className="field-help">录音用 Android 12 及以上设备提供的离线普通话识别。</p>}
    {notice && <p role="status">{notice}</p>}
    {error && <p className="inline-error" role="alert">{error}</p>}
    <VoiceControls voice={voice}/>
  </section>;
}
