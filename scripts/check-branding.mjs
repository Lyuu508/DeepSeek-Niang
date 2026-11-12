// Packaging gate only. Never downloads an image or quietly reuses the old DS icons.
import {readFileSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,relative,isAbsolute,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
export function checkBranding(root=resolve(dirname(fileURLToPath(import.meta.url)),'..')) {
 const manifest=JSON.parse(readFileSync(resolve(root,'assets/branding/manifest.json'),'utf8'));
 if(manifest.status!=='ready'||!manifest.source_sha256)throw new Error('原始海滩 PNG 尚未导入。请提供同一张原始文件并运行 scripts/prepare_branding.py；禁止使用旧 DS 图标打包新版。');
 const digest=file=>createHash('sha256').update(readFileSync(file)).digest('hex');
 if(digest(resolve(root,'assets/branding/deepseek-niang-master.png'))!==manifest.source_sha256)throw new Error('原始 PNG 校验失败，停止打包。');
 const files=manifest.derived_sha256??{};
 const required=['assets/branding/deepseek-niang-icon.png','assets/branding/deepseek-niang-avatar.png','assets/branding/deepseek-niang-splash.png','apps/companion/src-tauri/icons/icon.ico','apps/companion/src-tauri/icons/32x32.png','apps/companion/src-tauri/icons/128x128.png','apps/companion/src-tauri/icons/128x128@2x.png','apps/companion/public/branding/deepseek-niang-avatar.png','apps/companion/public/branding/deepseek-niang-splash.png'];
 for(const density of ['mdpi','hdpi','xhdpi','xxhdpi','xxxhdpi'])for(const suffix of [`mipmap-${density}/ic_launcher.png`,`mipmap-${density}/ic_launcher_foreground.png`,`drawable-${density}/deepseek_splash_icon.png`,`drawable-${density}/ic_stat_deepseek.png`])required.push(`assets/branding/android/${suffix}`);
 for(const file of required)if(!files[file])throw new Error('缺少正式图标清单条目：'+file);
 for(const [name,hash] of Object.entries(files)){
  const file=resolve(root,name);const rel=relative(root,file);
  if(rel.startsWith('..')||isAbsolute(rel)||!existsSync(file)||digest(file)!==hash)throw new Error('品牌资源缺失或哈希不符：'+name);
 }
 const config=JSON.parse(readFileSync(resolve(root,'apps/companion/src-tauri/tauri.conf.json'),'utf8'));
 if(config.productName!=='DeepSeek娘'||config.app.windows[0].title!=='DeepSeek娘'||config.identifier!=='io.local.deepseekcompanion')throw new Error('应用名称或固定身份不符。');
 return {status:'ready',source_sha256:manifest.source_sha256,verified_files:Object.keys(files).length};
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{console.log(JSON.stringify(checkBranding()));}catch(error){console.error(error.message);process.exitCode=1;}
}
