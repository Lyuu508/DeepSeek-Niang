// This check is scoped to a clean emulator and the two public test APKs.
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { writeFileSync } = require('node:fs');
const { join } = require('node:path');
const { setTimeout: delay } = require('node:timers/promises');
const { _android: android } = require('playwright');

const identity = 'io.local.deepseekcompanion.test2';
const evidence = process.env.COMPANION_NATIVE_EVIDENCE;
const apkDirectory = process.env.APK_DIRECTORY;
const testPersona = '模拟器升级留存检查';
const report = {
  scope: 'Installed signed APKs in an Android emulator with synthetic local persona data. No physical device or DeepSeek network request.',
  identity,
  expected_release_commit: 'b4f56f55b4cbf567c0b57194a0936cc7c8eeb664',
  screenshots: [],
};
const adb = (...args) => execFileSync('adb', args, { encoding: 'utf8', timeout: 90_000 }).trim();
const button = (page, name) => page.getByRole('button', { name, exact: true });
const save = () => writeFileSync(join(evidence, 'runtime-results.json'), JSON.stringify(report, null, 2) + '\n');

async function screenshot(page, name) {
  if (page) {
    await page.locator('img').evaluateAll(async images => {
      const visible = images.filter(image => image.getBoundingClientRect().height > 0);
      await Promise.all(visible.map(image => image.decode().catch(() => null)));
    });
  }
  await delay(250);
  const png = execFileSync('adb', ['exec-out', 'screencap', '-p'], { timeout: 15_000, maxBuffer: 8 * 1024 * 1024 });
  assert(png.length > 10_000 && png.toString('ascii', 1, 4) === 'PNG');
  const file = `${name}.png`;
  writeFileSync(join(evidence, file), png);
  report.screenshots.push(file);
  save();
}

async function installAndOpen(device, version, upgrade = false) {
  const command = upgrade ? ['install', '-r'] : ['install'];
  assert.match(adb(...command, join(apkDirectory, `${version}.apk`)), /Success/);
  report[upgrade ? 'upgrade_install_success' : 'initial_install_success'] = true;
  const info = adb('shell', 'dumpsys', 'package', identity);
  const versionDescription = info.match(/^\s*versionName=(.+)$/m)?.[1];
  assert.equal(versionDescription, version);
  assert.match(info, new RegExp(`versionCode=${upgrade ? 1010 : 1009}\\b`));
  const activity = adb('shell', 'cmd', 'package', 'resolve-activity', '--brief', identity).split('\n').at(-1);
  assert(activity.startsWith(`${identity}/`), activity);
  assert.match(adb('shell', 'am', 'start', '-W', '-n', activity), /Status:\s*ok/);
  report[upgrade ? 'new_activity_started' : 'old_activity_started'] = true;
  save();
  const page = await (await device.webView({ pkg: identity }, { timeout: 30_000 })).page();
  page.setDefaultTimeout(15_000);
  await page.locator('#chat-input').waitFor();
  await page.locator('.app[data-platform=android]').waitFor();
  report[upgrade ? 'new_webview_rendered' : 'old_webview_rendered'] = true;
  return page;
}

async function go(page, id, name) {
  const mobileName = { chat: '手机聊天', personas: '手机人格', settings: '手机设置' }[id];
  if (mobileName) await button(page, mobileName).click();
  else {
    await button(page, '打开侧栏').click();
    await page.getByRole('navigation', { name: '主要页面' }).getByRole('button', { name, exact: true }).click();
  }
  await page.locator(`.app[data-page="${id}"]`).waitFor();
}

async function data(page) {
  return page.evaluate(() => window.__TAURI_INTERNALS__.invoke('bootstrap'));
}

