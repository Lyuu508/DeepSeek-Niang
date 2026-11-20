import { isTauri } from '@tauri-apps/api/core';
import { createRoot } from 'react-dom/client';
import { useCallback, useEffect, useRef, useState, type PointerEvent } from 'react';
import { desktopPet, petError, type PetAction, type PetMotion, type PetPose, type PetSnapshot } from './api';
import standing from './media/standing.png';
import idle from './media/idle.webp';
import walk from './media/walk.webp';
import wave from './media/wave.webp';
import './pet.css';

type Pane = 'activity' | 'chat' | 'menu';
const sprites = { standing, idle, walk, wave };
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
const waveDuration = 4900;
const actionLabel: Record<string, string> = {
  queued: '等待执行', running: '进行中', paused: '已暂停', completed: '已完成',
  cancelled: '已取消', failed: '已失败', uncertain: '待核对',
};
const poseGlyph: Partial<Record<PetPose, string>> = {
  happy: '✧', shy: '♡', thinking: '…', annoyed: '!', replying: '···', resting: 'Z', reading: '▤',
};
const time = (value: number) => new Date(value).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });

function currentStatus(state: PetSnapshot | null, offline: boolean): string {
  if (offline) return '本机状态暂不可用';
  if (!state) return '读取桌宠状态…';
  if (state.chat === 'thinking') return '正在思考';
  if (state.chat === 'replying') return '正在生成回复';
  if (state.unread_share) return '有一条新分享';
  if (state.activity?.stale) return '活动状态已过期';
  if (state.activity) return `${state.activity.label} · ${actionLabel[state.activity.state] ?? '已记录'}`;
  return '待机';
}

