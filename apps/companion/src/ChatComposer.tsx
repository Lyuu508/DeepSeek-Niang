import { Icon } from './OceanDesign';
import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { VoiceControls } from './voice/VoiceControls';
import type { VoiceController } from './voice/useVoice';
import { attachmentSource, MAX_CHAT_IMAGES, readChatImage, readTextAttachment, type ChatImage } from './ChatAttachment';
import { runtimeMode } from './native';
import { StickerPicker } from './StickerPicker';
import { canSendClip, type VideoMode } from './video-media';

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
  videoFile?: File;
  videoMode?: VideoMode;
  videoStatus: string;
  onImages: (images: ChatImage[]) => void;
  onVideo: (file: File) => void;
  onVideoMode: (mode: VideoMode) => void;
  onRemoveVideo: () => void;
  onEdit: (text: string) => void;
  onSend: (imageConsent: boolean, videoConsent: boolean) => void;
  onStop: () => void;
  onConfigure: () => void;
  onSettings: () => void;
}

export function ChatComposer(props: Props) {
  const { mobile, voice, generation } = props;
  const showVoiceStatus = voice.phase !== 'idle' || Boolean(voice.problem || voice.message);
  const [open, setOpen] = useState(false);
  const [stickers, setStickers] = useState(false);
  const [attachmentError, setAttachmentError] = useState('');
  const [reading, setReading] = useState(false);
  const [imageConsent, setImageConsent] = useState(false);
  const [videoConsent, setVideoConsent] = useState(false);
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
  useEffect(() => {
    const surface = props.field.current?.closest<HTMLElement>('.chat-main');
    if (!surface || runtimeMode !== 'native') return;
    const allowDrop = (event: DragEvent) => {
      if (event.dataTransfer?.types.includes('Files')) event.preventDefault();
    };
    const drop = (event: DragEvent) => {
      const files = Array.from(event.dataTransfer?.files ?? []);
      if (!files.length) return;
      event.preventDefault();
      if (props.disabled || reading) return;
      const [first] = files;
      if (files.length === 1 && (first.type.startsWith('video/') || /\.(mp4|mov|webm)$/i.test(first.name))) {
        props.onVideo(first);
      } else {
        void attachImages(files);
      }
    };
    surface.addEventListener('dragover', allowDrop);
    surface.addEventListener('drop', drop);
    return () => {
      surface.removeEventListener('dragover', allowDrop);
      surface.removeEventListener('drop', drop);
    };
  });
  useEffect(() => { if (!props.images.length) setImageConsent(false); }, [props.images.length]);
  useEffect(() => { setVideoConsent(false); }, [props.videoFile, props.videoMode]);
  const hasDraft = Boolean(props.draft.trim() || props.images.length || props.videoFile);
  const maySend = !reading && props.canSend && hasDraft && (!props.videoFile || (videoConsent && props.images.length === 0));
  function submit() {
    if (maySend) props.onSend(props.images.length > 0 && imageConsent, Boolean(props.videoFile && videoConsent));
  }
  const stoppedLabel = generation?.elsewhere ? '停止其他人格的生成' : '停止生成';
  return <div className="composer-area">
    <form className="composer" data-tools-expanded={open} onSubmit={event => { event.preventDefault(); submit(); }}
      onKeyDown={event => { if (event.key === 'Escape' && !event.nativeEvent.isComposing) { setOpen(false); setStickers(false); } }}>
      {!props.configured && <button type="button" className="composer-access" onClick={props.onConfigure}><Icon name="key"/>配置 DeepSeek API<Icon name="arrow"/></button>}
      {props.images.length > 0 && <div className="attachment-gallery">
        {props.images.map((picture, index) => <div className="attachment-preview" key={index}>
          <img src={attachmentSource(picture)} alt={picture.name}/>
          <button type="button" aria-label={`移除图片 ${picture.name}`} onClick={() => props.onImages(props.images.filter((_, item) => item !== index))}><Icon name="close"/></button>
        </div>)}
        <label className="composer-vision"><input type="checkbox" checked={imageConsent} disabled={props.disabled || reading}
          onChange={event=>setImageConsent(event.currentTarget.checked)}/>
          <span>用千问识别图片<small>{imageConsent?'图片和本次问题会发给千问，DeepSeek 根据识别结果回答。':'关闭时仅保存本机，不上传图片。'}</small></span>
        </label>
      </div>}
      {props.videoFile && <div className="composer-video">
        <div className="composer-video-name"><Icon name="video"/><span>{props.videoFile.name}</span>
          <small>{(props.videoFile.size/1048576).toFixed(2)} MiB</small>
          <button type="button" aria-label="移除视频" disabled={!!props.videoStatus} onClick={props.onRemoveVideo}><Icon name="close"/></button>
        </div>
        <label className="composer-vision"><input type="checkbox" checked={videoConsent} disabled={!!props.videoStatus}
          onChange={event=>setVideoConsent(event.currentTarget.checked)}/>
          <span>我有权发送此视频，允许千问{props.videoMode==='clip'?'接收 MP4 原片（可能含音轨）并识别画面':'识别抽样画面'}，由 DeepSeek 回答。</span>
        </label>
        {canSendClip(props.videoFile) && <button className="composer-video-mode" type="button" disabled={!!props.videoStatus}
          onClick={()=>props.onVideoMode(props.videoMode==='clip'?'frames':'clip')}>
          {props.videoMode==='clip'?'改用抽样帧':'改用 MP4 原片'}
        </button>}
        {props.images.length>0 && <p className="field-help">图片与视频请分别发送。</p>}
        {props.videoStatus && <p role="status"><progress aria-label={props.videoStatus}/> {props.videoStatus}</p>}
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
      <div className="composer-body">
        <div className="composer-row">
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
                submit();
              }
            }}/>
        </div>
        <div className="composer-attachments">
          {mobile && <button type="button" className="composer-more" aria-label="输入工具" aria-expanded={open}
            onClick={() => setOpen(value => !value)}><Icon name={open?'close':'add'}/></button>}
          {(!mobile || open) && <button type="button" title="插入文本文件" aria-label="添加附件" disabled={props.disabled || reading}
            onClick={() => file.current?.click()}><Icon name="add"/></button>}
          {(!mobile || open) && <button type="button" title="附加图片" aria-label="上传图片" disabled={props.disabled || reading || runtimeMode !== 'native'}
            onClick={() => image.current?.click()}><Icon name="picture"/></button>}
          {(!mobile || open) && <button type="button" title="视频提问" aria-label="添加视频" disabled={props.disabled || reading || runtimeMode !== 'native'}
            onClick={() => video.current?.click()}><Icon name="video"/></button>}
          {(!mobile || open) && props.personaId && runtimeMode === 'native' &&
            <button type="button" title="本机表情库" aria-label="表情包" aria-expanded={stickers}
              disabled={props.disabled || reading} onClick={() => setStickers(value => !value)}><Icon name="bond"/></button>}
          {(!mobile || open) && voice.enabled && voice.phase === 'idle' && <button type="button" title="语音输入" aria-label="语音输入"
            disabled={!props.recording} onClick={() => void voice.start()}><Icon name="voice"/></button>}
        </div>
        <div className="composer-toolbar">
          {!mobile && <span className="composer-shortcut">Enter 发送 · Shift+Enter 换行</span>}
          {generation
            ? <button type="button" className="stop-button" aria-label={stoppedLabel} disabled={!generation.started} onClick={props.onStop}>
              <Icon name="stop"/>
            </button>
            : <button className="send-button" aria-label="发送消息" disabled={!maySend}>
              <Icon name="send"/>
            </button>}
        </div>
      </div>
      {showVoiceStatus && <VoiceControls voice={voice} recording={props.recording && voice.phase !== 'idle'} openSettings={props.onSettings}/>}
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
