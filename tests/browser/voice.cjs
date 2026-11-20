// Main-client voice controls with explicit native IPC fixtures and Chromium audio.
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');

const base = 'http://127.0.0.1:1439';
const root = path.resolve(__dirname, '../..');
const output = process.env.COMPANION_VOICE_EVIDENCE_DIR || path.join(root, '.cache/voice-ui');
const checks = [];
const vite = path.join(path.dirname(require.resolve('vite/package.json')), 'bin/vite.js');
const server = spawn(process.execPath, [vite, '--host', '127.0.0.1', '--port', '1439', '--strictPort'], {
  cwd: path.join(root, 'apps/companion'),
  stdio: 'ignore',
});
let browser;

function wave() {
  const bytes = Buffer.alloc(44 + 16000 * 2 * 2);
  bytes.write('RIFF');
  bytes.writeUInt32LE(bytes.length - 8, 4);
  bytes.write('WAVEfmt ', 8);
  bytes.writeUInt32LE(16, 16);
  bytes.writeUInt16LE(1, 20);
  bytes.writeUInt16LE(1, 22);
  bytes.writeUInt32LE(16000, 24);
  bytes.writeUInt32LE(32000, 28);
  bytes.writeUInt16LE(2, 32);
  bytes.writeUInt16LE(16, 34);
  bytes.write('data', 36);
  bytes.writeUInt32LE(bytes.length - 44, 40);
  for (let i = 0; i < 32000; i++) bytes.writeInt16LE(Math.round(2400 * Math.sin(i * 440 * Math.PI / 8000)), 44 + i * 2);
  return bytes.toString('base64');
}

async function check(name, action) {
  await action();
  checks.push(name);
}

async function open(platform) {
  const page = await browser.newPage({ viewport: platform === 'android' ? { width: 390, height: 844 } : { width: 1440, height: 940 } });
  const external = [];
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (route.request().url().startsWith(base + '/')) return route.continue();
    external.push(route.request().url());
    return route.abort();
  });
  await page.addInitScript(({ platform, wav }) => {
    window.isTauri = true;
    window.calls = [];
    window.voiceFixture = { allowMic: true, focus: true, chunks: 0, pcmBytes: 0, meter: 0 };
    let resolveSpeech;
    let prefs = { voice_id: '', speed_percent: 100, volume_percent: 75, muted: false, follow_mood: false };
    window.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (command, args) => {
        const fixture = window.voiceFixture;
        if (command.startsWith('plugin:event|')) return 1;
        if (command === 'bootstrap' || command === 'companion_snapshot') {
          const { fixtureApi } = await import('/src/test-fixture.ts');
          return command === 'bootstrap'
            ? { ...await fixtureApi.bootstrap(), platform }
            : fixtureApi.companion(args.personaId);
        }
        if (command === 'list_messages') return [{
          id: 'fixture-voice-reply', role: 'assistant', state: 'complete',
          selected: true, created_at_ms: null, content: '朗读界面夹具。'.repeat(50), revision: 1,
        }];
        if (command === 'voice_preferences') {
          if (args.update) prefs = args.update;
          return prefs;
        }
        if (command === 'voice_voices') return [{ id: 'offline-zh', name: '本地测试音色', language: 'zh-CN' }];
        if (command === 'video_select_whisper') return 'fixture.bin';
        if (command === 'voice_guide') return { audio: { wav }, notes: [] };
        if (command === 'voice_speak') { fixture.chunks++; return { wav }; }
        if (command === 'voice_meter') { fixture.meter = Math.max(fixture.meter, args.level); fixture.latestMeter = args.level; return; }
        if (command === 'voice_focus') return fixture.focus;
        if (command === 'voice_record') {
          const kind = args.action.kind;
          window.calls.push(kind);
          if (kind === 'check') return null;
          if (kind === 'begin') {
            if (!fixture.allowMic) throw 'VOICE_PERMISSION_DENIED';
            return new Promise(resolve => { resolveSpeech = resolve; });
          }
          if (kind === 'status') return 'recording';
          if (kind === 'transcribe') {
            fixture.pcmBytes = atob(args.action.pcm).length;
            return '语音测试文本';
          }
          if (kind === 'finish' || kind === 'cancel') {
            resolveSpeech?.(kind === 'finish' ? '语音测试文本' : null);
            resolveSpeech = undefined;
          }
          return null;
        }
        window.calls.push(command);
        throw 'UNKNOWN_FIXTURE_COMMAND';
      },
    };
  }, { platform, wav: wave() });
  await page.goto(base);
  await page.getByRole('button', { name: '语音输入', exact: true }).waitFor();
  return { page, external, errors };
}