function Pet() {
  const [snapshot, setSnapshot] = useState<PetSnapshot | null>(null);
  const [pane, setPane] = useState<Pane | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [error, setError] = useState('');
  const [offline, setOffline] = useState(false);
  const [visible, setVisible] = useState(document.visibilityState === 'visible');
  const [sending, setSending] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [waving, setWaving] = useState(false);
  const [systemReduced, setSystemReduced] = useState(reducedMotion.matches);
  const [motion, setMotion] = useState<PetMotion>({ walking: false, facing_left: true });
  const [speechLevel, setSpeechLevel] = useState(0);
  const requestRunning = useRef(false);
  const tap = useRef<number | undefined>(undefined);
  const waveTimer = useRef<number | undefined>(undefined);
  const lastState = useRef<PetSnapshot | null>(null);
  const layoutChanging = useRef(false);
  const hoverQueue = useRef(Promise.resolve());
  const press = useRef<{ x: number; y: number; pointerId: number } | null>(null);
  const dragged = useRef(false);

  const apply = useCallback((state: PetSnapshot) => {
    lastState.current = state;
    setSnapshot(state);
    setOffline(false);
  }, []);
  const refresh = useCallback(async () => apply(await desktopPet.snapshot()), [apply]);
  useEffect(() => {
    let closed = false;
    let timer: number | undefined;
    async function poll() {
      if (closed || document.hidden) return;
      if (!requestRunning.current) {
        requestRunning.current = true;
        try {
          const state = await desktopPet.snapshot();
          if (!closed && !document.hidden) apply(state);
        } catch {
          if (!closed) setOffline(true);
        } finally {
          requestRunning.current = false;
        }
      }
      if (!closed && !document.hidden) {
        const active = lastState.current?.chat !== 'idle';
        timer = window.setTimeout(poll, active ? 500 : 2200);
      }
    }
    function onVisibility() {
      window.clearTimeout(timer);
      setVisible(!document.hidden);
      if (!document.hidden) void poll();
    }
    function onSystemMotion() {
      setSystemReduced(reducedMotion.matches);
      void desktopPet.action({ kind: 'system_reduced_motion', value: reducedMotion.matches }).catch(() => {});
    }
    void desktopPet.watchMotion(value => { if (!closed) setMotion(value); }).catch(err => {
      if (!closed) setError(petError(err));
    });
    onSystemMotion();
    reducedMotion.addEventListener('change', onSystemMotion);
    document.addEventListener('visibilitychange', onVisibility);
    void desktopPet.action({ kind: 'panel', value: false }).catch(() => {});
    void poll();
    return () => {
      closed = true;
      window.clearTimeout(timer);
      window.clearTimeout(tap.current);
      window.clearTimeout(waveTimer.current);
      reducedMotion.removeEventListener('change', onSystemMotion);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [apply]);

  async function perform(action: PetAction) {
    window.clearTimeout(tap.current);
    setError('');
    try {
      await desktopPet.action(action);
      if (['toggle_top', 'toggle_roam', 'scale_step', 'reset_position'].includes(action.kind)) await refresh();
    } catch (err) { setError(petError(err)); }
  }

  async function display(next: Pane | null) {
    window.clearTimeout(tap.current);
    if (layoutChanging.current) return;
    layoutChanging.current = true;
    setError('');
    try {
      if (!!pane !== !!next) await desktopPet.action({ kind: 'panel', value: next !== null });
      setPane(next);
    } catch (err) {
      setError(petError(err));
    } finally {
      layoutChanging.current = false;
    }
  }

  const persona = snapshot?.persona_id ?? '';
  useEffect(() => {
    setSpeechLevel(0);
    if (!persona || offline || !visible) return;
    let alive = true;
    let timer: number | undefined;
    async function update() {
      const level = await desktopPet.speechLevel(persona).catch(() => 0);
      if (!alive) return;
      setSpeechLevel(level);
      timer = window.setTimeout(update, level > 0 ? 105 : 300);
    }
    void update();
    return () => { alive = false; window.clearTimeout(timer); };
  }, [persona, visible, offline]);
  const draft = drafts[persona] ?? '';
  const reply = snapshot?.reply;
  const replyActive = reply?.phase === 'sending' || reply?.phase === 'streaming';
  async function send() {
    if (!persona || !draft.trim() || sending || offline || snapshot?.chat !== 'idle') return;
    setSending(true);
    setError('');
    try {
      await desktopPet.send(persona, crypto.randomUUID(), draft);
      setDrafts(old => ({ ...old, [persona]: '' }));
      await refresh();
    } catch (err) {
      setError(petError(err));
    } finally {
      setSending(false);
    }
  }

  async function stop(request: string) {
    try { await desktopPet.stop(request); await refresh(); }
    catch (err) { setError(petError(err)); }
  }

  function hover(value: boolean) {
    setHovered(value);
    hoverQueue.current = hoverQueue.current.then(() => desktopPet.action({ kind: 'hover', value })).catch(() => {});
  }

  function startPress(event: PointerEvent<HTMLButtonElement>) {
    if (!event.isPrimary || event.button !== 0) return;
    dragged.current = false;
    press.current = { x: event.clientX, y: event.clientY, pointerId: event.pointerId };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function dragCharacter(event: PointerEvent<HTMLButtonElement>) {
    const origin = press.current;
    if (!origin || origin.pointerId !== event.pointerId || !(event.buttons & 1)
      || Math.hypot(event.clientX - origin.x, event.clientY - origin.y) < 5) return;
    dragged.current = true;
    press.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    void perform({ kind: 'drag' });
  }

  const animated = visible && !systemReduced && !snapshot?.settings.reduce_motion;
  function greet() {
    if (!animated) {
      void display(pane === 'activity' ? null : 'activity');
      return;
    }
    void perform({ kind: 'interact' });
    setWaving(true);
    window.clearTimeout(waveTimer.current);
    waveTimer.current = window.setTimeout(() => setWaving(false), waveDuration);
  }

  const activity = snapshot?.activity;
  const pose = offline ? 'idle' : snapshot?.pose ?? 'idle';
  const name = snapshot?.name ?? 'DeepSeek娘';
  const controls = hovered || !!pane;
  const animation = !animated ? 'standing' : waving ? 'wave' : motion.walking && !hovered && !pane ? 'walk' : 'idle';
  return <main className="pet" data-pose={pose} data-motion={animated ? 'on' : 'off'} data-expanded={!!pane}
    onPointerEnter={() => hover(true)} onPointerLeave={() => hover(false)}
    onKeyDown={event => { if (event.key === 'Escape' && !event.nativeEvent.isComposing && pane) void display(null); }}
    onContextMenu={event => { if ((event.target as HTMLElement).closest('textarea')) return; event.preventDefault(); void display(pane === 'menu' ? null : 'menu'); }}>
    {controls && <button className="pet-status" type="button" onClick={() => void display(pane === 'activity' ? null : 'activity')}
      aria-label="查看桌宠状态" aria-expanded={pane === 'activity'}>
      <strong>{error && !pane ? '操作失败' : name}</strong>
      <span title={error || currentStatus(snapshot, offline)}>{error && !pane ? error : currentStatus(snapshot, offline)}</span>
    </button>}
    <button type="button" className="pet-face" aria-label="桌宠角色，双击打开聊天"
      title="按住拖动 · 双击打开聊天"
      onPointerDown={startPress} onPointerMove={dragCharacter}
      onPointerUp={() => { press.current = null; }} onPointerCancel={() => { press.current = null; }}
      onClick={event => {
        if (dragged.current && event.detail !== 0) return;
        if (event.detail === 0) { void perform({ kind: 'open_chat' }); return; }
        if (event.detail === 1) tap.current = window.setTimeout(greet, 280);
        else window.clearTimeout(tap.current);
      }}
      onDoubleClick={() => { if (!dragged.current) void perform({ kind: 'open_chat' }); }}>
      <span className="pet-character" data-facing={motion.facing_left ? 'left' : 'right'} data-animation={animation}>
        {snapshot?.builtin_avatar !== false
          ? <img className="pet-sprite" src={sprites[animation]} alt="" draggable={false} />
          : <span className="pet-initial">{Array.from(name).slice(0, 2).join('')}</span>}
      </span>
      {speechLevel > 0 && <span className="pet-voice" title="正在发声" aria-label="正在发声">
        {[0.65, 1, 0.84].map((scale, index) => <i key={index} style={{height: `${3 + (systemReduced ? 5 : scale * speechLevel * 0.14)}px`}}/>)}
      </span>}
      {(controls || pose === 'replying' || pose === 'thinking') && poseGlyph[pose]
        && <span className="pet-expression" aria-hidden="true">{poseGlyph[pose]}</span>}
      {!controls && snapshot?.unread_share && <span className="pet-share" title="有一条新分享" />}
    </button>
    {controls && <nav className="pet-tools" aria-label="桌宠操作">
      <button type="button" className="pet-drag" title="按住拖动" aria-label="拖动桌宠"
        onPointerDown={event => { if (event.button === 0) { event.preventDefault(); void perform({ kind: 'drag' }); } }}>⠿</button>
      <button type="button" onClick={() => void perform({ kind: 'open_chat' })}>聊天</button>
      <button type="button" onClick={() => void display(pane === 'chat' ? null : 'chat')} aria-expanded={pane === 'chat'}>快聊</button>
      <button type="button" title="桌宠菜单" aria-label="桌宠菜单" onClick={() => void display(pane === 'menu' ? null : 'menu')}
        aria-expanded={pane === 'menu'}>···</button>
    </nav>}
    {pane && <section className="pet-panel" aria-label={pane === 'activity' ? '活动详情' : pane === 'chat' ? '桌宠快捷聊天' : '桌宠菜单'}>
      <div className="pet-panel-title"><strong>{pane === 'activity' ? '她的状态' : pane === 'chat' ? `与${name}快聊` : '桌宠设置'}</strong>
        <button type="button" aria-label="折叠面板" onClick={() => void display(null)}>×</button></div>
      {error && <p role="alert" className="pet-error">{error}</p>}
      {pane === 'activity' && <>
        <p>心情 {offline ? '暂不可用' : snapshot?.mood_label ?? '待机'}</p>
        {activity ? <>
          <p>{activity.stale || offline ? '最后记录' : activity.is_virtual === false ? '本机视频任务' : '虚拟活动记录'} · {activity.label}</p>
          <progress aria-label="已记录的活动进度" value={activity.progress_milli} max={1000} />
          <p className="pet-muted">{activity.stale || offline ? '状态已过期' : actionLabel[activity.state] ?? '已记录'} · {time(activity.updated_at_ms)}</p>
        </> : <p className="pet-muted">暂无活动记录</p>}
        {snapshot?.unread_share && <button type="button" className="pet-accent" onClick={() => void perform({ kind: 'open_chat' })}>打开新分享</button>}
      </>}
      {pane === 'chat' && <>
        <form className="pet-form" onSubmit={event => { event.preventDefault(); void send(); }}>
          <textarea aria-label="桌宠快捷消息" rows={2} maxLength={500} placeholder="和她说一句…" value={draft} disabled={!persona || sending}
            onChange={event => setDrafts(old => ({ ...old, [persona]: event.target.value }))}
            onKeyDown={event => {
              if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing && event.keyCode !== 229) {
                event.preventDefault(); void send();
              }
            }} />
          <div className="pet-form-actions">
            {replyActive ? <button type="button" onClick={() => void stop(reply.request_id)}>停止</button>
              : <button className="pet-accent" disabled={offline || sending || !draft.trim() || snapshot?.chat !== 'idle'}>发送</button>}
            <small>{snapshot?.chat === 'busy' ? '另一个人格正在生成' : '同步写入这条聊天'}</small>
          </div>
        </form>
        {reply && <p className={reply.error ? 'pet-error' : 'pet-reply'} aria-live="polite">
          {reply.error?.message ?? (reply.preview || (replyActive ? '等待回复…' : reply.phase === 'interrupted' ? '已停止' : ''))}
        </p>}
      </>}
      {pane === 'menu' && <div className="pet-menu">
        <button type="button" disabled={snapshot?.settings.reduce_motion || systemReduced}
          onClick={() => void perform({ kind: 'toggle_roam' })}>{snapshot?.settings.roam ? '暂停走动' : '恢复走动'}</button>
        <button type="button" onClick={() => void perform({ kind: 'toggle_top' })}>{snapshot?.settings.always_on_top ? '取消置顶' : '置顶窗口'}</button>
        <div className="pet-scale">
          <button type="button" aria-label="缩小桌宠" onClick={() => void perform({ kind: 'scale_step', value: -1 })}>−</button>
          <span>{snapshot?.settings.scale_percent ?? 100}%</span>
          <button type="button" aria-label="放大桌宠" onClick={() => void perform({ kind: 'scale_step', value: 1 })}>＋</button>
        </div>
        <button type="button" onClick={() => void perform({ kind: 'reset_position' })}>找回位置</button>
        <button type="button" onClick={() => void perform({ kind: 'hide' })}>隐藏桌宠</button>
      </div>}
    </section>}
  </main>;
}

createRoot(document.getElementById('pet-root')!).render(isTauri() ? <Pet /> : <p className="pet-preview">请从 Windows 应用设置打开桌宠。</p>);
