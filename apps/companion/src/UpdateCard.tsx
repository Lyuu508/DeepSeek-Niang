import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Confirmation } from './Confirmation';
import { Icon } from './OceanDesign';
import { runtimeMode } from './native';

type Check = {
  current_version: string;
  latest: { version: string; published_at: string; notes: string; newer: boolean; can_download: boolean } | null;
  ready: boolean;
  previous_install_failed: boolean;
};

const problems: { [key: string]: string } = {
  UPDATE_CHECK_REQUIRED: '请重新检查更新后再继续。',
  UPDATE_TIMEOUT: '请求超时，请稍后重试。',
  UPDATE_NETWORK_FAILED: '暂时无法访问 GitHub 正式发布，请检查网络。',
  UPDATE_METADATA_INVALID: '正式发布的版本信息无法核验。',
  UPDATE_ASSET_MISSING: '该发行版未提供此平台的完整安装包及校验文件。',
  UPDATE_CHECKSUM_INVALID: '安装包的 SHA-256 校验未通过。请重新检查并下载。',
  UPDATE_SIGNATURE_INVALID: 'Windows 无法验证安装包的数字签名，请联系发行方。',
  UPDATE_ANDROID_IDENTITY: 'APK 证书、安装身份或版本与当前应用不兼容，请保留现有安装和数据。',
  UPDATE_INSTALLER_FAILED: '未能启动系统安装程序。',
  UPDATE_STORAGE_FAILED: '无法写入本机下载目录，请检查空间和权限。',
  UPDATE_CHAT_BUSY: '请等待当前回复完成或先停止生成。',
  UPDATE_BUSY: '更新任务正在执行。',
};

export function UpdateCard({ version }: { version?: string }) {
  const [check, setCheck] = useState<Check>();
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const [hint, setHint] = useState('');
  const [confirm, setConfirm] = useState<'download' | 'install' | null>(null);
  const latest = check?.latest;
  async function perform(label: string, work: () => Promise<void>) {
    setBusy(label);
    setError('');
    setHint('');
    try { await work(); }
    catch (problem) { setError(problems[String(problem)] ?? '更新操作未完成。'); }
    finally { setBusy(''); }
  }

  return <section className="settings-card update-card" aria-labelledby="update-title">
    <div className="card-heading"><span className="card-icon"><Icon name="download"/></span><div><h2 id="update-title">应用更新</h2><p>从项目 GitHub 正式发布检查更新。</p></div></div>
    <dl className="update-versions">
      <div><dt>当前版本</dt><dd>{check?.current_version ?? version ?? '0.1.0'}</dd></div>
      <div><dt>最新正式版</dt><dd>{latest?.version ?? (check ? '暂无正式发布' : '尚未检查')}</dd></div>
      {latest && <div><dt>发布日期</dt><dd>{new Date(latest.published_at).toLocaleDateString('zh-CN')}</dd></div>}
    </dl>
    {latest?.notes && <details className="update-notes"><summary>更新日志</summary><p>{latest.notes}</p></details>}
    {latest && !latest.newer && <p role="status">当前已是最新正式版。</p>}
    {latest?.newer && !latest.can_download && <p role="status">这个版本尚无适用于当前平台的完整附件。</p>}
    {check?.previous_install_failed && <p className="field-help">上一次安装未完成。请核对当前版本后再试。</p>}
    {check?.ready && <p role="status">安装包已下载并通过校验。</p>}
    <div className="button-row">
      <button type="button" disabled={runtimeMode !== 'native' || !!busy} onClick={() => void perform('正在检查正式发布', async () => {
        setCheck(await invoke<Check>('update_check'));
      })}><Icon name="sync"/>检查更新</button>
      {latest?.can_download && (check?.ready
        ? <button className="primary" type="button" disabled={!!busy} onClick={() => setConfirm('install')}>安装更新</button>
        : <button className="primary" type="button" disabled={!!busy} onClick={() => setConfirm('download')}>下载更新</button>)}
    </div>
    {busy && <p role="status">{busy}…</p>}
    {hint && <p role="status">{hint}</p>}
    {error && <p className="inline-error" role="alert">{error}</p>}
    {runtimeMode !== 'native' && <p className="field-help">请在原生应用检查更新。</p>}
    {confirm && latest && <Confirmation title={confirm === 'download' ? `下载 DeepSeek娘 ${latest.version}？` : '安装已验证的更新？'}
      body={confirm === 'download' ? '下载后核对文件摘要和安装签名，再由你决定何时安装。' : 'Windows 会退出后由安装程序更新并重新启动。Android 将打开系统安装界面，由你确认。'}
      cancelLabel="取消" confirmLabel={confirm === 'download' ? '开始下载' : '继续安装'} busy={!!busy} onCancel={() => setConfirm(null)} onConfirm={() => {
        const action = confirm;
        setConfirm(null);
        void perform(action === 'download' ? '正在下载和校验' : '正在交给系统安装', async () => {
          if (action === 'download') {
            await invoke('update_download', { version: latest.version, confirmed: true });
            setCheck(current => current && { ...current, ready: true });
          } else {
            const status = await invoke<string>('update_install', { version: latest.version, confirmed: true });
            if (status === 'permission_required') setHint('请在 Android 系统设置允许此来源安装，再返回点击安装更新。');
            if (status === 'installer_open') setHint('系统安装界面已打开。请按系统提示确认。');
          }
        });
      }}/>}
  </section>;
}
