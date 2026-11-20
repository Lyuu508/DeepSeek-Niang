// Repeatable WebView-sized browser fixture with synthetic chats and native IPC stubs.
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');
const { exerciseOcean } = require('./ocean-evidence.cjs');

const root = path.resolve(__dirname, '../..');
const output = process.env.COMPANION_V009_EVIDENCE || path.join(root, '.cache/v009');
const baseline = process.env.COMPANION_V009_BASELINE === '1';
const base = 'http://127.0.0.1:1441';
const vite = path.join(path.dirname(require.resolve('vite/package.json')), 'bin/vite.js');
const server = spawn(process.execPath, [vite, '--host', '127.0.0.1', '--port', '1441', '--strictPort'], {
  cwd: path.join(root, 'apps/companion'),
  env: { ...process.env, VITE_COMPANION_TEST_PACKAGE: '1' },
  stdio: 'ignore',
});
const sizes = [[360, 800], [375, 812], [393, 852], [412, 915], [1024, 768], [812, 375]];
const report = { scope: 'Chromium browser fixture; native IPC and keyboard geometry simulated', mobile: [], checks: [] };
let browser;

async function pageFor(platform, [width, height]) {
  const page = await browser.newPage({
    viewport: { width, height },
    deviceScaleFactor: 1,
    hasTouch: platform === 'android',
    isMobile: platform === 'android',
  });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => route.request().url().startsWith(base + '/') ? route.continue() : route.abort());
  await page.addInitScript(platform => {
    window.isTauri = true;
    const recycled = new Map();
    const erased = new Set();
    const plugins = new Map();
    let initialized = false;
    window.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        if (cmd.startsWith('plugin:event|')) return 1;
        const { fixtureApi } = await import('/src/test-fixture.ts');
        if (cmd === 'bootstrap') {
          if (!initialized) { await fixtureApi.setKey('fixture-layout'); initialized = true; }
          const boot = await fixtureApi.bootstrap();
          return { ...boot, platform, font_scale: Number(sessionStorage.getItem('fixture-font') ?? '100'),
            personas: boot.personas.filter(p => !recycled.has(p.id) && !erased.has(p.id)) };
        }
        if (cmd === 'set_font_scale') { sessionStorage.setItem('fixture-font', String(args.percent)); return; }
        if (cmd === 'select_persona') return fixtureApi.selectPersona(args.id);
        if (cmd === 'create_persona') return fixtureApi.createPersona(args.name,args.personality);
        if (cmd === 'save_chat_settings') return fixtureApi.saveSettings(args.personaId,args.settings);
        if (cmd === 'companion_snapshot') {
          const data = await fixtureApi.companion(args.personaId);
          data.memories = [...data.memories, {
            id:'fixture-video-memory', text:'本地视频《海边的船》的抽帧与语音分析摘要：镜头中出现一艘小船。',
            version:1, revision:'fixture-video-v1', kind:'episodic', pinned:false, api_allowed:false,
            owner_locked:true, source_kind:'manual', importance_milli:480, source_message_id:null,
            created_at_ms:1795184460000, updated_at_ms:1795184460000,
          }];
          return data;
        }
        if (cmd === 'diagnostics') return fixtureApi.diagnostics();
        if (cmd === 'video_list') return [{
          id:'video-1', title:'海边的船', duration_ms:24000, state:'completed', has_audio:true,
          error:null, updated_at_ms:1795184460000, decoded_ms:24000, visual_ms:24000, audio_ms:24000,
          samples:8, memory_saved:true, summary:'镜头中出现一艘小船。',
          segments:[{ interval:{start_ms:0,end_ms:24000}, state:'completed', evidence:null }],
        }];
        if (cmd === 'sync_status') {
          const { syncFixture } = await import('/src/sync-fixture.ts');
          return syncFixture.status();
        }
        if (cmd === 'list_messages') return [{
          id: 'layout-user', role: 'user', state: 'complete', selected: true, revision: 1, created_at_ms: 1795184400000,
          content: '今天在海边散步，风很舒服。',
        }, {
          id: 'layout-assistant', role: 'assistant', state: 'complete', selected: true, revision: 1, created_at_ms: 1795184460000,
          user_message_id: 'layout-user', content: '听起来是个适合慢慢走的下午。你最喜欢海边的什么声音？\n\n\x60\x60\x60ts\nconst tide = "平静";\n\x60\x60\x60',
        }];
        if (cmd === 'voice_preferences') return { voice_id: '', speed_percent: 100, volume_percent: 75, muted: false, follow_mood: false };
        if (cmd === 'voice_voices') return [];
        if (cmd === 'due_plugin_reminders') return [];
        if (cmd === 'desktop_pet_settings') return { visible: false, always_on_top:true, scale_percent:100, roam:true, reduce_motion:false };
        if (cmd === 'list_recycled_personas') return Array.from(recycled.values());
        if (cmd === 'change_recycled_personas') {
          const action = args.action;
          if (action.kind === 'trash') {
            const boot = await fixtureApi.bootstrap();
            const p = boot.personas.find(p => p.id === action.persona_id && !p.is_builtin);
            if (!p) throw 'FIXTURE_INVALID';
            recycled.set(p.id, {
              persona: p, entry: { persona_id: p.id, decision_id: crypto.randomUUID() },
              personality: '喜欢海边速写', changed_at_ms: 1795184460000, sync_state: 'pending',
            });
            if (boot.active_persona_id === p.id) await fixtureApi.selectPersona(boot.personas.find(p => p.is_builtin).id);
            return [p.main_conversation_id];
          }
          const selected = action.kind === 'restore' ? [action.entry] : action.entries;
          const out = [];
          for (const entry of selected) {
            const record = recycled.get(entry.persona_id);
            if (record?.entry.decision_id !== entry.decision_id) throw 'FIXTURE_CHANGED';
            out.push(record.persona.main_conversation_id);
            recycled.delete(entry.persona_id);
            if (action.kind === 'purge') erased.add(entry.persona_id);
          }
          return out;
        }
        if (cmd === 'inspect_plugin') {
          const m = JSON.parse(args.raw);
          return { manifest: m, source: args.source, compatible:true, operation: plugins.has(m.id) ? 'installed' : 'install' };
        }
        if (cmd === 'list_plugins') return [...plugins.values()];
        if (cmd === 'change_plugin') {
          const a = args.action;
          if (a.kind === 'install') {
            const m = JSON.parse(a.raw);
            const config = Object.fromEntries(m.configSchema.map(field => [field.key,field.default]));
            plugins.set(m.id,{manifest:m,source:a.source,global_enabled:false,persona_enabled:null,
              granted:[],global_config:config,persona_config:null,effective_config:config,status:'disabled',last_action:'installed'});
            return null;
          }
          const plugin = plugins.get(a.id);
          if (a.kind === 'uninstall') { plugins.delete(a.id); return null; }
          if (a.kind === 'create_reminder') return { due_at_ms: Date.now() + a.minutes * 60_000 };
          if (a.kind === 'grants') plugin.granted = a.permissions;
          if (a.kind === 'settings') { plugin.global_enabled = a.enabled; plugin.effective_config = a.config; }
          plugin.status = !plugin.global_enabled ? 'disabled' :
            !plugin.manifest.permissions.every(p=>plugin.granted.includes(p)) ? 'permission_required' : 'enabled';
          return null;
        }
        throw 'FIXTURE_UNAVAILABLE';
      },
    };
  }, platform);
  await page.goto(base);
  await page.getByText('今天在海边散步，风很舒服。', { exact: true }).waitFor();
  return { page, errors };
}

