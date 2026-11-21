const assert = require('node:assert/strict');

async function exerciseOcean(pageFor, shot, button) {
  const screenshots = [];
  const checked = [];
  const capture = async (page, name) => {
    await page.locator('img').evaluateAll(async pictures => {
      await Promise.all(pictures.filter(image => image.getBoundingClientRect().height > 0)
        .map(image => image.decode().catch(() => null)));
    });
    await shot(page, name);
    screenshots.push(name + '.png');
  };
  for (const platform of ['windows', 'android']) {
    const mobile = platform === 'android';
    const label = mobile ? 'phone' : 'desktop';
    const { page, errors } = await pageFor(platform, mobile ? [393, 852] : [1440, 900]);
    const go = async (pageId, name) => {
      const mobileName = { chat: '手机聊天', personas: '手机人格', settings: '手机设置' }[pageId];
      if (mobile && !mobileName) await button(page, '打开侧栏').click();
      if (mobile && mobileName) await button(page, mobileName).click();
      else await page.getByRole('navigation',{name:'主要页面'}).getByRole('button',{name,exact:true}).click();
      await page.locator('.app[data-page="'+pageId+'"]').waitFor();
    };
    await page.locator('.chat-code code').waitFor();
    const selectors = [
      ['chat','聊天'], ['personas','人格管理'], ['settings','设置'],
      ['plugins','插件管理'], ['video','视频中心'], ['storage','本地数据'],
      ['sync','局域网同步'], ['companion','关系与日常'], ['about','关于'],
    ];
    for (const [id,name] of selectors) {
      if (id !== 'chat') await go(id,name);
      if (id === 'video') {
        if (mobile) {
          await page.getByText('海边的船', {exact:true}).click();
          await page.getByText('镜头中出现一艘小船。', {exact:false}).first().waitFor();
        } else {
          const previous = page.locator('details.settings-card').filter({hasText:'旧版本地视频记录'});
          await previous.getByText('旧版本地视频记录', {exact:true}).click();
          await previous.getByText('海边的船 · 已完成', {exact:true}).click();
          await previous.getByText('镜头中出现一艘小船。', {exact:true}).waitFor();
        }
      }
      if (id === 'about') {
        await button(page,'比耶').click();
        assert.match(await page.getByRole('img',{name:'DeepSeek娘 · 比耶'}).getAttribute('src'), /peace\.webp$/);
      }
      await capture(page, label+'-'+id);
    }
    await go('personas','人格管理');
    await button(page, '新建人格').click();
    assert.equal(await page.evaluate(()=>document.activeElement?.id),'new-persona-name');
    await page.getByLabel('人格名称', {exact:true}).fill('海边绘师');
    await button(page, '创建人格并进入聊天').click();
    await page.locator('.persona-list button[aria-current=true]').getByText('海边绘师').waitFor({ state:'attached' });
    await go('personas','人格管理');
    const tile = page.locator('.persona-tile').filter({has:page.getByRole('heading',{name:'海边绘师',exact:true})});
    await tile.locator('summary').click();
    await tile.getByRole('button',{name:'移入回收站',exact:true}).click();
    const dialog = page.getByRole('dialog',{name:'主人，真的要把我收起来吗？'});
    await dialog.locator('img[src*="peeking.webp"]').waitFor();
    await capture(page,label+'-move-to-recycle');
    await dialog.getByRole('button',{name:'移入回收站',exact:true}).click();
    await button(page,'人格回收站').click();
    await page.locator('.recycle-tile').getByRole('heading',{name:'海边绘师'}).waitFor();
    await capture(page,label+'-recycle');
    await go('chat','聊天');
    await button(page,'切换深浅色').click();
    await page.locator('.app[data-theme=dark]').waitFor();
    for (const [id,name] of selectors) {
      if (id !== 'chat') await go(id,name);
      if (id === 'settings') {
        const radius = await page.locator('.settings-index button').first().evaluate(node=>parseFloat(getComputedStyle(node).borderTopLeftRadius));
        assert(radius >= 8, 'settings navigation must use the shared button radius');
        await button(page,'贝壳紫').click();
        await page.locator('.app[data-palette=shell]').waitFor();
        const contrast = await page.locator('.appearance-palette').evaluate(card => {
          const canvas = document.createElement('canvas');
          canvas.width = canvas.height = 1;
          const context = canvas.getContext('2d');
          function luminance(color) {
            context.clearRect(0,0,1,1);
            context.fillStyle = color;
            context.fillRect(0,0,1,1);
            const pixels = context.getImageData(0,0,1,1).data;
            return [0.2126,0.7152,0.0722].reduce((sum,weight,index)=>{
              const component = pixels[index]/255;
              return sum + weight*(component<=0.04045 ? component/12.92 : ((component+0.055)/1.055)**2.4);
            },0);
          }
          const style = getComputedStyle(card);
          const surface = luminance(style.backgroundColor);
          return { surface, ratio:(luminance(style.color)+0.05)/(surface+0.05) };
        });
        assert(contrast.surface<0.22 && contrast.ratio>=4.5, JSON.stringify(contrast));
        await button(page,'特大').click();
        await page.waitForFunction(()=>getComputedStyle(document.documentElement).fontSize==='22.4px');
        assert(await page.locator('.page-scroll').evaluate(el=>el.scrollWidth<=el.clientWidth+1));
        if (mobile) {
          const sizes = await page.getByRole('group', {name:'字号预设'}).getByRole('button')
            .evaluateAll(buttons => buttons.map(button => button.getBoundingClientRect().width));
          assert.equal(sizes.length, 6);
          assert(Math.max(...sizes) - Math.min(...sizes) < 1, 'font presets should keep equal touch targets');
        }
        await capture(page,label+'-large-font');
        await button(page,'标准').click();
        await button(page,'海洋蓝').click();
      }
      await capture(page,label+'-'+id+'-dark');
      const fits = await page.evaluate(() => {
        const main = document.querySelector('.workspace-body > main');
        const detail = main.querySelector('.page-scroll');
        const rect = main.getBoundingClientRect();
        return rect.width > 0 && rect.right <= innerWidth + 1 && (!detail || detail.scrollWidth <= detail.clientWidth + 1);
      });
      assert(fits, label+' '+id+' overflows the usable pane');
    }
    await go('personas','人格管理');
    await button(page,'人格回收站').click();
    await page.locator('.recycle-tile').waitFor();
    await capture(page,label+'-recycle-dark');
    await page.getByRole('button',{name:'永久删除',exact:true}).click();
    await page.getByRole('dialog').waitFor();
    await capture(page,label+'-permanent-delete-dark');
    await button(page,'再想想').click();
    if (!mobile) {
      await go('chat','聊天');
      await page.locator('.companion-showcase img').waitFor();
      await capture(page,'desktop-character-status');
    }
    const broken = await page.locator('img[src*="/branding/ocean/"]').evaluateAll(images => images.filter(img=>img.complete&&img.naturalWidth===0).map(img=>img.src));
    assert.deepEqual(broken, []);
    assert.equal(await page.locator('.error-banner').count(), 0);
    assert.deepEqual(errors, []);
    checked.push({ platform, routes: selectors.map(([id])=>id), light_dark: true, font_change: true,
      installed_art_verified: true, recycle_confirmation: true, video_details: true });
    await page.close();
  }
  return { scope:'Browser WebView layout fixture using synthetic IPC data', checked, screenshots };
}

module.exports = { exerciseOcean };
