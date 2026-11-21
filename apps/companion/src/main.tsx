import {listen} from '@tauri-apps/api/event';
import {invoke} from '@tauri-apps/api/core';
import {CloudVideoPanel} from './CloudVideoPanel';
import {PetSettingsCard} from './desktop-pet/PetSettingsCard';
import {notifyShare} from './notifications';
import {StrictMode,useCallback,useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import type {Bootstrap,ChatEvent,ChatFailure,ChatMessage,ChatSettings,Diagnostics,CompanionSnapshot} from '@deepseek/contracts';
import {failure,native,nativeAvailable,runtimeMode} from './native';
import {previewSettings} from './defaults';
import './design-tokens.css';
import './style.css';
import './interface.css';
import './button-motion.css';
import './ocean.css';
import './release-ui.css';
import { Art, Icon, PageHeader, WhaleMark, type IconName } from './OceanDesign';
import { AboutPage, AppearanceSettings, MessageText, useAppearance, type AppearanceControl } from './OceanPages';
import {installButtonMotion} from './button-motion';
import {FontSettings,useFontScale,type FontScale} from './appearance';
import {ChatComposer} from './ChatComposer';
import {ChatImagePreview,type ChatImage} from './ChatAttachment';
import {DialogueFields} from './DialogueFields';
import {PluginManager} from './plugins/PluginManager';
import {useReminders} from './plugins/useReminders';
import {Avatar} from './Avatar';
import {SyncPanel} from './SyncPanel';
import {MobileNav} from './MobileNav';
import {syncAPI} from './sync-native';
import {PersonaManager} from './PersonaManager';
import {CompanionPanel} from './CompanionPanel';
import {CompanionRail} from './CompanionRail';
import {MemoryPanel} from './MemoryPanel';
import {brandingReady} from './branding';
import {VoiceSettings} from './voice/VoiceControls';
import {CloudVoiceSettings} from './voice/CloudVoiceSettings';
import {useVoice,type VoiceController} from './voice/useVoice';

const terminal=(m:ChatMessage)=>['complete','failed','interrupted'].includes(m.state);
function merge(rows:ChatMessage[],incoming:ChatMessage):ChatMessage[]{
 const old=rows.find(m=>m.id===incoming.id);
 if(old&&(old.revision>incoming.revision||(terminal(old)&&!terminal(incoming))))return rows;
 // Regeneration only deselects old replies. A delayed read cannot select one again.
 if(old&&!old.selected&&incoming.role==='assistant'&&incoming.selected)incoming={...incoming,selected:false};
 return old?rows.map(m=>m.id===incoming.id?incoming:m):[...rows,incoming];
}
function time(value:number|null){return value===null?'早期记录':new Intl.DateTimeFormat(undefined,{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}).format(value);}
type AppPage = 'chat' | 'settings' | 'storage' | 'personas' | 'sync' | 'companion' | 'video' | 'plugins' | 'about';
const navigation: { page: AppPage; label: string; icon: IconName }[] = [
 {page:'chat',label:'聊天',icon:'chat'}, {page:'personas',label:'人格管理',icon:'persona'},
 {page:'companion',label:'关系与日常',icon:'bond'}, {page:'sync',label:'局域网同步',icon:'sync'},
 {page:'video',label:'视频中心',icon:'video'}, {page:'plugins',label:'插件管理',icon:'plugin'},
 {page:'storage',label:'本地数据',icon:'memory'}, {page:'settings',label:'设置',icon:'settings'},
 {page:'about',label:'关于',icon:'about'},
];

function SettingsPanel({boot,onRefresh,onError,voice,font,appearance,onNavigate,compact=false}:{compact?:boolean;appearance:AppearanceControl;onNavigate:(page:AppPage)=>void;boot:Bootstrap|null;voice:VoiceController;font:FontScale;onRefresh:()=>Promise<void>;onError:(e:ChatFailure|null)=>void}){
 const [settings,setSettings]=useState<ChatSettings>({...boot?.settings??previewSettings});
 const [selectedSection,setSelectedSection]=useState('appearance');
 const sections = useRef<HTMLDivElement>(null);
 const [key,setKey]=useState('');const [busy,setBusy]=useState(false);const [notice,setNotice]=useState('');
 async function perform(work:()=>Promise<string>){setBusy(true);setNotice('');onError(null);try{setNotice(await work());await onRefresh();}catch(e){onError(failure(e));}finally{setBusy(false);setKey('');}}
 const change=<K extends keyof ChatSettings>(name:K,value:ChatSettings[K])=>setSettings(s=>({...s,[name]:value}));
 return <div ref={sections} className={`settings-content ${compact?'rail-settings':''}`}>
  <PageHeader title="设置" icon="settings" art={compact?undefined:'wave'} scene={compact?undefined:'scene-settings'}
   description="管理本机外观、聊天与设备偏好。"/>
  {!compact && <nav className="settings-index" aria-label="设置分类">
   {([['appearance','外观','sun'],['api','API 配置','key'],['character','聊天与人设','persona'],
     ...(voice.enabled ? [['voice','语音','voice']] : []),
     ...(boot?.platform === 'windows' ? [['pet','桌宠','pet']] : [])] as [string,string,IconName][]).map(([id,label,icon]) =>
     <button key={id} aria-current={selectedSection===id?'location':undefined} onClick={()=>{
       setSelectedSection(id);
       sections.current?.querySelector('[data-setting="'+id+'"]')?.scrollIntoView({block:'start',behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth'});
     }}><Icon name={icon}/>{label}</button>)}
   <button onClick={()=>onNavigate('storage')}><Icon name="book"/>记忆</button>
   <button onClick={()=>onNavigate('sync')}><Icon name="sync"/>同步</button>
  </nav>}
  <div data-setting="appearance"><AppearanceSettings control={appearance}/><FontSettings font={font}/></div>
  {runtimeMode==='native'&&boot?.platform==='windows'&&<div data-setting="pet"><PetSettingsCard/></div>}
  {voice.enabled&&<div data-setting="voice"><VoiceSettings voice={voice} platform={boot?.platform??''}/>{runtimeMode==='native'&&boot?.active_persona_id&&
    <CloudVoiceSettings persona={boot.active_persona_id} voice={voice}/>}</div>}
  {notice&&<div className="notice" role="status">{notice}</div>}
  <section className="settings-card" data-setting="api"><div className="card-heading"><span className="card-icon"><Icon name="key"/></span><div><h2>DeepSeek API</h2><p>直接连接官方服务，无云端中转。</p></div><span className={`badge ${boot?.key_status.configured?'success':''}`}>{boot?.key_status.configured?'已配置密钥':'未配置密钥'}</span></div>
   <div className="storage-status"><span>本机存储</span><strong>{boot?.key_status.backend??'原生安全存储未连接'}</strong></div>
   {boot?.key_status.error&&<p className="inline-error">{boot.key_status.error.message}</p>}
   <form onSubmit={e=>{e.preventDefault();void perform(async()=>{await native.setKey(key.trim());return runtimeMode==='mock'?'模拟测试：已更新配置标记，未保存真实凭据。':'密钥已保存并通过系统存储读取核对。';});}}>
    <label htmlFor="api-key">API Key</label><div className="key-row"><input id="api-key" type="password" value={key} onChange={e=>setKey(e.target.value)} disabled={!nativeAvailable||busy} autoComplete="off" autoCapitalize="none" spellCheck={false} placeholder={runtimeMode==='mock'?'只输入 fixture- 开头的测试文本，勿填真实密钥':'输入或替换本机 DeepSeek API Key'}/><button className="primary" disabled={!nativeAvailable||busy||!key.trim()}>保存密钥</button></div>
    <p className="field-help">本设备的所有人格共用这份密钥；不会写入聊天数据库或普通同步日志。电脑出码、手机主动扫码时，可通过加密连接交接 DeepSeek、千问和语音设置。已有冲突配置由本机保留。保存后输入框会清空。</p>
   </form>
   <div className="button-row"><button disabled={!nativeAvailable||busy||!boot?.key_status.configured} onClick={()=>void perform(async()=>{await native.deleteKey();return '本机密钥已删除。';})}>删除密钥</button><button disabled={!nativeAvailable||busy} onClick={()=>void perform(async()=>{const backend=await native.selfTestVault();return runtimeMode==='mock'?'模拟测试完成；未验证 Windows 或 Android 安全存储。':`${backend} 写入、读取、删除自测通过。`;})}>验证安全存储</button><small>替换或删除密钥会先停止正在生成的回复。</small></div>
  </section>
  <form onSubmit={e=>{e.preventDefault();void perform(async()=>{if(!boot)throw new Error();await native.saveSettings(boot.active_persona_id,settings);return '聊天与人设设置已保存，新消息将使用这些设置。';});}}>
   <section className="settings-card" data-setting="character"><div className="card-heading"><span className="card-icon"><Icon name="persona"/></span><div><h2>她的聊天风格</h2><p>保留稳定的基础性格，称呼和表达由你调整。</p></div></div>
    <div className="two-fields"><div><label htmlFor="character-name">角色名称</label><input id="character-name" value={settings.character_name} maxLength={32} disabled={!nativeAvailable||busy} onChange={e=>change('character_name',e.target.value)}/></div><div><label htmlFor="user-title">如何称呼你</label><input id="user-title" value={settings.user_title} maxLength={32} disabled={!nativeAvailable||busy} onChange={e=>change('user_title',e.target.value)}/></div></div>
    <label htmlFor="personality">人物性格</label><textarea id="personality" className="personality" value={settings.personality} maxLength={3000} disabled={!nativeAvailable||busy} onChange={e=>change('personality',e.target.value)}/>
    <p className="field-help">关系阶段与未过期情绪参与聊天上下文。可在「关系与日常」查看来源和设置。</p>
    <DialogueFields value={settings.dialogue} disabled={!nativeAvailable||busy} onChange={value=>change('dialogue',value)}/>
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
 return <div className="settings-content"><PageHeader title="记忆与本地数据" icon="memory" art="thinking" scene="scene-settings" description="每个人格的记忆独立保存，配对后使用同一身份同步。"/><MemoryPanel personaId={personaId} onError={onError} onRefresh={onRefresh}/><section className="settings-card"><div className="card-heading"><h2>数据库诊断</h2><button disabled={!nativeAvailable} onClick={()=>void refresh()}>刷新</button></div>{diag?<dl><dt>SQLite / 迁移版本</dt><dd>{diag.sqlite_version} / {diag.schema_version}</dd><dt>完整性</dt><dd>{diag.integrity}</dd><dt>整个本地档案：消息 / 记忆 / 事件</dt><dd>{diag.message_count} / {diag.memory_count} / {diag.event_count}</dd></dl>:<p>请通过原生客户端读取本地数据库。</p>}</section></div>;
}
function App(){
 const [boot,setBoot]=useState<Bootstrap|null>(null);
 const [companion,setCompanion]=useState<CompanionSnapshot|null>(null);
 const [page,setPage]=useState<AppPage>('chat');
 const readingShares=useRef(new Set<string>());
 const notifyingShares=useRef(new Set<string>());
 useEffect(()=>{
  if(!companion||companion.persona_id!==boot?.active_persona_id||runtimeMode!=='native')return;
  function updateShares(){
   if(!companion)return;
   const reading=page==='chat'&&document.visibilityState==='visible'&&document.hasFocus();
   for(const share of companion.life.proactive_messages){
    if(share.state!=='delivered'||share.read_at_ms)continue;
    const key=companion.persona_id+'/'+share.id;
    if(reading&&!readingShares.current.has(key)){
     readingShares.current.add(key);
     void native.shareReceipt(companion.persona_id,share.id,true,share.notification_state??'pending')
      .catch(()=>readingShares.current.delete(key));
    }else if(!reading&&companion.preferences.notifications_enabled&&!notifyingShares.current.has(key)){
     notifyingShares.current.add(key);
     void notifyShare(companion,share.id,'DeepSeek娘 · 新分享').catch(()=>notifyingShares.current.delete(key));
    }
   }
  }
  updateShares();
  window.addEventListener('focus',updateShares);
  document.addEventListener('visibilitychange',updateShares);
  return()=>{window.removeEventListener('focus',updateShares);document.removeEventListener('visibilitychange',updateShares);};
 },[companion,page,boot?.active_persona_id]);

 const [allMessages,setAllMessages]=useState<Record<string,ChatMessage[]>>({});
 const [drafts,setDrafts]=useState<Record<string,string>>({});
 const [images,setImages]=useState<Record<string,ChatImage[]>>({});
 const [videoDraft,setVideoDraft]=useState<{persona:string;file:File;question:string}|undefined>();
 const [localImages,setLocalImages]=useState<Record<string,string[]>>({});
 const [copied,setCopied]=useState('');
 const [error,setError]=useState<ChatFailure|null>(null);const [loading,setLoading]=useState(false);
 const [active,setActive]=useState<{request:string;conversation:string;started:boolean;detached?:boolean}|null>(null);
 const activeRef=useRef(active);const initialized=useRef(false);const refreshSequence=useRef(0);const switchLock=useRef(false);const eventEpoch=useRef<Record<string,number>>({});
 const [imeOpen,setImeOpen]=useState(false);
 useEffect(()=>{
  const ime=(event:Event)=>setImeOpen((event as CustomEvent<boolean>).detail===true);
  window.addEventListener('companion-ime',ime);
  return()=>window.removeEventListener('companion-ime',ime);
 },[]);
 const [drawer,setDrawer]=useState(false);const [switching,setSwitching]=useState(false);
 const appearance=useAppearance();const {dark}=appearance;const [showHistory,setShowHistory]=useState(false);
 const bottom=useRef<HTMLDivElement>(null);const scroll=useRef<HTMLDivElement>(null);const nearBottom=useRef(true);const input=useRef<HTMLTextAreaElement>(null);
 const font=useFontScale(boot?.font_scale,scroll,e=>setError(failure(e)));
 const settings=boot?.settings??previewSettings;
 const persona=boot?.personas.find(p=>p.id===boot.active_persona_id);
 const conversation=persona?.main_conversation_id??'';
 useEffect(()=>{
  if(!conversation||runtimeMode!=='native')return;
  let current=true;
  void invoke<string[]>('chat_image_ids',{conversationId:conversation}).then(ids=>{
   if(current)setLocalImages(old=>({...old,[conversation]:ids}));
  }).catch(()=>{});
  return()=>{current=false;};
 },[conversation]);
 const reminders=useReminders(persona?.id??'',runtimeMode==='native');
 const voice=useVoice(persona?.id??'',runtimeMode==='native' ? boot?.platform??'' : '',page,text=>{
  setDrafts(old=>{
   const existing=old[conversation]??'';
   return {...old,[conversation]:(existing+(existing?'\n':'')+text).slice(0,8000)};
  });
 });
 const refresh=useCallback(async()=>{
  if(!nativeAvailable)return;const sequence=++refreshSequence.current;const data=await native.bootstrap();if(sequence!==refreshSequence.current)return;setBoot(data);
  const job=activeRef.current;
  if(job&&!data.personas.some(p=>p.main_conversation_id===job.conversation)){activeRef.current=null;setActive(null);}
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
  const poll=async()=>{
   try{
    const data=await native.bootstrap();
    if(stale||activeRef.current?.request!==job.request)return;
    if(!data.personas.some(p=>p.main_conversation_id===job.conversation)){
     activeRef.current=null;setActive(null);await refresh();return;
    }
    const messages=await native.messages(job.conversation);
    if(stale||activeRef.current?.request!==job.request)return;
    setAllMessages(old=>({...old,[job.conversation]:messages}));
    if(data.active_request!==job.request){activeRef.current=null;setActive(null);await refresh();}
   }catch(e){if(!stale&&activeRef.current?.request===job.request)setError(failure(e));}
  };
  void poll();const timer=setInterval(()=>void poll(),1000);return()=>{stale=true;clearInterval(timer);};
 },[active?.request,active?.detached,refresh]);
 const voiceContext={voice,persona:boot?.active_persona_id,conversation,page};
 const latestVoice=useRef(voiceContext);latestVoice.current=voiceContext;
 const requestIsActive=(id:string)=>activeRef.current?.request===id;
 const updateActive=(value:typeof active)=>{activeRef.current=value;setActive(value);};
 const onEvent=(event:ChatEvent,fromPet=false)=>{
  const thread=event.type==='started'?event.request.conversation_id:event.conversation_id;
  eventEpoch.current[thread]=(eventEpoch.current[thread]??0)+1;
  if(event.type==='started'){
   setAllMessages(old=>{let list=(old[event.request.conversation_id]??[]).map(m=>m.user_message_id===event.request.user_message_id?{...m,selected:false}:m);list=merge(merge(list,event.user),event.message);return {...old,[event.request.conversation_id]:list};});
   if(activeRef.current?.request===event.request.request_id)updateActive({...activeRef.current,started:true});
   else if(fromPet&&!activeRef.current)updateActive({request:event.request.request_id,conversation:event.request.conversation_id,started:true});
  }else if(event.type==='delta'){
   setAllMessages(old=>({...old,[event.conversation_id]:(old[event.conversation_id]??[]).map(m=>m.id===event.message_id&&m.revision<=event.revision&&!terminal(m)?{...m,content:event.content,revision:event.revision,state:'generating'}:m)}));
  }else{
   if(event.type==='finished'){
    setAllMessages(old=>({...old,[event.conversation_id]:merge(old[event.conversation_id]??[],event.message)}));
    if (runtimeMode==='native') {
     void invoke<string[]>('chat_image_ids',{conversationId:event.conversation_id}).then(ids=>{
      setLocalImages(old=>({...old,[event.conversation_id]:ids}));
     }).catch(()=>{});
    }
    const context=latestVoice.current;
    if(!fromPet&&runtimeMode==='native'&&requestIsActive(event.request_id)&&event.message.state==='complete'
      &&context.persona&&context.page==='chat'&&context.conversation===event.conversation_id){
     void invoke<boolean>('cloud_voice_auto',{persona:context.persona}).then(enabled=>{
      const current=latestVoice.current;
      if(enabled&&current.persona===context.persona&&current.conversation===context.conversation
        &&current.page==='chat'&&current.voice.phase==='idle'&&!activeRef.current){
        void current.voice.play(event.message.content);
      }
     }).catch(()=>{});
    }
   }
   if(event.type==='failed')setError(event.error);else if(event.error&&event.error.code!=='CANCELLED')setError(event.error);
   if(activeRef.current?.request===event.request_id)updateActive(null);void refresh().catch(e=>setError(failure(e)));
  }
 };
 const petChatHandler=useRef(onEvent);
 petChatHandler.current=onEvent;
 useEffect(()=>{
  if(runtimeMode!=='native'||boot?.platform!=='windows')return;
  const unlistenChat=listen<ChatEvent>('desktop-pet:chat',event=>petChatHandler.current(event.payload,true));
  const unlistenOpen=listen('desktop-pet:open-chat',()=>{setPage('chat');setDrawer(false);});
  const listeners=[unlistenChat,unlistenOpen];listeners.forEach(listener=>void listener.catch(()=>{}));
  return()=>listeners.forEach(listener=>void listener.then(unlisten=>unlisten()).catch(()=>{}));
 },[boot?.platform]);
 async function switchPersona(id:string,toSettings=false){
  if(switchLock.current)return;switchLock.current=true;setSwitching(true);setError(null);
  voice.stop();
  try{await native.selectPersona(id);await refresh();setPage(toSettings?'settings':'chat');setDrawer(false);}catch(e){setError(failure(e));}finally{switchLock.current=false;setSwitching(false);}
 }
 async function send(regenerateUserId?:string){
  if(!conversation||activeRef.current||!boot?.key_status.configured||switching)return;
  const pictures=regenerateUserId?undefined:images[conversation];
  const draft=drafts[conversation]??'';
  const text=draft.trim()?draft:pictures?.length?'［发送了本机图片附件，请等我描述画面。］':'';
  if(!regenerateUserId&&!text.trim())return;
  const request=crypto.randomUUID();const selected=conversation;setError(null);nearBottom.current=true;updateActive({request,conversation:selected,started:false});
  if(!regenerateUserId)setDrafts(old=>({...old,[selected]:''}));
  try{
   const result=await native.startChat({request_id:request,conversation_id:selected,text:regenerateUserId?null:text,regenerate_user_id:regenerateUserId??null},onEvent,pictures);
   if(pictures?.length){
    setImages(old=>({...old,[selected]:old[selected]===pictures?[]:old[selected]}));
    setLocalImages(old=>({...old,[selected]:[...new Set([...(old[selected]??[]),result.user_message_id])]}));
   }
  }
  catch(e){setError(failure(e));if(requestIsActive(request)){updateActive(null);if(!regenerateUserId)setDrafts(old=>({...old,[selected]:old[selected]||draft}));}}
 }
 async function stopCurrent(){const job=activeRef.current;if(!job)return;try{await native.stop(job.request);const list=await native.messages(job.conversation);setAllMessages(old=>({...old,[job.conversation]:list}));if(activeRef.current?.request===job.request)updateActive(null);await refresh();}catch(e){setError(failure(e));}}
 const lastUser=[...rows].reverse().find(m=>m.role==='user');const variants=rows.filter(m=>m.role==='assistant'&&!m.selected).length;
 const mobile=boot?.platform==='android'||(!boot&&/Android/i.test(navigator.userAgent))||(runtimeMode!=='native'&&matchMedia('(pointer:coarse)').matches);
 const connected=nativeAvailable&&!!boot;const canSend=connected&&!!conversation&&boot.key_status.configured&&!active&&!loading&&!switching;
 const title=page==='chat'?persona?.name??'DeepSeek娘':page==='sync'&&mobile?'设备连接':navigation.find(item=>item.page===page)?.label;
 const currentCompanion=companion?.persona_id===boot?.active_persona_id?companion:null;
 const navigate=(destination:AppPage)=>{setPage(destination);setDrawer(false);};
 async function copyMessage(message:ChatMessage){
  try{await navigator.clipboard.writeText(message.content);setCopied(message.id);}
  catch{setError({code:'CLIPBOARD_UNAVAILABLE',message:'无法复制到系统剪贴板。可以选中文字后手动复制。',retryable:true});}
 }
 return <div className="app" data-theme={dark?'dark':'light'} data-palette={appearance.value.palette} data-platform={mobile?'android':'desktop'} data-page={page} data-ime={imeOpen}>
  <header>
   <button className="menu-toggle" aria-label="打开侧栏" aria-expanded={drawer} onClick={()=>setDrawer(true)}>
    <span className="desktop-menu-symbol"><Icon name="more"/></span>
    <span className="phone-menu-avatar"><Avatar persona={persona} small/></span>
   </button>
   <div className="header-title"><strong>{title}</strong></div>
   <div className="header-actions">
    {runtimeMode==='native'&&boot?.platform==='windows'&&<button className="desktop-pet-entry" onClick={()=>navigate('settings')}><Icon name="pet"/>桌宠</button>}
    <span className={`connection-badge ${boot?.key_status.configured?'ready':''}`}><i/>{boot?.key_status.configured?'密钥已配置':'待配置密钥'}</span>
    <button className="icon-button" aria-label="切换深浅色" onClick={()=>appearance.update({mode:dark?'light':'dark'})}><Icon name={dark?'sun':'moon'}/></button>
   </div>
  </header>
  <div className="workspace-body">
  {drawer&&<button className="drawer-backdrop" aria-label="关闭侧栏" onClick={()=>setDrawer(false)}/>}
  <aside className={drawer?'open':''}>
   <a className="brand" href="#" onClick={event=>{event.preventDefault();navigate('chat');}}><WhaleMark/><span>DeepSeek娘<small>你的本地 AI 伙伴</small></span></a>
   <nav className="persona-list" aria-label="人格切换">{boot?.personas.map(p=>
    <button key={p.id} data-persona-id={p.id} aria-current={boot.active_persona_id===p.id?'true':undefined} disabled={switching}
     className={boot.active_persona_id===p.id?'selected':''} onClick={()=>void switchPersona(p.id)}>
      <Avatar persona={p} small/><span>{p.name}</span>{active?.conversation===p.main_conversation_id&&<i className="working-dot"/>}
    </button>)}</nav>
   <button className="add-persona mobile-add-persona" disabled={!connected} onClick={()=>navigate('personas')}><Icon name="add"/>新增人格</button>
   <nav className="sidebar-bottom" aria-label="主要页面">
    {navigation.filter(item=>runtimeMode==='native'||item.page!=='plugins').map(item=>
     <button key={item.page} aria-label={item.label} aria-current={page===item.page?'page':undefined}
      className={(item.page==='companion'?'sidebar-companion-nav ':'')+(page===item.page?'selected':'')}
      onClick={()=>navigate(item.page)}><Icon name={item.icon}/>{item.label}</button>)}
   </nav>
   <div className="sidebar-end">
    <Art name="sleeping" className="sidebar-art"/>
    <div className="sidebar-profile"><Icon name="shield"/><div>本地个人空间<small>{boot?.version??'本地开发版'}</small></div></div>
   </div>
  </aside>
  <main>
   {import.meta.env.VITE_COMPANION_TEST_PACKAGE==='1'&&<div className="test-build-banner notice" role="status">{mobile?'独立测试版 · 不覆盖正式版数据':'独立测试版 · 与正式版数据隔离，不用于覆盖正式版。请使用合成测试资料验收。'}</div>}
   {runtimeMode==='mock'&&<div className="mode-banner mock" role="status">模拟验收模式 · 无真实 DeepSeek API、SQLite 或系统密钥存储 · 请勿输入真实密钥</div>}
   {runtimeMode==='preview'&&<div className="mode-banner" role="status">浏览器外观预览 · 原生功能未连接，请通过 Windows 或 Android 应用聊天</div>}
   {!brandingReady&&<div className="branding-pending">已指定海滩角色图；原始 PNG 文件待导入，暂不使用替代人物图标。</div>}
   {error&&<div className="error-banner" role="alert"><span>{error.message}<small>{error.code}</small></span><button aria-label="关闭错误提示" onClick={()=>setError(null)}>×</button></div>}
   {reminders.reminder&&<div className="notice plugin-reminder" role="status"><span>提醒 · {reminders.reminder.label}</span><button onClick={()=>void reminders.dismiss(reminders.reminder!.id).catch(e=>setError(failure(e)))}>知道了</button></div>}
   {page==='about'?<div className="page-scroll"><AboutPage version={boot?.version}/></div>:page==='plugins'?<div className="page-scroll"><PluginManager key={persona?.id} persona={persona?.id??''} name={persona?.name??''}/></div>:page==='video'?<div className="page-scroll"><CloudVideoPanel key={boot?.active_persona_id} personaId={boot?.active_persona_id??''} deepseekConfigured={!!boot?.key_status.configured} memories={currentCompanion?.memories??[]} previousWindows={boot?.platform==='windows'}
        initialVideo={videoDraft?.persona===boot?.active_persona_id?videoDraft:undefined}
        onSaved={()=>{setVideoDraft(undefined);navigate('chat');void refresh();}}
        onMemories={()=>navigate('storage')}/></div>:page==='companion'?<div className="page-scroll"><CompanionPanel key={boot?.active_persona_id} data={currentCompanion} persona={persona} onRefresh={refresh} onError={setError} onMemories={()=>setPage('storage')}/></div>:page==='sync'?<div className="page-scroll"><SyncPanel mobile={mobile} onPaired={refresh}/></div>:page==='settings'?<div className="page-scroll"><SettingsPanel key={boot?.active_persona_id??'loading'} boot={boot} voice={voice} font={font} appearance={appearance} onNavigate={navigate} onRefresh={refresh} onError={setError}/></div>:page==='storage'?<div className="page-scroll"><StoragePanel key={boot?.active_persona_id??'loading'} personaId={boot?.active_persona_id??''} onError={setError} onRefresh={refresh}/></div>:page==='personas'?<div className="page-scroll"><PersonaManager boot={boot} onSwitch={switchPersona} onRefresh={refresh} onError={setError} onSettings={()=>setPage('settings')} onErased={threads=>{
      setAllMessages(old=>Object.fromEntries(Object.entries(old).filter(([id])=>!threads.includes(id))));
      setDrafts(old=>Object.fromEntries(Object.entries(old).filter(([id])=>!threads.includes(id))));
      setImages(old=>Object.fromEntries(Object.entries(old).filter(([id])=>!threads.includes(id))));
      setLocalImages(old=>Object.fromEntries(Object.entries(old).filter(([id])=>!threads.includes(id))));
     }}/></div>:<div className="chat-layout"><section className="chat-main">
    <div className="chat-scenery" aria-hidden="true"/>
    <div className="message-scroll" ref={scroll} onScroll={()=>{const n=scroll.current;if(n)nearBottom.current=n.scrollHeight-n.scrollTop-n.clientHeight<100;}}>
     {!visible.length&&!loading&&<div className="welcome">{persona?.is_builtin!==false?<Art name="peeking" className="welcome-character" eager/>:<Avatar persona={persona}/>}<h1>和{persona?.name??'DeepSeek娘'}，接着聊</h1><div className="suggestions">{['聊聊今天的小事','一起想一个新点子','今天想学点什么？'].map(s=><button key={s} disabled={!connected} onClick={()=>{setDrafts(old=>({...old,[conversation]:s}));input.current?.focus();}}>{s}<span>↗</span></button>)}</div></div>}
     {loading&&<div className="loading-history" role="status"><Art name="waiting"/>正在读取人格时间线…</div>}
     <div className="messages" aria-label="永久聊天时间线">{visible.map(m=><article className={`message ${m.role==='user'?'user':'assistant'} ${!m.selected?'old-variant':''}`} key={m.id} data-message-id={m.id} data-state={m.state}>
      {m.role!=='user'&&<Avatar persona={persona} small/>}<div className="message-body"><div className="message-meta"><strong>{m.role==='user'?'你':persona?.name??settings.character_name}</strong><time>{time(m.created_at_ms)}</time>{!m.selected&&<span>历史回复</span>}</div><div className={`bubble ${m.state==='failed'?'failed':''}`}>{m.content?<MessageText text={m.content}/>:('sending'===m.state||'generating'===m.state)?<span className="typing" aria-label="正在等待回复"><i/><i/><i/></span>:m.state==='failed'?'本次未收到回复。':'生成已中断，尚未收到正文。'}{m.state==='generating'&&<span className="stream-caret"/>}{m.role==='user'&&localImages[conversation]?.includes(m.id)&&<ChatImagePreview conversation={conversation} id={m.id}/>}</div><div className="message-foot">{m.state==='sending'?'发送中 · 等待模型':m.state==='generating'?'正在生成':m.state==='interrupted'?(m.error_code==='APP_RESTARTED'?'应用退出时中断':'已停止 / 中断'):m.state==='failed'?'生成失败':'已保存本地'}{m.finish_reason==='length'&&<span> · 达到输出 Token 上限</span>}{voice.enabled&&m.role==='assistant'&&terminal(m)&&!!m.content.trim()&&<button type="button" disabled={!voice.prefs||switching||['starting','recording','transcribing'].includes(voice.phase)} onClick={()=>void voice.play(m.content)} title="朗读"><Icon name="speaker"/>朗读</button>}{m.role==='assistant'&&m.selected&&m.user_message_id===lastUser?.id&&terminal(m)&&<button disabled={!!active||!boot?.key_status.configured} onClick={()=>void send(m.user_message_id!)}><Icon name="sync"/> {m.state==='failed'?'重试':'重新生成'}</button>}{m.content&&terminal(m)&&<button type="button" aria-label="复制消息" title="复制消息" onClick={()=>void copyMessage(m)}><Icon name="copy"/>{copied===m.id?'已复制':'复制'}</button>}</div></div>{m.role==='user'&&<span className="user-avatar" aria-label="你的头像"><Icon name="persona"/></span>}
     </article>)}</div>
     {variants>0&&<button className="variant-toggle" onClick={()=>setShowHistory(!showHistory)}>{showHistory?'收起历史回复':`查看历史回复（${variants}）`}</button>}<div ref={bottom}/>
    </div>
    <ChatComposer key={conversation} field={input} draft={drafts[conversation]??''} name={persona?.name} personaId={persona?.id} images={images[conversation]??[]}
     onImages={images=>setImages(old=>({...old,[conversation]:images}))}
     onVideo={file=>{if(boot?.active_persona_id){setVideoDraft({persona:boot.active_persona_id,file,question:(drafts[conversation]??'').slice(0,1000)});navigate('video');}}}
     connected={connected} disabled={!connected||!conversation||loading||switching} configured={!!boot?.key_status.configured}
     canSend={canSend} mobile={mobile} voice={voice} recording={!switching}
     generation={active?{started:active.started,elsewhere:active.conversation!==conversation}:undefined}
     onEdit={text=>setDrafts(old=>({...old,[conversation]:text}))} onSend={()=>void send()}
     onStop={()=>void stopCurrent()} onSettings={()=>setPage('settings')}/>
   </section>{!mobile&&<CompanionRail key={boot?.active_persona_id} boot={boot} data={currentCompanion} onRefresh={refresh} onError={setError} onMemories={()=>setPage('storage')} settingsContent={<SettingsPanel compact boot={boot} voice={voice} font={font} appearance={appearance} onNavigate={navigate} onRefresh={refresh} onError={setError}/>}/>}</div>}
  </main>
  </div>
  <MobileNav page={page} onSelect={destination=>{if(destination==='more'){setDrawer(v=>!v);return;}setPage(destination);setDrawer(false);}}/>
 </div>;
}
installButtonMotion();
createRoot(document.getElementById('root')!).render(<StrictMode><App/></StrictMode>);
