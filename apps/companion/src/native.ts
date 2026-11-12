import {Channel,invoke,isTauri} from '@tauri-apps/api/core';
import type {Bootstrap,ChatEvent,ChatFailure,ChatMessage,ChatSettings,Persona,Diagnostics,KeyStatus,MemoryRecord,PreparedChat,StartChat} from '@deepseek/contracts';
export const runtimeMode:'native'|'mock'|'preview'=isTauri()?'native':import.meta.env.DEV&&new URLSearchParams(location.search).get('test')==='mock'?'mock':'preview';
export const nativeAvailable=runtimeMode!=='preview';

export function failure(error:unknown):ChatFailure {
 if(error&&typeof error==='object'&&'code' in error&&'message' in error&&typeof error.message==='string')return error as ChatFailure;
 return {code:'OPERATION_FAILED',message:'操作未能完成，请检查本机运行状态和输入。',retryable:false};
}
export interface RuntimeAPI {
 bootstrap():Promise<Bootstrap>;messages(id:string):Promise<ChatMessage[]>;
 createPersona(name:string,personality:string):Promise<Persona>;selectPersona(id:string):Promise<void>;
 saveSettings(personaId:string,settings:ChatSettings):Promise<void>;setKey(key:string):Promise<KeyStatus>;deleteKey():Promise<KeyStatus>;selfTestVault():Promise<string>;
 startChat(input:StartChat,onEvent:(event:ChatEvent)=>void):Promise<PreparedChat>;stop(requestId:string):Promise<boolean>;
 diagnostics():Promise<Diagnostics>;memories(personaId:string):Promise<MemoryRecord[]>;saveMemory(personaId:string,text:string):Promise<void>;deleteMemory(personaId:string,id:string):Promise<void>;
}
async function call<T>(command:string,args?:Record<string,unknown>):Promise<T>{
 if(!isTauri())throw {code:'NATIVE_REQUIRED',message:'浏览器仅提供界面预览。请使用 Windows 或 Android 原生应用。',retryable:false};
 return invoke<T>(command,args);
}
const channels=new Map<string,Channel<ChatEvent>>();
const nativeApi:RuntimeAPI={
 bootstrap:()=>call('bootstrap'),messages:conversationId=>call('list_messages',{conversationId}),
 createPersona:(name,personality)=>call('create_persona',{name,personality}),selectPersona:id=>call('select_persona',{id}),
 saveSettings:(personaId,settings)=>call('save_chat_settings',{personaId,settings}),setKey:key=>call('set_api_key',{key}),deleteKey:()=>call('delete_api_key'),selfTestVault:()=>call('vault_self_test'),
 startChat:async(input,onEvent)=>{
  const channel=new Channel<ChatEvent>();channels.set(input.request_id,channel);
  channel.onmessage=event=>{onEvent(event);if(event.type==='finished'||event.type==='failed')channels.delete(event.request_id);};
  try{return await call('start_chat',{input,onEvent:channel});}catch(error){channels.delete(input.request_id);throw error;}
 },stop:requestId=>call('stop_generation',{requestId}),
 diagnostics:()=>call('diagnostics'),memories:personaId=>call('list_memories',{personaId}),saveMemory:(personaId,text)=>call('save_memory',{personaId,text}),deleteMemory:(personaId,id)=>call('delete_memory',{personaId,id}),
};
// DEV-only fixture branch is eliminated from production bundles. Never an API fallback.
export const native:RuntimeAPI=import.meta.env.DEV&&runtimeMode==='mock'?(await import('./test-fixture')).fixtureApi:nativeApi;
