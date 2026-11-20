import { Icon, type IconName } from './OceanDesign';

type Destination = 'chat' | 'personas' | 'settings' | 'more';
const items: { page: Destination; label: string; icon: IconName }[] = [
  { page: 'chat', label: '聊天', icon: 'chat' },
  { page: 'personas', label: '人格', icon: 'persona' },
  { page: 'settings', label: '设置', icon: 'settings' },
  { page: 'more', label: '更多', icon: 'more' },
];
export function MobileNav({ page, onSelect }: { page: string; onSelect: (page: Destination) => void }) {
  const active = items.some(item => item.page === page) ? page : 'more';
  return <nav className="mobile-nav" aria-label="手机主导航">{items.map(item =>
    <button key={item.page} aria-label={'手机'+item.label} aria-current={active===item.page?'page':undefined}
      onClick={()=>onSelect(item.page)}><Icon name={item.icon}/><span>{item.label}</span></button>)}
  </nav>;
}
