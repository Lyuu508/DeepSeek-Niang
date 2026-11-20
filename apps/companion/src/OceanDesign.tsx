import type { ReactNode } from 'react';
import type { CompanionSnapshot } from '@deepseek/contracts';

const iconPaths = {
  chat: 'M20 11.5a8 8 0 0 1-8 8H4l1.5-4A8 8 0 1 1 20 11.5ZM8 11.5h.01m4 0h.01m4 0h.01',
  persona: 'M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM4 21v-2a8 5 0 0 1 16 0v2',
  sync: 'M4 10a8 8 0 0 1 13-5l3 3m0-5v5h-5m5 6a8 8 0 0 1-13 5l-3-3m0 5v-5h5',
  video: 'M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6Zm7 3 5 3-5 3V9Z',
  plugin: 'M3 9h5V5a2 2 0 0 1 4 0v4h5v4a2 2 0 0 1 0 4v4H3v-4a2 2 0 0 0 0-4V9Z',
  memory: 'M5 6a7 2.5 0 1 0 14 0A7 2.5 0 1 0 5 6Zm0 0v6c0 1.4 3 2.5 7 2.5s7-1.1 7-2.5V6M5 12v6c0 1.4 3 2.5 7 2.5s7-1.1 7-2.5v-6',
  settings: 'M10 2h4l.7 3 2.3 1.3 3-.8 2 3.5-2.2 2.2v2.6L22 16l-2 3.5-3-.8-2.3 1.3-.7 3h-4l-.7-3L7 18.7l-3 .8L2 16l2.2-2.2v-2.6L2 9l2-3.5 3 .8L9.3 5l.7-3ZM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  bond: 'M12 21 4.5 13.5C-.2 8.8 6.8 3 12 8c5.2-5 12.2.8 7.5 5.5L12 21Z',
  about: 'M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0ZM12 11v6m0-10h.01',
  more: 'M5 12h.01m7 0h.01m7 0h.01',
  moon: 'M20.5 15A8.5 8.5 0 0 1 9 3.5 8.7 8.7 0 1 0 20.5 15Z',
  sun: 'M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.4 1.4m11.2 11.2L19 19M19 5l-1.4 1.4M6.4 17.6 5 19',
  send: 'm21 3-7.3 18-3.9-7.8L2 9.3 21 3Zm0 0L9.8 13.2',
  stop: 'M6 6h12v12H6z',
  add: 'M12 5v14M5 12h14',
  close: 'm6 6 12 12M18 6 6 18',
  pet: 'M3 6 8 8a10 10 0 0 1 8 0l5-2v9a7 6 0 0 1-7 6h-4a7 6 0 0 1-7-6V6Zm5 8h.01m8 0h.01m-6 2 2 2 2-2',
  trash: 'M4 7h16M9 7V4h6v3m3 0-1 14H7L6 7m4 4v6m4-6v6',
  key: 'M15 8a5 5 0 1 1-10 0 5 5 0 0 1 10 0ZM13 12l7 7m-3-3 2-2m0 4 2-2',
  voice: 'M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0V5Zm-3 6v1a6 6 0 0 0 12 0v-1m-6 7v4m-4 0h8',
  arrow: 'M5 12h14m-5-5 5 5-5 5',
  shield: 'M12 3 4 6v6c0 4 3 7 8 10 5-3 8-6 8-10V6l-8-3Zm-3 9 2 2 4-4',
} as const;

export type IconName = keyof typeof iconPaths;
export function Icon({ name }: { name: IconName }) {
  return <svg className="ocean-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={iconPaths[name]}/></svg>;
}

export function WhaleMark() {
  return <svg className="whale-mark" viewBox="0 0 40 32" aria-hidden="true">
    <path fill="currentColor" d="M3 16c0-5 5-9 11-9 7 0 11 4 13 8 2-1 4-4 5-7-2-1-3-3-3-6 3 1 5 2 6 4 1-2 3-3 5-3 0 4-2 6-5 7-1 8-5 14-12 17C14 30 3 24 3 16Z"/>
    <path fill="var(--surface)" d="M5 18c5 3 12 4 19 3-3 4-7 5-11 4-4-1-7-4-8-7Z"/>
    <circle fill="var(--surface)" cx="11" cy="14.5" r="1.35"/>
  </svg>;
}

export const expressions = {
  'avatar-standard': '标准头像', 'avatar-chibi': 'Q版头像',
  happy: '开心', shy: '害羞', sad: '委屈', angry: '生气',
  thinking: '思考', sleepy: '困倦', sleeping: '睡觉',
  peace: '比耶', wave: '挥手', resting: '趴桌', peeking: '探头', waiting: '等待',
} as const;
export type CharacterArt = keyof typeof expressions | 'portrait-half' | 'portrait-full';
export type SceneArt = 'scene-chat' | 'scene-settings' | 'scene-plugins' | 'scene-video';
export type Artwork = CharacterArt | SceneArt;
export const artUrl = (name: Artwork) => '/branding/ocean/' + name + '.webp';

export function Art({ name, className = '', eager = false, label = '' }: {
  name: Artwork; className?: string; eager?: boolean; label?: string;
}) {
  return <img src={artUrl(name)} alt={label} className={'ocean-art ' + className}
    loading={eager ? 'eager' : 'lazy'} decoding="async" draggable={false}/>;
}

export function PageHeader({ title, description, icon, art, scene, children }: {
  title: string; description?: string; icon: IconName; art?: CharacterArt; scene?: SceneArt; children?: ReactNode;
}) {
  return <div className="page-heading ocean-page-heading" data-scene={scene}>
    <div className="ocean-page-intro">
      <h1><Icon name={icon}/>{title}</h1>
      {description && <p>{description}</p>}
      {children}
    </div>
    {art && <Art name={art} className="page-header-art"/>}
  </div>;
}

export function dismissDialog(dialog: HTMLDialogElement | null, close: () => void) {
  if (!dialog || matchMedia('(prefers-reduced-motion: reduce)').matches || typeof dialog.animate !== 'function') {
    close();
    return;
  }
  if (dialog.dataset.closing) return;
  dialog.dataset.closing = 'true';
  const transition = dialog.animate([{ opacity: 1, transform: 'none' }, { opacity: 0, transform: 'translateY(.28rem)' }],
    { duration: 135, easing: 'ease-in', fill: 'forwards' });
  void transition.finished.then(() => { if (dialog.isConnected) close(); }).catch(() => {});
}

export function moodArt(snapshot: CompanionSnapshot | null): CharacterArt {
  const life = snapshot?.life;
  if (life?.host_online && life.current?.state === 'running' && ['rest', 'sleep'].includes(life.current.kind)) return 'sleeping';
  switch (snapshot?.mood.kind) {
    case 'happy': case 'excited': return 'happy';
    case 'shy': return 'shy';
    case 'sad': case 'lonely': return 'sad';
    case 'annoyed': case 'angry': return 'angry';
    case 'curious': return 'thinking';
    case 'tired': return 'sleepy';
    default: return 'avatar-chibi';
  }
}
