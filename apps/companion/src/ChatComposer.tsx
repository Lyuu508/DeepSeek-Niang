import { Icon } from './OceanDesign';
import { useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { VoiceControls } from './voice/VoiceControls';
import type { VoiceController } from './voice/useVoice';
import { attachmentSource, readChatImage, readTextAttachment, type ChatImage } from './ChatAttachment';
import { runtimeMode } from './native';

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
  image?: ChatImage | null;
  onImage: (image: ChatImage | null) => void;
  onEdit: (text: string) => void;
  onSend: () => void;
  onStop: () => void;
  onSettings: () => void;
}

export function ChatComposer(props: Props) {
  const { mobile, voice, generation } = props;
  const [open, setOpen] = useState(false);
  const [attachmentError, setAttachmentError] = useState('');
  const [reading, setReading] = useState(false);
  const file = useRef<HTMLInputElement>(null);
  const image = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    const field = props.field.current;
    if (!field || CSS.supports('field-sizing', 'content')) return;
    field.style.height = 'auto';
    field.style.height = `${Math.min(field.scrollHeight, 128)}px`;
  }, [props.field, props.draft, mobile]);

  async function attach(value: File, picture: boolean) {
    setAttachmentError('');
    setReading(true);
    try {
      if (picture) props.onImage(await readChatImage(value));
      else {
        const next = [props.draft, await readTextAttachment(value)].filter(Boolean).join('\n\n');
        if (next.length > 8000 || new TextEncoder().encode(next).length > 32_000) throw new Error();
        props.onEdit(next);
      }
    } catch { setAttachmentError(picture ? '请选择 8 MiB 以内的 PNG、JPEG 或 WebP 图片。' : '请选择 12 KB 以内的 UTF-8 文本文件，并预留输入长度。'); }
    finally { setReading(false); }
  }
  const privacy = `本次使用的人格设定、输入${props.historyTurns ? `和最近 ${props.historyTurns} 轮聊天` : ''}会发给 DeepSeek。可在陪伴设置关闭相关记忆引用。`;
  const stoppedLabel = generation?.elsewhere ? '停止其他人格的生成' : '停止生成';
  return <div className="composer-area">
    {!props.configured && <button className="setup-hint" onClick={props.onSettings}>配置 DeepSeek API Key <span>前往设置 →</span></button>}
    <form className="composer" onSubmit={event => { event.preventDefault(); if (!reading) props.onSend(); }}
      onKeyDown={event => { if (event.key === 'Escape' && !event.nativeEvent.isComposing) setOpen(false); }}>
      {props.image && <div className="attachment-preview"><img src={attachmentSource(props.image)} alt={props.image.name}/><div><strong>{props.image.name}</strong><small>图片仅保存在本机。请在文字中描述相关画面。</small></div><button type="button" aria-label="移除图片" onClick={() => props.onImage(null)}><Icon name="close"/></button></div>}
      {attachmentError && <p className="inline-error" role="alert">{attachmentError}</p>}
      <input type="file" ref={file} hidden accept=".txt,.md,.csv,.json,.log,.yaml,.yml" onChange={event => {
        const value = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (value) void attach(value, false);
      }}/>
      <input type="file" ref={image} hidden accept="image/png,image/jpeg,image/webp" onChange={event => {
        const value = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (value) void attach(value, true);
      }}/>
      <div className="composer-row">
        {mobile && <button type="button" className="composer-more" aria-label="输入工具" aria-expanded={open}
          onClick={() => setOpen(value => !value)}><span aria-hidden="true"><Icon name={open?'close':'add'}/></span></button>}
        <label className="sr-only" htmlFor="chat-input">发送给当前人格的消息</label>
        <textarea id="chat-input" ref={props.field} rows={1} value={props.draft} maxLength={8000}
          placeholder={props.connected ? `和${props.name ?? '她'}说说吧…` : '原生应用连接后即可聊天'}
          disabled={props.disabled || reading} onChange={event => props.onEdit(event.target.value)}
          onKeyDown={event => {
            if (event.key === 'Enter' && !event.shiftKey && !mobile && !event.nativeEvent.isComposing && event.keyCode !== 229) {
              event.preventDefault();
              if (!reading) props.onSend();
            }
          }}/>
        <div className="composer-toolbar">
          {!mobile && <span>Enter 发送 · Shift + Enter 换行</span>}
          {generation
            ? <button type="button" className="stop-button" aria-label={stoppedLabel} disabled={!generation.started} onClick={props.onStop}>
              <span aria-hidden="true"><Icon name="stop"/></span>{!mobile && ` ${stoppedLabel}`}
            </button>
            : <button className="send-button" aria-label="发送消息" disabled={reading || !props.canSend || !props.draft.trim()}>
              <span aria-hidden="true"><Icon name="send"/></span>
            </button>}
        </div>
      </div>
      {(!mobile || open || voice.phase !== 'idle' || voice.problem) &&
        <div className="composer-tools">
          <div className="composer-attachments">
            <button type="button" title="插入文本文件" aria-label="添加附件" disabled={props.disabled || reading} onClick={() => file.current?.click()}><Icon name="add"/></button>
            <button type="button" title="附加本机图片" aria-label="上传图片" disabled={props.disabled || reading || runtimeMode !== 'native'} onClick={() => image.current?.click()}><Icon name="picture"/></button>
            {voice.enabled && voice.phase === 'idle' && <button type="button" title="语音输入" aria-label="语音输入" disabled={!props.recording} onClick={() => void voice.start()}><Icon name="voice"/></button>}
          </div>
          {voice.phase !== 'idle' || voice.problem ? <VoiceControls voice={voice} recording={props.recording} openSettings={props.onSettings}/> : null}
          <details className="composer-privacy"><summary>发送范围</summary><p>{privacy}</p></details>
        </div>}
    </form>
  </div>;
}
