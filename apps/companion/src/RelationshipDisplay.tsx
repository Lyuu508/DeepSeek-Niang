import type { CompanionSnapshot, Persona } from '@deepseek/contracts';
import { Avatar } from './Avatar';
import { Art, Icon, moodArt } from './OceanDesign';

export const bondDimensions = [
  ['familiarity', '熟悉', 'persona'],
  ['trust', '信任', 'star'],
  ['affection', '好感', 'bond'],
  ['intimacy', '亲密', 'pair'],
  ['comfort', '舒适', 'sofa'],
] as const;
type Bond = CompanionSnapshot['relationship'];
export function relationLevel(stage: string) {
  const index = ['初识', '熟悉', '信任', '亲密', '深度陪伴'].indexOf(stage);
  return index < 0 ? null : index + 1;
}

export function BondDimensions({ bond }: { bond?: Bond }) {
  return <div className="bond-dimensions relation-meters" aria-label="当前关系维度">
    {bondDimensions.map(([key, label, icon]) => <div className="relation-meter" data-dimension={key} key={key}>
      <span className="meter-icon"><Icon name={icon}/></span>
      <span className="meter-label">{label}</span>
      {bond ? <progress aria-label={label + '程度'} value={bond[key]} max={100000} /> : <span className="meter-placeholder"/>}
      <strong>{bond ? <>{(bond[key] / 1000).toFixed(1)}<small> / 100</small></> : '—'}</strong>
    </div>)}
  </div>;
}

const hints: Record<string, string> = {
  happy: '现在心情不错，乐意分享刚想到的事。', excited: '兴致正好，想多聊一点。',
  shy: '有点害羞，想慢慢说。', curious: '对接下来的话题有些好奇。',
  sad: '情绪有些低落，愿意安静地聊。', lonely: '这会儿有些落寞，慢慢说也好。',
  angry: '情绪还没平复，想先缓一缓。', annoyed: '这会儿想把节奏放慢。',
  tired: '有些疲惫，想轻松地聊几句。', calm: '心情平静，可以接着聊。',
};

export function MoodDisplay({ data, persona }: { data: CompanionSnapshot | null; persona?: Persona }) {
  const current = data?.life.host_online && data.life.current?.state === 'running' ? data.life.current : null;
  const text = data ? `${persona?.name ?? 'DeepSeek娘'}${current ? `正在${current.label}。` : ` · ${hints[data.mood.kind] ?? `当前心情是${data.mood.label}。`}`}` : '正在读取本机陪伴状态。';
  return <div className="mood-display">
    {persona?.is_builtin !== false ? <Art key={moodArt(data)} name={moodArt(data)} label={data?.mood.label ?? ''}/> : <Avatar persona={persona} small/>}
    <div><div className="mood-heading"><strong>当前心情</strong><span className="mood-pill">{data?.mood.label ?? '正在读取'}</span></div>
      <p className="mood-bubble">{text}</p>
    </div>
  </div>;
}
