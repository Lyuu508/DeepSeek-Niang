import { useEffect, useId, useRef, useState } from 'react';
import { Confirmation } from '../Confirmation';
import { failure, runtimeMode } from '../native';
import { pluginAPI, type Config, type Manifest, type Plugin, type Preview } from './api';
import exampleStyle from './examples/chat-style.json?raw';
import exampleTime from './examples/time-reminder.json?raw';

const permissions: Record<string, string> = {
  chat_style: '为新回复提供已选的固定风格',
  local_clock: '显示本机时间',
  local_reminder: '为当前人格设置本机提醒',
  memory_write: '保存你提交的内容到当前人格的记忆',
  display_card: '显示清单中的纯文本',
  media_note: '整理你填写的媒体备注',
};
const statuses = { enabled: '已启用', disabled: '已停用', permission_required: '待授权', unsupported: '平台不支持' };
const actions: Record<string, string> = {
  installed: '已安装', updated: '已更新', permissions_updated: '权限已保存', configuration_saved: '配置已保存',
  style_applied: '风格已应用到请求', reminder_scheduled: '提醒已保存', memory_created: '笔记已保存',
};

function ConfigFields({ manifest, config, onChange }: { manifest: Manifest; config: Config; onChange: (next: Config) => void }) {
  return <div className="plugin-fields">{manifest.configSchema.map(field => {
    const value = config[field.key] ?? field.default;
    const update = (next: Config[string]) => onChange({ ...config, [field.key]: next });
    return <label key={field.key}>{field.label}
      {field.kind === 'toggle' ? <input type="checkbox" checked={!!value} onChange={e => update(e.target.checked)}/>
       : field.kind === 'choice' ? <select value={String(value)} onChange={e => update(e.target.value)}>
          {field.choices.map(choice => <option key={choice}>{choice}</option>)}</select>
       : field.kind === 'number' ? <input type="number" min={0} max={1440} value={Number(value)} onChange={e => update(e.target.value ? e.target.valueAsNumber : 0)}/>
       : <input type="text" maxLength={150} value={String(value)} onChange={e => update(e.target.value)}/>}
    </label>;
  })}</div>;
}

type Perform = <T = unknown>(action: Record<string, unknown>, message: string | ((result: T) => string)) => Promise<boolean>;

function TimeReminder({ plugin, run }: { plugin: Plugin; run: Perform }) {
  const [label, setLabel] = useState('');
  const [minutes, setMinutes] = useState(String(Math.max(1, Number(plugin.effective_config.minutes ?? 15))));
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  return <form className="plugin-run" onSubmit={e => {
    e.preventDefault();
    void run<{ due_at_ms: number }>({ kind: 'create_reminder', minutes: Number(minutes), label },
      value => `已保存提醒，预计 ${new Date(value.due_at_ms).toLocaleTimeString()} 到期。`).then(ok => { if (ok) setLabel(''); });
  }}>
    <time dateTime={new Date(now).toISOString()}>{new Date(now).toLocaleTimeString(undefined, { hour12: !!plugin.effective_config.hour12 })}</time>
    <label>提醒内容<input value={label} onChange={e => setLabel(e.target.value)} maxLength={110} required/></label>
    <label>几分钟后提醒<input type="number" min={1} max={1440} value={minutes} onChange={e => setMinutes(e.target.value)} required/></label>
    <button className="primary">添加提醒</button>
  </form>;
}

function NoteComposer({ media, run }: { media: boolean; run: Perform }) {
  const [text, setText] = useState('');
  return <form className="plugin-run" onSubmit={e => {
    e.preventDefault();
    void run({ kind: 'write_note', text }, '已保存到当前人格的记忆。').then(ok => { if (ok) setText(''); });
  }}>
    <label>{media ? '媒体名称、时间点和备注' : '想保存的笔记'}<textarea value={text} onChange={e => setText(e.target.value)} rows={3} maxLength={600}/></label>
    <button className="primary" disabled={!text.trim()}>保存笔记</button>
  </form>;
}

