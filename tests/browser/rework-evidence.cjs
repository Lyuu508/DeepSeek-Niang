const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs');
const { execFileSync } = require('node:child_process');

async function exerciseRework(pageFor, shot, button, output) {
  const { page, errors } = await pageFor('windows', [1203, 882]);
  const send = await button(page,'发送消息').boundingBox();
  const tool = await button(page,'添加附件').boundingBox();
  assert.equal(send.width, send.height);
  assert.equal(send.width, tool.width);
  assert.equal(await page.getByText('发送范围', {exact:true}).count(), 0);
  await page.evaluate(() => {
    const previous = window.__TAURI_INTERNALS__.invoke;
    const calls = [];
    let record;
    let entries = [];
    window.reworkCalls = calls;
    window.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === 'start_chat') {
        calls.push([command, args.imageConsent, args.images?.map(image => atob(image.jpeg).slice(0, 3))]);
        throw 'SYNTHETIC_FIXTURE_STOPS_CHAT';
      }
      if (command === 'sticker_catalog') {
        if (args.action.kind === 'import') {
          entries = args.action.entries.map((item, id) => ({...item, id: String(id)}));
          calls.push(['import_stickers', entries.map(item => [item.image.name, item.tag])]);
        }
        return { entries, automatic: false };
      }
      if (command === 'cloud_video_plan') {
        calls.push([command, args.authorized]);
        record = { id: 'synthetic-video', question: args.question, duration_ms: args.durationMs,
          plan: {start_ms: 0, end_ms: args.durationMs, focus: '抽样画面'}, evidence: null, answer: null };
        return record;
      }
      if (command === 'cloud_video_recognize') {
        calls.push([command, args.input.kind, args.input.frames?.length ?? args.input.mp4.length, args.authorized]);
        return {...record, evidence: { observations: [], limitations: '测试夹具' }};
      }
      if (command === 'cloud_video_answer') {
        calls.push([command, args.authorized]);
        return {...record, answer: '合成结果'};
      }
      if (command === 'cloud_video_save_chat') {calls.push([command]); return null;}
      return previous(command, args);
    };
  });
  const png = fs.readFileSync(path.resolve(__dirname,'../../crates/companion-chat/src/video_cloud_probe.png'));
  const transfer = await page.evaluateHandle(base => {
    const data = new DataTransfer();
    data.items.add(new File([Uint8Array.from(atob(base), c=>c.charCodeAt(0))], 'fixture.png', {type:'image/png'}));
    return data;
  }, png.toString('base64'));
  await page.locator('.message-scroll').dispatchEvent('drop', {dataTransfer:transfer});
  const gallery = page.locator('.attachment-gallery');
  await gallery.locator('img').waitFor();
  const input = page.locator('#chat-input');
  await input.focus();
  await input.evaluate((element, clipboardData) => element.dispatchEvent(new ClipboardEvent('paste', {
    bubbles: true, cancelable: true, clipboardData,
  })), transfer);
  await gallery.locator('img').nth(1).waitFor();
  assert.equal(await gallery.locator('img').count(),2);
  await gallery.getByRole('button',{name:'移除图片 fixture.png',exact:true}).first().click();
  assert.equal(await gallery.locator('img').count(),1);
  const photoConsent = page.getByRole('checkbox',{name:/用千问识别图片/});
  assert(!(await photoConsent.isChecked()));
  await photoConsent.check();
  await shot(page,'desktop-chat-image-preview');
  await button(page,'发送消息').click();
  await page.waitForFunction(()=>window.reworkCalls.some(call=>call[0]==='start_chat'));
  const imageCall = await page.evaluate(()=>window.reworkCalls.find(call=>call[0]==='start_chat'));
  assert.deepEqual(imageCall, ['start_chat', true, ['\xff\xd8\xff']]);
  await page.getByRole('alert').filter({hasText:'操作未能完成'}).waitFor();
  await gallery.getByRole('button',{name:'移除图片 fixture.png',exact:true}).click();

  await button(page,'表情包').click();
  const stickerPicker = page.getByRole('region',{name:'本机表情库'});
  const fileChooser = page.waitForEvent('filechooser');
  await stickerPicker.getByRole('button',{name:'导入 dsh-meme 清单与本机图片'}).click();
  await (await fileChooser).setFiles([
    { name:'manifest.json', mimeType:'application/json', buffer:Buffer.from(JSON.stringify({
      id:'fixture', memes:[{url:'https://example.invalid/fixture.png',file:'fixture.png',tag:'happy',caption:'测试表情'}],
    })) },
    {name:'fixture.png', mimeType:'image/png', buffer:png},
  ]);
  await stickerPicker.getByRole('img',{name:'测试表情'}).waitFor();
  assert.deepEqual(await page.evaluate(()=>window.reworkCalls.find(call=>call[0]==='import_stickers')),
    ['import_stickers',[['测试表情','开心']]]);
  await stickerPicker.getByRole('img',{name:'测试表情'}).click();
  await gallery.getByRole('img',{name:'测试表情'}).waitFor();
  await gallery.getByRole('button',{name:'移除图片 测试表情',exact:true}).click();

  const video=path.join(output,'synthetic.webm');
  const clip=path.join(output,'synthetic.mp4');
  execFileSync('ffmpeg',['-loglevel','error','-y','-f','lavfi','-i','color=c=blue:s=64x64:d=0.8',
    '-c:v','libvpx','-pix_fmt','yuv420p',video],{timeout:15000});
  execFileSync('ffmpeg',['-loglevel','error','-y','-f','lavfi','-i','color=c=blue:s=64x64:d=0.8',
    '-c:v','libx264','-pix_fmt','yuv420p',clip],{timeout:15000});
  const clipChooser = page.waitForEvent('filechooser');
  await button(page,'添加视频').click();
  await (await clipChooser).setFiles(clip);
  const clipConsent = page.getByRole('checkbox',{name:/接收 MP4 原片/});
  await clipConsent.check();
  await button(page,'改用抽样帧').click();
  assert(!(await page.getByRole('checkbox',{name:/识别抽样画面/}).isChecked()));
  await button(page,'移除视频').click();
  const videoChooser = page.waitForEvent('filechooser');
  await button(page,'添加视频').click();
  await (await videoChooser).setFiles(video);
  const consent = page.getByRole('checkbox',{name:/我有权发送此视频/});
  await consent.waitFor();
  await input.fill('画面是什么颜色？');
  assert(await button(page,'发送消息').isDisabled());
  await consent.check();
  await shot(page,'desktop-chat-video-ready');
  await button(page,'发送消息').click();
  await page.waitForFunction(()=>window.reworkCalls.some(call=>call[0]==='cloud_video_save_chat'));
  assert.deepEqual(await page.evaluate(()=>window.reworkCalls.filter(call=>call[0].startsWith('cloud_video'))),[
    ['cloud_video_plan',true],['cloud_video_recognize','frames',4,true],['cloud_video_answer',true],['cloud_video_save_chat'],
  ]);
  assert.equal(await page.locator('.app').getAttribute('data-page'),'chat');
  assert.deepEqual(errors,[]);
  await page.close();

  const empty = (await pageFor('windows', [1203,882], true)).page;
  const configure = empty.locator('.composer-access');
  await configure.waitFor();
  assert((await configure.boundingBox()).width < (await empty.locator('.composer').boundingBox()).width / 2);
  await shot(empty,'desktop-chat-no-key');
  await configure.click();
  assert(await empty.locator('#setting-api').isVisible());
  await empty.close();

  const narrow = (await pageFor('android',[295,663])).page;
  await button(narrow,'手机设置').click();
  const nav=narrow.getByRole('navigation',{name:'设置分类'});
  for (const [label,id] of [['API 配置','api'],['聊天与人设','character'],['外观','appearance']]) {
    const tab=button(nav,label);
    assert((await tab.boundingBox()).height>=44);
    await tab.click();
    assert.equal(await tab.getAttribute('aria-current'),'location');
    assert(await narrow.locator('#setting-'+id).isVisible());
    assert.equal(await narrow.locator('.settings-workbench-body > [data-setting]:visible').count(),1);
    assert.equal(await narrow.evaluate(()=>document.documentElement.scrollWidth),295);
  }
  await shot(narrow,'phone-settings-295');
  await narrow.close();
  return ['chat drag and paste, image consent and removal','dsh-meme manifest import and local selection',
    'changing MP4 mode resets consent; real WebM decode and synthetic video IPC',
    'compact API access and circular send control','295px settings navigation and touch targets'];
}

module.exports = { exerciseRework };
