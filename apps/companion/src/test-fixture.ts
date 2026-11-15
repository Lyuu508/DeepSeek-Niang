// Explicit DEV-only UI fixture. Not an API, SQLite, persistence or OS vault test.
import type {Bootstrap,ChatEvent,ChatMessage,ChatSettings,MemoryRecord,Persona,PreparedChat,UsageSummary} from '@deepseek/contracts';
import type {RuntimeAPI} from './native';
import type {CompanionPreferences,CompanionSnapshot,ManagedMemory} from '@deepseek/contracts';
import {previewSettings} from './defaults';
const initial=crypto.randomUUID();
const personas:Persona[]=[{id:initial,name:'DeepSeek娘',main_conversation_id:initial,is_builtin:true,created_at_ms:Date.now(),revision:1}];
const settings:Record<string,ChatSettings>={[initial]:{...previewSettings}};
const messages:Record<string,ChatMessage[]>={[initial]:[]};const memories:Record<string,MemoryRecord[]>={[initial]:[]};
const usage:Record<string,UsageSummary>={[initial]:{requests:0,measured_requests:0,prompt_tokens:0,completion_tokens:0}};
const managed:Record<string,ManagedMemory>={};const preferences:Record<string,CompanionPreferences>={};
function companionPreferences(id:string):CompanionPreferences{return preferences[id]??{relationship_enabled:true,automatic_memory:true,memory_context:true,memory_limit:6,memory_budget_bytes:2000,context_budget_bytes:48000,life_enabled:true,preferred_activity:'auto',proactive_enabled:false,proactive_interval_minutes:240,proactive_daily_limit:2,quiet_start_minute:1320,quiet_end_minute:480,timezone_offset_minutes:480};}
let selected:string=initial;let configured=false;
let active:{request:string;persona:string;message:ChatMessage;timer:ReturnType<typeof setTimeout>;callback:(event:ChatEvent)=>void}|null=null;
function owned(id:string){const p=personas.find(p=>p.id===id);if(!p)throw {code:'INVALID_PERSONA',message:'模拟测试：人格不存在。',retryable:false};return p;}
function status(){return {configured,backend:'模拟内存凭据（非系统安全存储）',error:null};}
function end(error=false,stopped=false){
 if(!active)return;const job=active;clearTimeout(job.timer);active=null;
 job.message.state=stopped?'interrupted':error?'failed':'complete';job.message.revision++;job.message.error_code=stopped?'CANCELLED':error?'NETWORK_ERROR':null;
 const count=usage[job.persona];count.requests++;if(!error&&!stopped){count.measured_requests++;count.prompt_tokens+=12;count.completion_tokens+=18;}
 job.callback({type:'finished',request_id:job.request,conversation_id:job.persona,message:{...job.message},usage:!error&&!stopped?{prompt_tokens:12,completion_tokens:18,total_tokens:30}:null,error:error?{code:'NETWORK_ERROR',message:'模拟测试：网络连接失败。',retryable:true}:null});
}
export const fixtureApi:RuntimeAPI={
 companion:async id=>{owned(id);const zero={requests:0,measured_requests:0,unknown_requests:0,prompt_tokens:0,completion_tokens:0};return structuredClone({persona_id:id,preferences:companionPreferences(id),relationship:{familiarity:5000,trust:10000,affection:10000,intimacy:0,comfort:30000,stage:'初识',rule_version:2,applied_events:0,distinct_days:0,history:[]},mood:{kind:'calm',label:'平静',intensity_milli:200,source:'UI 模拟夹具',started_at_ms:null,expires_at_ms:null},memories:memories[id].map(m=>managed[m.id]??{id:m.id,text:m.text,version:m.version_lamport,revision:m.id,source_excerpt:null,kind:'semantic',importance_milli:500,pinned:false,api_allowed:false,owner_locked:true,source_kind:'legacy',source_message_id:null,created_at_ms:null,updated_at_ms:null}),memory_candidates:[],life:{host_device_id:null,heartbeat_at_ms:null,host_online:false,current:null,next_activity:null,today_events:[],proactive_messages:[],day_start_ms:0,execution_kind:'virtual',external_media_executed:false},daily_usage:{day_start_ms:0,timezone_offset_minutes:480,by_purpose:{chat:{...zero,...usage[id],unknown_requests:usage[id].requests-usage[id].measured_requests},memory:{...zero},life:{...zero}},memory_and_life_provider:'UI fixture'}} satisfies CompanionSnapshot);},
 saveCompanionPreferences:async(id,value)=>{owned(id);preferences[id]=structuredClone(value);},
 editMemory:async(id,edit)=>{owned(id);let row=edit.id?memories[id].find(m=>m.id===edit.id):undefined;if(edit.id&&(!row||edit.expected_version!==row.version_lamport||edit.expected_revision!==(managed[row.id]?.revision??row.id)))throw {code:'DATA_VERSION_CONFLICT',message:'模拟测试：记忆版本已改变。',retryable:false};if(!row){row={id:crypto.randomUUID(),text:'',version_lamport:0};memories[id].push(row);}row.text=edit.text;row.version_lamport++;managed[row.id]={id:row.id,text:row.text,version:row.version_lamport,revision:crypto.randomUUID(),source_excerpt:null,kind:edit.kind,importance_milli:edit.importance_milli,pinned:edit.pinned,api_allowed:edit.api_allowed,owner_locked:true,source_kind:'manual',source_message_id:null,created_at_ms:managed[row.id]?.created_at_ms??Date.now(),updated_at_ms:Date.now()};return row.id;},
 resolveMemoryCandidate:async()=>{throw {code:'MOCK_NO_CANDIDATE',message:'UI 夹具没有真实记忆候选。',retryable:false};},
 bootstrap:async()=>structuredClone({version:'0.0.4',platform:window.matchMedia('(pointer:coarse)').matches?'android':'windows',settings:settings[selected],personas,active_persona_id:selected,persona_summary:{memory_count:memories[selected].length,activity_count:0,interest_count:0,relationship:{status:'not_evaluated'},mood:{status:'unknown'},life:{status:'not_started'}},key_status:status(),usage:usage[selected],active_request:active?.request??null,active_conversation:active?.persona??null,schema_version:7} satisfies Bootstrap),
 messages:async id=>{owned(id);const snapshot=structuredClone(messages[id]);const delay=Math.min(3000,Math.max(0,Number(new URLSearchParams(location.search).get('read-delay'))||0));if(delay)await new Promise(resolve=>setTimeout(resolve,delay));return snapshot;},
 createPersona:async(name,personality)=>{const id=crypto.randomUUID();const p={id,name:name.trim(),main_conversation_id:id,is_builtin:false,created_at_ms:Date.now(),revision:1};personas.push(p);settings[id]={...previewSettings,character_name:p.name,personality};messages[id]=[];memories[id]=[];usage[id]={requests:0,measured_requests:0,prompt_tokens:0,completion_tokens:0};return {...p};},
 selectPersona:async id=>{owned(id);selected=id;},
 saveSettings:async(id,value)=>{const p=owned(id);settings[id]={...value};p.name=value.character_name;p.revision++;},
 setKey:async key=>{if(!key.startsWith('fixture-'))throw {code:'MOCK_KEY_ONLY',message:'模拟模式仅接受 fixture- 开头的测试文本，请勿输入真实密钥。',retryable:false};end(false,true);configured=true;return status();},
 deleteKey:async()=>{end(false,true);configured=false;return status();},selfTestVault:async()=>'模拟内存测试：未验证 Windows / Android 安全存储',
 startChat:async(input,callback)=>{
  if(active)throw {code:'BUSY',message:'模拟测试：已有生成任务。',retryable:false};const owner=owned(input.conversation_id);const list=messages[owner.id];
  const user=input.regenerate_user_id?list.find(m=>m.id===input.regenerate_user_id)!:{id:crypto.randomUUID(),conversation_id:owner.id,role:'user',content:input.text??'',state:'complete' as const,created_at_ms:Date.now(),model_id:null,request_id:input.request_id,user_message_id:null,revision:0,selected:true,error_code:null,finish_reason:null};
  if(!input.regenerate_user_id)list.push(user);list.filter(m=>m.user_message_id===user.id).forEach(m=>{m.selected=false;});
  const message:ChatMessage={id:crypto.randomUUID(),conversation_id:owner.id,role:'assistant',content:'',state:'sending',created_at_ms:Date.now(),model_id:settings[owner.id].model,request_id:input.request_id,user_message_id:user.id,revision:0,selected:true,error_code:null,finish_reason:null};list.push(message);
  const request:PreparedChat={request_id:input.request_id,conversation_id:owner.id,user_message_id:user.id,assistant_id:message.id,is_new:true};callback({type:'started',request,user:{...user},message:{...message}});
  const chunks=Array.from(`模拟测试回复：这里是${owner.name}的独立时间线。这段文字来自 UI 测试夹具，不是真实 DeepSeek API。`);let index=0;
  const next=()=>{if(!active)return;if(user.content.includes('触发错误')){end(true);return;}if(index===chunks.length){end();return;}message.content+=chunks[index++];message.state='generating';message.revision++;callback({type:'delta',request_id:request.request_id,conversation_id:owner.id,message_id:message.id,content:message.content,revision:message.revision});active.timer=setTimeout(next,22);};
  active={request:request.request_id,persona:owner.id,message,timer:setTimeout(next,30),callback};return request;
 },
 stop:async id=>{if(active?.request===id){end(false,true);return true;}return false;},
 diagnostics:async()=>({version:'UI fixture',sqlite_version:'未连接（模拟）',schema_version:7,device_id:'fixture',profile_id:'fixture',message_count:Object.values(messages).flat().length,memory_count:Object.values(memories).flat().length,event_count:0,integrity:'mock_not_checked'}),
 memories:async id=>{owned(id);return structuredClone(memories[id]);},saveMemory:async(id,text)=>{owned(id);memories[id].push({id:crypto.randomUUID(),text,version_lamport:1});},deleteMemory:async(id,key)=>{owned(id);const i=memories[id].findIndex(m=>m.id===key);if(i<0)throw {code:'UNTRUSTED',message:'模拟测试：记忆不属于此人格。',retryable:false};memories[id].splice(i,1);},
};
