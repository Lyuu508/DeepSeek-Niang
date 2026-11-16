import {notifyShare} from './notifications';
import {StrictMode,useCallback,useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import type {Bootstrap,ChatEvent,ChatFailure,ChatMessage,ChatSettings,Diagnostics,CompanionSnapshot} from '@deepseek/contracts';
import {failure,native,nativeAvailable,runtimeMode} from './native';
import {previewSettings} from './defaults';
import './style.css';
import {Avatar} from './Avatar';
import {SyncPanel} from './SyncPanel';
import {MobileNav} from './MobileNav';
import {syncAPI} from './sync-native';
import {PersonaManager} from './PersonaManager';
import {CompanionPanel} from './CompanionPanel';
import {CompanionRail} from './CompanionRail';
import {MemoryPanel} from './MemoryPanel';
import {brandingReady} from './branding';

const terminal=(m:ChatMessage)=>['complete','failed','interrupted'].includes(m.state);
function merge(rows:ChatMessage[],incoming:ChatMessage):ChatMessage[]{
 const old=rows.find(m=>m.id===incoming.id);
 if(old&&(old.revision>incoming.revision||(terminal(old)&&!terminal(incoming))))return rows;
 // Regeneration only deselects old replies. A delayed read cannot select one again.
 if(old&&!old.selected&&incoming.role==='assistant'&&incoming.selected)incoming={...incoming,selected:false};
 return old?rows.map(m=>m.id===incoming.id?incoming:m):[...rows,incoming];
}
function time(value:number|null){return value===null?'早期记录':new Intl.DateTimeFormat(undefined,{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}).format(value);}
function SettingsPanel({boot,onRefresh,onError,compact=false}:{compact?:boolean;boot:Bootstrap|null;onRefresh:()=>Promise<void>;onError:(e:ChatFailure|null)=>void}){
 const [settings,setSettings]=useState<ChatSettings>({...boot?.settings??previewSettings});
 const [key,setKey]=useState('');const [busy,setBusy]=useState(false);const [notice,setNotice]=useState('');
 async function perform(work:()=>Promise<string>){setBusy(true);setNotice('');onError(null);try{setNotice(await work());await onRefresh();}catch(e){onError(failure(e));}finally{setBusy(false);setKey('');}}
 const change=<K extends keyof ChatSettings>(name:K,value:ChatSettings[K])=>setSettings(s=>({...s,[name]:value}));
 return <div className={`settings-content ${compact?'rail-settings':''}`}>
  <div className="page-heading"><span className="eyebrow">MAKE IT YOURS</span><h1>让陪伴更合心意</h1><p>人设和聊天偏好保存在本机。API Key 使用独立的系统安全存储。</p></div>
  {notice&&<div className="notice" role="status">{notice}</div>}
  <section className="settings-card"><div className="card-heading"><span className="card-icon">⌘</span><div><h2>DeepSeek API</h2><p>直接连接官方服务，无云端中转。</p></div><span className={`badge ${boot?.key_status.configured?'success':''}`}>{boot?.key_status.configured?'已配置密钥':'未配置密钥'}</span></div>
   <div className="storage-status"><span>本机存储</span><strong>{boot?.key_status.backend??'原生安全存储未连接'}</strong></div>
   {boot?.key_status.error&&<p className="inline-error">{boot.key_status.error.message}</p>}
   <form onSubmit={e=>{e.preventDefault();void perform(async()=>{await native.setKey(key.trim());return runtimeMode==='mock'?'模拟测试：已更新配置标记，未保存真实凭据。':'密钥已保存并通过系统存储读取核对。';});}}>
    <label htmlFor="api-key">API Key</label><div className="key-row"><input id="api-key" type="password" value={key} onChange={e=>setKey(e.target.value)} disabled={!nativeAvailable||busy} autoComplete="off" autoCapitalize="none" spellCheck={false} placeholder={runtimeMode==='mock'?'只输入 fixture- 开头的测试文本，勿填真实密钥':'输入或替换本机 DeepSeek API Key'}/><button className="primary" disabled={!nativeAvailable||busy||!key.trim()}>保存密钥</button></div>
    <p className="field-help">本设备的所有人格共用这份密钥；不会写入聊天数据库或普通同步日志。电脑出码、手机主动扫码时，可通过加密连接配置手机。保存后输入框会清空。</p>
   </form>
   <div className="button-row"><button disabled={!nativeAvailable||busy||!boot?.key_status.configured} onClick={()=>void perform(async()=>{await native.deleteKey();return '本机密钥已删除。';})}>删除密钥</button><button disabled={!nativeAvailable||busy} onClick={()=>void perform(async()=>{const backend=await native.selfTestVault();return runtimeMode==='mock'?'模拟测试完成；未验证 Windows 或 Android 安全存储。':`${backend} 写入、读取、删除自测通过。`;})}>验证安全存储</button><small>替换或删除密钥会先停止正在生成的回复。</small></div>
  </section>
  <form onSubmit={e=>{e.preventDefault();void perform(async()=>{if(!boot)throw new Error();await native.saveSettings(boot.active_persona_id,settings);return '聊天与人设设置已保存，新消息将使用这些设置。';});}}>
   <section className="settings-card"><div className="card-heading"><span className="card-icon">✧</span><div><h2>她的聊天风格</h2><p>保留稳定的基础性格，称呼和表达由你调整。</p></div></div>
    <div className="two-fields"><div><label htmlFor="character-name">角色名称</label><input id="character-name" value={settings.character_name} maxLength={32} disabled={!nativeAvailable||busy} onChange={e=>change('character_name',e.target.value)}/></div><div><label htmlFor="user-title">如何称呼你</label><input id="user-title" value={settings.user_title} maxLength={32} disabled={!nativeAvailable||busy} onChange={e=>change('user_title',e.target.value)}/></div></div>
    <label htmlFor="personality">人物性格</label><textarea id="personality" className="personality" value={settings.personality} maxLength={3000} disabled={!nativeAvailable||busy} onChange={e=>change('personality',e.target.value)}/>
    <p className="field-help">关系阶段与未过期情绪已参与聊天上下文，保留稳定性格和合理边界；可在“关系与日常”中查看来源与设置。</p>
   </section>
   <section className="settings-card"><div className="card-heading"><span className="card-icon">≋</span><div><h2>生成偏好</h2><p>更改设置不会改变已经发出的请求。</p></div></div>
    <div className="two-fields"><div><label htmlFor="model">模型</label><select id="model" value={settings.model} disabled={!nativeAvailable||busy} onChange={e=>change('model',e.target.value)}><option value="deepseek-flash">DeepSeek Flash</option><option value="deepseek-v4-pro">DeepSeek V4 Pro</option></select></div><div><label htmlFor="max-tokens">单次最大输出 Token</label><input id="max-tokens" type="number" min={256} max={8192} step={256} disabled={!nativeAvailable||busy} value={settings.max_tokens} onChange={e=>change('max_tokens',Number(e.target.value))}/></div><div><label htmlFor="temperature">表达温度 · {settings.temperature.toFixed(1)}</label><input id="temperature" type="range" min={0} max={2} step={.1} disabled={!nativeAvailable||busy} value={settings.temperature} onChange={e=>change('temperature',Number(e.target.value))}/></div><div><label htmlFor="timeout">请求超时（秒）</label><input id="timeout" type="number" min={15} max={300} disabled={!nativeAvailable||busy} value={settings.timeout_seconds} onChange={e=>change('timeout_seconds',Number(e.target.value))}/></div></div>
    <label className="check-row"><input type="checkbox" checked={settings.include_history} disabled={!nativeAvailable||busy} onChange={e=>change('include_history',e.target.checked)}/><span>发送当前人格的近期上下文</span></label>
    <button type="button" className="quiet-button" onClick={()=>change('history_turns',6)}>节省 Token：近期 6 轮 + 按需找回旧聊</button><label htmlFor="history-turns">最多保留轮数</label><input id="history-turns" type="number" min={1} max={40} value={settings.history_turns} disabled={!nativeAvailable||busy||!settings.include_history} onChange={e=>change('history_turns',Number(e.target.value))}/>
    <p className="field-help">发送范围为人设、本次输入与当前人格的近期聊天；不会上传整个数据库或其他人格；旧聊天与长期记忆按话题共用固定检索预算，可在陪伴设置关闭。</p>
    <button className="primary" disabled={!nativeAvailable||busy}>保存聊天设置</button>
   </section>
  </form>
 </div>;
}
function StoragePanel({personaId,onError,onRefresh}:{personaId:string;onError:(e:ChatFailure|null)=>void;onRefresh:()=>Promise<void>}){
 const [diag,setDiag]=useState<Diagnostics>();
 const refresh=useCallback(async()=>{if(!nativeAvailable)return;try{setDiag(await native.diagnostics());}catch(e){onError(failure(e));}},[onError]);
 useEffect(()=>{void refresh();},[refresh]);
 return <div className="settings-content"><div className="page-heading"><h1>记忆与本地数据</h1><p>每个人格的记忆独立保存，配对后使用同一身份同步。</p></div><MemoryPanel personaId={personaId} onError={onError} onRefresh={onRefresh}/><section className="settings-card"><div className="card-heading"><h2>数据库诊断</h2><button disabled={!nativeAvailable} onClick={()=>void refresh()}>刷新</button></div>{diag?<dl><dt>SQLite / 迁移版本</dt><dd>{diag.sqlite_version} / {diag.schema_version}</dd><dt>完整性</dt><dd>{diag.integrity}</dd><dt>整个本地档案：消息 / 记忆 / 事件</dt><dd>{diag.message_count} / {diag.memory_count} / {diag.event_count}</dd></dl>:<p>请通过原生客户端读取本地数据库。</p>}</section></div>;
}
function App(){
 const [boot,setBoot]=useState<Bootstrap|null>(null);
 const [companion,setCompanion]=useState<CompanionSnapshot|null>(null);
 const [page,setPage]=useState<'chat'|'settings'|'storage'|'personas'|'sync'|'companion'>('chat');
 const handledShares=useRef(new Set<string>());
 useEffect(()=>{if(!companion||companion.persona_id!==boot?.active_persona_id||runtimeMode!=='native')return;for(const share of companion.life.proactive_messages){if(share.state!=='delivered'||share.read_at_ms)continue;const key=companion.persona_id+'/'+share.id;if(handledShares.current.has(key))continue;if(page==='chat'&&document.visibilityState==='visible'){handledShares.current.add(key);void native.shareReceipt(companion.persona_id,share.id,true,share.notification_state??'pending').catch(()=>handledShares.current.delete(key));}else if(companion.preferences.notifications_enabled){handledShares.current.add(key);void notifyShare(companion,share.id,'DeepSeek娘 · 新分享').catch(()=>handledShares.current.delete(key));}}},[companion,page,boot?.active_persona_id]);

 const [allMessages,setAllMessages]=useState<Record<string,ChatMessage[]>>({});
 const [drafts,setDrafts]=useState<Record<string,string>>({});
 const [error,setError]=useState<ChatFailure|null>(null);const [loading,setLoading]=useState(false);
 const [active,setActive]=useState<{request:string;conversation:string;started:boolean;detached?:boolean}|null>(null);
 const activeRef=useRef(active);const initialized=useRef(false);const refreshSequence=useRef(0);const switchLock=useRef(false);const eventEpoch=useRef<Record<string,number>>({});
 const [drawer,setDrawer]=useState(false);const [switching,setSwitching]=useState(false);
 const [dark,setDark]=useState(matchMedia('(prefers-color-scheme:dark)').matches);const [showHistory,setShowHistory]=useState(false);
 const bottom=useRef<HTMLDivElement>(null);const scroll=useRef<HTMLDivElement>(null);const nearBottom=useRef(true);const input=useRef<HTMLTextAreaElement>(null);
 const settings=boot?.settings??previewSettings;
 const persona=boot?.personas.find(p=>p.id===boot.active_persona_id);
 const conversation=persona?.main_conversation_id??'';
 const refresh=useCallback(async()=>{
  if(!nativeAvailable)return;const sequence=++refreshSequence.current;const data=await native.bootstrap();if(sequence!==refreshSequence.current)return;setBoot(data);
  const companionData=await native.companion(data.active_persona_id);if(sequence!==refreshSequence.current)return;setCompanion(companionData);
  if(!initialized.current){initialized.current=true;if(data.active_request&&data.active_conversation){const restored={request:data.active_request,conversation:data.active_conversation,started:true,detached:true};activeRef.current=restored;setActive(restored);}}
 },[]);
 useEffect(()=>{void refresh().catch(e=>setError(failure(e)));},[refresh]);
 useEffect(()=>{
  if(runtimeMode!=='native')return;let stopped=false;let last='';
  const timer=setInterval(()=>{void syncAPI.status().then(async status=>{
   const digest=JSON.stringify(status.peers.map(p=>[p.id,p.last_success_ms,p.pending,p.progress.state,p.progress.error,p.api_configuration]));
   if(!last){last=digest;return;}if(digest===last)return;last=digest;
   const epoch=eventEpoch.current[conversation]??0;await refresh();
   if(conversation&&!activeRef.current){const messages=await native.messages(conversation);if(!stopped&&epoch===(eventEpoch.current[conversation]??0))setAllMessages(old=>({...old,[conversation]:messages}));}
  }).catch(()=>{});},3000);return()=>{stopped=true;clearInterval(timer);};
 },[conversation,refresh]);
 useEffect(()=>{if(runtimeMode!=='native')return;let closed=false;const timer=setInterval(()=>{const epoch=eventEpoch.current[conversation]??0;void refresh().then(async()=>{if(conversation&&!activeRef.current){const messages=await native.messages(conversation);if(!closed&&epoch===(eventEpoch.current[conversation]??0))setAllMessages(old=>({...old,[conversation]:messages}));}}).catch(()=>{});},15000);return()=>{closed=true;clearInterval(timer);};},[conversation,refresh]);
 useEffect(()=>{const viewport=window.visualViewport;const resize=()=>document.documentElement.style.setProperty('--viewport-height',`${viewport?.height??window.innerHeight}px`);resize();viewport?.addEventListener('resize',resize);window.addEventListener('resize',resize);return()=>{viewport?.removeEventListener('resize',resize);window.removeEventListener('resize',resize);};},[]);
 useEffect(()=>{
  if(!conversation||!nativeAvailable)return;let stale=false;const epoch=eventEpoch.current[conversation]??0;setLoading(true);nearBottom.current=true;setShowHistory(false);
  void native.messages(conversation).then(rows=>{if(!stale)setAllMessages(old=>({...old,[conversation]:epoch===(eventEpoch.current[conversation]??0)?rows:(old[conversation]??[]).reduce(merge,rows)}));}).catch(e=>{if(!stale)setError(failure(e));}).finally(()=>{if(!stale)setLoading(false);});return()=>{stale=true;};
 },[conversation]);
 const rows=allMessages[conversation]??[];const visible=useMemo(()=>rows.filter(m=>m.role==='user'||m.selected||showHistory),[rows,showHistory]);
 useLayoutEffect(()=>{if(nearBottom.current)bottom.current?.scrollIntoView({block:'end'});},[visible,loading]);
 useEffect(()=>{
  if(!active?.detached)return;let stale=false;const job=active;
  const poll=async()=>{try{const [data,messages]=await Promise.all([native.bootstrap(),native.messages(job.conversation)]);if(stale)return;setAllMessages(old=>({...old,[job.conversation]:messages}));if(data.active_request!==job.request&&activeRef.current?.request===job.request){activeRef.current=null;setActive(null);void refresh();}}catch(e){if(!stale)setError(failure(e));}};
  void poll();const timer=setInterval(()=>void poll(),1000);return()=>{stale=true;clearInterval(timer);};
 },[active?.request,active?.detached,refresh]);
 const requestIsActive=(id:string)=>activeRef.current?.request===id;
 const updateActive=(value:typeof active)=>{activeRef.current=value;setActive(value);};
 const onEvent=(event:ChatEvent)=>{
  const thread=event.type==='started'?event.request.conversation_id:event.conversation_id;
  eventEpoch.current[thread]=(eventEpoch.current[thread]??0)+1;
  if(event.type==='started'){
   setAllMessages(old=>{let list=(old[event.request.conversation_id]??[]).map(m=>m.user_message_id===event.request.user_message_id?{...m,selected:false}:m);list=merge(merge(list,event.user),event.message);return {...old,[event.request.conversation_id]:list};});
   if(activeRef.current?.request===event.request.request_id)updateActive({...activeRef.current,started:true});
  }else if(event.type==='delta'){
   setAllMessages(old=>({...old,[event.conversation_id]:(old[event.conversation_id]??[]).map(m=>m.id===event.message_id&&m.revision<=event.revision&&!terminal(m)?{...m,content:event.content,revision:event.revision,state:'generating'}:m)}));
  }else{
   if(event.type==='finished')setAllMessages(old=>({...old,[event.conversation_id]:merge(old[event.conversation_id]??[],event.message)}));
   if(event.type==='failed')setError(event.error);else if(event.error&&event.error.code!=='CANCELLED')setError(event.error);
   if(activeRef.current?.request===event.request_id)updateActive(null);void refresh().catch(e=>setError(failure(e)));
  }
 };
 async function switchPersona(id:string){
  if(switchLock.current)return;switchLock.current=true;setSwitching(true);setError(null);
  try{await native.selectPersona(id);await refresh();setPage('chat');setDrawer(false);}catch(e){setError(failure(e));}finally{switchLock.current=false;setSwitching(false);}
 }
 async function send(regenerateUserId?:string){
  if(!conversation||activeRef.current||!boot?.key_status.configured||switching)return;
  const text=drafts[conversation]??'';if(!regenerateUserId&&!text.trim())return;
  const request=crypto.randomUUID();const selected=conversation;setError(null);nearBottom.current=true;updateActive({request,conversation:selected,started:false});
  if(!regenerateUserId)setDrafts(old=>({...old,[selected]:''}));
  try{await native.startChat({request_id:request,conversation_id:selected,text:regenerateUserId?null:text,regenerate_user_id:regenerateUserId??null},onEvent);}
  catch(e){setError(failure(e));if(requestIsActive(request)){updateActive(null);if(!regenerateUserId)setDrafts(old=>({...old,[selected]:old[selected]||text}));}}
 }
 async function stopCurrent(){const job=activeRef.current;if(!job)return;try{await native.stop(job.request);const list=await native.messages(job.conversation);setAllMessages(old=>({...old,[job.conversation]:list}));if(activeRef.current?.request===job.request)updateActive(null);await refresh();}catch(e){setError(failure(e));}}
 const lastUser=[...rows].reverse().find(m=>m.role==='user');const variants=rows.filter(m=>m.role==='assistant'&&!m.selected).length;
 const mobile=boot?.platform==='android'||(!boot&&/Android/i.test(navigator.userAgent))||(runtimeMode!=='native'&&matchMedia('(pointer:coarse)').matches);
 const connected=nativeAvailable&&!!boot;const canSend=connected&&!!conversation&&boot.key_status.configured&&!active&&!loading&&!switching;
 const title=page==='companion'?'关系与日常':page==='sync'?(mobile?'设备连接':'局域网同步'):page==='settings'?'设置':page==='storage'?'本地数据':page==='personas'?'人格管理':persona?.name??'DeepSeek娘';
 const currentCompanion=companion?.persona_id===boot?.active_persona_id?companion:null;
 return <div className="app" data-theme={dark?'dark':'light'} data-platform={mobile?'android':'desktop'}>
  <header><button className="menu-toggle" aria-label="打开侧栏" onClick={()=>setDrawer(true)}><span className="desktop-menu-symbol">☰</span><span className="phone-menu-avatar"><Avatar persona={persona} small/></span></button>{page==='chat'&&<div className="mobile-persona-avatar"><Avatar persona={persona} small/></div>}<div className="header-title"><strong>{title}</strong><small>{mobile?(page==='chat'?'你的专属聊天空间':'DeepSeek娘'):page==='chat'?'一直延续的聊天时间线':'DeepSeek娘'}</small></div><div className="header-actions"><span className={`connection-badge ${boot?.key_status.configured?'ready':''}`}><i/>{boot?.key_status.configured?'密钥已配置':'待配置密钥'}</span><button className="icon-button" aria-label="切换深浅色" onClick={()=>setDark(!dark)}>{dark?'☀':'☾'}</button></div></header>
  <div className="workspace-body">
  {drawer&&<button className="drawer-backdrop" aria-label="关闭侧栏" onClick={()=>setDrawer(false)}/>}
  <aside className={drawer?'open':''}>
   <a className="brand" href="#" onClick={e=>{e.preventDefault();setPage('chat');setDrawer(false);}}><Avatar persona={boot?.personas.find(p=>p.is_builtin)} small/><span>DeepSeek娘<small>AI COMPANION</small></span></a>
   <div className="current-persona"><Avatar persona={persona}/><strong>{persona?.name??'DeepSeek娘'}</strong><small>永久聊天空间</small></div>
   <div className="sidebar-label">切换人格 <span>{boot?.personas.length??'—'}</span></div>
   <nav className="persona-list" aria-label="人格切换">{boot?.personas.map(p=><button key={p.id} data-persona-id={p.id} aria-current={boot.active_persona_id===p.id?'true':undefined} disabled={switching} className={boot.active_persona_id===p.id?'selected':''} onClick={()=>void switchPersona(p.id)}><Avatar persona={p} small/><span>{p.name}</span>{active?.conversation===p.main_conversation_id&&<i className="working-dot"/>}</button>)}</nav>
   <button className="add-persona mobile-add-persona" disabled={!connected} onClick={()=>{setPage('personas');setDrawer(false);}}>＋ 新增人格</button>
   <div className="sidebar-bottom"><button aria-label="关系与日常" className={`sidebar-companion-nav ${page==='companion'?'selected':''}`} onClick={()=>{setPage('companion');setDrawer(false);}}>♡ 关系与日常</button><button aria-label="人格管理" className={page==='personas'?'selected':''} onClick={()=>{setPage('personas');setDrawer(false);}}>♧ 人格管理</button><button aria-label="局域网同步" className={page==='sync'?'selected':''} onClick={()=>{setPage('sync');setDrawer(false);}}>⇄ 局域网同步</button><button aria-label="本地数据" className={page==='storage'?'selected':''} onClick={()=>{setPage('storage');setDrawer(false);}}>▤ 本地数据</button><button aria-label="设置" className={page==='settings'?'selected':''} onClick={()=>{setPage('settings');setDrawer(false);}}>⚙ 设置</button><div className="sidebar-profile"><span className="user-dot">你</span><div>本地个人空间<small>{boot?.version??'本地开发版'}</small></div><span className="private-dot" title="未启用云数据库"/></div></div>
  </aside>
  <main>
   {import.meta.env.VITE_COMPANION_TEST_PACKAGE==='1'&&<div className="test-build-banner notice" role="status">{mobile?'独立测试版 · 不覆盖正式版数据':'独立测试版 · 与正式版数据隔离，不用于覆盖正式版。请使用合成测试资料验收。'}</div>}
   {runtimeMode==='mock'&&<div className="mode-banner mock" role="status">模拟验收模式 · 无真实 DeepSeek API、SQLite 或系统密钥存储 · 请勿输入真实密钥</div>}
   {runtimeMode==='preview'&&<div className="mode-banner" role="status">浏览器外观预览 · 原生功能未连接，请通过 Windows 或 Android 应用聊天</div>}
   {!brandingReady&&<div className="branding-pending">已指定海滩角色图；原始 PNG 文件待导入，暂不使用替代人物图标。</div>}
   {error&&<div className="error-banner" role="alert"><span>{error.message}<small>{error.code}</small></span><button aria-label="关闭错误提示" onClick={()=>setError(null)}>×</button></div>}
   {page==='companion'?<div className="page-scroll"><CompanionPanel key={boot?.active_persona_id} data={currentCompanion} onRefresh={refresh} onError={setError} onMemories={()=>setPage('storage')}/></div>:page==='sync'?<div className="page-scroll"><SyncPanel mobile={mobile} onPaired={refresh}/></div>:page==='settings'?<div className="page-scroll"><SettingsPanel key={boot?.active_persona_id??'loading'} boot={boot} onRefresh={refresh} onError={setError}/></div>:page==='storage'?<div className="page-scroll"><StoragePanel key={boot?.active_persona_id??'loading'} personaId={boot?.active_persona_id??''} onError={setError} onRefresh={refresh}/></div>:page==='personas'?<div className="page-scroll"><PersonaManager boot={boot} onSwitch={switchPersona} onRefresh={refresh} onError={setError} onSettings={()=>setPage('settings')}/></div>:<div className="chat-layout"><section className="chat-main">
    <div className="message-scroll" ref={scroll} onScroll={()=>{const n=scroll.current;if(n)nearBottom.current=n.scrollHeight-n.scrollTop-n.clientHeight<100;}}>
     {!visible.length&&!loading&&<div className="welcome"><span className="welcome-label">ONE CHARACTER · ONE SHARED STORY</span><Avatar persona={persona}/><h1>和{persona?.name??'DeepSeek娘'}，接着聊</h1><p>每一次问候，都留在同一条时间线。<br/>下次回来，我们从这里继续。</p><div className="suggestions">{['聊聊今天的小事','一起想一个新点子','今天想学点什么？'].map(s=><button key={s} disabled={!connected} onClick={()=>{setDrafts(old=>({...old,[conversation]:s}));input.current?.focus();}}>{s}<span>↗</span></button>)}</div></div>}
     {loading&&<div className="loading-history" role="status">正在读取人格时间线…</div>}
     <div className="messages" aria-label="永久聊天时间线">{visible.map(m=><article className={`message ${m.role==='user'?'user':'assistant'} ${!m.selected?'old-variant':''}`} key={m.id} data-message-id={m.id} data-state={m.state}>
      {m.role!=='user'&&<Avatar persona={persona} small/>}<div className="message-body"><div className="message-meta"><strong>{m.role==='user'?'你':persona?.name??settings.character_name}</strong><time>{time(m.created_at_ms)}</time>{!m.selected&&<span>历史回复</span>}</div><div className={`bubble ${m.state==='failed'?'failed':''}`}>{m.content||(['sending','generating'].includes(m.state)?<span className="typing" aria-label="正在等待回复"><i/><i/><i/></span>:m.state==='failed'?'本次未收到回复。':'生成已中断，尚未收到正文。')}{m.state==='generating'&&<span className="stream-caret"/>}</div><div className="message-foot">{m.state==='sending'?'发送中 · 等待模型':m.state==='generating'?'正在生成':m.state==='interrupted'?(m.error_code==='APP_RESTARTED'?'应用退出时中断':'已停止 / 中断'):m.state==='failed'?'生成失败':'已保存本地'}{m.finish_reason==='length'&&<span> · 达到输出 Token 上限</span>}{m.role==='assistant'&&m.selected&&m.user_message_id===lastUser?.id&&terminal(m)&&<button disabled={!!active||!boot?.key_status.configured} onClick={()=>void send(m.user_message_id!)}>↻ {m.state==='failed'?'重试':'重新生成'}</button>}</div></div>{m.role==='user'&&<span className="user-avatar">你</span>}
     </article>)}</div>
     {variants>0&&<button className="variant-toggle" onClick={()=>setShowHistory(!showHistory)}>{showHistory?'收起历史回复':`查看历史回复（${variants}）`}</button>}<div ref={bottom}/>
    </div>
    <div className="composer-area">{!boot?.key_status.configured&&<button className="setup-hint" onClick={()=>setPage('settings')}>配置本机 DeepSeek API Key，开始聊天 <span>前往设置 →</span></button>}<form className="composer" onSubmit={e=>{e.preventDefault();void send();}}><label className="sr-only" htmlFor="chat-input">发送给当前人格的消息</label><textarea id="chat-input" ref={input} value={drafts[conversation]??''} maxLength={8000} placeholder={connected?`和${persona?.name??'她'}说说吧…`:'原生应用连接后即可聊天'} disabled={!connected||!conversation||loading||switching} onChange={e=>setDrafts(old=>({...old,[conversation]:e.target.value}))} onKeyDown={e=>{if(e.key==='Enter'&&!e.shiftKey&&!mobile&&!e.nativeEvent.isComposing&&e.keyCode!==229){e.preventDefault();void send();}}}/><div className="composer-toolbar"><span>{mobile?'点击发送 · 支持中文输入':'Enter 发送 · Shift + Enter 换行'}</span>{active?<button type="button" className="stop-button" disabled={!active.started} onClick={()=>void stopCurrent()}>■ 停止{active.conversation!==conversation?'其他人格的':''}生成</button>:<button className="send-button" aria-label="发送消息" disabled={!canSend||!(drafts[conversation]??'').trim()}>发送 ↑</button>}</div></form><p className="privacy-line">当前人格设定 + 本次输入{settings.include_history?` + 最近 ${settings.history_turns} 轮聊天`:''}将发送至 DeepSeek；其他人格和整个数据库不会发送。相关记忆引用可在陪伴设置中关闭。</p></div>
   </section>{!mobile&&<CompanionRail key={boot?.active_persona_id} boot={boot} data={currentCompanion} onRefresh={refresh} onError={setError} onMemories={()=>setPage('storage')} settingsContent={<SettingsPanel compact boot={boot} onRefresh={refresh} onError={setError}/>}/>}</div>}
  </main>
  </div>
  <MobileNav page={page} onSelect={destination=>{setPage(destination);setDrawer(false);}}/>
 </div>;
}
createRoot(document.getElementById('root')!).render(<StrictMode><App/></StrictMode>);
