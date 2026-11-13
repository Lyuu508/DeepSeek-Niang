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
 await check('ordinary browser cannot mutate native data or configure a key',async()=>{await page.goto(base,{waitUntil:'networkidle'});assert(await page.getByLabel('发送给当前人格的消息').isDisabled());await settings(page);assert(await page.getByLabel('API Key',{exact:true}).isDisabled());});
 await page.goto(base+'/?test=mock',{waitUntil:'networkidle'});
 await check('one default persona, permanent space, and no traditional conversation entry',async()=>{await page.locator('.persona-list button').waitFor();assert.equal(await page.locator('.persona-list button').count(),1);assert.match(await page.locator('.mode-banner').innerText(),/模拟验收模式/);for(const text of ['开启新对话','最近的对话','默认对话'])assert.equal(await page.getByText(text,{exact:true}).count(),0);assert.match(await page.locator('.brand').innerText(),/DeepSeek娘/);assert(!(await page.locator('.avatar').allInnerTexts()).some(x=>x==='DS'));});
 const defaultId=await page.locator('.persona-list button').getAttribute('data-persona-id');
 await page.screenshot({path:path.join(artifact,'persona-welcome-mock.png'),fullPage:true});
 await check('mock key form clears and does not use browser credential caches',async()=>{await configure(page);assert(!JSON.stringify(await page.evaluate(()=>({...localStorage}))).includes('fixture-ui-key-only'));assert(!(await context.cookies()).some(c=>c.value.includes('fixture-ui-key-only')));});
 await check('persona-specific preferences can be edited',async()=>{await page.getByLabel('如何称呼你').fill('旅人A');await page.getByRole('button',{name:'保存聊天设置',exact:true}).click();await page.getByText('聊天与人设设置已保存，新消息将使用这些设置。',{exact:true}).waitFor();await chat(page);assert.match(await page.locator('.companion-card').innerText(),/旅人A/);});
 await check('Chinese streaming stop is durable within the UI fixture',async()=>{const input=page.getByLabel('发送给当前人格的消息');await input.fill('中文停止测试');await input.press('Enter');await page.locator('.message.assistant').filter({hasText:'模拟'}).first().waitFor();await page.getByRole('button',{name:/停止.*生成/}).click();await page.locator('.message.assistant[data-state="interrupted"]').waitFor();assert.equal(await page.locator('.message.user').count(),1);});
 await check('regeneration keeps one user turn and preserves reply variants',async()=>{await page.getByRole('button',{name:/重新生成/}).click();await page.locator('.message.assistant[data-state="complete"]').waitFor();assert.equal(await page.locator('.message.user').count(),1);await page.getByRole('button',{name:/查看历史回复/}).click();assert.equal(await page.locator('.message.assistant').count(),2);await page.getByRole('button',{name:'收起历史回复',exact:true}).click();});
 await check('network failure is labeled simulated and retains input',async()=>{await page.getByLabel('发送给当前人格的消息').fill('触发错误');await page.getByRole('button',{name:'发送消息',exact:true}).click();await page.locator('.message.assistant[data-state="failed"]').waitFor();assert.match(await page.getByRole('alert').innerText(),/模拟测试：网络连接失败/);assert.equal(await page.locator('.message.user').count(),2);await page.getByRole('button',{name:'关闭错误提示',exact:true}).click();});
 await check('memory A is saved under A',async()=>{await page.getByRole('button',{name:'本地数据',exact:true}).click();await page.getByLabel('记忆内容').fill('人格A的独立记忆');await page.getByRole('button',{name:'保存记忆',exact:true}).click();await page.locator('.memory').filter({hasText:'人格A的独立记忆'}).waitFor();});
 await check('creating persona B automatically creates an empty independent permanent space',async()=>{await page.getByRole('button',{name:/新增人格/}).click();await page.getByLabel('人格名称',{exact:true}).fill('测试角色B');await page.getByLabel('独立人物设定',{exact:true}).fill('理性，安静，喜欢天文。');await page.getByRole('button',{name:'创建人格并进入聊天',exact:true}).click();await page.waitForFunction(()=>document.querySelector('header strong')?.textContent==='测试角色B');assert.equal(await page.locator('.persona-list button').count(),2);assert.equal(await page.locator('.message.user').count(),0);assert.match(await page.locator('.companion-card').innerText(),/主人/);assert.equal((await page.locator('.usage-count').innerText()).replace(/\s/g,''),'0次生成');});
 await check('B cannot see A memory; B memory survives switches independently',async()=>{await page.getByRole('button',{name:'本地数据',exact:true}).click();assert.equal(await page.locator('.memory').count(),0);await page.getByLabel('记忆内容').fill('人格B的独立记忆');await page.getByRole('button',{name:'保存记忆',exact:true}).click();await page.locator('.memory').filter({hasText:'人格B的独立记忆'}).waitFor();await switchTo(page,'DeepSeek娘');await page.getByRole('button',{name:'本地数据',exact:true}).click();await page.locator('.memory').filter({hasText:'人格A的独立记忆'}).waitFor();assert.equal(await page.getByText('人格B的独立记忆',{exact:true}).count(),0);await chat(page);assert.equal(await page.locator('.message.user').count(),2);assert.equal(await page.locator('.persona-list button[aria-current="true"]').getAttribute('data-persona-id'),defaultId);});
 await check('switching during generation retains reply and usage under its original persona',async()=>{await page.getByLabel('发送给当前人格的消息').fill('生成中切换人格');await page.getByRole('button',{name:'发送消息',exact:true}).click();await page.locator('.message.assistant[data-state="generating"]').waitFor();await switchTo(page,'测试角色B');await page.getByRole('button',{name:'发送消息',exact:true}).waitFor();assert.equal(await page.locator('.message.user').count(),0);assert.equal((await page.locator('.usage-count').innerText()).replace(/\s/g,''),'0次生成');await switchTo(page,'DeepSeek娘');assert.equal(await page.locator('.message.user').count(),3);assert.match((await page.locator('.message.assistant').last().innerText()),/DeepSeek娘/);});
 await check('desktop Shift Enter and IME Enter do not accidentally send',async()=>{const input=page.getByLabel('发送给当前人格的消息');await input.fill('第一行');await input.press('Shift+Enter');await input.type('第二行');assert.equal(await input.inputValue(),'第一行\n第二行');const count=await page.locator('.message.user').count();await input.dispatchEvent('keydown',{key:'Enter',code:'Enter',isComposing:true,bubbles:true});assert.equal(await page.locator('.message.user').count(),count);await input.fill('');});
 await check('theme changes preserve persona and messages',async()=>{await page.getByRole('button',{name:'切换深浅色',exact:true}).click();assert.equal(await page.locator('.app').getAttribute('data-theme'),'dark');assert.equal(await page.locator('.message.user').count(),3);});
 await page.screenshot({path:path.join(artifact,'persona-desktop-mock.png'),fullPage:true});
 await check('no desktop JavaScript errors or external requests',async()=>{assert.deepEqual(errors,[]);assert.deepEqual(outbound,[]);});
 const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});const phone=await mobile.newPage();const mobileErrors=[];phone.on('pageerror',e=>mobileErrors.push(e.message));await phone.goto(base+'/?test=mock',{waitUntil:'networkidle'});
 await check('mobile persona menu and Enter newline with tap send',async()=>{await configure(phone);await chat(phone);const input=phone.getByLabel('发送给当前人格的消息');await input.fill('手机输入');await input.press('Enter');assert.equal(await input.inputValue(),'手机输入\n');assert.equal(await phone.locator('.message.user').count(),0);await phone.getByRole('button',{name:'发送消息',exact:true}).click();await phone.locator('.message.assistant[data-state="complete"]').waitFor();assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.deepEqual(mobileErrors,[]);});
 await phone.screenshot({path:path.join(artifact,'persona-mobile-mock.png'),fullPage:true});
 await check('a delayed history snapshot cannot overwrite a completed stream after persona switching',async()=>{
  const delayed=await context.newPage();await delayed.goto(base+'/?test=mock&read-delay=1800',{waitUntil:'networkidle'});
  await delayed.waitForFunction(()=>document.querySelector('#chat-input')?.disabled===false);
  await configure(delayed);await chat(delayed);
  await delayed.getByRole('button',{name:/新增人格/}).click();await delayed.getByLabel('人格名称',{exact:true}).fill('延迟读取B');
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
  await page.getByRole('button',{name:'启用局域网同步',exact:true}).click();
  await page.getByLabel('对端一次性配对码',{exact:true}).fill('{}');
  await page.getByLabel('对端局域网 IP:端口',{exact:true}).fill('8.8.8.8:443');
  assert(await page.getByRole('button',{name:'连接并核对设备身份',exact:true}).isDisabled());
  await page.getByLabel('对端局域网 IP:端口',{exact:true}).fill('192.168.1.20:54321');
  await page.getByRole('button',{name:'连接并核对设备身份',exact:true}).click();
  await page.getByText('配对码已过期、错误或已取消，请重新生成。',{exact:true}).waitFor();
  assert.equal(await page.getByText('同步成功',{exact:true}).count(),0);
  await page.screenshot({path:path.join(artifact,'sync-desktop-mock.png'),fullPage:true});
  await page.getByRole('button',{name:'关闭同步',exact:true}).click();
  await switchTo(page,'DeepSeek娘');assert.equal(await page.locator('.message.user').count(),3);
 });
 await check('mobile sync management stays within the viewport and labels offline use',async()=>{
  await menu(phone);await phone.getByRole('button',{name:'局域网同步',exact:true}).click();
  await phone.getByRole('button',{name:'启用局域网同步',exact:true}).waitFor();
  assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.deepEqual(mobileErrors,[]);
  await phone.screenshot({path:path.join(artifact,'sync-mobile-mock.png'),fullPage:true});
 });
 fs.writeFileSync(path.join(artifact,'results.json'),JSON.stringify({scope:'Explicit UI Mock; no real API, SQLite, file migration or OS credentials',passed:checks.length,checks,browser:browser.version()},null,2)+'\n');console.log(JSON.stringify({scope:'explicit UI Mock',passed:checks.length,checks}));
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{await browser?.close();server?.kill('SIGTERM');});
