import { useId } from 'react';
import type { Dialogue } from '@deepseek/contracts';

const defaults: Dialogue = {
  mode: 'standard', tone: 'natural', reply_length: 'balanced',
  humor: 2, creativity: 2, expression: 2, roleplay: 'daily', extra_style: '',
};

export function DialogueFields({ value = defaults, onChange, disabled }: {
  value?: Dialogue; onChange: (value: Dialogue) => void; disabled: boolean;
}) {
  const id = useId();
  return <fieldset className="dialogue-fields" disabled={disabled}>
    <legend>当前人格的对话模式</legend>
    <label htmlFor={id}>对话模式</label>
    <select id={id} value={value.mode} onChange={e => onChange({ ...value, mode: e.target.value as Dialogue['mode'] })}>
      <option value="standard">标准模式</option><option value="free">自由对话模式</option><option value="custom">自定义模式</option>
    </select>
    {value.mode === 'free' && <p className="field-help">自然承接话题，减少固定开场和复述。保留人格设定、关系和记忆。</p>}
    {value.mode === 'custom' && <>
      <div className="two-fields">
        <label>语气<select value={value.tone} onChange={e => onChange({ ...value, tone: e.target.value as Dialogue['tone'] })}>
          <option value="natural">自然</option><option value="gentle">温柔</option><option value="calm">冷静</option><option value="playful">俏皮</option>
        </select></label>
        <label>回复长度<select value={value.reply_length} onChange={e => onChange({ ...value, reply_length: e.target.value as Dialogue['reply_length'] })}>
          <option value="short">简短</option><option value="balanced">适中</option><option value="detailed">详细</option>
        </select></label>
      </div>
      <div className="dialogue-sliders">{(['humor', 'creativity', 'expression'] as const).map((key, index) =>
        <label key={key}>{['幽默程度', '创意程度', '情绪表达'][index]} <output>{value[key]}/4</output>
          <input type="range" min={0} max={4} value={value[key]} onChange={e => onChange({ ...value, [key]: e.target.valueAsNumber })}/>
        </label>)}</div>
      <label>角色扮演风格<select value={value.roleplay} onChange={e => onChange({ ...value, roleplay: e.target.value as Dialogue['roleplay'] })}>
        <option value="daily">日常聊天</option><option value="in_character">沿用角色口吻</option><option value="co_write">共同创作虚构故事</option>
      </select></label>
      <label>额外聊天风格<textarea maxLength={600} rows={3} value={value.extra_style}
        onChange={e => onChange({ ...value, extra_style: e.target.value })} placeholder="例如，多些具体例子，少用固定的总结句。"/></label>
    </>}
  </fieldset>;
}
