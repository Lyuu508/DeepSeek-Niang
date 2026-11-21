import { FileDrop } from './FileDrop';
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import type { ManagedMemory } from '@deepseek/contracts';
import { Confirmation } from './Confirmation';
import { Art, Icon, PageHeader } from './OceanDesign';
import { MessageText, PhoneVideoPage } from './OceanPages';
import { PreviousVideoResults } from './VideoPanel';
import { runtimeMode } from './native';
import { canSendClip, prepareVideo, videoDuration, type Plan } from './video-media';

type ProviderConfig = { base_url: string; model: string; timeout_seconds: number };
type Provider = { settings: ProviderConfig; configured: boolean; connection: 'ready' | 'untested' | 'error' | 'unconfigured' };
type Evidence = { at_ms: number; scene: string; objects: string[]; action: string; text: string };
type Record = { id: string; question: string; duration_ms: number; plan: Plan; evidence: { observations: Evidence[]; limitations: string } | null; answer: string | null };
const defaults: ProviderConfig = { base_url: 'https://dashscope.aliyuncs.com/api/v1', model: 'qwen3-vl-plus', timeout_seconds: 120 };
const errorMessages: { [key: string]: string } = {
  VIDEO_FILE_LIMIT: '请选择不超过 128 MiB 的视频。原片上传限 12 MiB 的 MP4。',
  VIDEO_FILE_DURATION: '请选择设备可解码的、时长不超过 5 分钟的视频。',
  VIDEO_WEBVIEW_DECODE: '设备无法读取这段画面。可以转换为常用编码的 MP4 后重试。',
  VIDEO_MEDIA_INVALID: '没有得到有效视频或采样帧，请重新选择文件。',
  VIDEO_MEDIA_LIMIT: '发送素材超过大小或数量限制。请重新选择或裁剪视频。',
  VIDEO_QUESTION_INVALID: '请填写视频问题并确认时长。',
  VIDEO_PLAN_INVALID: 'DeepSeek 未返回可用的识别计划。请尝试补充对象或所关注的时间段。',
  VIDEO_DEEPSEEK_KEY_REQUIRED: '请先在设置保存 DeepSeek API Key。',
  VIDEO_DEEPSEEK_AUTH: 'DeepSeek 拒绝身份验证，请核对它的 Key 和服务权限。',
  VIDEO_DEEPSEEK_BUDGET: 'DeepSeek 当前额度不足或请求太频繁。',
  VIDEO_DEEPSEEK_TIMEOUT: 'DeepSeek 请求超时，已保存的画面证据可用于重试。',
  VIDEO_DEEPSEEK_FAILED: 'DeepSeek 暂未生成完整结果，请稍后重试。',
  VIDEO_CONSENT_REQUIRED: '请确认授权范围。',
  VIDEO_BUSY: '另一项视频操作正在运行。',
  VIDEO_PERSONA_CHANGED: '当前人格已切换，请返回视频页面重新操作。',
  VIDEO_RECORD_CHANGED: '记录已变化，请重新打开页面。',
  VIDEO_STORAGE_ERROR: '本机未能保存视频结果。请检查剩余空间。',
  VIDEO_EVIDENCE_REQUIRED: '请先取得有效的画面观察结果。',
  QWEN_KEY_REQUIRED: '请先保存千问 API Key。',
  QWEN_KEY_INVALID: '密钥格式不正确。',
  QWEN_SETTINGS_INVALID: '请选择百炼官方区域地址、列表中的模型和 15–300 秒超时。',
  QWEN_STORAGE_FAILED: '本机系统安全存储无法保存或读取千问 Key。',
  QWEN_AUTH_FAILED: '百炼拒绝该区域、模型或密钥。请核对账号权限。',
  QWEN_BUDGET: '百炼额度不足或请求过于频繁。',
  QWEN_UPLOAD_POLICY_INVALID: '百炼未提供可用的私有上传授权，请核对模型和区域。',
  QWEN_UPLOAD_FAILED: '素材未上传成功。选择和识别计划已保留。',
  QWEN_MEDIA_REJECTED: '百炼拒绝了请求。核对模型与区域，或切换到抽样帧后手动重试。',
  QWEN_EVIDENCE_INVALID: '千问没有返回可用的画面证据，未进入最终回答。',
  QWEN_TIMEOUT: '百炼请求超时，请检查网络和超时设置后再试。',
  QWEN_UNAVAILABLE: '无法连接百炼，请检查网络、区域和模型。',
};
const errorText = (error: unknown) => errorMessages[String(error)] ?? '视频操作未完成，请稍后重试。';
const connectionLabels = { ready: '连接已验证', untested: '已配置 · 待验证', error: '连接测试失败', unconfigured: '未配置' };

