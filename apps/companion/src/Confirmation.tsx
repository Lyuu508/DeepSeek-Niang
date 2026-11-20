import { useEffect, useId, useRef, type ReactNode } from 'react';

export function Confirmation({ title, body, cancelLabel, confirmLabel, danger, busy, canConfirm = true, onCancel, onConfirm }: {
  title: string; body: ReactNode; cancelLabel: string; confirmLabel: string;
  danger?: boolean; busy: boolean; canConfirm?: boolean; onCancel: () => void; onConfirm: () => void;
}) {
  const root = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const node = root.current;
    if (!node) return;
    node.showModal();
    return () => node.close();
  }, []);
  return <dialog ref={root} className="confirmation" aria-labelledby={titleId} onCancel={event => {
    event.preventDefault();
    if (!busy) onCancel();
  }}>
    <h2 id={titleId}>{title}</h2>
    <div className="confirmation-body">{body}</div>
    <div className="confirmation-actions">
      <button type="button" autoFocus disabled={busy} onClick={onCancel}>{cancelLabel}</button>
      <button type="button" className={danger ? 'danger' : 'primary'} data-danger={!!danger}
        disabled={busy || !canConfirm} onClick={onConfirm}>{busy ? '正在处理…' : confirmLabel}</button>
    </div>
  </dialog>;
}
