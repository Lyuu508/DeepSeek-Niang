import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Icon } from './OceanDesign';

type PreviousVideo = {
  id: string;
  title: string;
  state: string;
  updated_at_ms: number;
  summary: string | null;
  segments: {
    interval: { start_ms: number; end_ms: number };
    evidence: {
      summary: string | null;
      speech: { start_ms: number; end_ms: number; text: string }[];
      events: { at_ms: number; description: string }[];
    } | null;
  }[];
};

const states: Record<string, string> = {
  completed: '已完成', failed: '未完成', partial: '部分完成', cancelled: '已取消',
};
const seconds = (milliseconds: number) => (milliseconds / 1000).toFixed(1) + ' 秒';

// Old jobs remain readable. This view never starts a local decoder or provider.
export function PreviousVideoResults({ personaId }: { personaId: string }) {
  const [jobs, setJobs] = useState<PreviousVideo[] | null>(null);
  const [error, setError] = useState(false);
  return <details className="settings-card" onToggle={event => {
    if (!event.currentTarget.open || jobs !== null || !personaId) return;
    setError(false);
    void invoke<PreviousVideo[]>('video_list', { persona: personaId }).then(setJobs).catch(() => setError(true));
  }}>
    <summary><Icon name="book"/>旧版本地视频记录</summary>
    {error && <p role="alert">本机旧版记录暂时无法读取。可关闭此处再打开重试。</p>}
    {!error && jobs === null && <p role="status">正在读取…</p>}
    {jobs?.length === 0 && <p>此人格尚无旧版视频记录。</p>}
    {jobs?.map(job => <details className="video-analysis" key={job.id}>
      <summary>{job.title} · {states[job.state] ?? '已保留任务'}</summary>
      <p className="field-help">{new Date(job.updated_at_ms).toLocaleString('zh-CN')}</p>
      {job.summary && <p>{job.summary}</p>}
      {job.segments.filter(segment => segment.evidence).map(({ interval, evidence }) => <details key={interval.start_ms}>
        <summary>{seconds(interval.start_ms)} – {seconds(interval.end_ms)}</summary>
        {evidence?.summary && <p>{evidence.summary}</p>}
        {evidence?.events.map((event, index) => <p key={index}>{seconds(event.at_ms)} · {event.description}</p>)}
        {evidence?.speech.map((cue, index) => <p key={index}>{seconds(cue.start_ms)} · {cue.text}</p>)}
      </details>)}
    </details>)}
  </details>;
}
