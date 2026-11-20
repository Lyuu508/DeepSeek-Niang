import { PageHeader, moodArt } from './OceanDesign';
import {LifePanel} from './LifePanel';
import {enableNotifications} from './notifications';
import {useEffect,useState} from 'react';
import type {CompanionPreferences,CompanionSnapshot,ChatFailure} from '@deepseek/contracts';
import {native,nativeAvailable,failure,runtimeMode} from './native';

export function dateTime(value:number|null){return value===null?'旧版未记录':new Date(value).toLocaleString();}
const dimensions=[['familiarity','熟悉'],['trust','信任'],['affection','好感'],['intimacy','亲密'],['comfort','舒适']] as const;
const activityNames:Record<string,string>={auto:'按日常顺序',wake:'起床',read:'阅读保存的话题',interest:'兴趣构想',create:'创作构思',journal:'记录生活',rest:'休息',study:'学习',video:'视频话题构想（不播放）',music:'音乐话题构想（不播放）',organize:'整理记忆',recall:'回忆往事',wait:'等待主人',share:'分享发现'};
const minuteText=(value:number)=>String(Math.floor(value/60)).padStart(2,'0')+':'+String(value%60).padStart(2,'0');
const parseMinute=(value:string)=>{const parts=value.split(':').map(Number);return parts[0]*60+parts[1];};

