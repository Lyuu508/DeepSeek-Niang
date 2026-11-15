import {useRef,useState} from 'react';
import type {Bootstrap,ChatFailure} from '@deepseek/contracts';
import {native,nativeAvailable,failure} from './native';
import {Avatar} from './Avatar';
import {previewSettings} from './defaults';
export function PersonaManager({boot,onSwitch,onRefresh,onError,onSettings}:{boot:Bootstrap|null;onSwitch:(id:string)=>Promise<void>;onRefresh:()=>Promise<void>;onError:(e:ChatFailure|null)=>void;onSettings:()=>void}){
 const [name,setName]=useState('');const [personality,setPersonality]=useState(previewSettings.personality);const [busy,setBusy]=useState(false);const lock=useRef(false);
 async function create(){if(lock.current||!name.trim())return;lock.current=true;setBusy(true);onError(null);try{const persona=await native.createPersona(name,personality);setName('');await onRefresh();await onSwitch(persona.id);}catch(e){onError(failure(e));}finally{lock.current=false;setBusy(false);}}
 return <div className="settings-content"><div className="page-heading"><span className="eyebrow">CHARACTERS, NOT SESSIONS</span><h1>人格管理</h1><p>每个人格拥有唯一的永久聊天空间。切换人格会回到她自己的时间线。</p></div>
 <section className="persona-grid">{boot?.personas.map(p=><article className="persona-tile" key={p.id}><Avatar persona={p}/><div><h2>{p.name}</h2><p>{p.is_builtin?'默认人格':'自定义人格'} · 独立记忆与上下文</p></div><button className="quiet-button" disabled={busy} onClick={()=>void onSwitch(p.id)}>{boot.active_persona_id===p.id?'进入聊天':'切换人格'} →</button>{boot.active_persona_id===p.id&&<button className="quiet-button" onClick={onSettings}>编辑人物设定 →</button>}</article>)}</section>
 <form className="settings-card" onSubmit={e=>{e.preventDefault();void create();}}><h2>新增人格</h2><p className="field-help">自动建立独立永久聊天空间、人物配置与状态。已有的人格和记录会继续保留。</p><label htmlFor="new-persona-name">人格名称</label><input id="new-persona-name" value={name} maxLength={32} disabled={!nativeAvailable||busy} onChange={e=>setName(e.target.value)}/><label htmlFor="new-persona-personality">独立人物设定</label><textarea id="new-persona-personality" value={personality} maxLength={3000} disabled={!nativeAvailable||busy} onChange={e=>setPersonality(e.target.value)}/><button className="primary" disabled={!nativeAvailable||busy||!name.trim()||!personality.trim()}>{busy?'正在创建…':'创建人格并进入聊天'}</button></form>
 <p className="field-help">这里只创建你主动新增的人格。扫码连接采用电脑现有人格，聊天、记忆和关系按相同身份同步。</p></div>;
}
