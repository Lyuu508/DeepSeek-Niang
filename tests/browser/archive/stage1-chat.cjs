// UI-only Mock tests. Persistence/provider/OS credentials have separate Rust/native checks.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {spawn}=require('node:child_process');
const root=path.resolve(__dirname,'../..');
const artifact=process.env.COMPANION_TEST_ARTIFACT_DIR||path.join(root,'.cache/ui-tests');
const base=process.env.COMPANION_TEST_BASE_URL||'http://127.0.0.1:1431';
assert(['localhost','127.0.0.1'].includes(new URL(base).hostname),'UI tests must use a loopback fixture');
fs.mkdirSync(artifact,{recursive:true});
let server,browser;
const checks=[];
async function check(name,work){await work();checks.push(name);}
async function configure(page){
 await page.getByRole('button',{name:'偏好设置',exact:true}).click();
 await page.getByLabel('API Key',{exact:true}).fill('fixture-ui-test-only');
 await page.getByRole('button',{name:'保存密钥',exact:true}).click();
 await page.getByText('模拟测试：已更新配置标记，未保存真实凭据。',{exact:true}).waitFor();
 await page.waitForFunction(()=>document.querySelector('#api-key').value==='');
}
(async()=>{
 if(!process.env.COMPANION_TEST_BASE_URL){
  const vite=path.join(path.dirname(require.resolve('vite/package.json')),'bin/vite.js');
  server=spawn(process.execPath,[vite,'--host','127.0.0.1','--port',String(new URL(base).port),'--strictPort'],{cwd:path.join(root,'apps/companion'),stdio:'ignore'});
 }
 for(let n=0;n<80;n++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 const executable=process.env.COMPANION_CHROMIUM_EXECUTABLE||(fs.existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined);
 browser=await chromium.launch({headless:true,executablePath:executable,args:['--no-sandbox']});
 const desktop=await browser.newContext({viewport:{width:1440,height:1000}});
 const page=await desktop.newPage();const errors=[];const outbound=[];
 page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(!r.url().startsWith(base+'/'))outbound.push(r.url());});
 await check('normal browser is read-only and shows native-disconnected state',async()=>{
  await page.goto(base,{waitUntil:'networkidle'});assert.match(await page.getByRole('status').innerText(),/原生功能未连接/);assert(await page.getByLabel('发送给 DeepSeek 娘的消息').isDisabled());
  await page.getByRole('button',{name:'偏好设置',exact:true}).click();assert(await page.getByLabel('API Key',{exact:true}).isDisabled());
 });
 await page.goto(base+'/?test=mock',{waitUntil:'networkidle'});
 await page.getByRole('button',{name:'偏好设置',exact:true}).waitFor();
 await check('Mock mode is explicitly labeled on every test page',async()=>{assert.match(await page.locator('.mode-banner').innerText(),/模拟验收模式.*无真实 DeepSeek API/);});
 await page.screenshot({path:path.join(artifact,'desktop-welcome-mock.png'),fullPage:true});
 await check('key form clears input; no credential in localStorage or cookies',async()=>{
  await configure(page);assert(!JSON.stringify(await page.evaluate(()=>({...localStorage}))).includes('fixture-ui-test-only'));assert(!(await desktop.cookies()).some(c=>c.value.includes('fixture-ui-test-only')));
 });
 await check('editable character title and preferences reach the UI fixture',async()=>{
  await page.getByLabel('如何称呼你').fill('旅人');await page.getByRole('button',{name:'保存聊天设置',exact:true}).click();await page.getByText('聊天与人设设置已保存，新消息将使用这些设置。',{exact:true}).waitFor();
  await page.getByRole('link',{name:/DeepSeek 娘/}).click();assert.match(await page.locator('.companion-card').innerText(),/旅人/);
 });
 await check('Chinese Enter send, streaming updates and stop preserve one user turn',async()=>{
  await page.getByLabel('发送给 DeepSeek 娘的消息').fill('中文输入与停止');await page.getByLabel('发送给 DeepSeek 娘的消息').press('Enter');
  await page.locator('.message.assistant').filter({hasText:'模拟'}).first().waitFor();await page.getByRole('button',{name:/停止生成/}).click();
  await page.locator('.message.assistant[data-state="interrupted"]').waitFor();assert.equal(await page.locator('.message.user').count(),1);
 });
 await check('regeneration keeps the same user and exposes previous reply versions',async()=>{
  await page.getByRole('button',{name:/重新生成/}).click();await page.locator('.message.assistant[data-state="complete"]').waitFor();assert.equal(await page.locator('.message.user').count(),1);
  await page.getByRole('button',{name:/查看历史回复/}).click();assert.equal(await page.locator('.message.assistant').count(),2);await page.getByRole('button',{name:'收起历史回复',exact:true}).click();
 });
 await check('simulated network failure is visible and sent text remains',async()=>{
  await page.getByLabel('发送给 DeepSeek 娘的消息').fill('触发错误');await page.getByRole('button',{name:'发送消息',exact:true}).click();await page.locator('.message.assistant[data-state="failed"]').waitFor();assert.match(await page.getByRole('alert').innerText(),/模拟测试：网络连接失败/);assert.equal(await page.locator('.message.user').count(),2);
  await page.getByRole('button',{name:'关闭错误提示',exact:true}).click();
 });
 await check('new conversation and reloading history do not mix conversations',async()=>{
  await page.getByRole('button',{name:/开启新对话/}).click();assert.equal(await page.locator('.message.user').count(),0);
  await page.locator('.conversation-list button').nth(1).click();await page.locator('.message.user').first().waitFor();assert.equal(await page.locator('.message.user').count(),2);
 });
 await check('Shift Enter inserts newline and IME Enter does not send',async()=>{
  const input=page.getByLabel('发送给 DeepSeek 娘的消息');await input.fill('第一行');await input.press('Shift+Enter');await input.type('第二行');assert.equal(await input.inputValue(),'第一行\n第二行');
  const count=await page.locator('.message.user').count();await input.dispatchEvent('keydown',{key:'Enter',code:'Enter',isComposing:true,bubbles:true});assert.equal(await page.locator('.message.user').count(),count);assert.equal(await input.inputValue(),'第一行\n第二行');await input.fill('');
 });
 await check('theme can change without losing visible messages',async()=>{await page.getByRole('button',{name:'切换深浅色',exact:true}).click();assert.equal(await page.locator('.app').getAttribute('data-theme'),'dark');assert.equal(await page.locator('.message.user').count(),2);});
 await page.screenshot({path:path.join(artifact,'desktop-chat-mock.png'),fullPage:true});
 await check('desktop page has no JS exceptions or external requests',async()=>{assert.deepEqual(errors,[]);assert.deepEqual(outbound,[]);});
 const mobile=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});const phone=await mobile.newPage();const mobileErrors=[];phone.on('pageerror',e=>mobileErrors.push(e.message));
 await phone.goto(base+'/?test=mock',{waitUntil:'networkidle'});
 await check('mobile drawer and native-style send interaction',async()=>{
  await phone.getByRole('button',{name:'打开侧栏',exact:true}).click();await configure(phone);await phone.getByRole('button',{name:'打开侧栏',exact:true}).click();await phone.locator('.conversation-list button').first().click();
  const input=phone.getByLabel('发送给 DeepSeek 娘的消息');await input.fill('手机第一行');await input.press('Enter');assert.equal(await input.inputValue(),'手机第一行\n');assert.equal(await phone.locator('.message.user').count(),0);
  await phone.getByRole('button',{name:'发送消息',exact:true}).click();await phone.locator('.message.assistant[data-state="complete"]').waitFor();
 });
 await check('390px viewport fits and includes explicit Mock notice',async()=>{assert(await phone.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.match(await phone.locator('.mode-banner').innerText(),/模拟验收模式/);assert.deepEqual(mobileErrors,[]);});
 await phone.screenshot({path:path.join(artifact,'mobile-chat-mock.png'),fullPage:true});
 fs.writeFileSync(path.join(artifact,'results.json'),JSON.stringify({scope:'UI Mock only; no actual DeepSeek, SQLite or OS keystore verified',checks,passed:checks.length,browser:browser.version()},null,2)+'\n');
 console.log(JSON.stringify({scope:'explicit UI Mock',passed:checks.length,checks}));
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{await browser?.close();server?.kill('SIGTERM');});
