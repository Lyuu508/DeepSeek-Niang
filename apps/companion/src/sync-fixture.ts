// Explicit DEV UI fixture only; never performs discovery, TLS or database access.
import type {SyncAPI} from './sync-native';
import type {SyncStatus} from './sync-model';
const status:SyncStatus={enabled:false,fingerprint:'fixture-not-a-certificate',device_id:'fixture-only-device',name:'模拟设备',port:54321,candidates:[],peers:[],pending:[],conflicts:[],discovery_error:null};
export const syncFixture:SyncAPI={status:async()=>structuredClone(status),command:async(action)=>{if(action==='enable')status.enabled=true;if(action==='disable')status.enabled=false;if(action==='pair')throw 'PAIR_TOKEN_EXPIRED_OR_INVALID';return null;}};
