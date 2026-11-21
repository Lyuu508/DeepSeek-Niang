import {Channel,invoke,isTauri} from '@tauri-apps/api/core';
import type {Bootstrap,ChatEvent,ChatFailure,ChatMessage,ChatSettings,Persona,Diagnostics,KeyStatus,MemoryRecord,PreparedChat,StartChat} from '@deepseek/contracts';
import type {CompanionSnapshot,CompanionPreferences,MemoryEdit,LifeDiary,LifePlanInput} from '@deepseek/contracts';
import type {ChatImage} from './ChatAttachment';
export const runtimeMode:'native'|'mock'|'preview'=isTauri()?'native':import.meta.env.DEV&&new URLSearchParams(location.search).get('test')==='mock'?'mock':'preview';
export const nativeAvailable=runtimeMode!=='preview';

export function failure(error:unknown):ChatFailure {
 if(error&&typeof error==='object'&&'code' in error&&'message' in error&&typeof error.message==='string')return error as ChatFailure;
 return {code:'OPERATION_FAILED',message:'操作未能完成，请检查本机运行状态和输入。',retryable:false};
}
export interface RuntimeAPI {
 createLifePlan(personaId:string,input:LifePlanInput):Promise<string>;
 controlLifeTask(personaId:string,taskId:string,action:string):Promise<void>;
 lifeDiary(personaId:string,day:number):Promise<LifeDiary>;
 shareReceipt(personaId:string,id:string,read:boolean,notification:string):Promise<void>;
 claimNotification(personaId:string,id:string):Promise<boolean>;
 companion(personaId:string):Promise<CompanionSnapshot>;
 saveCompanionPreferences(personaId:string,preferences:CompanionPreferences):Promise<void>;
 editMemory(personaId:string,edit:MemoryEdit):Promise<string>;
 resolveMemoryCandidate(personaId:string,id:string,accept:boolean):Promise<void>;
 bootstrap():Promise<Bootstrap>;messages(id:string):Promise<ChatMessage[]>;
 createPersona(name:string,personality:string):Promise<Persona>;selectPersona(id:string):Promise<void>;
 saveSettings(personaId:string,settings:ChatSettings):Promise<void>;setKey(key:string):Promise<KeyStatus>;deleteKey():Promise<KeyStatus>;selfTestVault():Promise<string>;
 startChat(input:StartChat,onEvent:(event:ChatEvent)=>void,image?:ChatImage):Promise<PreparedChat>;stop(requestId:string):Promise<boolean>;
 diagnostics():Promise<Diagnostics>;memories(personaId:string):Promise<MemoryRecord[]>;saveMemory(personaId:string,text:string):Promise<void>;deleteMemory(personaId:string,id:string):Promise<void>;
}
async function call<T>(command:string,args?:Record<string,unknown>):Promise<T>{
 if(!isTauri())throw {code:'NATIVE_REQUIRED',message:'浏览器仅提供界面预览。请使用 Windows 或 Android 原生应用。',retryable:false};
 return invoke<T>(command,args);
}
const channels=new Map<string,Channel<ChatEvent>>();
const nativeApi:RuntimeAPI={
 createLifePlan:(personaId,input)=>call('life_plan_create',{personaId,input}),
 controlLifeTask:(personaId,taskId,action)=>call('life_task_control',{personaId,taskId,action}),
 lifeDiary:(personaId,day)=>call('life_diary',{personaId,day}),
 shareReceipt:(personaId,id,read,notification)=>call('share_receipt',{personaId,id,read,notification}),
 claimNotification:(personaId,id)=>call('claim_share_notification',{personaId,id}),
 companion:personaId=>call('companion_snapshot',{personaId}),
 saveCompanionPreferences:(personaId,preferences)=>call('save_companion_preferences',{personaId,preferences}),
 editMemory:(personaId,edit)=>call('edit_companion_memory',{personaId,edit}),
 resolveMemoryCandidate:(personaId,id,accept)=>call('resolve_memory_candidate',{personaId,id,accept}),
 bootstrap:()=>call('bootstrap'),messages:conversationId=>call('list_messages',{conversationId}),
 createPersona:(name,personality)=>call('create_persona',{name,personality}),selectPersona:id=>call('select_persona',{id}),
 saveSettings:(personaId,settings)=>call('save_chat_settings',{personaId,settings}),setKey:key=>call('set_api_key',{key}),deleteKey:()=>call('delete_api_key'),selfTestVault:()=>call('vault_self_test'),
 startChat:async(input,onEvent,image)=>{
  const channel=new Channel<ChatEvent>();channels.set(input.request_id,channel);
  channel.onmessage=event=>{onEvent(event);if(event.type==='finished'||event.type==='failed')channels.delete(event.request_id);};
  try{return await call('start_chat',{input,onEvent:channel,image:image??null});}catch(error){channels.delete(input.request_id);throw error;}
 },stop:requestId=>call('stop_generation',{requestId}),
 diagnostics:()=>call('diagnostics'),memories:personaId=>call('list_memories',{personaId}),saveMemory:(personaId,text)=>call('save_memory',{personaId,text}),deleteMemory:(personaId,id)=>call('delete_memory',{personaId,id}),
};
// DEV-only fixture branch is eliminated from production bundles. Never an API fallback.
export const native:RuntimeAPI=import.meta.env.DEV&&runtimeMode==='mock'?(await import('./test-fixture')).fixtureApi:nativeApi;
