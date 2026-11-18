/* Production pet renderer with a fixture for the Windows IPC and native motion channel. */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require('playwright');

const output = process.env.COMPANION_PET_EVIDENCE_DIR || 'output/desktop-pet-ui';
const dist = path.resolve(__dirname, '../../apps/companion/dist');
const mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.webp': 'image/webp' };
const server = http.createServer((req, res) => {
  const filename = path.resolve(dist, '.' + new URL(req.url, 'http://localhost').pathname);
  if (!filename.startsWith(dist + path.sep) || !fs.existsSync(filename)) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, { 'Content-Type': mimeTypes[path.extname(filename)] || 'application/octet-stream' });
  fs.createReadStream(filename).pipe(res);
});

async function run() {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch({ executablePath: process.env.COMPANION_CHROMIUM_EXECUTABLE, headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 224, height: 406 } });
    const errors = [];
    const external = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => {
      if (route.request().url().startsWith(base + '/')) return route.continue();
      external.push(route.request().url());
      return route.abort();
    });
    await page.goto(base + '/pet.html');
    await page.getByText('请从 Windows 应用设置打开桌宠。').waitFor();
    const checks = ['ordinary browser requires the real Windows app'];
    await page.addInitScript(() => {
      window.isTauri = true;
      window.petCalls = [];
      window.petFixture = {
        persona_id: 'first', name: 'DeepSeek娘', builtin_avatar: true,
        mood_label: '平静', pose: 'idle', chat: 'idle', unread_share: false,
        activity: { label: '虚拟休息', state: 'running', updated_at_ms: 1800000000000, progress_milli: 310, stale: true },
        settings: { visible: true, always_on_top: true, scale_percent: 100, roam: true, reduce_motion: false }, reply: null,
      };
      const callbacks = new Map();
      let callbackId = 1;
      window.__TAURI_INTERNALS__ = { transformCallback: fn => {
        const id = callbackId++;
        callbacks.set(id, fn);
        return id;
      }, unregisterCallback: id => callbacks.delete(id), invoke: async (command, args) => {
        window.petCalls.push({ command, args });
        if (command === 'pet_snapshot') return structuredClone(window.petFixture);
        if (command === 'pet_motion') {
          const id = Number(args.channel.toJSON().split(':')[1]);
          let index = 0;
          window.publishPetMotion = message => callbacks.get(id)({ index: index++, message });
          window.publishPetMotion({ walking: false, facing_left: true });
          return;
        }
        if (command === 'pet_action') {
          if (args.action.kind === 'toggle_roam') window.petFixture.settings.roam = !window.petFixture.settings.roam;
          return;
        }
        if (command === 'pet_send') {
          if (args.personaId !== window.petFixture.persona_id) throw { message: '已切换人格' };
          window.petFixture.reply = { persona_id: args.personaId, request_id: args.requestId,
            phase: 'streaming', preview: '回环界面夹具，仅供验证', error: null };
          window.petFixture.chat = 'replying';
          window.petFixture.pose = 'replying';
          return;
        }
        if (command === 'pet_stop') {
          window.petFixture.reply.phase = 'interrupted';
          window.petFixture.chat = 'idle';
          window.petFixture.pose = 'idle';
          return;
        }
        throw Error(`Unexpected IPC ${command}`);
      } };
    });
    await page.reload();
    await page.mouse.move(0, 240);
    const avatar = page.getByRole('button', { name: '桌宠角色，双击打开聊天' });
    const controls = page.getByRole('navigation', { name: '桌宠操作' });
    const actor = page.locator('.pet-character');
    const screenshotDir = path.resolve(output);
    fs.mkdirSync(screenshotDir, { recursive: true });
    await page.waitForFunction(() => window.publishPetMotion);
    const sampleAlpha = await page.locator('.pet-sprite').evaluate(async sprite => {
      await sprite.decode();
      const canvas = document.createElement('canvas');
      canvas.width = sprite.naturalWidth;
      canvas.height = sprite.naturalHeight;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(sprite, 0, 0);
      return [ctx.getImageData(0, 0, 1, 1).data[3], ctx.getImageData(160, 150, 1, 1).data[3]];
    });
    assert(sampleAlpha[0] <= 2, 'corner remains transparent after alpha video conversion');
    assert(sampleAlpha[1] > 180);
    assert.equal(await controls.count(), 0);
    assert.equal(await page.getByRole('button', { name: '查看桌宠状态' }).count(), 0);
    await page.locator('main.pet').screenshot({ path: path.join(screenshotDir, 'desktop-pet-character.png'), omitBackground: true });
    await page.evaluate(() => window.publishPetMotion({ walking: true, facing_left: false }));
    await page.locator('.pet-character[data-animation="walk"][data-facing="right"]').waitFor();
    assert.equal(await actor.locator('img').evaluate(el => getComputedStyle(el).transform), 'matrix(-1, 0, 0, 1, 0, 0)');
    checks.push('transparent character, no permanent controls, real clip selection and mirrored heading from motion IPC');
    await avatar.hover();
    await controls.waitFor();
    await page.waitForFunction(() => window.petCalls.some(c => c.args?.action?.kind === 'hover' && c.args.action.value === true));
    assert.equal(await actor.getAttribute('data-animation'), 'idle');
    await avatar.click();
    await page.locator('.pet-character[data-animation="wave"]').waitFor();
    assert.equal(await page.evaluate(() => window.petCalls.filter(c => c.args?.action?.kind === 'interact').length), 1);
    assert.equal(await page.evaluate(() => window.petCalls.filter(c => c.command === 'pet_send').length), 0);
    const body = await avatar.boundingBox();
    await page.mouse.move(body.x + body.width / 2, body.y + body.height / 2);
    await page.mouse.down();
    await page.mouse.move(body.x + body.width / 2 + 18, body.y + body.height / 2 + 6, { steps: 5 });
    await page.mouse.up();
    assert.equal(await page.evaluate(() => window.petCalls.filter(c => c.args?.action?.kind === 'drag').length), 1);
    checks.push('hover stops gait, single click waves, and a character drag requests one native move');

    await page.getByText('活动状态已过期').waitFor();
    await page.getByRole('button', { name: '查看桌宠状态' }).click();
    await page.getByText('最后记录 · 虚拟休息').waitFor();
    assert.equal(await page.locator('progress').getAttribute('value'), '310');
    assert.equal(await page.locator('main.pet').getAttribute('data-pose'), 'idle');
    checks.push('stale state retains recorded progress without a live pose');

    await page.getByRole('button', { name: '快聊', exact: true }).click();
    const draft = page.getByRole('textbox', { name: '桌宠快捷消息' });
    await draft.fill('第一位的草稿');
    await page.evaluate(() => Object.assign(window.petFixture, {
      persona_id: 'second', name: '星河', builtin_avatar: false, mood_label: '好奇', pose: 'reading',
    }));
    await page.getByText('与星河快聊').waitFor();
    assert.equal(await draft.inputValue(), '');
    await draft.fill('第二位的消息');
    await draft.evaluate(el => el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, isComposing: true })));
    assert.equal(await page.evaluate(() => window.petCalls.filter(c => c.command === 'pet_send').length), 0);
    await page.getByRole('button', { name: '发送', exact: true }).click();
    await page.getByText('回环界面夹具，仅供验证').waitFor();
    const calls = await page.evaluate(() => window.petCalls.filter(c => c.command === 'pet_send'));
    assert.equal(calls.length, 1);
    assert.equal(calls[0].args.personaId, 'second');
    assert.equal(calls[0].args.text, '第二位的消息');
    await page.getByRole('button', { name: '停止', exact: true }).click();
    assert.equal(await page.evaluate(() => window.petCalls.filter(c => c.command === 'pet_stop').length), 1);
    assert.equal(await page.locator('main.pet').getAttribute('data-pose'), 'idle');
    checks.push('personas keep distinct drafts and send/stop only one matching request');

    await page.evaluate(() => {
      Object.assign(window.petFixture, { persona_id: 'first', name: 'DeepSeek娘', builtin_avatar: true,
        mood_label: '开心', pose: 'happy', unread_share: true, reply: null });
      window.petFixture.settings.reduce_motion = true;
    });
    await page.getByText('与DeepSeek娘快聊').waitFor();
    assert.equal(await draft.inputValue(), '第一位的草稿');
    assert.equal(await page.locator('main.pet').getAttribute('data-motion'), 'off');
    await page.getByRole('button', { name: '折叠面板' }).click();
    await avatar.dblclick();
    await page.waitForTimeout(320);
    const actions = await page.evaluate(() => window.petCalls.filter(c => c.command === 'pet_action').map(c => c.args.action));
    assert.equal(actions.filter(a => a.kind === 'open_chat').length, 1);
    assert.equal(await page.getByRole('region', { name: '活动详情' }).count(), 0);
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForFunction(() => window.petCalls.some(c => c.args?.action?.kind === 'system_reduced_motion' && c.args.action.value === true));
    await page.locator('.pet-character[data-animation="standing"]').waitFor();
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.waitForFunction(() => window.petCalls.filter(c => c.args?.action?.kind === 'system_reduced_motion').at(-1)?.args.action.value === false);
    checks.push('double-click opens chat once and respects reduced motion');

    await page.evaluate(() => { window.petFixture.settings.reduce_motion = false; });
    await page.locator('main[data-motion="on"]').waitFor();
    await avatar.click({ button: 'right' });
    await page.getByRole('button', { name: '暂停走动' }).click();
    await page.getByRole('button', { name: '恢复走动' }).waitFor();
    await page.getByRole('button', { name: '恢复走动' }).click();
    await page.getByRole('button', { name: '暂停走动' }).waitFor();
    await page.getByRole('button', { name: '缩小桌宠' }).click();
    await page.getByRole('button', { name: '取消置顶' }).click();
    await page.getByRole('button', { name: '隐藏桌宠' }).click();
    assert.deepEqual(await page.evaluate(() => window.petCalls.filter(c => ['scale_step', 'toggle_top', 'hide'].includes(c.args?.action?.kind)).map(c => c.args.action.kind)),
      ['scale_step', 'toggle_top', 'hide']);
    checks.push('menu calls only its allowed window actions');
    assert.deepEqual(errors, []);
    assert.deepEqual(external, []);
    await page.screenshot({ path: path.join(output, 'desktop-pet-ui-fixture.png'), omitBackground: true });
    const result = { passed: checks.length, checks, scope: 'Browser IPC fixture. Windows window movement and DeepSeek service are outside this run.', external_requests: external.length };
    fs.writeFileSync(path.join(output, 'results.json'), JSON.stringify(result, null, 2) + '\n');
    console.log(JSON.stringify(result));
  } finally {
    await browser.close();
  }
}

run().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => server.close());
