import { useEffect, useState } from 'react';
import { pluginAPI, type DueReminder } from './api';

export function useReminders(persona: string, enabled: boolean) {
  const [found, setFound] = useState<{ persona: string; entries: DueReminder[] }>();
  useEffect(() => {
    if (!persona || !enabled) return;
    let live = true;
    async function poll() {
      try {
        const entries = await pluginAPI.due(persona);
        if (live) setFound({ persona, entries });
      } catch { /* Device or selection may change before the next poll. */ }
    }
    void poll();
    const timer = window.setInterval(poll, 10_000);
    return () => { live = false; clearInterval(timer); };
  }, [persona, enabled]);
  async function dismiss(id: string) {
    await pluginAPI.change(persona, { kind: 'dismiss_reminder', reminder_id: id });
    setFound(old => old?.persona === persona
      ? { persona, entries: old.entries.filter(reminder => reminder.id !== id) } : old);
  }
  return { reminder: found?.persona === persona ? found.entries[0] : undefined, dismiss };
}
