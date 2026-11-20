import { Art, PageHeader } from './OceanDesign';
import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';

type Evidence = {
  samples_ms: number[];
  audio_ms: number;
  speech: { start_ms: number; end_ms: number; text: string }[];
  events: { at_ms: number; description: string }[];
  summary: string | null;
  uncertainty: string | null;
  error: string | null;
};
type VideoTask = {
  id: string;
  title: string;
  duration_ms: number;
  state: string;
  has_audio: boolean;
  error: string | null;
  updated_at_ms: number;
  decoded_ms: number;
  visual_ms: number;
  audio_ms: number;
  samples: number;
  memory_saved: boolean;
  summary: string | null;
  segments: { interval: { start_ms: number; end_ms: number }; state: string; evidence: Evidence | null }[];
};

const phases: Record<string, string> = {
  queued: '等待开始', preparing: '核对文件与模型', decoding: '正在解码',
  analyzing: '正在分析画面和转写', organizing: '正在整理时间线',
  completed: '已分析计划片段', failed: '分析失败', partial: '部分片段有缺口',
  paused: '已暂停', uncertain: '上次执行中断', cancelled: '已取消',
};
const errors: Record<string, string> = {
  VIDEO_FFMPEG_MISSING: '在本机 PATH 中未找到可运行的 FFmpeg 和 ffprobe。',
  VIDEO_SPEECH_MODEL_REQUIRED: '这个视频包含音轨。请先选择本机 Whisper 模型文件。',
  VIDEO_WHISPER_MISSING: '在本机 PATH 中未找到 whisper-cli。',
  VIDEO_OLLAMA_UNAVAILABLE: '无法连接本机 Ollama。请确认它正在运行。',
  VIDEO_VISION_MODEL_MISSING: 'Ollama 中没有这个本地 Qwen3-VL 模型。',
  VIDEO_VISION_UNSUPPORTED: '本机模型接口未声明图像能力。',
  VIDEO_MODEL_CHANGED: '本机模型版本已变更。请用当前模型重新导入视频。',
  VIDEO_TOOL_CHANGED: '解码器版本已变更。请重新导入视频。',
  VIDEO_FILE_FORMAT: '请选择 MP4、MOV、M4V、MKV 或 WebM 文件。',
  VIDEO_FILE_UNAVAILABLE: '读取不到所选的本地文件。',
  VIDEO_FILE_CHANGED: '原视频内容已变更。请恢复原文件或重新导入。',
  VIDEO_FILE_SIZE: '请选择不超过 256 MiB 的文件。',
  VIDEO_DURATION: '当前支持最长 5 分钟的视频。',
  VIDEO_DAILY_LIMIT: '最近 24 小时的导入已达到 3 部或共 10 分钟的上限。',
  VIDEO_NO_VIDEO_TRACK: '这个文件中没有视频轨。',
  VIDEO_DIMENSIONS: '请选择包含主画面且尺寸在 4096 × 4096 以内的视频。',
  VIDEO_PROBE_FAILED: '无法读取可信的视频轨和时长。',
  VIDEO_DECODE_FAILED: '此段画面解码失败。',
  VIDEO_AUDIO_DECODE_FAILED: '此段音轨解码失败。',
  VIDEO_TRANSCRIBE_FAILED: '此段语音转写失败。',
  VIDEO_TOOL_TIMEOUT: '本机媒体工具处理超时。',
  VIDEO_MODEL_TIMEOUT: '本机模型处理超时。',
  VIDEO_MODEL_RESPONSE: '本机模型未返回可保存的分析结果。',
  VIDEO_NO_ANALYSIS: '没有可用于汇总的画面分析。展开片段查看原因。',
  VIDEO_PARTIAL_COVERAGE: '片段尚有覆盖缺口。可重试尚未完成的片段。',
  VIDEO_PERMISSION_REQUIRED: '请确认你有权在本机分析该视频。',
  VIDEO_BUSY: '已有视频任务在执行或刚刚停止。请稍后重试。',
  VIDEO_BUSY_OR_CHANGED: '任务状态已经变更。请刷新后再试。',
  VIDEO_TASK_STATE: '当前任务状态无法执行此操作。',
  VIDEO_PERSONA_CHANGED: '当前人格已经切换。',
};
const message = (error: unknown) => errors[String(error)] ?? '视频操作失败。请检查本机工具和任务状态。';
const time = (ms: number) => {
  const total = Math.floor(ms / 1000);
  return String(Math.floor(total / 60)) + ':' + String(total % 60).padStart(2, '0') + '.' + Math.floor(ms / 100) % 10;
};
const active = (state: string) => ['preparing', 'decoding', 'analyzing', 'organizing'].includes(state);

