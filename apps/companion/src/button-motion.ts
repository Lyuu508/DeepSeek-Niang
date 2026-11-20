/** A single pointer listener serves each WebView, including buttons in dialogs. */
export function installButtonMotion() {
  const anchor = (event: PointerEvent) => {
    if (event.pointerType !== 'mouse' || !(event.target instanceof Element)) return;
    const button = event.target.closest('button');
    if (!button || button.disabled || (event.relatedTarget instanceof Node && button.contains(event.relatedTarget))) return;
    const rect = button.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    button.style.setProperty('--hover-x', `${100 * (event.clientX - rect.left) / rect.width}%`);
    button.style.setProperty('--hover-y', `${100 * (event.clientY - rect.top) / rect.height}%`);
  };
  document.addEventListener('pointerover', anchor, { passive: true });
}
