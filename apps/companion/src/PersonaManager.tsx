import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { Bootstrap, ChatFailure, Persona } from '@deepseek/contracts';
import { native, nativeAvailable, failure, runtimeMode } from './native';
import { Avatar } from './Avatar';
import { Confirmation } from './Confirmation';
import { previewSettings } from './defaults';

type Entry = { persona_id: string; decision_id: string };
interface Recycled {
  persona: Persona;
  entry: Entry;
  personality: string;
  changed_at_ms: number;
  sync_state: 'local' | 'pending' | 'acknowledged' | 'received';
}
type Action = { kind: 'trash'; persona_id: string }
  | { kind: 'restore'; entry: Entry }
  | { kind: 'purge'; entries: Entry[] };
type Pending = { kind: 'trash'; persona: Persona } | { kind: 'purge'; items: Recycled[]; all: boolean };
const syncText = {
  local: '本机状态',
  pending: '等待已配对设备确认',
  acknowledged: '已配对设备已确认',
  received: '已从对端同步',
};

export function PersonaManager({ boot, onSwitch, onRefresh, onError, onSettings, onErased }: {
  boot: Bootstrap | null; onSwitch: (id: string, toSettings?: boolean) => Promise<void>;
  onRefresh: () => Promise<void>; onError: (e: ChatFailure | null) => void;
  onSettings: () => void; onErased: (conversations: string[]) => void;
}) {
  const [name, setName] = useState('');
  const [personality, setPersonality] = useState(previewSettings.personality);
  const [busy, setBusy] = useState(false);
  const [trashOpen, setTrashOpen] = useState(false);
  const [entries, setEntries] = useState<Recycled[]>([]);
  const [pending, setPending] = useState<Pending>();
  const [notice, setNotice] = useState('');
  const [localError, setLocalError] = useState('');
  const [detail, setDetail] = useState<Persona>();
  const lock = useRef(false);
  const refreshTrash = useCallback(async () => setEntries(await invoke<Recycled[]>('list_recycled_personas')), []);

  useEffect(() => {
    if (trashOpen && runtimeMode === 'native') void refreshTrash().catch(e => setLocalError(failure(e).message));
  }, [trashOpen, boot, refreshTrash]);

  async function create() {
    if (lock.current || !name.trim()) return;
    lock.current = true;
    setBusy(true);
    onError(null);
    try {
      const persona = await native.createPersona(name, personality);
      setName('');
      await onRefresh();
      await onSwitch(persona.id);
    } catch (error) {
      onError(failure(error));
    } finally {
      lock.current = false;
      setBusy(false);
    }
  }

  async function change(action: Action) {
    if (lock.current || runtimeMode !== 'native') return;
    lock.current = true;
    setBusy(true);
    setLocalError('');
    setNotice('');
    try {
      const impacted = await invoke<string[]>('change_recycled_personas', { action });
      if (action.kind === 'purge') onErased(impacted);
      setPending(undefined);
      setNotice(action.kind === 'restore' ? '我又能继续陪你聊天啦！'
        : action.kind === 'trash' ? '已移入人格回收站。' : `已永久删除 ${impacted.length} 个人格。`);
      await onRefresh();
      if (trashOpen) await refreshTrash();
    } catch (error) {
      setLocalError(failure(error).message);
      if (trashOpen) void refreshTrash().catch(() => {});
    } finally {
      lock.current = false;
      setBusy(false);
    }
  }

  return <div className="settings-content persona-manager">
    <div className="page-heading">
      <h1>{trashOpen ? '人格回收站' : '人格管理'}</h1>
      {trashOpen && <p>暂时放在这里的人格，还可以重新找回来哦。</p>}
      <div className="button-row">
        {runtimeMode === 'native' && <button disabled={busy} onClick={() => { setTrashOpen(!trashOpen); setNotice(''); setLocalError(''); }}>
          {trashOpen ? '返回人格管理' : '人格回收站'}
        </button>}
        {trashOpen && <button className="danger" disabled={busy || !entries.length}
          onClick={() => setPending({ kind: 'purge', items: [...entries], all: true })}>清空回收站</button>}
      </div>
    </div>
    {notice && <p className="notice" role="status">{notice}</p>}
    {!pending && localError && <p className="inline-error" role="alert">{localError}</p>}
    {trashOpen ? <section className="persona-grid" aria-label="回收站人格">
      {entries.map(item => <article className="persona-tile recycle-tile" key={item.persona.id}>
        <Avatar persona={item.persona}/>
        <div><h2>{item.persona.name}</h2><p className="persona-summary">{item.personality}</p>
          <p><time>{new Date(item.changed_at_ms).toLocaleString()}</time> · {syncText[item.sync_state]}</p></div>
        <div className="button-row">
          <button disabled={busy} onClick={() => void change({ kind: 'restore', entry: item.entry })}>恢复</button>
          <button className="danger" disabled={busy} onClick={() => setPending({ kind: 'purge', items: [item], all: false })}>永久删除</button>
        </div>
      </article>)}
      {!entries.length && <p className="field-help">回收站为空。</p>}
    </section> : <>
      <section className="persona-grid">{boot?.personas.map(p => <article className="persona-tile" key={p.id}>
        <Avatar persona={p}/><div><h2>{p.name}</h2>
          <p>{p.is_builtin ? '默认人格' : '自定义人格'}{boot.active_persona_id === p.id ? ' · 使用中' : ''}</p>
          {p.personality && <p className="persona-summary">{p.personality}</p>}
        </div>
        <div className="button-row">
          <button className="quiet-button" disabled={busy} onClick={() => void onSwitch(p.id)}>{boot.active_persona_id === p.id ? '进入聊天' : '切换人格'} →</button>
          {boot.active_persona_id === p.id && <button className="quiet-button" onClick={onSettings}>编辑人物设定 →</button>}
        </div>
        <details className="persona-menu"><summary>操作</summary>
          <button onClick={() => setDetail(p)}>查看人格信息</button>
          {boot.active_persona_id !== p.id && <button disabled={busy} onClick={() => void onSwitch(p.id, true)}>编辑人物设定</button>}
          {runtimeMode === 'native' && !p.is_builtin && <button className="danger" disabled={busy || boot.personas.length <= 1}
            onClick={() => { setLocalError(''); setPending({ kind: 'trash', persona: p }); }}>移入回收站</button>}
        </details>
      </article>)}</section>
      <form className="settings-card" onSubmit={e => { e.preventDefault(); void create(); }}>
        <h2>新增人格</h2>
        <label htmlFor="new-persona-name">人格名称</label>
        <input id="new-persona-name" value={name} maxLength={32} disabled={!nativeAvailable || busy} onChange={e => setName(e.target.value)}/>
        <label htmlFor="new-persona-personality">独立人物设定</label>
        <textarea id="new-persona-personality" value={personality} maxLength={3000} disabled={!nativeAvailable || busy} onChange={e => setPersonality(e.target.value)}/>
        <button className="primary" disabled={!nativeAvailable || busy || !name.trim() || !personality.trim()}>{busy ? '正在创建…' : '创建人格并进入聊天'}</button>
      </form>
    </>}
    {detail && <Confirmation title={detail.name} body={<><Avatar persona={detail}/><p>{detail.personality || '可在人物设定中编辑性格。'}</p></>}
      cancelLabel="返回" confirmLabel="编辑设定" busy={false} onCancel={() => setDetail(undefined)}
      onConfirm={() => { const p = detail; setDetail(undefined); void onSwitch(p.id, true); }}/>}
    {pending?.kind === 'trash' && <Confirmation title="主人，真的要把我收起来吗？"
      body={<><Avatar small/><p>主人，真的要暂时把我放进回收站吗？<br/>
        虽然有一点舍不得，但我会乖乖待在那里……<br/>如果以后还想找我聊天，记得把我恢复回来哦。</p>
        {localError && <p className="inline-error" role="alert">{localError}</p>}</>}
      cancelLabel="再想想" confirmLabel="移入回收站" busy={busy} onCancel={() => setPending(undefined)}
      onConfirm={() => void change({ kind: 'trash', persona_id: pending.persona.id })}/>}
    {pending?.kind === 'purge' && <Confirmation title={pending.all ? '确定要清空所有回忆吗？' : '主人，真的要和我告别了吗？'} danger
      body={<><Avatar small/>
        <p>{pending.all ? '主人，回收站里还有一些暂时收起来的人格呢……' : '主人，这次是真的要把我删掉了吗？'}</p>
        <p>将永久删除 {pending.items.length} 个人格和对应的聊天、记忆、关系、生活记录及人格插件设置。</p>
        <ul>{pending.items.map(item => <li key={item.persona.id}>{item.persona.name}</li>)}</ul>
        <p>确认后无法从回收站恢复。旧备份、离线设备和存储残留可能保留副本。</p>
        {localError && <p className="inline-error" role="alert">{localError}</p>}</>}
      cancelLabel={pending.all ? '取消清空' : '再想想'} confirmLabel={pending.all ? '确认永久清空' : '永久删除'}
      busy={busy} onCancel={() => setPending(undefined)}
      onConfirm={() => void change({ kind: 'purge', entries: pending.items.map(item => item.entry) })}/>}
  </div>;
}
