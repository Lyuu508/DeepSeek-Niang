import { useEffect, useState, type ReactNode } from 'react';
import type { ManagedMemory } from '@deepseek/contracts';
import { Art, Icon, PageHeader, WhaleMark, expressions } from './OceanDesign';
import { UpdateCard } from './UpdateCard';

export const palettes = { ocean: '海洋蓝', shell: '贝壳紫', seafoam: '海沫青' } as const;
export type Palette = keyof typeof palettes;
type ColorMode = 'light' | 'dark' | 'system';
const applicationAppearance = 'deepseek-appearance-v1';
type Appearance = { mode: ColorMode; palette: Palette };

function readAppearance(): Appearance {
  try {
    const saved = JSON.parse(localStorage.getItem(applicationAppearance) ?? '{}');
    if (['light', 'dark', 'system'].includes(saved.mode) && Object.hasOwn(palettes, saved.palette)) return saved;
  } catch { /* Appearance is optional when WebView storage is unavailable. */ }
  return { mode: 'system', palette: 'ocean' };
}

export function useAppearance() {
  const [value, setValue] = useState(readAppearance);
  const [systemDark, setSystemDark] = useState(() => matchMedia('(prefers-color-scheme:dark)').matches);
  useEffect(() => {
    const system = matchMedia('(prefers-color-scheme:dark)');
    const onScheme = () => setSystemDark(system.matches);
    const onStorage = (event: StorageEvent) => { if (event.key === applicationAppearance) setValue(readAppearance()); };
    system.addEventListener('change', onScheme);
    window.addEventListener('storage', onStorage);
    return () => { system.removeEventListener('change', onScheme); window.removeEventListener('storage', onStorage); };
  }, []);
  const dark = value.mode === 'dark' || (value.mode === 'system' && systemDark);
  function update(next: Partial<Appearance>) {
    const appearance: Appearance = { ...value, ...next };
    setValue(appearance);
    try { localStorage.setItem(applicationAppearance, JSON.stringify(appearance)); } catch { /* Retain the in-memory preference. */ }
  }
  return { value, dark, update };
}
export type AppearanceControl = ReturnType<typeof useAppearance>;

export function AppearanceSettings({ control }: { control: AppearanceControl }) {
  const { value, update } = control;
  return <section className="settings-card appearance-palette" aria-label="主题外观">
    <h2>主题外观</h2>
    <div className="segment-control" role="group" aria-label="色彩模式">
      {(['system', 'light', 'dark'] as const).map((mode, index) => <button type="button" key={mode}
        aria-pressed={value.mode === mode} onClick={() => update({mode})}><Icon name={(['settings','sun','moon'] as const)[index]}/>{['跟随系统', '浅色', '深海夜色'][index]}</button>)}
    </div>
    <div className="palette-options" aria-label="主题色">
      {(Object.entries(palettes) as [Palette,string][]).map(([palette,label]) => <button type="button" key={palette}
        aria-pressed={value.palette === palette} onClick={() => update({palette})}>
        <span className="palette-swatch" data-swatch={palette}/>{label}
      </button>)}
    </div>
  </section>;
}

export function AboutPage({ version }: { version?: string }) {
  const [selection, setSelection] = useState<keyof typeof expressions>('peeking');
  return <div className="settings-content about-page">
    <section className="about-hero" aria-label="DeepSeek娘角色介绍">
      <div><WhaleMark/><h1>关于 DeepSeek娘</h1><p>Windows 和 Android 的本地 AI 伙伴。</p>
        {version && <span className="badge">v{version}</span>}</div>
      <Art name="portrait-full" label="DeepSeek娘的全身立绘" eager/>
    </section>
    <UpdateCard version={version}/>
    <section className="settings-card">
      <h2>角色表情</h2>
      <div className="expression-gallery">
        <div className="expression-preview"><Art name={selection} label={'DeepSeek娘 · ' + expressions[selection]} eager/></div>
        <div className="button-row">
          {Object.entries(expressions).map(([art,label]) => <button type="button" aria-pressed={selection === art} key={art}
            onClick={() => setSelection(art as keyof typeof expressions)}>{label}</button>)}
        </div>
      </div>
    </section>
    <section className="settings-card"><h2>本地使用</h2><p>聊天、人格和记忆保存在设备上。可以在设置管理模型凭据，在局域网连接页与可信设备配对。</p></section>
  </div>;
}

export function PhoneVideoPage({ memories, onMemories, embedded = false }: { memories: ManagedMemory[]; onMemories: () => void; embedded?: boolean }) {
  const summaries = memories.flatMap(memory => {
    const match = /^本地视频《([^\r\n]{1,120})》的抽帧与语音分析摘要：([\s\S]+)$/u.exec(memory.text);
    return match ? [{ id: memory.id, title: match[1], content: match[2] }] : [];
  });
  return <div className="video-center">
    {!embedded && <PageHeader title="视频中心" description="查看此人格已保存的视频摘要。" icon="video" scene="scene-video" art="thinking"/>}
    <section className="settings-card">
      <h2>已保存的视频摘要</h2>
      <p className="field-help">这里保留已在长期记忆中保存的视频摘要。</p>
      {summaries.length ? <div className="video-analysis">
        {summaries.map(summary => <details key={summary.id}><summary>{summary.title}</summary><p>{summary.content}</p></details>)}
      </div> : <div className="ocean-empty"><Art name="waiting"/><p>这里还没有视频摘要。</p></div>}
      <button onClick={onMemories}><Icon name="memory"/>打开本地数据</button>
    </section>
  </div>;
}

export function MessageText({ text }: { text: string }) {
  const fenced = /^\x60\x60\x60([\w+.-]{0,24})[^\S\r\n]*\r?\n([\s\S]*?)^\x60\x60\x60[^\S\r\n]*(?:\r?\n|$)/gm;
  const nodes: ReactNode[] = [];
  let offset = 0;
  for (const chunk of text.matchAll(fenced)) {
    const start = chunk.index;
    if (start > offset) nodes.push(text.slice(offset, start));
    nodes.push(<pre key={start} className="chat-code" dir="ltr" tabIndex={0} aria-label={chunk[1] ? chunk[1] + ' 代码块' : '代码块'}>
      {chunk[1] && <span className="code-language">{chunk[1]}</span>}<code>{chunk[2].trimEnd()}</code>
    </pre>);
    offset = start + chunk[0].length;
  }
  nodes.push(text.slice(offset));
  return <>{nodes}</>;
}
