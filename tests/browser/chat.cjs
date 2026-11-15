// Explicit UI Mock only. SQLite migration/isolation and restart have real Rust file tests.
const {chromium}=require('playwright');const assert=require('node:assert/strict');const fs=require('node:fs');const path=require('node:path');const {spawn}=require('node:child_process');
const root=path.resolve(__dirname,'../..');const artifact=process.env.COMPANION_TEST_ARTIFACT_DIR||path.join(root,'.cache/ui-tests');const base=process.env.COMPANION_TEST_BASE_URL||'http://127.0.0.1:1431';assert(['localhost','127.0.0.1'].includes(new URL(base).hostname));fs.mkdirSync(artifact,{recursive:true});
let server,browser;const checks=[];async function check(name,action){await action();checks.push(name);}
async function menu(page){if(await page.getByRole('button',{name:'打开侧栏',exact:true}).isVisible())await page.getByRole('button',{name:'打开侧栏',exact:true}).click();}
async function settings(page){await menu(page);await page.getByRole('button',{name:'设置',exact:true}).click();}
async function chat(page){await menu(page);await page.locator('.persona-list button[aria-current="true"]').click();}
async function switchTo(page,name){await menu(page);await page.locator('.persona-list button').filter({hasText:name}).click();await page.waitForFunction(expected=>document.querySelector('header strong')?.textContent===expected,name);}
async function configure(page){await settings(page);await page.getByLabel('API Key',{exact:true}).fill('fixture-ui-key-only');await page.getByRole('button',{name:'保存密钥',exact:true}).click();await page.getByText('模拟测试：已更新配置标记，未保存真实凭据。',{exact:true}).waitFor();await page.waitForFunction(()=>document.querySelector('#api-key').value==='');}
(async()=>{
 if(!process.env.COMPANION_TEST_BASE_URL){const vite=path.join(path.dirname(require.resolve('vite/package.json')),'bin/vite.js');server=spawn(process.execPath,[vite,'--host','127.0.0.1','--port',String(new URL(base).port),'--strictPort'],{cwd:path.join(root,'apps/companion'),stdio:'ignore'});}
 for(let i=0;i<80;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 browser=await chromium.launch({headless:true,executablePath:process.env.COMPANION_CHROMIUM_EXECUTABLE||(fs.existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined),args:['--no-sandbox']});
 const context=await browser.newContext({viewport:{width:1440,height:1000}});const page=await context.newPage();const errors=[],outbound=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(!r.url().startsWith(base+'/'))outbound.push(r.url());});
 await check('ordinary browser cannot mutate native data or configure a key',async()=>{await page.goto(base,{waitUntil:'networkidle'});assert.equal(await page.locator('.test-build-banner').count(),process.env.VITE_COMPANION_TEST_PACKAGE==='1'?1:0);assert(await page.getByLabel('发送给当前人格的消息').isDisabled());await settings(page);assert(await page.getByLabel('API Key',{exact:true}).isDisabled());});
 await page.goto(base+'/?test=mock',{waitUntil:'networkidle'});
 await check('one default persona, permanent space, and no traditional conversation entry',async()=>{await page.locator('.persona-list button').waitFor();assert.equal(await page.locator('.persona-list button').count(),1);assert.match(await page.locator('.mode-banner').innerText(),/模拟验收模式/);for(const text of ['开启新对话','最近的对话','默认对话'])assert.equal(await page.getByText(text,{exact:true}).count(),0);assert.match(await page.locator('.brand').innerText(),/DeepSeek娘/);assert(!(await page.locator('.avatar').allInnerTexts()).some(x=>x==='DS'));});
 const defaultId=await page.locator('.persona-list button').getAttribute('data-persona-id');
 await check('desktop has a full width header floating persona list and independent status controls',async()=>{
  const header=await page.locator('header').boundingBox();assert(header.x===0&&header.width===1440);
  assert.equal(await page.getByRole('button',{name:/新增人格/}).count(),0);
  const list=await page.locator('.persona-list').boundingBox();const center=await page.locator('.chat-main').boundingBox();const rail=await page.getByRole('region',{name:'当前人格资料'}).boundingBox();
  assert(list.x+list.width<=center.x&&center.x+center.width<=rail.x&&rail.width>=280);
  assert.equal(await page.locator('.persona-list button').count(),1);
  await page.screenshot({path:path.join(artifact,'desktop-three-pane-ui-fixture.png'),animations:'disabled'});
 });
 await page.screenshot({path:path.join(artifact,'persona-welcome-mock.png'),fullPage:true,animations:'disabled'});
 await check('mock key form clears and does not use browser credential caches',async()=>{await configure(page);assert(!JSON.stringify(await page.evaluate(()=>({...localStorage}))).includes('fixture-ui-key-only'));assert(!(await context.cookies()).some(c=>c.value.includes('fixture-ui-key-only')));});
 await check('persona-specific preferences can be edited',async()=>{await page.getByLabel('如何称呼你').fill('旅人A');await page.getByRole('button',{name:'保存聊天设置',exact:true}).click();await page.getByText('聊天与人设设置已保存，新消息将使用这些设置。',{exact:true}).waitFor();await chat(page);assert.match(await page.locator('.companion-card').innerText(),/旅人A/);});
 await check('Chinese streaming stop is durable within the UI fixture',async()=>{const input=page.getByLabel('发送给当前人格的消息');await input.fill('中文停止测试');await input.press('Enter');await page.locator('.message.assistant').filter({hasText:'模拟'}).first().waitFor();await page.getByRole('button',{name:/停止.*生成/}).click();await page.locator('.message.assistant[data-state="interrupted"]').waitFor();assert.equal(await page.locator('.message.user').count(),1);});
 await check('regeneration keeps one user turn and preserves reply variants',async()=>{await page.getByRole('button',{name:/重新生成/}).click();await page.locator('.message.assistant[data-state="complete"]').waitFor();assert.equal(await page.locator('.message.user').count(),1);await page.getByRole('button',{name:/查看历史回复/}).click();assert.equal(await page.locator('.message.assistant').count(),2);await page.getByRole('button',{name:'收起历史回复',exact:true}).click();});
 await check('network failure is labeled simulated and retains input',async()=>{await page.getByLabel('发送给当前人格的消息').fill('触发错误');await page.getByRole('button',{name:'发送消息',exact:true}).click();await page.locator('.message.assistant[data-state="failed"]').waitFor();assert.match(await page.getByRole('alert').innerText(),/模拟测试：网络连接失败/);assert.equal(await page.locator('.message.user').count(),2);await page.getByRole('button',{name:'关闭错误提示',exact:true}).click();});
 await check('memory A is saved under A',async()=>{await page.getByRole('button',{name:'本地数据',exact:true}).click();await page.getByLabel('记忆内容').fill('人格A的独立记忆');await page.getByRole('button',{name:'保存记忆',exact:true}).click();await page.locator('.memory').filter({hasText:'人格A的独立记忆'}).waitFor();});
 await check('creating persona B automatically creates an empty independent permanent space',async()=>{await page.getByRole('button',{name:'人格管理',exact:true}).click();await page.getByLabel('人格名称',{exact:true}).fill('测试角色B');await page.getByLabel('独立人物设定',{exact:true}).fill('理性，安静，喜欢天文。');await page.getByRole('button',{name:'创建人格并进入聊天',exact:true}).click();await page.waitForFunction(()=>document.querySelector('header strong')?.textContent==='测试角色B');assert.equal(await page.locator('.persona-list button').count(),2);assert.equal(await page.locator('.message.user').count(),0);assert.match(await page.locator('.companion-card').innerText(),/主人/);assert.equal((await page.locator('.usage-count').innerText()).replace(/\s/g,''),'0次生成');});
 await check('B cannot see A memory; B memory survives switches independently',async()=>{await page.getByRole('button',{name:'本地数据',exact:true}).click();assert.equal(await page.locator('.memory').count(),0);await page.getByLabel('记忆内容').fill('人格B的独立记忆');await page.getByRole('button',{name:'保存记忆',exact:true}).click();await page.locator('.memory').filter({hasText:'人格B的独立记忆'}).waitFor();await switchTo(page,'DeepSeek娘');await page.getByRole('button',{name:'本地数据',exact:true}).click();await page.locator('.memory').filter({hasText:'人格A的独立记忆'}).waitFor();assert.equal(await page.getByText('人格B的独立记忆',{exact:true}).count(),0);await chat(page);assert.equal(await page.locator('.message.user').count(),2);assert.equal(await page.locator('.persona-list button[aria-current="true"]').getAttribute('data-persona-id'),defaultId);});
 await check('switching during generation retains reply and usage under its original persona',async()=>{await page.getByLabel('发送给当前人格的消息').fill('生成中切换人格');await page.getByRole('button',{name:'发送消息',exact:true}).click();await page.locator('.message.assistant[data-state="generating"]').waitFor();await switchTo(page,'测试角色B');await page.getByRole('button',{name:'发送消息',exact:true}).waitFor();assert.equal(await page.locator('.message.user').count(),0);assert.equal((await page.locator('.usage-count').innerText()).replace(/\s/g,''),'0次生成');await switchTo(page,'DeepSeek娘');assert.equal(await page.locator('.message.user').count(),3);assert.match((await page.locator('.message.assistant').last().innerText()),/DeepSeek娘/);});
 await check('desktop Shift Enter and IME Enter do not accidentally send',async()=>{const input=page.getByLabel('发送给当前人格的消息');await input.fill('第一行');await input.press('Shift+Enter');await input.type('第二行');assert.equal(await input.inputValue(),'第一行\n第二行');const count=await page.locator('.message.user').count();await input.dispatchEvent('keydown',{key:'Enter',code:'Enter',isComposing:true,bubbles:true});assert.equal(await page.locator('.message.user').count(),count);await input.fill('');});
 await check('theme changes preserve persona and messages',async()=>{await page.getByRole('button',{name:'切换深浅色',exact:true}).click();assert.equal(await page.locator('.app').getAttribute('data-theme'),'dark');assert.equal(await page.locator('.message.user').count(),3);});
 await page.screenshot({path:path.join(artifact,'persona-desktop-mock.png'),fullPage:true,animations:'disabled'});
 await check('no desktop JavaScript errors or external requests',async()=>{assert.deepEqual(errors,[]);assert.deepEqual(outbound,[]);});
 const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});const phone=await mobile.newPage();const mobileErrors=[];phone.on('pageerror',e=>mobileErrors.push(e.message));await phone.goto(base+'/?test=mock',{waitUntil:'networkidle'});
 await check('mobile persona menu and Enter newline with tap send',async()=>{await configure(phone);await chat(phone);const input=phone.getByLabel('发送给当前人格的消息');await input.fill('手机输入');await input.press('Enter');assert.equal(await input.inputValue(),'手机输入\n');assert.equal(await phone.locator('.message.user').count(),0);await phone.getByRole('button',{name:'发送消息',exact:true}).click();await phone.locator('.message.assistant[data-state="complete"]').waitFor();assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.deepEqual(mobileErrors,[]);});
 await phone.screenshot({path:path.join(artifact,'persona-mobile-mock.png'),fullPage:true,animations:'disabled'});
 await check('a delayed history snapshot cannot overwrite a completed stream after persona switching',async()=>{
  const delayed=await context.newPage();await delayed.goto(base+'/?test=mock&read-delay=1800',{waitUntil:'networkidle'});
  await delayed.waitForFunction(()=>document.querySelector('#chat-input')?.disabled===false);
  await configure(delayed);await chat(delayed);
  await delayed.getByRole('button',{name:'人格管理',exact:true}).click();await delayed.getByLabel('人格名称',{exact:true}).fill('延迟读取B');
  await delayed.getByRole('button',{name:'创建人格并进入聊天',exact:true}).click();await delayed.waitForFunction(()=>document.querySelector('header strong')?.textContent==='延迟读取B');
  await delayed.waitForFunction(()=>document.querySelector('#chat-input')?.disabled===false);
  await switchTo(delayed,'DeepSeek娘');await delayed.waitForFunction(()=>document.querySelector('#chat-input')?.disabled===false);
  await delayed.getByLabel('发送给当前人格的消息').fill('读取与生成并发');await delayed.getByRole('button',{name:'发送消息',exact:true}).click();
  await delayed.locator('.message.assistant[data-state="generating"]').waitFor();await switchTo(delayed,'延迟读取B');await switchTo(delayed,'DeepSeek娘');
  // The read captured a partial reply; the stream completes before that read returns.
  await delayed.waitForFunction(()=>document.querySelector('#chat-input')?.disabled===false);
  await delayed.locator('.message.assistant[data-state="complete"]').waitFor();assert.equal(await delayed.locator('.message.user').count(),1);
  assert.match(await delayed.locator('.message.assistant').innerText(),/不是真实 DeepSeek API。/);await delayed.close();
 });
 await check('sync UI explicitly distinguishes fixtures, private endpoints, and failed pairing',async()=>{
  await page.getByRole('button',{name:'局域网同步',exact:true}).click();
  await page.getByText('本页为 UI 模拟夹具，未验证真实设备发现、配对或同步。',{exact:true}).waitFor();
  const connect=page.getByRole('region',{name:'扫码连接',exact:true});
  assert.equal(await connect.getByRole('button').count(),1);await connect.getByRole('button',{name:'连接手机',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'用手机扫一扫',exact:true});await dialog.waitFor();
  assert((await dialog.getByAltText('一次性连接二维码').getAttribute('src')).startsWith('data:image/svg+xml,'));
  assert.equal(await page.locator('input[type=file],.sync-panel textarea,.sync-panel input').count(),0);
  for(const name of ['扫一扫','旧版手机：显示兼容配对码','兼容旧版 / 手动配对'])assert.equal(await page.getByRole('button',{name,exact:true}).count(),0);
  await page.screenshot({path:path.join(artifact,'sync-qr-dialog-mock.png'),fullPage:true,animations:'disabled'});
  await page.keyboard.press('Escape');await dialog.waitFor({state:'detached'});
  assert.equal(await page.getByText('同步成功',{exact:true}).count(),0);
  assert.equal(await page.locator('.sync-panel>section').last().getAttribute('aria-label'),'本机信息');
  await page.screenshot({path:path.join(artifact,'sync-desktop-mock.png'),fullPage:true,animations:'disabled'});
  await switchTo(page,'DeepSeek娘');assert.equal(await page.locator('.message.user').count(),3);
 });
 await check('mobile sync management stays within the viewport and labels offline use',async()=>{
  await phone.getByRole('button',{name:'手机连接',exact:true}).click();
  await phone.getByRole('button',{name:'扫一扫',exact:true}).waitFor();
  assert.equal(await phone.getByRole('region',{name:'扫码连接'}).getByRole('button').count(),1);
  assert.equal(await phone.getByRole('button',{name:'连接手机',exact:true}).count(),0);
  assert.equal(await phone.locator('input[type=file],.sync-panel textarea,.sync-panel input').count(),0);
  assert.equal(await phone.locator('.sync-panel>section').last().getAttribute('aria-label'),'本机信息');
  assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.deepEqual(mobileErrors,[]);
  await phone.screenshot({path:path.join(artifact,'sync-mobile-mock.png'),fullPage:true,animations:'disabled'});
 });
 await check('camera denial is explained without claiming a device connected',async()=>{
  await phone.evaluate(()=>{window.__savedCamera=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);navigator.mediaDevices.getUserMedia=async()=>{throw new DOMException('denied','NotAllowedError');};});
  await phone.getByRole('button',{name:'扫一扫',exact:true}).click();
  await phone.getByText('相机未授权。请在系统设置允许相机后重试。',{exact:true}).waitFor();
  assert.equal(await phone.getByText('同步成功',{exact:true}).count(),0);
  await phone.getByRole('button',{name:'关闭扫码',exact:true}).click();
 });
 await check('closing scanner before camera permission resolves stops all late tracks',async()=>{
  await phone.evaluate(()=>{window.__cameraResolvers=[];navigator.mediaDevices.getUserMedia=()=>new Promise(resolve=>window.__cameraResolvers.push(resolve));});
  await phone.getByRole('button',{name:'扫一扫',exact:true}).click();
  await phone.waitForFunction(()=>window.__cameraResolvers.length>0);
  await phone.getByRole('button',{name:'关闭扫码',exact:true}).click();
  await phone.evaluate(()=>{const canvas=document.createElement('canvas');canvas.width=640;canvas.height=480;window.__cameraStreams=window.__cameraResolvers.map(resolve=>{const stream=canvas.captureStream();resolve(stream);return stream;});});
  await phone.waitForFunction(()=>window.__cameraStreams.every(stream=>stream.getTracks().every(track=>track.readyState==='ended')));
  await phone.evaluate(()=>{navigator.mediaDevices.getUserMedia=window.__savedCamera;});
  assert.equal(await phone.getByLabel('实时扫码',{exact:true}).count(),0);assert.deepEqual(mobileErrors,[]);
 });
 await check('live scan decodes an actual Rust QR frame and rejects the fixture backend without fake TLS success',async()=>{
  const qr=JSON.parse(fs.readFileSync(path.join(root,'.cache/phase2-quick-qr.json'),'utf8'));
  await phone.evaluate(async svg=>{
   const image=new Image();image.src='data:image/svg+xml,'+encodeURIComponent(svg);await image.decode();
   const canvas=document.createElement('canvas');canvas.width=774;canvas.height=774;
   const draw=()=>canvas.getContext('2d').drawImage(image,0,0,774,774);draw();
   window.__qrFrameInterval=setInterval(draw,100);window.__qrStreams=[];
   navigator.mediaDevices.getUserMedia=async()=>{const stream=canvas.captureStream(10);window.__qrStreams.push(stream);return stream;};
  },qr.svg);
  await phone.getByRole('button',{name:'扫一扫',exact:true}).click();
  await phone.getByText('二维码已过期或不可用。请在电脑重新点击“连接手机”后再扫。',{exact:true}).waitFor();
  await phone.waitForFunction(()=>window.__qrStreams.length>0&&window.__qrStreams.every(s=>s.getTracks().every(t=>t.readyState==='ended')));
  assert.equal(await phone.getByRole('dialog').count(),0);assert.equal(await phone.getByText('同步成功',{exact:true}).count(),0);
  await phone.evaluate(()=>{clearInterval(window.__qrFrameInterval);navigator.mediaDevices.getUserMedia=window.__savedCamera;});
 });
 await check('phone bottom navigation preserves the chat draft and remains distinct from desktop',async()=>{
  assert.equal(await phone.locator('.app').getAttribute('data-platform'),'android');
  assert.equal(await page.locator('.app').getAttribute('data-platform'),'desktop');
  assert(await phone.getByRole('navigation',{name:'手机主导航'}).isVisible());assert(!(await page.getByRole('navigation',{name:'手机主导航'}).isVisible()));
  await phone.getByRole('button',{name:'手机聊天',exact:true}).click();await phone.getByLabel('发送给当前人格的消息').fill('保留这份手机草稿');
  await phone.getByRole('button',{name:'手机设置',exact:true}).click();await phone.getByLabel('API Key',{exact:true}).waitFor();
  await phone.getByRole('button',{name:'手机聊天',exact:true}).click();assert.equal(await phone.getByLabel('发送给当前人格的消息').inputValue(),'保留这份手机草稿');
 });
 await check('phone compact viewport and landscape keep composer and navigation inside the usable bounds',async()=>{
  for(const viewport of [{width:390,height:520},{width:844,height:390}]){
   await phone.setViewportSize(viewport);await phone.getByLabel('发送给当前人格的消息').focus();
   const rect=await phone.getByLabel('发送给当前人格的消息').boundingBox();assert(rect.y>=0&&rect.y+rect.height<=viewport.height);
   assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   const button=await phone.getByRole('button',{name:'手机连接',exact:true}).boundingBox();assert(button.height>=44&&button.y+button.height<=viewport.height);
   assert(!(await phone.getByRole('region',{name:'当前人格资料'}).isVisible()));
  }
  await phone.setViewportSize({width:390,height:844});await phone.getByLabel('发送给当前人格的消息').fill('');
  await phone.screenshot({path:path.join(artifact,'phone-chat-redesign-mock.png'),fullPage:true,animations:'disabled'});
  await phone.getByRole('button',{name:'手机连接',exact:true}).click();
  await phone.screenshot({path:path.join(artifact,'phone-scan-redesign-mock.png'),fullPage:true,animations:'disabled'});
  assert.deepEqual(mobileErrors,[]);
 });
 await check('companion controls show five dimensions and preserve explicit proactive opt in',async()=>{
  await page.getByRole('button',{name:'关系与日常',exact:true}).click();await page.getByRole('heading',{name:'我们的关系',exact:true}).waitFor();
  assert.equal(await page.getByRole('progressbar').count(),5);assert.equal(await page.getByLabel('开启主动消息',{exact:true}).isChecked(),false);
  await page.getByLabel('开启主动消息',{exact:true}).check();await page.getByLabel('主动消息最小间隔').fill('180');await page.getByLabel('主动消息每日上限').fill('1');
  await page.getByRole('button',{name:'保存陪伴设置',exact:true}).click();await page.getByText('陪伴设置已保存，配对设备同步后生效。',{exact:true}).waitFor();
  await chat(page);await page.getByRole('button',{name:'关系与日常',exact:true}).click();assert(await page.getByLabel('开启主动消息',{exact:true}).isChecked());
  assert.equal(await page.getByLabel('主动消息每日上限').inputValue(),'1');assert.match(await page.locator('.companion-content').innerText(),/虚拟看视频、听音乐不代表实际播放/);
  await page.locator('.page-scroll').evaluate(el=>{el.scrollTop=0;});await page.screenshot({path:path.join(artifact,'phase3-companion-desktop-ui-fixture.png'),animations:'disabled'});
 });
 await check('memory editor retains the record and exposes importance privacy and provenance',async()=>{
  await page.getByRole('button',{name:'本地数据',exact:true}).click();const row=page.locator('.memory').filter({hasText:'人格A的独立记忆'});await row.getByRole('button',{name:'编辑',exact:true}).click();
  await page.getByLabel('记忆内容').fill('人格A已修订的重要记忆');await page.getByLabel('标记重要记忆',{exact:true}).check();await page.getByLabel('允许与当前话题相关时用于 DeepSeek 聊天',{exact:true}).check();
  await page.getByRole('button',{name:'保存记忆修订',exact:true}).click();const edited=page.locator('.memory').filter({hasText:'人格A已修订的重要记忆'});await edited.waitFor();
  assert.equal(await page.locator('.memory').count(),1);await edited.getByText('来源与时间',{exact:true}).click();assert.match(await edited.innerText(),/主人手动保存/);assert.match(await edited.innerText(),/允许相关检索后用于聊天/);
  await switchTo(page,'测试角色B');await page.getByRole('button',{name:'本地数据',exact:true}).click();assert.equal(await page.getByText('人格A已修订的重要记忆',{exact:true}).count(),0);
 });
 await check('phone companion page stays inside the viewport and retains its chat identity',async()=>{
  await phone.getByRole('button',{name:'手机陪伴',exact:true}).click();await phone.getByRole('heading',{name:'我们的关系',exact:true}).waitFor();assert.equal(await phone.getByRole('progressbar').count(),5);
  assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await phone.screenshot({path:path.join(artifact,'phase3-companion-phone-ui-fixture.png'),animations:'disabled'});
  await phone.getByRole('button',{name:'手机聊天',exact:true}).click();assert.equal(await phone.locator('.message.user').count(),1);assert.deepEqual(mobileErrors,[]);
 });
 await check('desktop right settings stay beside the conversation and apply the six turn preset',async()=>{
  await chat(page);await page.getByRole('button',{name:'编辑当前人格与聊天设置',exact:true}).click();
  assert(await page.getByLabel('发送给当前人格的消息').isVisible());assert(await page.getByLabel('如何称呼你').isVisible());
  await page.getByRole('button',{name:'节省 Token：近期 6 轮 + 按需找回旧聊',exact:true}).click();assert.equal(await page.getByLabel('最多保留轮数',{exact:true}).inputValue(),'6');
  await page.getByRole('button',{name:'保存聊天设置',exact:true}).click();await page.getByText('聊天与人设设置已保存，新消息将使用这些设置。',{exact:true}).waitFor();
  await page.getByRole('button',{name:'状态',exact:true}).click();assert(await page.locator('.companion-card').isVisible());
  await page.screenshot({path:path.join(artifact,'desktop-status-controls-ui-fixture.png'),animations:'disabled'});
 });
 fs.writeFileSync(path.join(artifact,'results.json'),JSON.stringify({scope:'Explicit UI Mock; no real API, SQLite, file migration or OS credentials; camera lifecycle uses a synthetic browser stream',passed:checks.length,checks,browser:browser.version()},null,2)+'\n');console.log(JSON.stringify({scope:'explicit UI Mock',passed:checks.length,checks}));
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{await browser?.close();server?.kill('SIGTERM');});
