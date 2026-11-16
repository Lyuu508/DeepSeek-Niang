import {invoke} from '@tauri-apps/api/core';
import {isPermissionGranted,requestPermission} from '@tauri-apps/plugin-notification';
import type {CompanionSnapshot} from '@deepseek/contracts';
import {native,runtimeMode} from './native';
export async function enableNotifications():Promise<boolean>{
 if(runtimeMode!=='native')return false;
 return await isPermissionGranted()||await requestPermission()==='granted';
}
export async function notifyShare(data:CompanionSnapshot,id:string,title:string):Promise<void>{
 if(runtimeMode!=='native'||!await isPermissionGranted())return;
 if(!await native.claimNotification(data.persona_id,id))return;
 try {await invoke('plugin:notification|notify',{options:{title,body:'有一条新的虚拟生活分享，打开应用查看。',id:parseInt(id.replaceAll('-','').slice(0,7),16)}});await native.shareReceipt(data.persona_id,id,false,'shown');}
 catch {await native.shareReceipt(data.persona_id,id,false,'failed');}
}
