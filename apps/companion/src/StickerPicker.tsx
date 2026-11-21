import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { attachmentSource, readChatImage, type ChatImage } from './ChatAttachment';
import { FileDrop } from './FileDrop';
import { Icon } from './OceanDesign';

type Sticker = { id: string; image: ChatImage; tag: string };
type Catalog = { entries: Sticker[]; automatic: boolean };
type Action = { kind: 'list' } | { kind: 'add'; image: ChatImage; tag: string }
  | { kind: 'remove'; id: string } | { kind: 'automatic'; enabled: boolean };

export function StickerPicker({ persona, onPick }: { persona: string; onPick: (image: ChatImage) => void }) {
  const [catalog, setCatalog] = useState<Catalog>();
  const [query, setQuery] = useState('');
  const [tag, setTag] = useState('平静');
  const [problem, setProblem] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    void invoke<Catalog>('sticker_catalog', { persona, action: { kind: 'list' } }).then(result => {
      if (active) setCatalog(result);
    }).catch(() => { if (active) setProblem('无法读取该人格的本机表情库。'); });
    return () => { active = false; };
  }, [persona]);

  async function perform(work: () => Promise<Action>) {
    if (busy) return;
    setBusy(true);
    setProblem('');
    try { setCatalog(await invoke<Catalog>('sticker_catalog', { persona, action: await work() })); }
    catch { setProblem('操作失败。请选择有效的图片和分类，表情库最多保存 24 张。'); }
    finally { setBusy(false); }
  }

  const entries = catalog?.entries.filter(item => `${item.tag} ${item.image.name}`.toLowerCase().includes(query.trim().toLowerCase())) ?? [];
  return <section className="sticker-picker" aria-label="本机表情库">
    <div className="two-fields">
      <label>搜索表情<input type="search" value={query} onChange={event => setQuery(event.target.value)}/></label>
      <label>导入分类<input value={tag} maxLength={20} onChange={event => setTag(event.target.value)}/></label>
    </div>
    <FileDrop label="导入有权使用的表情图片" accept="image/png,image/jpeg,image/webp" icon="picture" disabled={busy || !catalog}
      onFile={file => void perform(async () => ({ kind: 'add', image: await readChatImage(file), tag }))}/>
    {catalog && <label className="check-row">
      <input type="checkbox" checked={catalog.automatic} disabled={busy} onChange={event => {
        const enabled = event.target.checked;
        void perform(async () => ({ kind: 'automatic', enabled }));
      }}/>按心情自动附加同分类表情
    </label>}
    <div className="sticker-grid">
      {entries.map(item => <div key={item.id}>
        <button type="button" disabled={busy} onClick={() => onPick(item.image)}>
          <img src={attachmentSource(item.image)} alt={item.image.name}/><small>{item.tag}</small>
        </button>
        <button type="button" className="sticker-remove" aria-label={`移除表情 ${item.image.name}`} disabled={busy}
          onClick={() => void perform(async () => ({ kind: 'remove', id: item.id }))}><Icon name="close"/></button>
      </div>)}
    </div>
    {problem && <p className="inline-error" role="alert">{problem}</p>}
  </section>;
}
