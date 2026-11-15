import {useState,type ReactNode} from 'react';
import type {Bootstrap,CompanionSnapshot,ChatFailure} from '@deepseek/contracts';
import {Avatar} from './Avatar';
import {CompanionPanel} from './CompanionPanel';

const dimensions=[['familiarity','熟悉'],['trust','信任'],['affection','好感'],['intimacy','亲密'],['comfort','舒适']] as const;
export function CompanionRail({boot,data,onRefresh,onError,onMemories,settingsContent}:{boot:Bootstrap|null;data:CompanionSnapshot|null;onRefresh:()=>Promise<void>;onError:(e:ChatFailure|null)=>void;onMemories:()=>void;settingsContent:ReactNode}){
 const [mode,setMode]=useState<'status'|'companion'|'chat'>('status');
 const persona=boot?.personas.find(p=>p.id===boot.active_persona_id);
 const living=data?.life.host_online&&data.life.current?.state==='running';
 return <section className="companion-panel companion-rail" aria-label="当前人格资料">
  <nav className="rail-tabs" aria-label="人格侧栏功能"><button aria-current={mode==='status'?'page':undefined} onClick={()=>setMode('status')}>状态</button><button aria-label="关系与日常" aria-current={mode==='companion'?'page':undefined} onClick={()=>setMode('companion')}>陪伴</button><button aria-label="编辑当前人格与聊天设置" aria-current={mode==='chat'?'page':undefined} onClick={()=>setMode('chat')}>设置</button></nav>
  <div className="rail-content page-scroll">
   {mode==='status'?<><div className="companion-card"><span className="eyebrow">YOUR AI COMPANION</span><Avatar persona={persona}/><h2>{persona?.name??'DeepSeek娘'}</h2><span className="character-tag">{data?.relationship.stage??'正在读取关系'}</span>
    <div className="companion-divider"/><div className="detail-row"><span>称呼你</span><strong>{boot?.settings.user_title??'主人'}</strong></div><div className="detail-row"><span>当前心情</span><strong>{data?.mood.label??'正在读取'}</strong></div><div className="detail-row"><span>虚拟日常</span><strong>{living?data?.life.current?.label:'等待电脑主机'}</strong></div>
    <div className="rail-bond" aria-label="当前关系维度">{dimensions.map(([key,label])=><div key={key}><span>{label}<strong>{data?(data.relationship[key]/1000).toFixed(1):'—'}</strong></span><progress aria-label={label+'程度'} value={data?.relationship[key]??0} max={100000}/></div>)}</div>
    <button className="quiet-button" onClick={()=>setMode('companion')}>调整关系、生活与主动消息 <span>↗</span></button>
   </div><div className="usage-card"><h3>记忆与上下文</h3><div className="detail-row"><span>长期记忆</span><strong>{boot?.persona_summary.memory_count??0} 条</strong></div><div className="detail-row"><span>永久聊天</span><strong>{data?.recall?.stored_messages??'—'} 条</strong></div><p>按话题检索旧聊天和记忆，不把全部历史重复发送。每次引用共用 {data?.preferences.memory_budget_bytes??2000} 字节预算，不增加整理模型调用。</p><button className="quiet-button" onClick={onMemories}>管理记忆与来源 <span>↗</span></button></div>
   <div className="usage-card"><h3>当前人格实际 Token</h3><div className="usage-count">{boot?.usage.requests??'—'}<small>次生成</small></div><div className="detail-row"><span>输入</span><strong>{boot?.usage.prompt_tokens.toLocaleString()??'—'}</strong></div><div className="detail-row"><span>输出</span><strong>{boot?.usage.completion_tokens.toLocaleString()??'—'}</strong></div><small>提供商未返回的用量仍记为未知。虚拟视频/音乐不代表真实播放或理解。</small><button className="quiet-button" onClick={()=>setMode('chat')}>调整聊天与上下文 <span>↗</span></button></div></>:mode==='companion'?<CompanionPanel key={boot?.active_persona_id} compact data={data} onRefresh={onRefresh} onError={onError} onMemories={onMemories}/>:settingsContent}
  </div>
 </section>;
}
