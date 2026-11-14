// Explicit DEV UI fixture only; never performs discovery, TLS or database access.
import type {SyncAPI} from './sync-native';
import type {SyncStatus} from './sync-model';
const status:SyncStatus={enabled:false,fingerprint:'fixture-not-a-certificate',device_id:'fixture-only-device',name:'模拟设备',port:54321,candidates:[],peers:[],pending:[],conflicts:[],discovery_error:null};
export const syncFixture:SyncAPI={status:async()=>structuredClone(status),command:async(action)=>{
 if(action==='enable')status.enabled=true;if(action==='disable')status.enabled=false;
 if(action==='pair'||action==='pair_scanned')throw 'PAIR_TOKEN_EXPIRED_OR_INVALID';
 if(action==='cancel')status.pairing_code_active=false;
 if(action==='invite_quick'){
  status.enabled=true;status.pairing_session='fixture-session';status.pairing_code_active=true;
  return {text:JSON.stringify({session:'fixture-session'}),expires_in_seconds:120,svg:'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 240"><rect width="240" height="240" fill="white"/><text x="120" y="112" text-anchor="middle" fill="#273b57" font-size="14">UI 模拟占位</text><text x="120" y="140" text-anchor="middle" fill="#273b57" font-size="12">不能用于真实配对</text></svg>'};
 }
 return null;
}};
