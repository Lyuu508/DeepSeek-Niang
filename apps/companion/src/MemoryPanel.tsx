import { useCallback, useEffect, useState } from 'react';
import type { ChatFailure, CompanionSnapshot, ManagedMemory, MemoryContext, MemoryEdit, MemoryState } from '@deepseek/contracts';
import { failure, native, nativeAvailable, runtimeMode, searchManagedMemories } from './native';
import { dateTime } from './CompanionPanel';

const kinds: Record<string, string> = { episodic: '情景记忆', semantic: '语义记忆', relationship: '关系记忆', knowledge: '知识记忆' };
const states: Record<MemoryState, string> = { active: '有效', completed: '已完成', superseded: '历史版本', dismissed: '已停用' };
const newContext = (at: number): MemoryContext => ({
  is_event: false, shared_story: false, state: 'active', occurred_at_ms: at, due_at_ms: null,
  confidence_milli: 1000, recall_allowed: false, reminder_allowed: false, shared_with_personas: false,
});
const empty: MemoryEdit = { id: null, expected_version: null, expected_revision: null, text: '', kind: 'semantic', importance_milli: 700, pinned: false, api_allowed: false };

function existingEdit(memory: ManagedMemory): MemoryEdit {
  return {
    id: memory.id, expected_version: memory.version, expected_revision: memory.revision,
    text: memory.text, kind: memory.kind, importance_milli: memory.importance_milli,
    pinned: memory.pinned, api_allowed: memory.api_allowed,
    context: memory.context ?? newContext(memory.created_at_ms ?? Date.now()),
  };
}