async function measurements(page) {
  return page.evaluate(() => {
    const height = selector => document.querySelector(selector)?.getBoundingClientRect().height ?? 0;
    return {
      width: innerWidth, height: innerHeight, header: height('header'), nav: height('.mobile-nav'),
      composer: height('.composer-area'), message_px: height('.message-scroll'), overflow: document.documentElement.scrollWidth > innerWidth,
    };
  });
}

const button = (page, name) => page.getByRole('button', { name, exact: true });
const shot = (page, name) => page.screenshot({ path: path.join(output, name + '.png'), animations: 'disabled' });
async function openDrawer(page, name) {
  await button(page, '打开侧栏').click();
  await button(page, name).click();
}
async function addPersona(page, name) {
  await button(page, '手机人格').click();
  await page.getByLabel('人格名称', { exact:true }).fill(name);
  await button(page, '创建人格并进入聊天').click();
  await page.locator('.persona-list button[aria-current=true]').getByText(name).waitFor({state:'attached'});
  await button(page, '手机人格').click();
}
async function trash(page, name, takeProof = false) {
  const tile = page.locator('.persona-tile').filter({ has: page.getByRole('heading',{name,exact:true}) });
  await tile.locator('summary').click();
  await button(tile,'移入回收站').click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: '主人，真的要把我收起来吗？' }).waitFor();
  if (takeProof) {
    await shot(page,'persona-trash-confirm');
    await button(dialog,'再想想').click();
    assert.equal(await tile.count(),1);
    await button(tile,'移入回收站').click();
  }
  await button(dialog,'移入回收站').click();
  await tile.waitFor({ state:'detached' });
}