function PluginCard({ plugin, personaName, perform, busy }: { plugin: Plugin; personaName: string; perform: Perform; busy: boolean }) {
  const { manifest } = plugin;
  const [scope, setScope] = useState<'device' | 'persona'>('device');
  const [grants, setGrants] = useState(plugin.granted);
  const [global, setGlobal] = useState(plugin.global_enabled);
  const [personal, setPersonal] = useState(plugin.persona_enabled === null ? 'inherit' : String(plugin.persona_enabled));
  const [independent, setIndependent] = useState(!!plugin.persona_config);
  const [config, setConfig] = useState(plugin.global_config);
  const [confirm, setConfirm] = useState(false);
  const run: Perform = (action, message) => perform({ id: manifest.id, checksum: manifest.checksum, ...action }, message);
  return <article className="settings-card plugin-card">
    <div className="card-heading">
      <div><h2>{manifest.name}</h2><p>{manifest.description}</p></div>
      <span className="badge">{statuses[plugin.status]}</span>
    </div>
    <p className="field-help">{manifest.version} · {manifest.author} · {manifest.license} · {plugin.source}</p>
    {plugin.last_action && <p className="field-help">{actions[plugin.last_action]}</p>}
    <details className="plugin-details">
      <summary>权限与配置</summary>
      <fieldset disabled={busy}>
        <legend>权限</legend>
        {manifest.permissions.map(p => <label key={p} className="check-row">
          <input type="checkbox" checked={grants.includes(p)}
            onChange={e => setGrants(g => e.target.checked ? [...g,p] : g.filter(x => x!==p))}/>{permissions[p]}
        </label>)}
        <button type="button" onClick={() => void run({ kind: 'grants', permissions: grants }, '插件权限已保存。')}>保存权限</button>
      </fieldset>
      <fieldset disabled={busy}>
        <legend>使用范围</legend>
        <label>配置范围<select value={scope} onChange={e => {
          const selected = e.target.value as typeof scope;
          setScope(selected);
          setConfig(selected === 'device' ? plugin.global_config : plugin.effective_config);
        }}><option value="device">本设备全局</option><option value="persona">{personaName}</option></select></label>
        {scope === 'device' ? <label className="check-row"><input type="checkbox" checked={global} onChange={e => setGlobal(e.target.checked)}/>全局启用</label>
         : <><label>当前人格<select value={personal} onChange={e => setPersonal(e.target.value)}>
            <option value="inherit">沿用全局</option><option value="true">启用</option><option value="false">停用</option></select></label>
            <label className="check-row"><input type="checkbox" checked={independent} onChange={e=>setIndependent(e.target.checked)}/>使用该人格的独立配置</label></>}
        {(scope === 'device' || independent) && <ConfigFields manifest={manifest} config={config} onChange={setConfig}/>}
        <button type="button" onClick={() => void run({ kind: 'settings', scope, enabled: scope === 'device' ? global : personal === 'inherit' ? null : personal === 'true',
          config: scope === 'device' || independent ? config : null }, '插件配置已保存。')}>保存配置</button>
      </fieldset>
      <button type="button" disabled={busy} className="danger" onClick={() => setConfirm(true)}>卸载插件</button>
    </details>
    {plugin.status === 'enabled' && <fieldset disabled={busy} className="plugin-use">
      <legend>使用插件</legend>
      {manifest.entry.kind === 'chat_style' && <p>下一次聊天会应用这个风格。可在权限中撤销。</p>}
      {manifest.entry.kind === 'time_reminder' && <TimeReminder plugin={plugin} run={run}/>}
      {manifest.entry.kind === 'info_card' && <p>{manifest.entry.body}</p>}
      {(manifest.entry.kind === 'media_note' || manifest.entry.kind === 'memory_note') && <NoteComposer media={manifest.entry.kind === 'media_note'} run={run}/>}
    </fieldset>}
    {confirm && <Confirmation title="卸载这个插件？" body={<p>将移除 {manifest.name} 的本机配置和等待中的提醒。</p>}
      cancelLabel="取消" confirmLabel="卸载" danger busy={busy} onCancel={() => setConfirm(false)}
      onConfirm={() => void run({ kind: 'uninstall' }, '插件已卸载。').then(ok => { if (ok) setConfirm(false); })}/>}
  </article>;
}

