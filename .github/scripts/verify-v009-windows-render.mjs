import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const output = process.argv[2];
let page;
for (let n = 0; n < 30 && !page; n++) {
  try {
    const targets = await (await fetch('http://127.0.0.1:9224/json/list', { signal: AbortSignal.timeout(900) })).json();
    page = targets.find(entry => entry.type === 'page' && entry.url.includes('tauri.localhost'));
  } catch { /* Wait for the Windows WebView to initialize. */ }
  if (!page) await delay(1000);
}
if (!page) throw new Error('The installed app did not expose its WebView target');

const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener('open', resolve, { once: true });
  socket.addEventListener('error', reject, { once: true });
});
let sequence = 0;
const pending = new Map();
const exceptions = [];
socket.addEventListener('message', ({ data }) => {
  const message = JSON.parse(data);
  if (message.id) {
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    if (message.error) request.reject(new Error(message.error.message));
    else request.resolve(message.result);
  } else if (message.method === 'Runtime.exceptionThrown') {
    const error = message.params.exceptionDetails;
    exceptions.push((error.exception?.description || error.text).split('\n')[0].slice(0, 240));
  } else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
    exceptions.push(message.params.entry.text.split('\n')[0].slice(0, 240));
  }
});
const call = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timer = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out`)); }, 6000);
  pending.set(id, {
    resolve(value) { clearTimeout(timer); resolve(value); },
    reject(error) { clearTimeout(timer); reject(error); },
  });
  socket.send(JSON.stringify({ id, method, params }));
});
const inspect = async () => (await call('Runtime.evaluate', { returnByValue: true, expression: `({
  document: document.readyState,
  chat: !!document.querySelector('.app .chat-main'),
  persona_loaded_by_native_ipc: !!document.querySelector('.persona-list [data-persona-id]'),
  composer: !!document.querySelector('textarea'),
  scripts: document.querySelectorAll('script[src]').length,
  welcome_visible: !!document.querySelector('.welcome h1')
})` })).result.value;

try {
  await call('Runtime.enable');
  await call('Log.enable');
  let state;
  for (let n = 0; n < 35; n++) {
    state = await inspect();
    if (state.chat && state.persona_loaded_by_native_ipc && state.composer) break;
    await delay(1000);
  }
  const firstLoad = state;
  if (!state.chat) {
    await call('Page.reload');
    await delay(4000);
    state = await inspect();
  }
  const screenshot = await call('Page.captureScreenshot', { format: 'png' });
  await writeFile(join(output, 'windows-webview.png'), Buffer.from(screenshot.data, 'base64'));
  const result = { sha256: process.env.V009_EXE_SHA, first_load: firstLoad, after_capture: state, exceptions: [...new Set(exceptions)].slice(0, 12) };
  await writeFile(join(output, 'windows-webview.json'), JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify(result));
  if (!firstLoad.chat || !firstLoad.persona_loaded_by_native_ipc || !firstLoad.composer) process.exitCode = 1;
} finally {
  socket.close();
}
