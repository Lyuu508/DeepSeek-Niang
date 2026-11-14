export interface SyncProgress {state:string;error:string|null;transferred:number;last_attempt_ms:number|null}
export interface SyncPeer {id:string;name:string;fingerprint:string;revoked:boolean;address:string|null;last_success_ms:number|null;pending:number;progress:SyncProgress}
export interface SyncStatus {enabled:boolean;fingerprint:string;device_id:string;name:string;port:number;candidates:{instance:string;address:string}[];peers:SyncPeer[];pending:{id:string;name:string;fingerprint:string;direction:string;approved:boolean}[];conflicts:{entity:string;field:string;value:string;event:string}[];discovery_error:string|null;pairing_code_active?:boolean;local_addresses?:string[];pairing_session?:string|null}
export const stateLabels:Record<string,string>={discovering:'正在发现设备',connecting:'正在连接',authenticating:'正在验证设备',syncing:'正在同步',succeeded:'同步成功',failed:'同步失败',waiting_reconnect:'等待重新连接',revoked:'已解除配对'};
export function syncLabel(peer:SyncPeer,now=Date.now()):string {
 if(peer.revoked)return stateLabels.revoked;
 if(peer.progress.state==='succeeded'&&(!peer.last_success_ms||now-peer.last_success_ms>60000||peer.pending>0))return peer.pending>0?'有数据待同步':'等待重新连接';
 return stateLabels[peer.progress.state]??'等待重新连接';
}
export function localAddress(value:string):boolean {
 const m=/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3}):(\d{1,5})$/.exec(value);
 if(!m)return false;const [a,b,c,d,p]=m.slice(1).map(Number);
 return [a,b,c,d].every(n=>n<=255)&&p>0&&p<=65535&&(a===10||a===172&&b>=16&&b<=31||a===192&&b===168);
}
export function quickInvitation(value:string):boolean {
 try {const code=JSON.parse(value);return value.length<=8192&&code.version===2&&code.quick===true&&typeof code.session==='string'&&/^[0-9a-f]{64}$/.test(code.token)&&Array.isArray(code.addresses)&&code.addresses.length>0&&code.addresses.length<=8&&code.addresses.every((address:unknown)=>typeof address==='string'&&localAddress(address))&&typeof code.identity?.certificate==='string';}catch{return false;}
}
export const syncErrors:Record<string,string>={LAN_PRIVATE_IP_REQUIRED:'请输入局域网 IPv4 和端口，例如 192.168.1.20:54321。',PEER_UNREACHABLE:'设备暂时无法连接，将自动重试。请检查应用是否打开、路由器隔离和专用网络防火墙。',PAIR_TOKEN_EXPIRED_OR_INVALID:'配对码已过期、错误或已取消，请重新生成。',PEER_AUTHENTICATION_FAILED:'设备身份验证失败。请核对证书指纹，不要直接信任变化的身份。',SECURE_STORAGE_UNAVAILABLE:'系统安全存储不可用。聊天数据保留在本机，同步未启动。',DISCOVERY_UNAVAILABLE:'自动发现未能启动；检查本地网络权限后重试，或输入已知局域网地址。'};
Object.assign(syncErrors,{INVALID_SYNC_INPUT:'配对码无法识别，请重新扫描电脑上的连接码。',WAITING_FOR_BOTH_CONFIRMATIONS:'电脑尚未授权此扫码连接，请重新显示扫码连接码，或使用兼容配对。',CRYPTOGRAPHY_UNAVAILABLE:'系统加密组件暂时不可用，请重启应用后再试。'});
