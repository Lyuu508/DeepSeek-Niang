import { useRef, useState } from 'react';
import { Icon, type IconName } from './OceanDesign';

interface Props {
  id?: string;
  label: string;
  accept: string;
  hint: string;
  icon?: IconName;
  disabled?: boolean;
  busy?: string;
  file?: File;
  onFile: (file: File) => void;
  onClear?: () => void;
}

export function FileDrop({ id, label, accept, hint, icon = 'add', disabled, busy, file, onFile, onClear }: Props) {
  const input = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const inactive = disabled || Boolean(busy);
  return <div className="file-drop-area">
    <input id={id} type="file" ref={input} hidden accept={accept} disabled={inactive}
      onChange={event => {
        const value = event.currentTarget.files?.[0];
        event.currentTarget.value = '';
        if (value && !inactive) onFile(value);
      }}/>
    <button type="button" className="file-drop" aria-label={label} data-dragging={dragging} disabled={inactive}
      onClick={() => input.current?.click()} onDragOver={event => {
        if (!event.dataTransfer.types.includes('Files')) return;
        event.preventDefault();
        if (!inactive) setDragging(true);
      }}
      onDragLeave={() => setDragging(false)} onDrop={event => {
        event.preventDefault();
        setDragging(false);
        const value = event.dataTransfer.files[0];
        if (value && !inactive) onFile(value);
      }}>
      <Icon name={icon}/>
      <span><strong>{file?.name ?? label}</strong>
        <small>{file ? `${file.type || file.name.split('.').at(-1)} · ${(file.size / 1048576).toFixed(2)} MiB` : hint}</small>
      </span>
    </button>
    {busy && <p role="status"><progress aria-label={busy}/> {busy}</p>}
    {file && onClear && <button type="button" disabled={inactive} onClick={onClear}>取消选择</button>}
  </div>;
}
