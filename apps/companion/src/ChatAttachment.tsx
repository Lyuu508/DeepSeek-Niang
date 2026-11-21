import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Icon } from './OceanDesign';

export type ChatImage = { name: string; jpeg: string };
export const attachmentSource = (image: ChatImage) => `data:image/jpeg;base64,${image.jpeg}`;

export async function readChatImage(file: File): Promise<ChatImage> {
  if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type) || file.size > 8 * 1024 * 1024) throw new Error();
  const bitmap = await createImageBitmap(file);
  try {
    const scale = Math.min(1, 800 / Math.max(bitmap.width, bitmap.height));
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.floor(bitmap.width * scale));
    canvas.height = Math.max(1, Math.floor(bitmap.height * scale));
    const context = canvas.getContext('2d');
    if (!context) throw new Error();
    context.fillStyle = '#fff';
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const jpeg = canvas.toDataURL('image/jpeg', .76).split(',')[1];
    if (!jpeg || jpeg.length > 320 * 1024) throw new Error();
    return { name: [...file.name.replace(/[\p{Cc}]/gu, '')].slice(0, 90).join('') || '图片.jpg', jpeg };
  } finally { bitmap.close(); }
}

export async function readTextAttachment(file: File) {
  if (!/\.(txt|md|csv|json|log|yaml|yml)$/i.test(file.name) || file.size > 12_000) throw new Error();
  const body = new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer());
  if (!body.trim()) throw new Error();
  const name = [...file.name.replace(/[\p{Cc}]/gu, '')].slice(0, 80).join('');
  return `【文字附件：${name}】\n${body}`;
}

export function ChatImagePreview({ conversation, id }: { conversation: string; id: string }) {
  const [preview, setPreview] = useState<ChatImage | null>(null);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  return <div className="message-attachment">{preview
    ? <button type="button" className="image-close" aria-label="收起图片" onClick={() => setPreview(null)}><img alt={preview.name} src={attachmentSource(preview)}/></button>
    : <button type="button" disabled={busy} onClick={() => {
      setBusy(true);
      setError(false);
      void invoke<ChatImage | null>('chat_image', { conversationId: conversation, id }).then(value => {
        setPreview(value);setError(value === null);
      }).catch(() => setError(true)).finally(() => setBusy(false));
    }}><Icon name="picture"/>查看本机图片</button>}
    {error && <small role="alert">图片无法读取。</small>}
  </div>;
}