export function CompanionPanel({data,onRefresh,onError,onMemories,compact=false}:{compact?:boolean;data:CompanionSnapshot|null;onRefresh:()=>Promise<void>;onError:(error:ChatFailure|null)=>void;onMemories:()=>void}){
 const [preferences,setPreferences]=useState<CompanionPreferences|null>(data?.preferences??null);
 const [dirty,setDirty]=useState(false);const [busy,setBusy]=useState(false);const [notice,setNotice]=useState('');
 useEffect(()=>{if(!dirty)setPreferences(data?.preferences??null);},[data,dirty]);
 function change<K extends keyof CompanionPreferences>(key:K,value:CompanionPreferences[K]){setDirty(true);setPreferences(old=>old?{...old,[key]:value}:old);}
 async function save(){if(!data||!preferences)return;setBusy(true);setNotice('');onError(null);try{if(preferences.notifications_enabled)await enableNotifications();await native.saveCompanionPreferences(data.persona_id,{...preferences,timezone_offset_minutes:-new Date().getTimezoneOffset()});setDirty(false);await onRefresh();setNotice('陪伴设置已保存，配对设备同步后生效。');}catch(e){onError(failure(e));}finally{setBusy(false);}}
 if(!data||!preferences)return <div className="settings-content"><h1>关系与日常</h1><p>{nativeAvailable?'正在读取当前人格的记录…':'请使用原生客户端查看持久记录。'}</p></div>;
 const {relationship:bond,mood,life,daily_usage:usage}=data;
 return <div className={`settings-content companion-content ${compact?'rail-settings':''}`}>
  <PageHeader title="慢慢熟悉，一起生活" description="查看关系、情绪和虚拟日常，调整陪伴偏好。"
   icon="bond" art={compact?undefined:moodArt(data)} scene={compact?undefined:'scene-chat'}/>
  {runtimeMode==='mock'&&<p className="notice">以下是 UI 夹具，未验证真实情绪、SQLite、调度或模型回复。</p>}
  {notice&&<p className="notice" role="status">{notice}</p>}
  {data.worker?.error_code&&<p className="notice inline-error" role="status">有一部分记忆或日常处理尚待重试，原聊天已保留。请稍后刷新检查。</p>}
  <section className="settings-card bond-card"><div className="card-heading"><div><h2>我们的关系</h2><p>{bond.applied_events} 次有效变化 · {bond.distinct_days} 个有记录的相处日</p></div><span className="relation-stage">{bond.stage}</span></div>
   <div className="bond-dimensions">{dimensions.map(([key,label])=><div className="bond-dimension" key={key}><span>{label}<strong>{(bond[key]/1000).toFixed(1)}</strong></span><progress aria-label={label+'程度'} value={bond[key]} max={100000}/></div>)}</div>
   <p className="field-help">同类事件有冷却、重复递减与每日上限。关系会影响实际聊天语气，但不会替代稳定性格或要求你持续在线。</p>
   <details><summary>关系变化历史</summary>{bond.history.length?<ol className="evidence-list">{bond.history.map(event=><li key={event.id}><strong>{event.explanation}</strong><time>{dateTime(event.occurred_at_ms)}</time><p>{event.decision}{event.applied?' · '+event.delta.map((value,index)=>value?dimensions[index][1]+' '+(value>0?'+':'')+(value/1000).toFixed(2):'').filter(Boolean).join('，'):''}</p><small>来源消息：{event.source_message_id}</small></li>)}</ol>:<p>还没有重要关系变化。日常聊天可以保持零变化。</p>}</details>
  </section>
  <div className="companion-status-grid">
   <section className="settings-card"><h2>此刻的心情</h2><strong className="mood-name">{mood.label}</strong><p>{mood.source}</p><p className="field-help">强度 {(mood.intensity_milli/10).toFixed(0)}%{mood.expires_at_ms?' · '+dateTime(mood.expires_at_ms)+' 前逐渐平复':''}。情绪是持久的应用状态，不是随机标签。</p></section>
   <section className="settings-card"><div className="card-heading"><h2>她的虚拟日常</h2><span className="badge">{life.host_online?'电脑主机在线':'等待电脑主机'}</span></div><strong className="life-name">{life.current?`${life.current.label} · ${life.host_online?({queued:'等待执行',running:'进行中',paused:'已暂停',failed:'失败',uncertain:'待核对',completed:'已完成',cancelled:'已取消'}[life.current.state]??life.current.state):'状态已过期'}`:'尚未开始'}</strong><p>下一项：{life.next_activity??'电脑在线后安排'}</p><small>{life.heartbeat_at_ms?'最近状态：'+dateTime(life.heartbeat_at_ms):'手机仅查看同步状态，不独立重复调度。'}</small><p className="field-help">所有活动都是明确标记的本地虚拟安排；不执行网页浏览或媒体理解。电脑应用关闭后暂停。</p></section>
  </div>
  <section className="settings-card"><div className="card-heading"><h2>今天的小记</h2><button onClick={onMemories}>查看分层记忆 →</button></div>{life.today_events.length?<ol className="evidence-list">{life.today_events.map(event=><li key={event.id}><time>{dateTime(event.occurred_at_ms)}</time><p>{event.description}</p></li>)}</ol>:<p>今天还没有生活事件；不会填入未发生的经历。</p>}</section>
  <section className="settings-card"><h2>主动分享记录</h2><p className="field-help">由电脑在线时依据已有生活记录或记忆生成，当前使用本地文字规则，不额外调用模型。默认关闭；开启后受安静时段、间隔和每日上限限制，关系较浅时会进一步放慢。</p>{life.proactive_messages.length?<div className="proactive-list">{life.proactive_messages.map(message=><article key={message.id}><span className="badge">{message.state==='delivered'?'已加入聊天':message.state==='dismissed'?'已取消':'等待合适时段'}</span><p>{message.content}</p><small>{dateTime(message.delivered_at_ms??message.created_at_ms)}</small></article>)}</div>:<p>尚无主动消息。</p>}</section>
  <section className="settings-card"><h2>今天的实际 Token</h2><div className="usage-table-wrap"><table className="usage-table"><thead><tr><th>用途</th><th>请求</th><th>输入 / 输出</th><th>用量未知</th></tr></thead><tbody>{(['chat','memory','life'] as const).map(kind=><tr key={kind}><td>{({chat:'聊天',memory:'记忆',life:'生活'})[kind]}</td><td>{usage.by_purpose[kind].requests}</td><td>{usage.by_purpose[kind].prompt_tokens+' / '+usage.by_purpose[kind].completion_tokens}</td><td>{usage.by_purpose[kind].unknown_requests}</td></tr>)}</tbody></table></div><p className="field-help">只累计提供商实际返回的用量。记忆/关系/生活目前使用本地规则，不产生模型请求；未知用量不会估成真实账单。</p></section>
  <LifePanel data={data} onRefresh={onRefresh} onError={onError}/>
  <form className="settings-card" onSubmit={event=>{event.preventDefault();void save();}}><h2>陪伴设置</h2>
   <label className="check-row"><input type="checkbox" checked={preferences.relationship_enabled} onChange={e=>change('relationship_enabled',e.target.checked)}/>自动记录有来源的关系和情绪事件</label>
   <label className="check-row"><input type="checkbox" checked={preferences.automatic_memory} onChange={e=>change('automatic_memory',e.target.checked)}/>从明确偏好与“请记住”中整理长期记忆</label>
   <label className="check-row"><input type="checkbox" checked={preferences.memory_context} onChange={e=>change('memory_context',e.target.checked)}/>聊天时按话题检索允许发送的记忆和旧聊天摘录</label>
   <div className="two-fields"><label>最多检索条数<input aria-label="最多检索条数" type="number" min={1} max={12} value={preferences.memory_limit} onChange={e=>change('memory_limit',Number(e.target.value))}/></label><label>记忆引用预算（字节）<input aria-label="记忆引用预算" type="number" min={256} max={6000} value={preferences.memory_budget_bytes} onChange={e=>change('memory_budget_bytes',Number(e.target.value))}/></label><label>整个上下文预算（字节）<input aria-label="整个上下文预算" type="number" min={8000} max={120000} step={1000} value={preferences.context_budget_bytes} onChange={e=>change('context_budget_bytes',Number(e.target.value))}/></label></div>
   <p className="field-help">UTF-8 字节数用作保守 Token 上界；超预算先移除旧回合，保留本次输入。实际 Token 以上方提供商用量为准。</p>
   <label className="check-row"><input type="checkbox" checked={preferences.life_enabled} onChange={e=>change('life_enabled',e.target.checked)}/>电脑应用在线时安排虚拟日常</label>
   <label>偏好的下一项活动<select aria-label="偏好的下一项活动" value={preferences.preferred_activity} onChange={e=>change('preferred_activity',e.target.value)}>{Object.entries(activityNames).map(([key,label])=><option value={key} key={key}>{label}</option>)}</select></label>
   <label className="check-row"><input type="checkbox" checked={preferences.proactive_enabled} onChange={e=>change('proactive_enabled',e.target.checked)}/>开启主动消息</label>
   <label>主动交流方式<select aria-label="主动交流模式" value={preferences.proactive_mode} onChange={e=>change('proactive_mode',e.target.value as typeof preferences.proactive_mode)}><option value="off">关闭</option><option value="quiet">安静</option><option value="natural">自然</option><option value="active">活跃</option><option value="custom">自定义</option></select></label>
   <label className="check-row"><input type="checkbox" aria-label="模型生成主动分享" checked={preferences.proactive_model_enabled} onChange={e=>change('proactive_model_enabled',e.target.checked)}/>允许 DeepSeek 生成有来源的主动分享（使用本机 API Key）</label>
   <label>每日主动分享 Token 预算<input aria-label="每日主动分享Token预算" type="number" min={0} max={100000} value={preferences.proactive_token_budget} onChange={e=>change('proactive_token_budget',Number(e.target.value))}/></label>
   <label className="check-row"><input type="checkbox" checked={preferences.notifications_enabled} onChange={e=>change('notifications_enabled',e.target.checked)}/>系统通知（保存时申请本机权限）</label>
   <p className="field-help">不开启模型时使用简短本地分享；开启后，仅对实际完成事件按预算调用。电脑关闭或手机被停止时不保证通知送达。</p>
   <div className="two-fields"><label>最小间隔（分钟）<input aria-label="主动消息最小间隔" type="number" min={60} max={1440} value={preferences.proactive_interval_minutes} onChange={e=>change('proactive_interval_minutes',Number(e.target.value))}/></label><label>每日最多<input aria-label="主动消息每日上限" type="number" min={1} max={6} value={preferences.proactive_daily_limit} onChange={e=>change('proactive_daily_limit',Number(e.target.value))}/></label><label>安静时段开始<input aria-label="安静时段开始" type="time" value={minuteText(preferences.quiet_start_minute)} onChange={e=>change('quiet_start_minute',parseMinute(e.target.value))}/></label><label>安静时段结束<input aria-label="安静时段结束" type="time" value={minuteText(preferences.quiet_end_minute)} onChange={e=>change('quiet_end_minute',parseMinute(e.target.value))}/></label></div>
   <p className="field-help">支持跨午夜；开始与结束相同表示不设安静时段。保存时采用本机时区，当前记录 UTC{preferences.timezone_offset_minutes>=0?'+':''}{preferences.timezone_offset_minutes/60}。更换时区后请重新保存。</p>
   <button className="primary" disabled={!nativeAvailable||busy||!dirty}>{busy?'保存中…':'保存陪伴设置'}</button>
  </form>
 </div>;
}
