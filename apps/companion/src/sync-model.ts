export interface SyncProgress {state:string;error:string|null;transferred:number;last_attempt_ms:number|null}
export interface SyncPeer {api_configuration?:string|null;id:string;name:string;fingerprint:string;revoked:boolean;address:string|null;last_success_ms:number|null;pending:number;progress:SyncProgress}
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
 try {const code=JSON.parse(value);return value.length<=8192&&[2,3].includes(code.version)&&code.quick===true&&typeof code.session==='string'&&/^[0-9a-f]{64}$/.test(code.token)&&Array.isArray(code.addresses)&&code.addresses.length>0&&code.addresses.length<=8&&code.addresses.every((address:unknown)=>typeof address==='string'&&localAddress(address))&&typeof code.identity?.certificate==='string';}catch{return false;}
}
export const syncErrors:Record<string,string>={LAN_PRIVATE_IP_REQUIRED:'未找到可用的局域网地址。请先让电脑连接路由器，再重新显示二维码。',PEER_UNREACHABLE:'设备暂时无法连接，将自动重试。请检查应用是否打开、路由器隔离和专用网络防火墙。',PAIR_TOKEN_EXPIRED_OR_INVALID:'二维码已过期或不可用。请在电脑重新点击“连接手机”后再扫。',PEER_AUTHENTICATION_FAILED:'连接身份不匹配。请在电脑重新显示二维码后再扫，不要信任来源不明的二维码。',SECURE_STORAGE_UNAVAILABLE:'系统安全存储不可用。聊天数据保留在本机，同步未启动。',DISCOVERY_UNAVAILABLE:'自动发现暂不可用。请检查本地网络权限并重新扫码，二维码中已包含电脑的连接地址。'};
Object.assign(syncErrors,{INVALID_SYNC_INPUT:'配对码无法识别，请重新扫描电脑上的连接码。',WAITING_FOR_BOTH_CONFIRMATIONS:'电脑尚未授权本次连接。请在电脑重新点击“连接手机”，扫描新的二维码。',CRYPTOGRAPHY_UNAVAILABLE:'系统加密组件暂时不可用，请重启应用后再试。'});

Object.assign(syncErrors,{SYNC_VERSION_MISMATCH:'两端数据库版本不一致。请同时安装本次 Windows 和 Android 版本；原有数据仍保留。'});

Object.assign(syncErrors,{
 SYNC_DATA_SCHEMA_MISMATCH:'设备身份已验证，但数据格式不匹配。请同时更新电脑和手机到本次修复版；保留原数据，不要卸载或清空。',
 SYNC_DATA_REJECTED:'设备已建立加密连接，但数据校验未通过。原记录已保留；请查看下方错误代码和两端版本。',
 SYNC_DATA_CONFLICT:'检测到现有主机身份或历史记录冲突，已阻止覆盖。请保留原数据并核对两端安装来源。',
 SYNC_CHECKPOINT_GAP:'增量记录暂不连续，未确认接收未落盘的数据。请保留数据库并重新打开两端重试。',
 PAIR_KEY_TRANSFER_FAILED:'设备配对和人格同步已保留，但手机 API 配置尚未安全写入。请重新扫码或在手机设置中检查安全存储。',
 SYNC_FRAME_LIMIT:'本批数据超过传输上限，已保留本机记录。请保留错误代码供排查。'
});
export const apiConfigurationLabels:Record<string,string>={received:'已安全接收电脑 API 配置',updated:'已安全更新为电脑 API 配置',source_not_configured:'电脑未配置 API，手机已有配置保持',peer_update_required:'更新电脑端后可扫码配置 API',secure_storage_error:'手机安全存储未能完成 API 配置',transfer_failed:'API 配置传递未完成，请重新扫码'};
