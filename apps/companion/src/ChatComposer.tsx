import { Icon } from './OceanDesign';
import { useState, type RefObject } from 'react';
import { VoiceControls } from './voice/VoiceControls';
import type { VoiceController } from './voice/useVoice';

interface Props {
  field: RefObject<HTMLTextAreaElement | null>;
  draft: string;
  name?: string;
  connected: boolean;
  disabled: boolean;
  configured: boolean;
  canSend: boolean;
  mobile: boolean;
  voice: VoiceController;
  recording: boolean;
  generation?: { started: boolean; elsewhere: boolean };
  historyTurns: number;
  onEdit: (text: string) => void;
  onSend: () => void;
  onStop: () => void;
  onSettings: () => void;
}

export function ChatComposer(props: Props) {
  const { mobile, voice, generation } = props;
  const [open, setOpen] = useState(false);
  const privacy = `本次使用的人格设定、输入${props.historyTurns ? `和最近 ${props.historyTurns} 轮聊天` : ''}会发给 DeepSeek。可在陪伴设置关闭相关记忆引用。`;
  const stoppedLabel = generation?.elsewhere ? '停止其他人格的生成' : '停止生成';
  return <div className="composer-area">
    {!props.configured && <button className="setup-hint" onClick={props.onSettings}>配置 DeepSeek API Key <span>前往设置 →</span></button>}
    <form className="composer" onSubmit={event => { event.preventDefault(); props.onSend(); }}
      onKeyDown={event => { if (event.key === 'Escape' && !event.nativeEvent.isComposing) setOpen(false); }}>
      <div className="composer-row">
        {mobile && <button type="button" className="composer-more" aria-label="输入工具" aria-expanded={open}
          onClick={() => setOpen(value => !value)}><span aria-hidden="true"><Icon name={open?'close':'add'}/></span></button>}
        <label className="sr-only" htmlFor="chat-input">发送给当前人格的消息</label>
        <textarea id="chat-input" ref={props.field} rows={mobile ? 1 : 3} value={props.draft} maxLength={8000}
          placeholder={props.connected ? `和${props.name ?? '她'}说说吧…` : '原生应用连接后即可聊天'}
          disabled={props.disabled} onChange={event => props.onEdit(event.target.value)}
          onKeyDown={event => {
            if (event.key === 'Enter' && !event.shiftKey && !mobile && !event.nativeEvent.isComposing && event.keyCode !== 229) {
              event.preventDefault();
              props.onSend();
            }
          }}/>
        <div className="composer-toolbar">
          {!mobile && <span>Enter 发送 · Shift + Enter 换行</span>}
          {generation
            ? <button type="button" className="stop-button" aria-label={stoppedLabel} disabled={!generation.started} onClick={props.onStop}>
              <span aria-hidden="true"><Icon name="stop"/></span>{!mobile && ` ${stoppedLabel}`}
            </button>
            : <button className="send-button" aria-label="发送消息" disabled={!props.canSend || !props.draft.trim()}>
              <span aria-hidden="true">{!mobile && '发送 '}<Icon name="send"/></span>
            </button>}
        </div>
      </div>
      {(!mobile || open || voice.phase !== 'idle' || voice.problem) &&
        <div className="composer-tools">
          <VoiceControls voice={voice} recording={props.recording} openSettings={props.onSettings}/>
          <details className="composer-privacy"><summary>发送范围</summary><p>{privacy}</p></details>
        </div>}
    </form>
  </div>;
}
