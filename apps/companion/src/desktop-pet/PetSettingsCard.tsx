import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { desktopPet, petError, type PetSettings, type Setting } from './api';

export function PetSettingsCard() {
  const [settings, setSettings] = useState<PetSettings | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const refresh = useCallback(async () => setSettings(await desktopPet.settings()), []);
  useEffect(() => {
    let closed = false;
    const reload = () => void refresh().catch(err => { if (!closed) setError(petError(err)); });
    reload();
    const listener = listen('desktop-pet:settings', reload);
    void listener.catch(() => {});
    return () => { closed = true; void listener.then(unlisten => unlisten()).catch(() => {}); };
  }, [refresh]);

  async function change(setting: Setting) {
    setBusy(true);
    setError('');
    try {
      setSettings(await desktopPet.configure(setting));
    } catch (err) {
      setError(petError(err));
    } finally {
      setBusy(false);
    }
  }

  return <section className="settings-card desktop-pet-settings" aria-label="Windows 桌宠设置">
    <div className="card-heading">
      <div><h2>Windows 桌宠</h2><p>按住角色拖动，单击打招呼，双击打开聊天，右键查看菜单。</p></div>
      {settings && <span className="badge">{settings.visible ? '已显示' : '已隐藏'}</span>}
    </div>
    {error && <p className="inline-error" role="alert">{error}</p>}
    {settings ? <>
      <label className="check-row"><input type="checkbox" checked={settings.visible} disabled={busy}
        onChange={e => void change({ kind: 'visible', value: e.target.checked })} />显示桌宠</label>
      <label className="check-row"><input type="checkbox" checked={settings.always_on_top} disabled={busy}
        onChange={e => void change({ kind: 'always_on_top', value: e.target.checked })} />窗口置顶</label>
      <label className="check-row"><input type="checkbox" checked={settings.roam} disabled={busy || settings.reduce_motion}
        onChange={e => void change({ kind: 'roam', value: e.target.checked })} />自动在桌面走动</label>
      <label className="check-row"><input type="checkbox" checked={settings.reduce_motion} disabled={busy}
        onChange={e => void change({ kind: 'reduce_motion', value: e.target.checked })} />减少动态效果（暂停自动走动）</label>
      <label className="check-row"><input type="checkbox" checked={settings.startup_enabled === true}
        disabled={busy || settings.startup_enabled === null}
        onChange={e => void change({ kind: 'startup', value: e.target.checked })} />随当前 Windows 账户登录启动</label>
      {settings.startup_enabled === null && <p className="field-help">无法读取本机启动项。你仍可以手动启动应用。</p>}
      <label>桌宠大小<select aria-label="桌宠大小" value={settings.scale_percent} disabled={busy}
        onChange={e => void change({ kind: 'scale_percent', value: Number(e.target.value) })}>
        {[...new Set([75, 85, 90, 95, 100, 105, 110, 115, 120, 125, 130, 135, 140, 145, 150, settings.scale_percent])]
          .sort((a, b) => a - b).map(value => <option key={value} value={value}>{value}%</option>)}
      </select></label>
      <div className="button-row">
        <button type="button" disabled={busy} onClick={() => void change({ kind: 'reset_position' })}>找回位置</button>
        <button type="button" disabled={busy || !settings.tray_available} onClick={() => void change({ kind: 'pet_only' })}>仅显示桌宠</button>
      </div>
      <details className="field-help"><summary>桌宠动画署名</summary>
        <p>待机、走路和挥手动画来自 PC2005-cloud/dsh-pet。原型「溟月」由上善无形创作，女仆鲸鱼娘由 ZipZipPipe 设计。动画已裁剪转码，适用 CC BY-NC-SA 4.0。</p>
      </details>
    </> : <p role="status">正在读取桌宠设置…</p>}
  </section>;
}