async function main() {
  report.android_sdk = adb('shell', 'getprop', 'ro.build.version.sdk');
  report.abis = adb('shell', 'getprop', 'ro.product.cpu.abilist');
  const [device] = await android.devices();
  assert(device, 'The Android emulator must be connected');
  try {
    const old = await installAndOpen(device, '0.0.9');
    await screenshot(old, 'android-native-v0.0.9-chat');
    report.previous_viewport = await old.evaluate(() => ({ width: innerWidth, height: innerHeight }));
    await button(old, '手机人格').click();
    await old.getByLabel('人格名称', { exact: true }).fill(testPersona);
    await old.getByLabel('独立人物设定', { exact: true }).fill('只用于隔离模拟器中的升级验证。');
    await button(old, '创建人格并进入聊天').click();
    await old.locator('#chat-input').waitFor();
    const before = (await data(old)).personas.find(persona => persona.name === testPersona);
    assert(before?.id);
    save();
    adb('shell', 'am', 'force-stop', identity);

    const page = await installAndOpen(device, '0.0.10', true);
    const state = await data(page);
    assert.equal(state.personas.find(persona => persona.name === testPersona)?.id, before.id);
    assert.equal(state.active_persona_id, before.id);
    const defaultPersona = state.personas.find(persona => persona.is_builtin);
    assert(defaultPersona);
    report.synthetic_persona_and_identity_preserved = true;
    report.new_viewport = await page.evaluate(() => ({ width: innerWidth, height: innerHeight }));
    await go(page, 'personas', '人格管理');
    const defaultCard = page.locator('.persona-tile').filter({ has: page.getByRole('heading', { name: defaultPersona.name, exact: true }) });
    await defaultCard.getByRole('button', { name: /切换人格/ }).click();
    await page.locator('.app[data-page=chat]').waitFor();
    await screenshot(page, 'android-native-chat');
    await page.locator('#chat-input').click();
    for (let attempt = 0; attempt < 5; attempt++) {
      report.native_keyboard_shown = /mInputShown=true\b/.test(adb('shell', 'dumpsys', 'input_method'));
      if (report.native_keyboard_shown) break;
      await delay(350);
    }
    if (report.native_keyboard_shown) {
      await screenshot(page, 'android-native-keyboard');
      adb('shell', 'input', 'keyevent', '4');
    }
    for (const [id, name] of [
      ['personas', '人格管理'], ['settings', '设置'], ['plugins', '插件管理'], ['video', '视频中心'],
      ['storage', '本地数据'], ['sync', '局域网同步'], ['companion', '关系与日常'], ['about', '关于'],
    ]) {
      await go(page, id, name);
      await screenshot(page, `android-native-${id}`);
    }
    await go(page, 'personas', '人格管理');
    const tile = page.locator('.persona-tile').filter({ has: page.getByRole('heading', { name: testPersona, exact: true }) });
    await tile.locator('summary').click();
    await tile.getByRole('button', { name: '移入回收站', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '主人，真的要把我收起来吗？' });
    await dialog.locator('img[src*="peeking.webp"]').waitFor();
    await screenshot(page, 'android-native-move-to-recycle');
    await dialog.getByRole('button', { name: '移入回收站', exact: true }).click();
    await button(page, '人格回收站').click();
    await page.locator('.recycle-tile').getByRole('heading', { name: testPersona, exact: true }).waitFor();
    await screenshot(page, 'android-native-recycle');
    await button(page, '永久删除').click();
    await page.getByRole('dialog').waitFor();
    await screenshot(page, 'android-native-permanent-delete');
    await button(page, '再想想').click();
    await button(page, '恢复').click();
    await page.locator('.recycle-tile').getByRole('heading', { name: testPersona, exact: true }).waitFor({ state: 'detached' });
    assert.equal((await data(page)).personas.find(persona => persona.name === testPersona)?.id, before.id);
    report.recycle_restore_preserved_identity = true;
    await go(page, 'chat', '聊天');
    await button(page, '切换深浅色').click();
    await page.locator('.app[data-theme=dark]').waitFor();
    await screenshot(page, 'android-native-chat-dark');
    report.passed = true;
  } finally {
    await device.close();
  }
}

main().catch(error => {
  report.passed = false;
  report.issue = error.message;
  writeFileSync(join(evidence, 'android-runtime-errors.txt'), adb('logcat', '-d', '-t', '500', '-s', 'AndroidRuntime:E', 'chromium:E'));
  console.error(error);
  process.exitCode = 1;
}).finally(save);