async function exercisePhone() {
  const { page, errors } = await pageFor('android',[375,812]);
  const field=page.locator('#chat-input');
  await field.fill('这段草稿应当留下');
  await field.focus();
  await page.setViewportSize({width:375,height:508});
  await page.evaluate(() => window.dispatchEvent(new CustomEvent('companion-ime',{detail:true})));
  assert(await page.locator('.mobile-nav').isHidden());
  assert((await button(page,'发送消息').boundingBox()).height>=44);
  await shot(page,'android-keyboard-resized');
  await page.setViewportSize({width:375,height:812});
  await page.evaluate(() => window.dispatchEvent(new CustomEvent('companion-ime',{detail:false})));
  report.checks.push('IME event and reduced viewport keep input and send reachable');
  await button(page,'手机设置').click();
  await button(page,'特大').click();
  await page.waitForFunction(()=>getComputedStyle(document.documentElement).fontSize==='22.4px');
  await shot(page,'font-scale-140');
  await button(page,'手机聊天').click();
  assert.equal(await field.inputValue(),'这段草稿应当留下');
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),375);
  await page.reload();
  await page.waitForFunction(()=>getComputedStyle(document.documentElement).fontSize==='22.4px');
  await button(page,'手机设置').click();
  await button(page,'标准').click();
  await page.getByLabel('对话模式',{exact:true}).selectOption('free');
  await shot(page,'dialogue-free');
  await page.getByLabel('对话模式',{exact:true}).selectOption('custom');
  await page.getByLabel('额外聊天风格').fill('用自然的短句');
  await page.getByRole('button',{ name:/保存聊天/ }).click();
  await page.getByText('聊天与人设设置已保存，新消息将使用这些设置。').waitFor();
  report.checks.push('device font change, preserved draft and restored scale; dialogue save');
  await addPersona(page,'潮汐画师');
  await shot(page,'persona-management');
  await trash(page,'潮汐画师',true);
  await button(page,'人格回收站').click();
  const row=page.locator('.recycle-tile').filter({hasText:'潮汐画师'});
  await row.waitFor();
  const originalId=await page.evaluate(async()=> (await window.__TAURI_INTERNALS__.invoke('list_recycled_personas'))[0].persona.id);
  await shot(page,'persona-recycle');
  await button(row,'恢复').click();
  await page.getByText('我又能继续陪你聊天啦！',{exact:true}).waitFor();
  assert.equal(await page.locator(`.persona-list [data-persona-id="${originalId}"]`).count(),1);
  await shot(page,'persona-restored');
  await button(page,'返回人格管理').click();
  await trash(page,'潮汐画师');
  await button(page,'人格回收站').click();
  await button(row,'永久删除').click();
  const dialog=page.getByRole('dialog');
  await shot(page,'persona-delete-confirm');
  await button(dialog,'再想想').click();
  assert.equal(await row.count(),1);
  await button(row,'永久删除').click();
  await button(dialog,'永久删除').click();
  await row.waitFor({state:'detached'});
  await button(page,'返回人格管理').click();
  await addPersona(page,'清空夹具甲');
  await addPersona(page,'清空夹具乙');
  await trash(page,'清空夹具甲');
  await trash(page,'清空夹具乙');
  await button(page,'人格回收站').click();
  await button(page,'清空回收站').click();
  await shot(page,'persona-empty-confirm');
  await button(dialog,'取消清空').click();
  assert.equal(await page.locator('.recycle-tile').count(),2);
  await button(page,'清空回收站').click();
  await button(dialog,'确认永久清空').click();
  await page.getByText('回收站为空。',{exact:true}).waitFor();
  assert.equal(await page.locator('.persona-list button').count(),1);
  report.checks.push('trash cancel, fallback, same-ID restore, permanent and bulk cancel/confirm');
  await openDrawer(page,'插件管理');
  await button(page,'示例 · 本地时间与提醒').click();
  await dialog.getByText('CC0-1.0',{exact:true}).waitFor();
  await button(dialog,'确认安装').click();
  const card=page.locator('.plugin-card').filter({hasText:'本地时间与提醒'});
  await card.locator('summary').click();
  await card.getByLabel('显示本机时间',{exact:true}).check();
  await card.getByLabel('为当前人格设置本机提醒',{exact:true}).check();
  await button(card,'保存权限').click();
  await card.getByLabel('全局启用',{exact:true}).check();
  await button(card,'保存配置').click();
  await card.getByLabel('提醒内容',{exact:true}).fill('喝水');
  await card.getByLabel('几分钟后提醒',{exact:true}).fill('5');
  await button(card,'添加提醒').click();
  await page.getByText(/已保存提醒/).waitFor();
  await shot(page,'plugins');
  await card.getByLabel('为当前人格设置本机提醒',{exact:true}).uncheck();
  await button(card,'保存权限').click();
  await card.getByText('待授权',{exact:true}).waitFor();
  await button(card,'卸载插件').click();
  await button(dialog,'卸载').click();
  await card.waitFor({state:'detached'});
  report.checks.push('plugin install consent, configuration, reminder, revoke and uninstall UI');
  await page.close();
  assert.deepEqual(errors,[]);
}

