import { Icon, PageHeader } from './OceanDesign';
import {useCallback,useEffect,useRef,useState} from 'react';
import {syncAPI,type PairCode} from './sync-native';
import {syncErrors,syncLabel,quickInvitation,apiConfigurationLabels,type SyncStatus} from './sync-model';
import {runtimeMode,nativeAvailable} from './native';
import {PairScanner} from './PairScanner';
import {Modal} from './Modal';

export function SyncPanel({mobile,onPaired}:{mobile:boolean;onPaired?:()=>Promise<void>}){
 const [status,setStatus]=useState<SyncStatus|null>(null);const [error,setError]=useState('');const [busy,setBusy]=useState(false);
 const [code,setCode]=useState<PairCode|null>(null);const [seconds,setSeconds]=useState(0);const [scanning,setScanning]=useState(false);
 const [notice,setNotice]=useState('');const [revokeId,setRevokeId]=useState('');const alive=useRef(true);
 const refresh=useCallback(async()=>{if(!nativeAvailable)return;try{const data=await syncAPI.status();if(alive.current)setStatus(data);}catch(e){if(alive.current)setError(typeof e==='string'?e:'无法读取连接状态，请稍后重试。');}},[]);
 useEffect(()=>{alive.current=true;void refresh();const timer=setInterval(()=>void refresh(),2000);return()=>{alive.current=false;clearInterval(timer);void syncAPI.command('cancel').catch(()=>{});};},[refresh]);
 const cancelCode=useCallback(()=>{setCode(null);void syncAPI.command('cancel').catch(()=>{});},[]);
 useEffect(()=>{if(!code)return;const end=Date.now()+code.expires_in_seconds*1000;setSeconds(code.expires_in_seconds);const timer=setInterval(()=>{const remaining=Math.max(0,Math.ceil((end-Date.now())/1000));setSeconds(remaining);if(!remaining){cancelCode();setNotice('二维码已过期，请点击“连接手机”重新生成。');}},1000);return()=>clearInterval(timer);},[code,cancelCode]);
 useEffect(()=>{if(code&&status&&status.pairing_session===JSON.parse(code.text).session&&status.pairing_code_active===false){setCode(null);setNotice('二维码已使用或失效，请查看设备连接状态。');}},[code,status?.pairing_code_active,status?.pairing_session]);
 async function command(action:string,args:Parameters<typeof syncAPI.command>[1]={}){
  setBusy(true);setError('');setNotice('');
  try{
   const result=await syncAPI.command(action,args);if(!alive.current)return;
   if(action==='invite_quick'){if(!result)throw new Error();setCode(result);}
   if(action==='pair_scanned'){await onPaired?.();setNotice('电脑人格与聊天已同步，后续自动增量同步。');}
   if(action==='sync')setNotice('已请求同步，完成进度会显示在设备下方。');
   if(action==='revoke')setRevokeId('');
   await refresh();
  }catch(e){if(action==='pair_scanned')await onPaired?.().catch(()=>{});if(alive.current)setError(typeof e==='string'?e:'连接操作未完成，请稍后重试。');}finally{if(alive.current)setBusy(false);}
 }
 function scanned(value:string){setScanning(false);if(quickInvitation(value))void command('pair_scanned',{text:value});else setError('请扫描电脑“连接手机”弹窗中的二维码。');}
 return <div className="settings-content sync-panel">
  <PageHeader title={mobile?'连接电脑':'局域网连接'} icon="sync" art="wave" scene="scene-settings"
   description={mobile?'扫一扫电脑上的二维码，聊天和记忆自动同步。':'电脑和手机接入同一路由器即可配对。'}/>
  {runtimeMode==='mock'&&<p className="notice">本页为 UI 模拟夹具，未验证真实设备发现、配对或同步。</p>}
  {notice&&<p className="notice" role="status">{notice}</p>}
  {error&&<p className="notice inline-error" role="alert">{syncErrors[error]??error}</p>}
  <section className="settings-card connect-card" aria-label="扫码连接">
   <div className="connect-symbol" aria-hidden="true"><svg viewBox="0 0 48 48" fill="none" stroke="currentColor" strokeWidth="2.5"><path d="M5 17V7a2 2 0 0 1 2-2h10m14 0h10a2 2 0 0 1 2 2v10M5 31v10a2 2 0 0 0 2 2h10m14 0h10a2 2 0 0 0 2-2V31"/><path d="M13 13h7v7h-7zm15 0h7v7h-7zM13 28h7v7h-7zm15 0h7v7h-7z"/></svg></div>
   <h2>{mobile?'让手机和电脑连在一起':'手机一扫，接着聊'}</h2>
   <p>{mobile?'连接前请打开电脑端的“连接手机”。':'点击下方按钮，让手机扫描弹出的二维码。'}</p>
   <button className="primary connect-button" disabled={!nativeAvailable||busy} onClick={()=>{setError('');if(mobile)setScanning(true);else void command('invite_quick');}}>{busy?'正在连接…':mobile?'扫一扫':'连接手机'}</button>
   <small>扫码将同步电脑人格、聊天和记忆。电脑已配置 API 时，会通过加密连接更新手机配置；密钥只保存在系统安全存储。</small>
  </section>
  {code&&!mobile&&<Modal title="用手机扫一扫" closeLabel="关闭二维码" onClose={cancelCode} className="qr-modal"><p>打开手机端“连接”里的“扫一扫”。电脑已配置 API 时，会同时安全配置手机；已有手机密钥会在本次主动扫码时更新。</p><img className="pair-qr" alt="一次性连接二维码" src={`data:image/svg+xml,${encodeURIComponent(code.svg)}`}/><p className="qr-expiry" role="status">{seconds} 秒后过期 · 连接成功自动关闭</p><small>仅向你信任的手机展示。关闭弹窗会立即取消本次连接授权。</small></Modal>}
  {scanning&&mobile&&<Modal title="扫一扫" closeLabel="关闭扫码" onClose={()=>setScanning(false)} className="scanner-modal"><p>本次扫码同意使用电脑已配置的 API；现有手机配置可能更新。二维码不包含 API Key 明文。</p><PairScanner onResult={scanned}/></Modal>}
  <section className="settings-card connected-card"><div className="card-heading"><h2>已配对设备</h2><button disabled={!nativeAvailable||busy||!status?.enabled} onClick={()=>void command('sync')}>手动同步</button></div>
   {!status?.peers.some(peer=>!peer.revoked)&&<div className="connection-empty"><Icon name="sync"/><p>还没有连接设备</p><small>{mobile?'扫描电脑上的二维码开始连接':'让手机扫一扫，连接后会显示在这里'}</small></div>}
   {status?.peers.filter(peer=>!peer.revoked).map(peer=><article className="sync-device" key={peer.id}><h3>{peer.name}<span className="badge">{syncLabel(peer)}</span></h3><p>{peer.last_success_ms?`最近同步：${new Date(peer.last_success_ms).toLocaleString()}`:'正在等待首次同步'} · 待同步 {peer.pending} 项</p>{peer.progress.error&&<p className="inline-error">{syncErrors[peer.progress.error]??peer.progress.error}</p>}{peer.api_configuration&&apiConfigurationLabels[peer.api_configuration]&&<p className="credential-status">{apiConfigurationLabels[peer.api_configuration]}</p>}<details><summary>连接诊断与管理</summary><p>最近状态：{syncLabel(peer)}{peer.progress.error?' · 错误代码：'+peer.progress.error:''} · 已传输 {peer.progress.transferred} 项</p><p className="sync-fingerprint">设备：{peer.id}<br/>证书：{peer.fingerprint}</p><button onClick={()=>setRevokeId(peer.id)}>解除连接</button>{revokeId===peer.id&&<div className="button-row"><p>解除后已保存的记录和已接收的 API 配置仍保留。可在手机设置删除密钥；要让共享的旧 Key 失效，需要在服务商处更换。</p><button className="danger" disabled={busy} onClick={()=>void command('revoke',{id:peer.id})}>确认解除连接</button><button onClick={()=>setRevokeId('')}>取消</button></div>}</details></article>)}
  </section>
  {!!status?.conflicts.length&&<section className="settings-card"><h2>保留的并发修订</h2><p>两端同时修改时保留候选，不丢弃记录。可在对应人格或记忆页面重新编辑。</p>{status.conflicts.map((conflict,index)=><details key={index}><summary>{conflict.entity} · {conflict.field}</summary><p className="sync-fingerprint">{conflict.value}</p></details>)}</section>}
  <details className="settings-card connection-help"><summary>连接遇到问题？</summary><p>两端应用保持打开，使用同一路由器的普通网络。访客 Wi-Fi、AP 隔离或不同 VLAN 可能阻止连接。</p><p>Windows 允许本应用访问专用网络，保留防火墙。网络恢复后自动补传，也可点击“手动同步”重试。</p>{status?.discovery_error&&<p className="inline-error">设备发现暂时不可用。二维码包含连接地址，可重新扫码连接。</p>}</details>
  <section className="settings-card local-device-info" aria-label="本机信息"><details><summary>本机信息</summary><p>{status?.enabled?'局域网连接已启用，离线时仍可聊天。':'尚未启用局域网连接。'}</p><dl><dt>设备名称</dt><dd>{status?.name??(mobile?'Android 手机':'Windows 电脑')}</dd><dt>设备 UUID</dt><dd>{status?.device_id??'读取中'}</dd><dt>证书 SHA-256</dt><dd>{status?.fingerprint||'建立连接时生成'}</dd><dt>局域网地址</dt><dd>{status?.local_addresses?.join(' / ')||'建立连接后显示'}</dd></dl></details></section>
 </div>;
}
