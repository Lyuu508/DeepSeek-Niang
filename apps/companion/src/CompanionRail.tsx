import { Art, Icon } from './OceanDesign';
import {useState,type ReactNode} from 'react';
import type {Bootstrap,CompanionSnapshot,ChatFailure} from '@deepseek/contracts';
import {Avatar} from './Avatar';
import {CompanionPanel} from './CompanionPanel';
import {BondDimensions,MoodDisplay,relationLevel} from './RelationshipDisplay';

export function CompanionRail({boot,data,onRefresh,onError,onMemories,settingsContent}:{boot:Bootstrap|null;data:CompanionSnapshot|null;onRefresh:()=>Promise<void>;onError:(e:ChatFailure|null)=>void;onMemories:()=>void;settingsContent:ReactNode}){
 const [mode,setMode]=useState<'status'|'companion'|'chat'>('status');
 const persona=boot?.personas.find(p=>p.id===boot.active_persona_id);
 const living=data?.life.host_online&&data.life.current?.state==='running';
 const level=data?relationLevel(data.relationship.stage):null;
 return <section className="companion-panel companion-rail" aria-label="当前人格资料">
  <div className="companion-showcase">
   {persona?.is_builtin!==false?<Art name="portrait-half" className="showcase-portrait" eager/>:<div className="showcase-custom"><Avatar persona={persona}/></div>}
   <div className="showcase-caption"><div className="showcase-title"><h2>{persona?.name??'DeepSeek娘'}</h2>
    {level&&<span className="level-tag" title={data?.relationship.stage}>Lv. {level} · {data?.relationship.stage}</span>}</div>
    <div className="affection-line"><span><Icon name="bond"/>好感度</span>{data&&<progress aria-label="当前好感度" value={data.relationship.affection} max={100000}/>}
     <strong>{data?`${(data.relationship.affection/1000).toFixed(1)} / 100`:'—'}</strong></div>
   </div>
  </div>
  <nav className="rail-tabs" aria-label="人格侧栏功能"><button aria-current={mode==='status'?'page':undefined} onClick={()=>setMode('status')}><Icon name="pulse"/>状态</button><button aria-label="关系与日常" aria-current={mode==='companion'?'page':undefined} onClick={()=>setMode('companion')}><Icon name="bond"/>陪伴</button><button aria-label="编辑当前人格与聊天设置" aria-current={mode==='chat'?'page':undefined} onClick={()=>setMode('chat')}><Icon name="settings"/>设置</button></nav>
  <div className="rail-content page-scroll">
   {mode==='status'?<><div className="companion-card">
    <MoodDisplay data={data} persona={persona}/>
    <BondDimensions bond={data?.relationship}/>
    <div className="relation-meta"><span>称呼你 · {boot?.settings.user_title??'主人'}</span><span>{living?data?.life.current?.label:'等待电脑主机'}</span></div>
   </div><button className="quiet-button rail-memory" onClick={onMemories}><Icon name="book"/><span>记忆与上下文</span><small>{boot?.persona_summary.memory_count??0} 条</small><Icon name="arrow"/></button>
   <div className="usage-card"><h3>当前人格实际 Token</h3><div className="usage-count">{boot?.usage.requests??'—'}<small>次生成</small></div><div className="detail-row"><span>输入</span><strong>{boot?.usage.prompt_tokens.toLocaleString()??'—'}</strong></div><div className="detail-row"><span>输出</span><strong>{boot?.usage.completion_tokens.toLocaleString()??'—'}</strong></div><small>提供商未返回的用量仍记为未知。虚拟视频/音乐不代表真实播放或理解。</small><button className="quiet-button" onClick={()=>setMode('chat')}>调整聊天与上下文 <span>↗</span></button></div></>:mode==='companion'?<CompanionPanel key={boot?.active_persona_id} compact data={data} persona={persona} onRefresh={onRefresh} onError={onError} onMemories={onMemories}/>:settingsContent}
  </div>
 </section>;
}
