import {test} from 'node:test';
import assert from 'node:assert/strict';
import {localAddress,syncLabel} from '../apps/companion/src/sync-model.ts';
test('manual endpoint accepts private IPv4 only, never DNS/public/loopback',()=>{
 for(const s of ['192.168.0.1:54321','10.1.2.3:80','172.31.0.1:1421'])assert(localAddress(s));
 for(const s of ['127.0.0.1:80','8.8.8.8:443','192.168.0.1:0','192.168.0.999:80','172.32.0.1:80','example.com:80','[::1]:80'])assert(!localAddress(s));
});
test('stale success and pending work cannot be displayed as synchronized',()=>{
 const p={revoked:false,pending:0,last_success_ms:1000,progress:{state:'succeeded'}};
 assert.equal(syncLabel(p,1100),'同步成功');assert.equal(syncLabel(p,62000),'等待重新连接');
 assert.equal(syncLabel({...p,pending:1},1100),'有数据待同步');
 assert.equal(syncLabel({...p,revoked:true},1100),'已解除配对');
});

import fs from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);const jsQR=require('jsqr');
test('generated native pairing QR decodes to the exact one-time invitation',()=>{
 const {text,svg}=JSON.parse(fs.readFileSync(new URL('../.cache/phase2-qr.json',import.meta.url),'utf8'));
 const width=Number(/viewBox="0 0 (\d+)/.exec(svg)[1]);const scale=5;const size=width*scale;const rgba=new Uint8ClampedArray(size*size*4);rgba.fill(255);
 for(const m of svg.matchAll(/M(\d+) (\d+)h1v1h-1z/g))for(let y=Number(m[2])*scale;y<(Number(m[2])+1)*scale;y++)for(let x=Number(m[1])*scale;x<(Number(m[1])+1)*scale;x++){const i=(y*size+x)*4;rgba[i]=rgba[i+1]=rgba[i+2]=0;}
 assert.equal(jsQR(rgba,size,size)?.data,text);
 const invite=JSON.parse(text);assert.equal(invite.version,2);assert.equal(invite.token.length,64);assert(!('tls' in invite));assert(!('seed' in invite));
});
