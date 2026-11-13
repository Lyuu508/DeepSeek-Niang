import {invoke} from '@tauri-apps/api/core';
import {runtimeMode} from './native';
import type {SyncStatus} from './sync-model';
export interface PairCode {text:string;svg:string;expires_in_seconds:number}
export interface SyncAPI {status():Promise<SyncStatus>;command(action:string,args?:{id?:string;text?:string;address?:string}):Promise<PairCode|null>}
const api:SyncAPI={status:async()=>invoke('sync_status'),command:async(action,args={})=>invoke('sync_command',{action,id:null,text:null,address:null,...args})};
export const syncAPI:SyncAPI=import.meta.env.DEV&&runtimeMode==='mock'?(await import('./sync-fixture')).syncFixture:api;
