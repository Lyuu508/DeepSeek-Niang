import {useEffect,useId,useRef,type ReactNode} from 'react';

export function Modal({title,closeLabel,onClose,children,className=''}:{title:string;closeLabel:string;onClose:()=>void;children:ReactNode;className?:string}){
 const dialog=useRef<HTMLDialogElement>(null);const heading=useId();
 useEffect(()=>{const element=dialog.current;if(element&&!element.open)element.showModal();return()=>{element?.close();};},[]);
 return <dialog ref={dialog} className={`pair-modal ${className}`} aria-labelledby={heading} onCancel={event=>{event.preventDefault();onClose();}} onClick={event=>{if(event.target===dialog.current){const rect=dialog.current!.getBoundingClientRect();if(event.clientX<rect.left||event.clientX>rect.right||event.clientY<rect.top||event.clientY>rect.bottom)onClose();}}}>
  <div className="modal-heading"><h2 id={heading}>{title}</h2><button className="modal-close" aria-label={closeLabel} onClick={onClose}>×</button></div>{children}
 </dialog>;
}