function Coverage({ label, covered, duration }: { label: string; covered: number; duration: number }) {
  return <label className="video-coverage">
    <span>{label}</span><strong>{time(covered)} / {time(duration)}</strong>
    <progress aria-label={label} value={covered} max={duration} />
  </label>;
}

export function VideoPanel({ personaId, onMemories }: { personaId: string; onMemories: () => void }) {
  const [videos, setVideos] = useState<VideoTask[]>([]);
  const [selected, setSelected] = useState('');
  const [speech, setSpeech] = useState('');
  const [model, setModel] = useState('qwen3-vl:4b');
  const [authorized, setAuthorized] = useState(false);
  const [pending, setPending] = useState('');
  const [error, setError] = useState('');
  const first = useRef(true);
  const refresh = useCallback(async () => {
    const list = await invoke<VideoTask[]>('video_list', { persona: personaId });
    setVideos(list);
    return list;
  }, [personaId]);

  useEffect(() => {
    let closed = false;
    let timer: number | undefined;
    async function poll() {
      try {
        if (document.visibilityState === 'visible') await refresh();
      } catch (err) {
        if (first.current && !closed) setError(message(err));
      } finally { first.current = false; }
      if (!closed) timer = window.setTimeout(poll, 1800);
    }
    if (personaId) void poll();
    return () => { closed = true; window.clearTimeout(timer); };
  }, [personaId, refresh]);

  async function perform(name: string, action: () => Promise<void>) {
    setPending(name);
    setError('');
    try { await action(); }
    catch (err) { setError(message(err)); }
    finally { setPending(''); }
  }

  const video = videos.find(item => item.id === selected) ?? videos[0];
  const busy = videos.some(item => active(item.state));
  async function control(action: string) {
    if (!video) return;
    await invoke('video_control', { persona: personaId, id: video.id, action });
    await refresh();
  }

  return <div className="video-center">
    <PageHeader title="视频中心" icon="video" scene="scene-video" art="thinking" description="本机授权视频的画面分析与语音转写"/>
    <section className="settings-card">
      <h2>导入视频</h2>
      <label>本地画面模型
        <select value={model} onChange={event => setModel(event.target.value)} disabled={!!pending}>
          <option value="qwen3-vl:2b">Qwen3-VL 2B</option>
          <option value="qwen3-vl:4b">Qwen3-VL 4B</option>
          <option value="qwen3-vl:8b">Qwen3-VL 8B</option>
        </select>
      </label>
      <div className="button-row">
        <button type="button" disabled={!!pending} onClick={() => void perform('选择模型', async () => {
          const name = await invoke<string | null>('video_select_whisper');
          if (name) setSpeech(name);
        })}>选择 Whisper 模型</button>
        <span>{speech || '有音轨时需要本机 .bin 模型'}</span>
      </div>
      <label className="check-row">
        <input type="checkbox" checked={authorized} onChange={event => setAuthorized(event.target.checked)} disabled={!!pending} />
        我有权在本机分析所选视频
      </label>
      <button className="primary" type="button" disabled={!personaId || !authorized || !!pending}
        onClick={() => void perform('检查视频与模型', async () => {
          const id = await invoke<string | null>('video_import', { persona: personaId, model, authorized });
          if (id) { setSelected(id); setAuthorized(false); await refresh(); }
        })}>选择视频文件</button>
      {pending && <p role="status">正在{pending}…</p>}
      <details className="field-help"><summary>本机依赖与分析范围</summary>
        <p>需要 PATH 中的 FFmpeg 和 ffprobe，以及已运行的 Ollama 和所选 Qwen3-VL。带音轨时还需要 whisper-cli 与多语种模型。程序使用已安装的工具和模型。</p>
        <p>文件上限 256 MiB 和 5 分钟。最近 24 小时最多导入 3 部、共 10 分钟，同时执行 1 个任务。采用全时间线抽帧和语音转写。</p>
        <p>当前可处理从本机选取的文件。B站媒体直连等待可用的授权接口。</p>
      </details>
      {error && <p className="inline-error" role="alert">{error}</p>}
    </section>
    {!video && <section className="settings-card ocean-empty"><Art name="waiting"/><p>导入视频后可在这里查看任务和分析时间线。</p></section>}
    {video && <section className="settings-card">
      <label>本机任务<select value={video.id} onChange={event => setSelected(event.target.value)}>
        {videos.map(item => <option key={item.id} value={item.id}>{item.title}</option>)}
      </select></label>
      <div className="card-heading">
        <h2>{video.title}</h2>
        <span className="badge">{phases[video.state] ?? '等待核对'}</span>
      </div>
      <p className="field-help">执行设备 · 本机 Windows　播放 · 未播放　采样 · {video.samples} 帧</p>
      <p className="field-help">最近记录 · {new Date(video.updated_at_ms).toLocaleString('zh-CN')}</p>
      <Coverage label="已解码采样区间" covered={video.decoded_ms} duration={video.duration_ms} />
      <Coverage label="画面分析采样覆盖" covered={video.visual_ms} duration={video.duration_ms} />
      {video.has_audio
        ? <Coverage label="音轨转写处理" covered={video.audio_ms} duration={video.duration_ms} />
        : <p className="field-help">此文件没有音轨</p>}
      {video.error && <p className="inline-error">{message(video.error)}</p>}
      <div className="button-row">
        {active(video.state)
          ? <button type="button" disabled={!!pending} onClick={() => void perform('暂停', () => control('pause'))}>暂停</button>
          : ['queued', 'paused', 'uncertain', 'failed', 'partial'].includes(video.state) &&
          <button type="button" disabled={busy || !!pending} onClick={() => void perform('启动任务', () => control('start'))}>
            {video.state === 'queued' ? '开始分析' : '恢复分析'}
          </button>}
        {!['completed','cancelled'].includes(video.state) && <button type="button" disabled={!!pending}
          onClick={() => void perform('取消任务', () => control('cancel'))}>取消任务</button>}
        {video.state === 'completed' && !!video.summary &&
          <button type="button" disabled={video.memory_saved || !!pending} onClick={() => void perform('保存记忆', () => control('save_memory'))}>
            {video.memory_saved ? '已保存到长期记忆' : '保存到长期记忆'}
          </button>}
        {video.memory_saved && <button type="button" onClick={onMemories}>打开本地数据</button>}
      </div>
      {video.summary && <div className="video-analysis"><h3>{video.state === 'completed' ? '全时间线摘要' : '已保存的片段摘要'}</h3><p>{video.summary}</p></div>}
      <div className="video-analysis">
        <h3>时间线</h3>
        {video.segments.map(({interval, state, evidence}) => <details key={interval.start_ms}>
          <summary>{time(interval.start_ms)} – {time(interval.end_ms)} · {
            state === 'completed' ? evidence?.uncertainty ? '已处理，有不确定项' : '已处理'
            : state === 'failed' ? '有覆盖缺口' : '待处理'
          }</summary>
          {evidence && <>
            <p>采样 {evidence.samples_ms.length} 帧{video.has_audio && <> · 音轨处理 {time(evidence.audio_ms)}</>}</p>
            {evidence.summary && <p>{evidence.summary}</p>}
            {evidence.error && <p className="inline-error">{message(evidence.error)}</p>}
            {evidence.uncertainty && <p>不确定 · {evidence.uncertainty}</p>}
            {evidence.events.map((event,index) => <p key={index}>{time(event.at_ms)} · {event.description}</p>)}
            {evidence.audio_ms > 0 && evidence.speech.length === 0 && <p>这段音轨没有可辨认的语音文本。</p>}
            {!!evidence.speech.length && <details>
              <summary>语音转写</summary>
              {evidence.speech.map((cue,index) => <p key={index}>{time(cue.start_ms)} · {cue.text}</p>)}
            </details>}
          </>}
        </details>)}
      </div>
    </section>}
  </div>;
}
