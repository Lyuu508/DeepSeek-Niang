import {useEffect,useRef,useState} from 'react';
import jsQR from 'jsqr';

export function PairScanner({onResult,onClose}:{onResult:(text:string)=>void;onClose:()=>void}){
 const video=useRef<HTMLVideoElement>(null);const callback=useRef(onResult);callback.current=onResult;
 const [error,setError]=useState('');
 useEffect(()=>{
  let active=true;let stream:MediaStream|null=null;let frame=0;let previous=0;
  const stop=()=>{cancelAnimationFrame(frame);stream?.getTracks().forEach(track=>track.stop());if(video.current)video.current.srcObject=null;};
  const canvas=document.createElement('canvas');const context=canvas.getContext('2d',{willReadFrequently:true});
  const read=(now:number)=>{
   if(!active)return;
   const element=video.current;
   if(element&&context&&element.readyState>=2&&element.videoWidth&&now-previous>=150){
    previous=now;const scale=Math.min(1,960/Math.max(element.videoWidth,element.videoHeight));
    canvas.width=Math.round(element.videoWidth*scale);canvas.height=Math.round(element.videoHeight*scale);
    context.drawImage(element,0,0,canvas.width,canvas.height);
    const pixels=context.getImageData(0,0,canvas.width,canvas.height);
    const code=jsQR(pixels.data,pixels.width,pixels.height);
    if(code){active=false;stop();callback.current(code.data);return;}
   }
   frame=requestAnimationFrame(read);
  };
  void (async()=>{try{
   if(!navigator.mediaDevices?.getUserMedia||!context)throw new Error('CAMERA_UNAVAILABLE');
   const acquired=await navigator.mediaDevices.getUserMedia({audio:false,video:{facingMode:{ideal:'environment'},width:{ideal:1280}}});
   if(!active){acquired.getTracks().forEach(track=>track.stop());return;}
   stream=acquired;if(!video.current){stop();return;}
   video.current.srcObject=stream;await video.current.play();
   if(active)frame=requestAnimationFrame(read);
  }catch(reason){if(active){stop();const denied=reason instanceof DOMException&&['NotAllowedError','PermissionDeniedError'].includes(reason.name);setError(denied?'相机未授权。请在系统设置允许相机，或关闭扫码后从照片识别。':'相机暂时不可用。请关闭扫码后从照片识别配对码。');}}})();
  return()=>{active=false;stop();};
 },[]);
 return <section className="settings-card camera-scanner" aria-label="实时扫码">
  <h2>对准电脑上的二维码</h2><p>识别后自动连接。画面只在本机识码，不录音、不上传。</p>
  {error&&<p role="alert" className="inline-error">{error}</p>}
  <video ref={video} autoPlay muted playsInline aria-label="配对扫码预览"/>
  <button onClick={onClose}>关闭扫码</button>
 </section>;
}