(async () => {
  for (let attempt = 0; attempt < 80; attempt++) {
    try { if ((await fetch(base)).ok) break; } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({
    executablePath: process.env.COMPANION_CHROMIUM_EXECUTABLE,
    args: ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream', '--no-sandbox'],
  });
  fs.mkdirSync(output, { recursive: true });
  const windows = await open('windows');
  const win = windows.page;
  await check('Windows playback decodes audio, pauses, mutes, and clears the PCM meter', async () => {
    await win.getByRole('button', { name: '朗读', exact: true }).click();
    await win.getByText('正在播放', { exact: true }).waitFor();
    await win.waitForFunction(() => window.voiceFixture.chunks >= 2 && window.voiceFixture.meter > 0);
    await win.getByRole('button', { name: '暂停播放', exact: true }).click();
    await win.getByRole('button', { name: '继续播放', exact: true }).waitFor();
    await win.waitForFunction(() => window.voiceFixture.latestMeter === 0);
    await win.getByRole('button', { name: '静音', exact: true }).click();
    await win.getByRole('button', { name: '继续播放', exact: true }).click();
    await win.getByRole('button', { name: '取消静音', exact: true }).waitFor();
    await win.getByRole('button', { name: '停止播放', exact: true }).click();
    await win.getByRole('button', { name: '语音输入', exact: true }).waitFor();
  });
  await check('Windows worklet returns bounded PCM and appends a draft without sending', async () => {
    const draft = win.locator('#chat-input');
    await draft.fill('保留原来的草稿');
    await win.getByRole('button', { name: '语音输入', exact: true }).click();
    await win.getByText('正在录音', { exact: true }).waitFor();
    await win.waitForTimeout(750);
    await win.getByRole('button', { name: '完成录音', exact: true }).click();
    await win.getByText('已把识别结果填入输入框。').waitFor();
    assert.equal(await draft.inputValue(), '保留原来的草稿\n语音测试文本');
    assert(await win.evaluate(() => window.voiceFixture.pcmBytes >= 16000 && !window.calls.includes('start_chat')));
  });
  await check('Windows settings save the selected local voice and play a guide without a key', async () => {
    await win.getByRole('button', { name: '语音设置', exact: true }).click();
    await win.getByLabel('离线朗读音色').selectOption('offline-zh');
    await win.getByRole('button', { name: '保存语音设置' }).click();
    await win.getByText('语音设置已保存。').waitFor();
    await win.getByRole('button', { name: '选择 Whisper 模型' }).click();
    await win.getByText(/本次选择 fixture\.bin/).waitFor();
    await win.getByRole('button', { name: '试听原创元音唱段' }).click();
    await win.getByText('正在播放', { exact: true }).waitFor();
    await win.getByRole('button', { name: '停止播放' }).click();
    await win.screenshot({ path: path.join(output, 'windows-voice-fixture.png'), animations: 'disabled' });
  });
  const android = await open('android');
  const phone = android.page;
  await check('Android permission denial and cancellation keep the chat draft unchanged', async () => {
    await phone.evaluate(() => { window.voiceFixture.allowMic = false; });
    await phone.getByRole('button', { name: '语音输入', exact: true }).click();
    await phone.getByText('请在系统设置中允许应用使用麦克风。').waitFor();
    await phone.evaluate(() => { window.voiceFixture.allowMic = true; });
    await phone.getByRole('button', { name: '语音输入', exact: true }).click();
    await phone.getByText('正在录音', { exact: true }).waitFor();
    await phone.getByRole('button', { name: '取消录音' }).click();
    assert.equal(await phone.locator('#chat-input').inputValue(), '');
  });
  await check('Android completed recording fills the draft and audio focus loss stops playback', async () => {
    await phone.getByRole('button', { name: '语音输入', exact: true }).click();
    await phone.getByRole('button', { name: '完成录音' }).click();
    await phone.getByText('已把识别结果填入输入框。').waitFor();
    assert.equal(await phone.locator('#chat-input').inputValue(), '语音测试文本');
    await phone.getByRole('button', { name: '朗读', exact: true }).click();
    await phone.getByText('正在播放', { exact: true }).waitFor();
    await phone.evaluate(() => { window.voiceFixture.focus = false; });
    await phone.getByText('声音播放已停止，其他应用正在使用音频。').waitFor();
    assert(await phone.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  });
  assert.deepEqual([...windows.errors, ...android.errors, ...windows.external, ...android.external], []);
  fs.writeFileSync(path.join(output, 'results.json'), JSON.stringify({
    scope: 'Browser IPC fixture, synthetic microphone, real Web Audio; no native speech engine',
    passed: checks.length, checks,
  }, null, 2) + '\n');
  console.log(`Voice UI fixture: ${checks.length} checks passed.`);
})().catch(error => { console.error(error); process.exitCode = 1; })
  .finally(async () => { await browser?.close(); server.kill(); });