export function PluginManager({ persona, name }: { persona: string; name: string }) {
  const fileId = useId();
  const lock = useRef(false);
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [candidate, setCandidate] = useState<{ raw: string; preview: Preview }>();
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const [problem, setProblem] = useState('');
  const enabled = runtimeMode === 'native' && !!persona;
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    void pluginAPI.list(persona).then(value => { if (live) setPlugins(value); })
      .catch(e => { if (live) setProblem(failure(e).message); });
    return () => { live = false; };
  }, [enabled, persona]);

  async function inspect(raw: string, source: string) {
    setProblem('');
    setNotice('');
    try { setCandidate({ raw, preview: await pluginAPI.inspect(raw, source) }); }
    catch (error) { setProblem(failure(error).message); }
  }

  const perform: Perform = async (action, message) => {
    if (lock.current || !enabled) return false;
    lock.current = true;
    setBusy(true);
    setNotice('');
    setProblem('');
    try {
      const result = await pluginAPI.change(persona, action);
      setPlugins(await pluginAPI.list(persona));
      setNotice(typeof message === 'function' ? message(result as never) : message);
      return true;
    } catch (error) {
      setProblem(failure(error).message);
      return false;
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  const preview = candidate?.preview;
  return <div className="settings-content plugin-manager">
    <div className="page-heading"><h1>插件管理</h1><p>当前人格 {name}</p></div>
    <section className="settings-card">
      <h2>导入本地插件</h2>
      <label htmlFor={fileId}>选择 JSON 清单</label>
      <input id={fileId} type="file" accept=".json,application/json" disabled={!enabled || busy}
        onChange={e => {
          const file = e.target.files?.[0];
          e.target.value = '';
          if (!file) return;
          if (!file.name.toLowerCase().endsWith('.json') || file.size > 16_384) {
            setProblem('请选择不超过 16 KB 的 JSON 清单。');
            return;
          }
          void file.text().then(value => inspect(value, file.name)).catch(() => setProblem('读取本地清单失败。'));
        }}/>
      <div className="button-row">
        <button disabled={!enabled || busy} onClick={() => void inspect(exampleStyle, '应用内示例 · 简洁聊天风格')}>示例 · 简洁聊天风格</button>
        <button disabled={!enabled || busy} onClick={() => void inspect(exampleTime, '应用内示例 · 本地时间与提醒')}>示例 · 本地时间与提醒</button>
      </div>
    </section>
    {notice && <p className="notice" role="status">{notice}</p>}
    {problem && !preview && <p className="inline-error" role="alert">{problem}</p>}
    {plugins.map(plugin => <PluginCard key={plugin.manifest.id + plugin.manifest.checksum} plugin={plugin} personaName={name} busy={busy} perform={perform}/>)}
    {candidate && preview && <Confirmation title={preview.manifest.name} cancelLabel="关闭预览"
      confirmLabel={preview.operation === 'update' ? '确认更新' : '确认安装'} busy={busy}
      canConfirm={preview.compatible && preview.operation !== 'installed'}
      onCancel={() => setCandidate(undefined)}
      onConfirm={() => {
        if (!preview.compatible || preview.operation === 'installed') return;
        void perform({ kind: 'install', raw: candidate.raw, source: preview.source, checksum: preview.manifest.checksum },
          '已保存插件。可在卡片中授权和启用。').then(ok => { if (ok) setCandidate(undefined); });
      }}
      body={<>
        <p>{preview.manifest.description}</p>
        <dl className="plugin-preview">
          <dt>来源</dt><dd>{preview.source}</dd><dt>作者</dt><dd>{preview.manifest.author}</dd>
          <dt>许可证</dt><dd>{preview.manifest.license}</dd><dt>版本</dt><dd>{preview.manifest.version}</dd>
          <dt>平台</dt><dd>{preview.manifest.supportedPlatforms.join(' / ')}</dd>
          <dt>SHA-256</dt><dd>{preview.manifest.checksum}</dd>
        </dl>
        <p>所需权限</p><ul>{preview.manifest.permissions.map(p => <li key={p}>{permissions[p]}</li>)}</ul>
        <p>作者和许可证来自清单。安装或更新后先保持停用，确认权限后可启用。</p>
        {!preview.compatible && <p className="inline-error">当前平台不适合安装此清单。</p>}
        {preview.operation === 'installed' && <p className="inline-error">已安装相同或更新版本。</p>}
        {problem && <p className="inline-error" role="alert">{problem}</p>}
      </>}/>}
  </div>;
}
