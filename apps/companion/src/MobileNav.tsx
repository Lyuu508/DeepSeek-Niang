type Destination='chat'|'sync'|'personas'|'settings'|'companion';
const items:{page:Destination;label:string;path:string}[]=[
 {page:'chat',label:'聊天',path:'M5 4h14v12H9l-4 4V4Z'},
 {page:'sync',label:'连接',path:'M7 7h10l-3-3m3 13H7l3 3M4 10v4m16-4v4'},
 {page:'companion',label:'陪伴',path:'M12 20s-8-5-8-11a4 4 0 0 1 8-2 4 4 0 0 1 8 2c0 6-8 11-8 11Z'},
 {page:'personas',label:'人格',path:'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-7 8v-2a7 5 0 0 1 14 0v2'},
 {page:'settings',label:'设置',path:'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM12 2v3m0 14v3M2 12h3m14 0h3M5 5l2 2m10 10 2 2M5 19l2-2M17 7l2-2'},
];
export function MobileNav({page,onSelect}:{page:string;onSelect:(page:Destination)=>void}){
 return <nav className="mobile-nav" aria-label="手机主导航">{items.map(item=><button key={item.page} aria-label={`手机${item.label}`} aria-current={page===item.page?'page':undefined} onClick={()=>onSelect(item.page)}><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={item.path}/></svg><span>{item.label}</span></button>)}</nav>;
}