(async () => {
  for (let attempt = 0; attempt < 100; attempt++) {
    try { if ((await fetch(base)).ok) break; } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({ executablePath: process.env.COMPANION_CHROMIUM_EXECUTABLE, args: ['--no-sandbox'] });
  fs.mkdirSync(output, { recursive: true });
  for (const size of sizes) {
    const { page, errors } = await pageFor('android', size);
    const measured=await measurements(page);
    report.mobile.push(measured);
    if (!baseline) {
      assert.equal(measured.overflow,false);
      assert.equal(measured.nav,53);
      assert(measured.message_px >= size[1] - 165);
    }
    if (size[0] === 375) await page.screenshot({ path: path.join(output, baseline ? 'android-before.png' : 'android-after.png'), animations: 'disabled' });
    assert.deepEqual(errors, []);
    await page.close();
  }
  const { page, errors } = await pageFor('windows', [1240, 864]);
  await page.screenshot({ path: path.join(output, baseline ? 'windows-before.png' : 'windows-after.png'), animations: 'disabled' });
  if (!baseline) {
    const target=button(page,'设置');
    const rect=await target.boundingBox();
    await page.mouse.move(rect.x+5,rect.y+7);
    const hover=await target.evaluate(el=>({ x:el.style.getPropertyValue('--hover-x'), clip:getComputedStyle(el,'::before').clipPath }));
    assert(hover.x.endsWith('%'));
    await page.waitForTimeout(290);
    assert.match(await target.evaluate(el=>getComputedStyle(el,'::before').clipPath),/circle\(145%/);
    await page.emulateMedia({reducedMotion:'reduce'});
    assert.equal(await target.evaluate(el=>getComputedStyle(el,'::before').transitionDuration),'0s, 0s');
    await button(page,'切换深浅色').click();
    await page.locator('[data-theme=dark]').waitFor();
    await shot(page,'windows-dark');
    report.checks.push('Windows pointer-origin hover, reduced motion and dark theme');
    await exercisePhone();
  }
  assert.deepEqual(errors, []);
  await page.close();
  if (!baseline) report.ocean = await exerciseOcean(pageFor, shot, button);
  fs.writeFileSync(path.join(output, baseline ? 'before.json' : 'layout-results.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
})().catch(error => { console.error(error); process.exitCode = 1; })
  .finally(async () => { await browser?.close(); server.kill(); });
