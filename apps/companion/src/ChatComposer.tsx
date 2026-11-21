import { Icon } from './OceanDesign';
import { useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { VoiceControls } from './voice/VoiceControls';
import type { VoiceController } from './voice/useVoice';
import { attachmentSource, MAX_CHAT_IMAGES, readChatImage, readTextAttachment, type ChatImage } from './ChatAttachment';
import { runtimeMode } from './native';
import { StickerPicker } from './StickerPicker';

interface Props {
  field: RefObject<HTMLTextAreaElement | null>;
  draft: string;
  name?: string;
  personaId?: string;
  connected: boolean;
  disabled: boolean;
  configured: boolean;
  canSend: boolean;
  mobile: boolean;
  voice: VoiceController;
  recording: boolean;
  generation?: { started: boolean; elsewhere: boolean };
  images: ChatImage[];
  onImages: (images: ChatImage[]) => void;
  onVideo: (file: File) => void;
  onEdit: (text: string) => void;
  onSend: () => void;
  onStop: () => void;
  onSettings: () => void;
}

export function ChatComposer(props: Props) {
  const { mobile, voice, generation } = props;
  const showVoiceStatus = voice.phase !== 'idle' || Boolean(voice.problem || voice.message);
  const [open, setOpen] = useState(false);
  const [stickers, setStickers] = useState(false);
  const [attachmentError, setAttachmentError] = useState('');
  const [reading, setReading] = useState(false);
  const file = useRef<HTMLInputElement>(null);
  const image = useRef<HTMLInputElement>(null);
  const video = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    const field = props.field.current;
    if (!field || CSS.supports('field-sizing', 'content')) return;
    field.style.height = 'auto';
    field.style.height = `${Math.min(field.scrollHeight, 128)}px`;
  }, [props.field, props.draft, mobile]);

  async function attachText(value: File) {
    setAttachmentError('');
    setReading(true);
    try {
      const next = [props.draft, await readTextAttachment(value)].filter(Boolean).join('\n\n');
      if (next.length > 8000 || new TextEncoder().encode(next).length > 32_000) throw new Error();
      props.onEdit(next);
    } catch { setAttachmentError('请选择 12 KB 以内的 UTF-8 文本文件，并预留输入长度。'); }
    finally { setReading(false); }
  }
  async function attachImages(files: File[]) {
    if (!files.length || reading || props.disabled || runtimeMode !== 'native') return;
    if (props.images.length + files.length > MAX_CHAT_IMAGES) {
      setAttachmentError(`每条消息可附加 ${MAX_CHAT_IMAGES} 张图片。`);
      return;
    }
    setAttachmentError('');
    setReading(true);
    try {
      const images = await Promise.all(files.map(readChatImage));
      props.onImages([...props.images, ...images]);
    } catch { setAttachmentError('请选择 8 MiB 以内的 PNG、JPEG 或 WebP 图片。'); }
    finally { setReading(false); }
  }
  const hasDraft = Boolean(props.draft.trim() || props.images.length);
  const stoppedLabel = generation?.elsewhere ? '停止其他人格的生成' : '停止生成';
  return <div className="composer-area">
    {!props.configured && <button className="setup-hint" onClick={props.onSettings}><Icon name="key"/><span>还没有配置 DeepSeek API，主人可以前往设置完成配置哦。</span><Icon name="arrow"/></button>}
    <form className="composer" onSubmit={event => { event.preventDefault(); if (!reading) props.onSend(); }}
      onKeyDown={event => { if (event.key === 'Escape' && !event.nativeEvent.isComposing) setOpen(false); }}
      onDragOver={event => { if (event.dataTransfer.types.includes('Files')) event.preventDefault(); }}
      onDrop={event => {
        if (!event.dataTransfer.files.length) return;
        event.preventDefault();
        const files = Array.from(event.dataTransfer.files);
        const [first] = files;
        if (files.length === 1 && first?.type.startsWith('video/') && !props.disabled && !reading && runtimeMode === 'native') {
          props.onVideo(first);
        } else {
          void attachImages(files);
        }
      }}>
      {props.images.length > 0 && <div className="attachment-gallery">
        {props.images.map((picture, index) => <div className="attachment-preview" key={index}>
          <img src={attachmentSource(picture)} alt={picture.name}/>
          <button type="button" aria-label={`移除图片 ${picture.name}`} onClick={() => props.onImages(props.images.filter((_, item) => item !== index))}><Icon name="close"/></button>
        </div>)}
        <small>图片仅保存在本机。可以用文字描述画面。</small>
      </div>}
      {attachmentError && <p className="inline-error" role="alert">{attachmentError}</p>}
      <input type="file" ref={file} hidden accept=".txt,.md,.csv,.json,.log,.yaml,.yml" onChange={event => {
        const value = event.currentTarget.files?.[0]; event.currentTarget.value = ''; if (value) void attachText(value);
      }}/>
      <input type="file" ref={image} hidden multiple accept="image/png,image/jpeg,image/webp" onChange={event => {
        const files = Array.from(event.currentTarget.files ?? []);
        event.currentTarget.value = '';
        void attachImages(files);
      }}/>
      <input type="file" ref={video} hidden accept="video/*,.mp4,.mov,.webm" onChange={event => {
        const file = event.currentTarget.files?.[0];
        event.currentTarget.value = '';
        if (file) props.onVideo(file);
      }}/>
      <div className="composer-row">
        {mobile && <button type="button" className="composer-more" aria-label="输入工具" aria-expanded={open}
          onClick={() => setOpen(value => !value)}><span aria-hidden="true"><Icon name={open?'close':'add'}/></span></button>}
        <label className="sr-only" htmlFor="chat-input">发送给当前人格的消息</label>
        <textarea id="chat-input" ref={props.field} rows={1} value={props.draft} maxLength={8000}
          placeholder={props.connected ? `和${props.name ?? '她'}说说吧…` : '原生应用连接后即可聊天'}
          disabled={props.disabled || reading} onChange={event => props.onEdit(event.target.value)}
          onPaste={event => {
            const files = Array.from(event.clipboardData.files).filter(file => file.type.startsWith('image/'));
            if (!files.length) return;
            event.preventDefault();
            void attachImages(files);
          }}
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
            : <button className="send-button" aria-label="发送消息" disabled={reading || !props.canSend || !hasDraft}>
              <span aria-hidden="true"><Icon name="send"/></span>
            </button>}
        </div>
      </div>
      {(!mobile || open || showVoiceStatus) &&
        <div className="composer-tools">
          <div className="composer-attachments">
            <button type="button" title="插入文本文件" aria-label="添加附件" disabled={props.disabled || reading} onClick={() => file.current?.click()}><Icon name="add"/></button>
            <button type="button" title="附加本机图片" aria-label="上传图片" disabled={props.disabled || reading || runtimeMode !== 'native'} onClick={() => image.current?.click()}><Icon name="picture"/></button>
            <button type="button" title="视频提问" aria-label="添加视频" disabled={props.disabled || reading || runtimeMode !== 'native'}
              onClick={() => video.current?.click()}><Icon name="video"/></button>
            {props.personaId && runtimeMode === 'native' &&
              <button type="button" title="本机表情库" aria-label="表情包" aria-expanded={stickers}
                disabled={props.disabled || reading} onClick={() => setStickers(value => !value)}><Icon name="bond"/></button>}
            {voice.enabled && voice.phase === 'idle' && <button type="button" title="语音输入" aria-label="语音输入" disabled={!props.recording} onClick={() => void voice.start()}><Icon name="voice"/></button>}
          </div>
          {showVoiceStatus && <VoiceControls voice={voice} recording={props.recording && voice.phase !== 'idle'} openSettings={props.onSettings}/>}
        </div>}
      {stickers && props.personaId && <StickerPicker persona={props.personaId} onPick={image => {
        if (props.images.length >= MAX_CHAT_IMAGES) {
          setAttachmentError(`每条消息可附加 ${MAX_CHAT_IMAGES} 张图片。`);
          return;
        }
        props.onImages([...props.images, image]);
        setStickers(false);
      }}/>}
    </form>
  </div>;
}
