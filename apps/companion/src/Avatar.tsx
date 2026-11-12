import type {Persona} from '@deepseek/contracts';
import {brandingReady,builtinAvatarURL} from './branding';
export function Avatar({persona,small=false}:{persona?:Persona;small?:boolean}){
 const builtin=persona?.is_builtin??true;
 return <span className={`avatar ${small?'small':''} ${builtin&&brandingReady?'has-image':builtin?'pending-art':'custom-avatar'}`} aria-label={builtin?(brandingReady?'DeepSeek娘头像':'原始PNG待导入'):persona?.name}>
 {builtin?(brandingReady?<img src={builtinAvatarURL} alt="DeepSeek娘"/>:<span>{small?'…':'原图待导入'}</span>):Array.from(persona?.name??'人格').slice(0,2).join('')}
 </span>;
}
