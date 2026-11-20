import { Art, Icon, dismissDialog, type CharacterArt } from './OceanDesign';
import { useEffect, useId, useRef, type ReactNode } from 'react';

export function Confirmation({ title, body, cancelLabel, confirmLabel, danger, art, busy, canConfirm = true, onCancel, onConfirm }: {
  title: string; body: ReactNode; cancelLabel: string; confirmLabel: string;
  danger?: boolean; art?: CharacterArt; busy: boolean; canConfirm?: boolean; onCancel: () => void; onConfirm: () => void;
}) {
  const root = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const node = root.current;
    if (!node) return;
    node.showModal();
    return () => node.close();
  }, []);
  const cancel = () => { if (!busy) dismissDialog(root.current, onCancel); };
  return <dialog ref={root} className="confirmation" aria-labelledby={titleId} onCancel={event => {
    event.preventDefault();
    cancel();
  }}>
    <Art name={art ?? (danger ? 'sad' : 'peeking')} className="confirmation-illustration" eager/>
    <h2 id={titleId} className="confirmation-title">{danger && <Icon name="trash"/>}{title}</h2>
    <div className="confirmation-body">{body}</div>
    <div className="confirmation-actions">
      <button type="button" autoFocus disabled={busy} onClick={cancel}>{cancelLabel}</button>
      <button type="button" className={danger ? 'danger' : 'primary'} data-danger={!!danger}
        disabled={busy || !canConfirm} onClick={() => { if (!root.current?.dataset.closing) onConfirm(); }}>{busy ? '正在处理…' : confirmLabel}</button>
    </div>
  </dialog>;
}