function inputDate(value: number | null): string {
  if (value === null) return '';
  const local = new Date(value);
  return new Date(value - local.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
}

interface Props {
  personaId: string;
  onError: (error: ChatFailure | null) => void;
  onRefresh: () => Promise<void>;
  onSource?: (messageId: string) => void;
}

export function MemoryPanel({ personaId, onError, onRefresh, onSource }: Props) {
  const [data, setData] = useState<CompanionSnapshot | null>(null);
  const [matches, setMatches] = useState<ManagedMemory[] | null>(null);
  const [edit, setEdit] = useState<MemoryEdit>({ ...empty });
  const [hasSource, setHasSource] = useState(false);
  const [filter, setFilter] = useState('all');
  const [tab, setTab] = useState<'memories' | 'shared'>('memories');
  const [query, setQuery] = useState('');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const refresh = useCallback(async () => {
    if (!nativeAvailable || !personaId) return;
    setData(await native.companion(personaId));
    if (query.trim() && runtimeMode === 'native') setMatches(await searchManagedMemories(personaId, query));
  }, [personaId, query]);
  useEffect(() => {
    let current = true;
    if (nativeAvailable && personaId) void native.companion(personaId).then(value => { if (current) setData(value); })
      .catch(e => { if (current) onError(failure(e)); });
    return () => { current = false; };
  }, [personaId, onError]);
  useEffect(() => {
    let current = true;
    setMatches(null);
    if (runtimeMode !== 'native' || !personaId || !query.trim()) return;
    const timer = window.setTimeout(() => {
      void searchManagedMemories(personaId, query).then(value => { if (current) setMatches(value); })
        .catch(e => { if (current) onError(failure(e)); });
    }, 180);
    return () => { current = false; window.clearTimeout(timer); };
  }, [personaId, query, onError]);

  async function perform(work: () => Promise<unknown>) {
    setBusy(true);
    setNotice('');
    onError(null);
    try { await work(); await refresh(); await onRefresh(); }
    catch (e) { onError(failure(e)); await refresh().catch(() => {}); }
    finally { setBusy(false); }
  }
  function startEdit(memory: ManagedMemory) {
    setEdit(existingEdit(memory));
    setHasSource(!!memory.source_message_id && !!memory.source_excerpt);
    setTab('memories');
    setNotice('正在编辑已有记忆，保存时会核对版本。');
    requestAnimationFrame(() => document.getElementById('memory')?.focus());
  }
  function changeContext(values: Partial<MemoryContext>) {
    setEdit(old => ({ ...old, context: { ...(old.context ?? newContext(Date.now())), ...values } }));
  }

  const context = edit.context;
  const term = query.trim().toLocaleLowerCase();
  const rows = matches ?? (data?.persona_id === personaId ? data.memories : [])
    .filter(memory => !term || `${memory.text} ${memory.source_excerpt ?? ''}`.toLocaleLowerCase().includes(term));
  const currentRows = tab === 'shared'
    ? rows.filter(memory => memory.context?.shared_story && memory.source_excerpt && memory.context.state !== 'dismissed')
      .sort((a, b) => (b.context?.occurred_at_ms ?? b.created_at_ms ?? 0) - (a.context?.occurred_at_ms ?? a.created_at_ms ?? 0))
    : rows.filter(memory => filter === 'all' || (filter === 'event' ? memory.context?.is_event : memory.kind === filter));

  return <section className="settings-card memory-panel">
    <div className="card-heading"><h2>{tab === 'shared' ? '共同回忆时间轴' : '分层长期记忆'}</h2>
      <button disabled={!nativeAvailable || busy} onClick={() => void refresh().catch(e => onError(failure(e)))}>刷新记忆</button></div>
    <nav className="button-row memory-tabs" aria-label="记忆页面">
      <button aria-current={tab === 'memories' ? 'page' : undefined} onClick={() => setTab('memories')}>长期记忆</button>
      <button aria-current={tab === 'shared' ? 'page' : undefined} onClick={() => setTab('shared')}>共同回忆</button>
    </nav>
    {notice && <p className="notice" role="status">{notice}</p>}
    {tab === 'memories' && <>
      <form onSubmit={event => {
        event.preventDefault();
        void perform(async () => { await native.editMemory(personaId, edit); setEdit({ ...empty }); setHasSource(false); setNotice('记忆已保存。'); });
      }}>
        <label htmlFor="memory">记忆内容</label>
        <textarea id="memory" value={edit.text} disabled={!nativeAvailable || busy} onChange={e => setEdit(old => ({ ...old, text: e.target.value }))} maxLength={2500} />
        <div className="two-fields">
          <label>记忆类别<select aria-label="记忆类别" value={edit.kind} onChange={e => setEdit(old => ({ ...old, kind: e.target.value }))}>
            {Object.entries(kinds).map(([kind, label]) => <option value={kind} key={kind}>{label}</option>)}
          </select></label>
          <label>重要性 · {edit.importance_milli / 10}%<input aria-label="记忆重要性" type="range" min={0} max={1000} step={50} value={edit.importance_milli} onChange={e => setEdit(old => ({ ...old, importance_milli: Number(e.target.value) }))} /></label>
        </div>
        <label className="check-row"><input type="checkbox" checked={edit.pinned} onChange={e => setEdit(old => ({ ...old, pinned: e.target.checked }))} />标记重要记忆</label>
        <label className="check-row"><input type="checkbox" checked={edit.api_allowed} onChange={e => setEdit(old => ({
          ...old, api_allowed: e.target.checked, context: e.target.checked || !old.context ? old.context : { ...old.context, recall_allowed: false, reminder_allowed: false, shared_with_personas: false },
        }))} />允许与当前话题相关时用于 DeepSeek 聊天</label>
        <details open={edit.id ? true : undefined}>
          <summary>事件、主动回忆与共享</summary>
          <label className="check-row"><input type="checkbox" checked={context?.is_event ?? false} onChange={e => changeContext(e.target.checked ? { is_event: true } : { is_event: false, due_at_ms: null, reminder_allowed: false })} />记录为事件</label>
          {context?.is_event && <label>预计完成时间<input aria-label="预计完成时间" type="datetime-local" value={inputDate(context.due_at_ms)} onChange={e => changeContext({ due_at_ms: e.target.value ? new Date(e.target.value).getTime() : null })} /></label>}
          <label>有效状态<select aria-label="记忆状态" value={context?.state ?? 'active'} onChange={e => changeContext({ state: e.target.value as MemoryState })}>
            {Object.entries(states).map(([state, label]) => <option key={state} value={state}>{label}</option>)}
          </select></label>
          <label className="check-row"><input type="checkbox" disabled={!edit.api_allowed} checked={context?.recall_allowed ?? false} onChange={e => changeContext(e.target.checked ? { recall_allowed: true } : { recall_allowed: false, reminder_allowed: false })} />允许这条记忆用于主动回忆</label>
          {context?.is_event && <label className="check-row"><input type="checkbox" disabled={!context.recall_allowed} checked={context.reminder_allowed} onChange={e => changeContext({ reminder_allowed: e.target.checked })} />允许提醒这个事件</label>}
          <label className="check-row"><input type="checkbox" disabled={!edit.api_allowed} checked={context?.shared_with_personas ?? false} onChange={e => changeContext({ shared_with_personas: e.target.checked })} />允许其他人格引用这条记忆</label>
          {hasSource && <label className="check-row"><input type="checkbox" checked={context?.shared_story ?? false} onChange={e => changeContext({ shared_story: e.target.checked })} />收录到共同回忆</label>}
        </details>
        <div className="button-row">
          <button className="primary" disabled={!nativeAvailable || busy || !edit.text.trim()}>{edit.id ? '保存记忆修订' : '保存记忆'}</button>
          {edit.id && <button type="button" onClick={() => { setEdit({ ...empty }); setHasSource(false); }}>取消编辑</button>}
        </div>
      </form>
      {!!data?.memory_candidates.length && <div className="memory-candidates"><h3>待确认的记忆冲突</h3><p>旧值尚未被覆盖，请核对来源后选择。</p>
        {data.memory_candidates.map(candidate => <article className="memory-candidate" key={candidate.id}>
          <p>当前 · {data.memories.find(m => m.id === candidate.conflicting_memory_id)?.text ?? '需重新核对'}</p>
          <strong>新候选 · {candidate.content}</strong><small>{dateTime(candidate.created_at_ms)}</small>
          <div className="button-row"><button disabled={busy} onClick={() => void perform(() => native.resolveMemoryCandidate(personaId, candidate.id, true))}>确认采用修订</button>
            <button disabled={busy} onClick={() => void perform(() => native.resolveMemoryCandidate(personaId, candidate.id, false))}>保留旧值</button></div>
        </article>)}
      </div>}
    </>}
    <label>{tab === 'shared' ? '搜索共同回忆' : '搜索长期记忆'}<input type="search" value={query} maxLength={96} onChange={e => setQuery(e.target.value)} /></label>
    {tab === 'memories' && <label className="memory-filter">查看类别<select aria-label="查看记忆类别" value={filter} onChange={e => setFilter(e.target.value)}>
      <option value="all">全部类别</option><option value="event">事件</option>{Object.entries(kinds).map(([kind, label]) => <option value={kind} key={kind}>{label}</option>)}
    </select></label>}
    <div className={tab === 'shared' ? 'shared-memory-timeline' : undefined}>
      {currentRows.map(memory => <article className="memory" key={memory.id}>
        <div className="memory-tags">
          <span className="badge">{tab === 'shared' ? dateTime(memory.context?.occurred_at_ms ?? memory.created_at_ms) : memory.context?.is_event ? '事件记忆' : kinds[memory.kind] ?? memory.kind}</span>
          {memory.pinned && <span className="badge">重要</span>}
          {memory.context && memory.context.state !== 'active' && <span className="badge">{states[memory.context.state]}</span>}
          {memory.owner_locked && tab === 'memories' && <span className="badge">手动保护</span>}
        </div>
        {tab === 'shared' ? <h3>{Array.from(memory.text.split(/[。！？\n]/, 1)[0]).slice(0, 28).join('')}</h3> : <p>{memory.text}</p>}
        {tab === 'shared' && <p>{memory.text}</p>}
        <details><summary>来源与时间</summary>
          <p>{memory.source_kind === 'chat' ? '从明确聊天整理' : memory.source_kind === 'life' ? '虚拟生活事件' : memory.source_kind === 'legacy' ? '旧版保留记忆' : '主人手动保存'}</p>
          {memory.source_excerpt && <blockquote>{memory.source_excerpt}</blockquote>}
          <p>创建/登记 · {dateTime(memory.created_at_ms)}<br />更新 · {dateTime(memory.updated_at_ms)}<br />最近引用 · {dateTime(memory.last_used_at_ms ?? null)}</p>
          {memory.context?.due_at_ms != null && <p>预计完成 · {dateTime(memory.context.due_at_ms)}</p>}
          <p>{memory.api_allowed ? '允许相关检索后用于聊天' : '仅本地查看'} · 重要性 {memory.importance_milli / 10}%</p>
        </details>
        <div className="button-row">
          <button disabled={busy} onClick={() => startEdit(memory)}>编辑</button>
          {tab === 'shared' && <button disabled={busy} onClick={() => void perform(() => native.editMemory(personaId, { ...existingEdit(memory), pinned: !memory.pinned }))}>{memory.pinned ? '取消收藏' : '收藏'}</button>}
          {onSource && memory.source_message_id && memory.source_excerpt && <button disabled={busy} onClick={() => onSource(memory.source_message_id!)}>查看来源聊天</button>}
          <button disabled={busy} onClick={() => void perform(() => native.deleteMemory(personaId, memory.id))}>删除</button>
        </div>
      </article>)}
    </div>
    {data && !currentRows.length && <p>{term ? '没有匹配的记忆。' : tab === 'shared' ? '还没有收录来自聊天的共同回忆。' : '当前人格还没有符合条件的长期记忆。'}</p>}
  </section>;
}
