import {useCallback,useEffect,useRef,useState} from 'react';
import jsQR from 'jsqr';
import {syncAPI,type PairCode} from './sync-native';
import {syncErrors,syncLabel,localAddress,quickInvitation,type SyncStatus} from './sync-model';
import {runtimeMode,nativeAvailable} from './native';
import {PairScanner} from './PairScanner';
export function SyncPanel(){
 const [status,setStatus]=useState<SyncStatus|null>(null);const [error,setError]=useState('');const [busy,setBusy]=useState(false);
 const [code,setCode]=useState<PairCode|null>(null);const [seconds,setSeconds]=useState(0);const [text,setText]=useState('');const [address,setAddress]=useState('');const [confirmId,setConfirmId]=useState('');
 const [scanning,setScanning]=useState(false);const [notice,setNotice]=useState('');const [legacy,setLegacy]=useState(false);
 const alive=useRef(true);
 const refresh=useCallback(async()=>{if(!nativeAvailable)return;try{const data=await syncAPI.status();if(alive.current)setStatus(data);}catch(e){if(alive.current)setError(typeof e==='string'?e:'无法读取同步状态');}},[]);
 useEffect(()=>{alive.current=true;void refresh();const timer=setInterval(()=>void refresh(),2000);return()=>{alive.current=false;clearInterval(timer);void syncAPI.command('cancel').catch(()=>{});};},[refresh]);
 useEffect(()=>{if(!code)return;const end=Date.now()+code.expires_in_seconds*1000;setSeconds(code.expires_in_seconds);const timer=setInterval(()=>{const remaining=Math.max(0,Math.ceil((end-Date.now())/1000));setSeconds(remaining);if(!remaining){setCode(null);void syncAPI.command('cancel').catch(()=>{});}},1000);return()=>clearInterval(timer);},[code]);
 useEffect(()=>{if(code&&status?.pairing_session===JSON.parse(code.text).session&&status?.pairing_code_active===false){setCode(null);setNotice('配对码已使用或失效，请查看下方设备的同步进度。');}},[code,status?.pairing_code_active,status?.pairing_session]);
 async function command(action:string,args:Parameters<typeof syncAPI.command>[1]={}){setBusy(true);setError('');setNotice('');try{const result=await syncAPI.command(action,args);if(action==='invite'||action==='invite_quick')setCode(result);if(action==='pair_scanned'){setNotice('已安全配对，正在自动同步。完成后下方显示“同步成功”。');setText('');}if(action==='cancel'||action==='disable')setCode(null);await refresh();}catch(e){setError(typeof e==='string'?e:'同步操作未完成');}finally{setBusy(false);}}
 function scanned(value:string){setScanning(false);setText(value);if(quickInvitation(value)){void command('pair_scanned',{text:value});}else{setLegacy(true);setError('这是旧版配对码或无效二维码。旧版可在兼容模式中选择地址后配对。');}}
 async function scan(file:File){let bitmap:ImageBitmap|null=null;try{bitmap=await createImageBitmap(file);if(bitmap.width*bitmap.height>20000000)throw new Error();const canvas=document.createElement('canvas');canvas.width=bitmap.width;canvas.height=bitmap.height;const ctx=canvas.getContext('2d');if(!ctx)throw new Error();ctx.drawImage(bitmap,0,0);const image=ctx.getImageData(0,0,canvas.width,canvas.height);const found=jsQR(image.data,image.width,image.height);if(!found)throw new Error();scanned(found.data);}catch{setError('未识别到配对二维码，请重新拍摄或粘贴一次性配对码。');}finally{bitmap?.close();}}
 return <div className="settings-content sync-panel"><div className="page-heading"><span className="eyebrow">TOGETHER, LOCALLY</span><h1>局域网同步</h1><p>网线电脑与同一路由器的 Wi-Fi 手机可以互通。数据保留在两端，通过加密连接同步。</p></div>
 {runtimeMode==='mock'&&<p className="notice">本页为 UI 模拟夹具，未验证真实设备发现、配对或同步。</p>}
 {notice&&<p className="notice" role="status">{notice}</p>}
 {error&&<div className="notice inline-error" role="alert">{syncErrors[error]??error}</div>}
 <section className="settings-card"><h2>电脑出码，手机一扫连接</h2><p>电脑点击出码即同意让一部手机同步；手机点击扫码即同意同步两端人格、聊天和记忆，不共享 API Key 或私钥。仅向你信任的手机展示二维码，两分钟内一次有效。</p>
 <div className="button-row"><button className="primary" disabled={!nativeAvailable||busy||runtimeMode==='mock'} onClick={()=>{setLegacy(false);void command('invite_quick');}}>电脑：显示扫码连接码</button><button className="primary" disabled={!nativeAvailable||busy} onClick={()=>{setError('');setScanning(true);}}>手机：扫码连接电脑</button></div>
 <p className="field-help">一键扫码需要新版手机端。已发布的旧 0.0.2 手机测试包仍用下方兼容配对；其同签名更新尚待可用。</p>
 {busy&&<p role="status">正在验证并连接设备，请稍候…</p>}
 {code&&<div className="pair-code"><img alt="一次性配对二维码" src={`data:image/svg+xml,${encodeURIComponent(code.svg)}`}/><p role="status">{seconds} 秒后过期 · 用完自动关闭</p><details><summary>查看一次性配对码文本</summary><textarea aria-label="本机一次性配对码" readOnly value={code.text}/></details><button onClick={()=>void command('cancel')}>取消并销毁配对码</button></div>}
 <label htmlFor="scan-pair">拍摄 / 选择配对二维码</label><input id="scan-pair" type="file" accept="image/*" disabled={!nativeAvailable||busy} onChange={e=>{const file=e.target.files?.[0];if(file)void scan(file);e.target.value='';}}/>
 <button disabled={!nativeAvailable||busy||runtimeMode==='mock'} onClick={()=>{setLegacy(true);void command('invite');}}>旧版手机：显示兼容配对码</button></section>
 {scanning&&<PairScanner onResult={scanned} onClose={()=>setScanning(false)}/>}
 <section className="settings-card"><div className="card-heading"><div><h2>本机连接</h2><p>{status?.enabled?'正在发现设备 · 应用运行时自动重试同步':'同步未启用，仍可正常离线聊天'}</p></div><button className="primary" disabled={!nativeAvailable||busy} onClick={()=>void command(status?.enabled?'disable':'enable')}>{status?.enabled?'关闭同步':'启用局域网同步'}</button></div>
 {status?.enabled&&<details><summary>连接详情与手动排查</summary><dl><dt>设备名称</dt><dd>{status.name}</dd><dt>设备 UUID</dt><dd className="sync-fingerprint">{status.device_id}</dd><dt>本机证书 SHA-256</dt><dd className="sync-fingerprint">{status.fingerprint}</dd><dt>本机局域网地址</dt><dd>{status.local_addresses?.join(' / ')||`端口 ${status.port}`}</dd></dl><p className="field-help">兼容配对时对端填写本机地址。首次配对保留两端所有人格及 UUID；同名人格不会自动覆盖。</p></details>}
 {status?.discovery_error&&<p className="inline-error">自动发现不可用：{status.discovery_error}。允许系统本地网络权限后重新启用同步，或手动输入已知地址。</p>}</section>
 {status?.enabled&&<><details open={legacy} onToggle={e=>setLegacy(e.currentTarget.open)} className="settings-card"><summary>兼容旧版 / 手动配对</summary><p>旧版手机端仍需选择地址并核对身份；新版扫码不需要逐项填写。</p>
 <label htmlFor="remote-pair-code">对端一次性配对码</label><textarea id="remote-pair-code" value={text} onChange={e=>setText(e.target.value)} autoComplete="off" spellCheck={false} maxLength={8192}/>
 <label htmlFor="lan-address">对端局域网 IP:端口</label><input id="lan-address" value={address} onChange={e=>setAddress(e.target.value)} placeholder="192.168.1.20:54321" autoComplete="off"/>
 {status.candidates.length>0&&<div className="discovered-list"><p>已发现地址（配对前身份尚未验证）</p>{status.candidates.map(c=><button key={c.instance+c.address} onClick={()=>setAddress(c.address)}>{c.address}</button>)}</div>}
 <button className="primary" disabled={busy||!text||!localAddress(address)} onClick={()=>void command('pair',{text,address})}>连接并核对设备身份</button>
 {status.pending.map(p=><article className="sync-device" key={p.id}><h3>{p.name} · 等待双方确认</h3><p>证书 SHA-256</p><code className="sync-fingerprint">{p.fingerprint}</code><label className="check-row"><input type="checkbox" checked={confirmId===p.id} onChange={e=>setConfirmId(e.target.checked?p.id:'')}/>已核对对端指纹，并同意合并两端非敏感数据</label><button disabled={busy||p.approved||confirmId!==p.id} onClick={()=>void command('confirm',{id:p.id})}>{p.approved?'已确认，等待对端':'确认配对'}</button></article>)}</details>
 <section className="settings-card"><div className="card-heading"><h2>已配对设备</h2><button disabled={busy} onClick={()=>void command('sync')}>手动同步</button></div><p className="field-help">方向：双向增量。待发数量包括消息、人格、记忆等全部待发事件，不将连接成功算作同步成功。</p>
 {!status.peers.length&&<p>尚无可信设备。发现地址不会自动授予访问权限。</p>}
 {status.peers.map(p=><article className="sync-device" key={p.id}><h3>{p.name}<span className="badge">{syncLabel(p)}</span></h3><dl><dt>最近完成同步</dt><dd>{p.last_success_ms?new Date(p.last_success_ms).toLocaleString():'尚未成功同步'}</dd><dt>待发事件</dt><dd>{p.pending}</dd><dt>本轮已传输事件</dt><dd>{p.progress.transferred}</dd><dt>当前地址</dt><dd>{p.address??'等待自动发现或手动填写'}</dd></dl><details><summary>设备 UUID 与证书指纹</summary><p className="sync-fingerprint">{p.id}<br/>{p.fingerprint}</p></details>
 {p.progress.error&&<p className="inline-error">{syncErrors[p.progress.error]??p.progress.error}</p>}
 {!p.revoked&&<div className="button-row"><button disabled={busy||!localAddress(address)} onClick={()=>void command('reconnect',{id:p.id,address})}>使用上方地址重新连接</button><button disabled={busy} onClick={()=>void command('sync')}>立即重试</button><button onClick={()=>setConfirmId(`revoke:${p.id}`)}>解除配对</button>{confirmId===`revoke:${p.id}`&&<button className="danger" onClick={()=>void command('revoke',{id:p.id})}>确认撤销此设备的同步权限</button>}</div>}</article>)}</section>
 {status.conflicts.length>0&&<section className="settings-card"><h2>保留的并发修订</h2><p>使用逻辑版本稳定选主；以下候选保留供核对。可在对应人格或记忆页面重新编辑，形成新修订。</p>{status.conflicts.map((c,i)=><details key={i}><summary>{c.entity} · {c.field}</summary><p className="sync-fingerprint">{c.value}</p></details>)}</section>}</>}
 <section className="settings-card"><h2>连接排查与离线使用</h2><p>电脑可使用网线，手机使用 Wi-Fi。访客网络、无线客户端隔离或不同 VLAN 可能阻止发现或连接。自动发现失败时使用已知 IP 和端口；不扫描整个网段。</p><p>Windows 仅为本应用允许专用网络访问，保留防火墙与杀毒软件。不要开启路由器公网端口映射。Android 拒绝本地网络权限时仍可离线聊天。</p><p>电脑关机、手机离线或应用被系统暂停时显示等待重连。重新打开应用并恢复网络后自动补传；强制停止应用后不保证后台实时同步。</p></section></div>;
}