function timestamp(milliseconds: number) {
  const seconds = Math.floor(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}.${Math.floor(milliseconds % 1000 / 100)}`;
}

export function CloudVideoPanel({ personaId, deepseekConfigured, memories, previousWindows, initialVideo, onSaved, onMemories }: {
  personaId: string; deepseekConfigured: boolean; memories: ManagedMemory[]; previousWindows: boolean; initialVideo?: { file: File; question: string }; onSaved: () => void; onMemories: () => void;
}) {
  const native = runtimeMode === 'native';
  const [provider, setProvider] = useState<Provider>({ settings: defaults, configured: false, connection: 'unconfigured' });
  const [configuration, setConfiguration] = useState(defaults);
  const [key, setKey] = useState('');
  const [file, setFile] = useState<File>();
  const [duration, setDuration] = useState(0);
  const [mode, setMode] = useState<'clip' | 'frames'>('frames');
  const [question, setQuestion] = useState(initialVideo?.question ?? '');
  const [consent, setConsent] = useState(false);
  const [record, setRecord] = useState<Record | null>(null);
  const [matchingPlan, setMatchingPlan] = useState('');
  const [busy, setBusy] = useState('');
  const [problem, setProblem] = useState('');
  const [confirm, setConfirm] = useState<'test' | 'clear' | 'answer' | null>(null);

  useEffect(() => {
    if (!native || !personaId) return;
    let disposed = false;
    void Promise.all([invoke<Provider>('qwen_status'), invoke<Record | null>('cloud_video_status', { persona: personaId })])
      .then(([value, saved]) => {
        if (disposed) return;
        setProvider(value);
        setConfiguration(value.settings);
        setRecord(saved);
      }).catch(error => { if (!disposed) setProblem(errorText(error)); });
    return () => { disposed = true; };
  }, [native, personaId]);

  async function act(label: string, work: () => Promise<void>) {
    setProblem('');
    setBusy(label);
    try { await work(); }
    catch (error) { setProblem(errorText(error)); }
    finally { setBusy(''); }
  }

  async function select(next: File) {
    const value = await videoDuration(next);
    setFile(next);
    setDuration(value);
    setMode(canSendClip(next) ? 'clip' : 'frames');
    setMatchingPlan('');
    setConsent(false);
  }

  useEffect(() => {
    if (!initialVideo) return;
    let cancelled = false;
    setBusy('读取聊天视频');
    void videoDuration(initialVideo.file).then(duration => {
      if (cancelled) return;
      setDuration(duration);
      setFile(initialVideo.file);
      setMode(canSendClip(initialVideo.file) ? 'clip' : 'frames');
    }).catch(error => {
      if (!cancelled) setProblem(errorText(error));
    }).finally(() => { if (!cancelled) setBusy(''); });
    return () => { cancelled = true; };
  }, [initialVideo]);

  async function analyze() {
    if (!file || !consent) return;
    let current = record;
    if (!current || current.id !== matchingPlan || current.question !== question.trim()) {
      setBusy('DeepSeek 正在拟定识别计划');
      current = await invoke<Record>('cloud_video_plan', { persona: personaId, question, durationMs: duration, authorized: true });
      setRecord(current);
      setMatchingPlan(current.id);
    }
    if (!current.evidence) {
      setBusy(mode === 'clip' ? '千问正在识别画面' : '正在提取并识别抽样帧');
      const input = await prepareVideo(file, mode, current.plan);
      current = await invoke<Record>('cloud_video_recognize', { persona: personaId, id: current.id, input, authorized: true });
      setRecord(current);
    }
    setBusy('DeepSeek 正在组织回答');
    await finalAnswer(current.id);
    setConsent(false);
  }

  async function finalAnswer(id: string) {
    setRecord(await invoke<Record>('cloud_video_answer', { persona: personaId, id, authorized: true }));
  }

  return <div className="video-center cloud-video">
    <PageHeader title="视频中心" icon="video" scene="scene-video" art="thinking" description="千问提取画面证据，DeepSeek结合你的问题给出回答。" />
    <section className="settings-card">
      <div className="card-heading"><span className="card-icon"><Icon name="video"/></span><div><h2>视频提问</h2><p>选择有权分析的本地视频。</p></div></div>
      <FileDrop label="选择或拖入视频" accept="video/*,.mp4,.mov,.webm" icon="video" file={file}
        disabled={!native} busy={busy} onFile={next => void act('读取视频', () => select(next))}
        onClear={() => { setFile(undefined); setDuration(0); setMatchingPlan(''); setConsent(false); }}/>
      {file && <p className="field-help">视频时长 · {timestamp(duration)}</p>}
      <label>你想了解什么？<textarea rows={3} maxLength={1000} value={question} disabled={!!busy} onChange={event => { setQuestion(event.target.value); setConsent(false); }} placeholder="这个视频里的人做了什么？"/></label>
      {file && <div className="segment-control" role="group" aria-label="发送素材">
        <button type="button" disabled={!!busy || !canSendClip(file)} aria-pressed={mode === 'clip'} onClick={() => {setMode('clip');setConsent(false);}}>MP4 原片</button>
        <button type="button" disabled={!!busy} aria-pressed={mode === 'frames'} onClick={() => {setMode('frames');setConsent(false);}}>仅抽样帧</button>
      </div>}
      <label className="check-row"><input type="checkbox" checked={consent} disabled={!file || !!busy} onChange={event => setConsent(event.target.checked)}/>
        <span>我有权处理所选素材，同意将问题交给 DeepSeek，并将{mode === 'clip' ? '原片（可能包含音轨）' : '抽样画面'}发给千问，使用其画面结果回答。</span></label>
      <div className="button-row"><button type="button" className="primary" disabled={!native || !personaId || !deepseekConfigured || !provider.configured || !file || !question.trim() || !consent || !!busy} onClick={() => void act('准备任务', analyze)}><Icon name="send"/>分析并回答</button>
        {busy && <span role="status">{busy}…</span>}
      </div>
      <p className="field-help">最长 5 分钟。抽样模式不发送音轨，可能漏掉帧之间的动作。此功能只解释画面。云端识别可能计费。</p>
      {native && !deepseekConfigured && <p className="field-help">请先在设置保存 DeepSeek API Key。</p>}
      {!native && <p className="field-help">安装客户端后可选取视频并使用安全存储。</p>}
      {problem && <p className="inline-error" role="alert">{problem}</p>}
    </section>
    {record && <section className="settings-card">
      <h2>最近一次分析</h2>
      <p className="field-help">问题 · {record.question}</p>
      {!record.evidence && <p>DeepSeek 观察计划 · {record.plan.focus}（{timestamp(record.plan.start_ms)} – {timestamp(record.plan.end_ms)}）</p>}
      {record.answer && <div className="video-answer"><h3>DeepSeek娘的回答</h3><MessageText text={record.answer}/></div>}
      {record.evidence && <details className="video-analysis"><summary>画面证据 · {record.evidence.observations.length} 个观察节点</summary>
        {record.evidence.observations.map((item, index) => <article key={index}><strong>{timestamp(item.at_ms)}</strong><p>{item.scene}{item.objects.length > 0 && ` · ${item.objects.join('、')}`}</p>
          {item.action && <p>动作 · {item.action}</p>}{item.text && <p>可见文字 · {item.text}</p>}
        </article>)}
        {record.evidence.limitations && <p className="field-help">识别限制 · {record.evidence.limitations}</p>}
      </details>}
      <div className="button-row">
        {record.answer && <button type="button" disabled={!!busy || !native} onClick={() => void act('保存问答', async () => {
          await invoke('cloud_video_save_chat', { persona: personaId, id: record.id });
          onSaved();
        })}><Icon name="chat"/>保存问答到聊天</button>}
        {record.evidence && <button type="button" disabled={!!busy || !deepseekConfigured} onClick={() => setConfirm('answer')}><Icon name="sync"/>{record.answer ? '重新组织回答' : '重试 DeepSeek 回答'}</button>}
        <button type="button" disabled={!!busy} onClick={() => setConfirm('clear')}>清除本机结果</button>
      </div>
    </section>}
    <section className="settings-card" aria-label="千问视频识别配置">
      <div className="card-heading"><span className="card-icon"><Icon name="key"/></span><div><h2>千问视频识别配置</h2><p>普通聊天继续使用 DeepSeek。</p></div><span className="badge">{connectionLabels[provider.connection]}</span></div>
      <form onSubmit={event => { event.preventDefault(); void act('保存配置', async () => {
        try { setProvider(await invoke<Provider>('qwen_settings', { settings: configuration, key: key.trim() || null, deleteKey: false })); }
        finally { setKey(''); }
      }); }}>
        <label>API 地址<select value={configuration.base_url} onChange={event => setConfiguration({ ...configuration, base_url: event.target.value })}>
          <option value="https://dashscope.aliyuncs.com/api/v1">中国内地 · dashscope.aliyuncs.com</option>
          <option value="https://dashscope-intl.aliyuncs.com/api/v1">国际 · dashscope-intl.aliyuncs.com</option>
          <option value="https://dashscope-us.aliyuncs.com/api/v1">美国 · dashscope-us.aliyuncs.com</option>
        </select></label>
        <div className="two-fields">
          <label>识别模型<select value={configuration.model} onChange={event => setConfiguration({ ...configuration, model: event.target.value })}>
            <option value="qwen3-vl-plus">Qwen3-VL Plus</option><option value="qwen3-vl-flash">Qwen3-VL Flash</option><option value="qwen3-vl-235b-a22b-instruct">Qwen3-VL 235B Instruct</option>
          </select></label>
          <label>请求超时（秒）<input type="number" min={15} max={300} required value={configuration.timeout_seconds} onChange={event => setConfiguration({ ...configuration, timeout_seconds: event.target.valueAsNumber })}/></label>
        </div>
        <label>千问 API Key<input type="password" maxLength={512} value={key} disabled={!native || !!busy} autoComplete="off" spellCheck={false} placeholder={provider.configured ? '留空保持已有密钥' : '输入所在区域的百炼密钥'} onChange={event => setKey(event.target.value)}/></label>
        <p className="field-help">密钥保存在本机系统安全存储。测试连接使用一张固定色块图，可能产生少量费用。</p>
        <div className="button-row">
          <button className="primary" disabled={!native || !!busy}>保存千问配置</button>
          <button type="button" disabled={!native || !provider.configured || !!busy} onClick={() => setConfirm('test')}>测试连接</button>
          <button type="button" disabled={!native || !provider.configured || !!busy} onClick={() => void act('删除密钥', async () => {
            setProvider(await invoke<Provider>('qwen_settings', { settings: provider.settings, key: null, deleteKey: true }));setKey('');
          })}>删除千问密钥</button>
        </div>
      </form>
    </section>
    {memories.some(memory => memory.text.startsWith('本地视频《')) && <PhoneVideoPage embedded memories={memories} onMemories={onMemories}/>}
    {native && previousWindows && <PreviousVideoResults personaId={personaId}/>}
    <details className="settings-card field-help"><summary>处理范围</summary><p>授权后客户端使用百炼私有临时 OSS 上传。列表中的模型可用性由账号和区域决定。画面证据可以留在本机，以便仅重新调用 DeepSeek。</p><Art name="waiting" className="small-video-art"/></details>
    {confirm && <Confirmation title={confirm === 'test' ? '测试千问连接' : confirm === 'clear' ? '清除本机分析结果？' : '让 DeepSeek 重新回答？'}
      body={confirm === 'test' ? '发送一张固定色块图验证区域、密钥和模型，不会读取所选视频。' : confirm === 'clear' ? '会清除当前人格在这台设备的最近一次视频分析。' : '使用已经保存的画面证据和原问题。'}
      cancelLabel="取消" confirmLabel={confirm === 'clear' ? '清除' : '确认'} danger={confirm === 'clear'} busy={!!busy} onCancel={() => setConfirm(null)} onConfirm={() => {
        const action = confirm;
        setConfirm(null);
        void act(action === 'test' ? '测试千问连接' : action === 'clear' ? '清除结果' : 'DeepSeek 正在组织回答', async () => {
          if (action === 'test') {
            try { setProvider(await invoke<Provider>('qwen_connection_test', { authorized: true })); }
            catch (error) { setProvider({ ...provider, connection: 'error' }); throw error; }
          } else if (action === 'clear') {
            await invoke('cloud_video_clear', { persona: personaId }); setRecord(null);setMatchingPlan('');
          } else if (record) await finalAnswer(record.id);
        });
      }}/>}
  </div>;
}
