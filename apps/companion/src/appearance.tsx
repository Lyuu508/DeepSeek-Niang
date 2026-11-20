import { invoke } from '@tauri-apps/api/core';
import { useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { runtimeMode } from './native';

export function useFontScale(saved: number | undefined, scroll: RefObject<HTMLDivElement | null>, onError: (error: unknown) => void) {
  const [percent, setPercent] = useState(100);
  const initialized = useRef(false);
  const queue = useRef(Promise.resolve());

  function apply(value: number) {
    const node = scroll.current;
    const top = node?.getBoundingClientRect().top ?? 0;
    const atEnd = node && node.scrollHeight - node.clientHeight - node.scrollTop < 36;
    const anchor = node && !atEnd
      ? Array.from(node.querySelectorAll<HTMLElement>('[data-message-id]')).find(row => row.getBoundingClientRect().bottom > top)
      : undefined;
    const offset = anchor?.getBoundingClientRect().top;
    document.documentElement.style.setProperty('--ui-font-size', `${value}%`);
    setPercent(value);
    if (node && atEnd) node.scrollTop = node.scrollHeight;
    else if (node && anchor && offset !== undefined) node.scrollTop += anchor.getBoundingClientRect().top - offset;
  }
  useLayoutEffect(() => {
    if (saved === undefined || initialized.current) return;
    initialized.current = true;
    apply(saved >= 85 && saved <= 150 ? saved : 100);
  }, [saved]);

  function change(value: number) {
    initialized.current = true;
    apply(value);
    if (runtimeMode === 'native') {
      queue.current = queue.current.then(() => invoke<void>('set_font_scale', { percent: value })).catch(onError);
    }
  }
  return { percent, change };
}

export type FontScale = ReturnType<typeof useFontScale>;

export function FontSettings({ font }: { font: FontScale }) {
  return <section className="settings-card font-settings">
    <h2>字体大小</h2>
    <label>界面字号 <output>{font.percent}%</output>
      <input type="range" min="85" max="150" step="1" value={font.percent} onChange={e => font.change(e.target.valueAsNumber)}/>
    </label>
    <div className="button-row">{[85, 95, 100, 110, 125, 140].map((value, index) =>
      <button key={value} type="button" aria-pressed={font.percent === value} onClick={() => font.change(value)}>
        {['很小', '较小', '标准', '较大', '大', '特大'][index]}
      </button>)}</div>
  </section>;
}
