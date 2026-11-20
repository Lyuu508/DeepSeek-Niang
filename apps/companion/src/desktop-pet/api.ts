import { Channel, invoke } from '@tauri-apps/api/core';
import type { ChatFailure } from '@deepseek/contracts';

export interface PetSettings {
  visible: boolean;
  always_on_top: boolean;
  scale_percent: number;
  roam: boolean;
  reduce_motion: boolean;
  tray_available: boolean;
  startup_enabled: boolean | null;
}

export type Setting =
  | { kind: 'visible' | 'always_on_top' | 'roam' | 'reduce_motion' | 'startup'; value: boolean }
  | { kind: 'scale_percent'; value: number }
  | { kind: 'reset_position' | 'pet_only' };

export type PetAction =
  | { kind: 'drag' | 'interact' | 'open_chat' | 'hide' | 'toggle_top' | 'toggle_roam' | 'reset_position' }
  | { kind: 'panel' | 'hover' | 'system_reduced_motion'; value: boolean }
  | { kind: 'scale_step'; value: -1 | 1 };

export interface PetMotion {
  walking: boolean;
  facing_left: boolean;
}

export type PetPose = 'idle' | 'happy' | 'shy' | 'thinking' | 'annoyed' | 'replying' | 'resting' | 'reading';

export interface PetSnapshot {
  persona_id: string;
  name: string;
  builtin_avatar: boolean;
  mood_label: string;
  pose: PetPose;
  chat: 'idle' | 'busy' | 'thinking' | 'replying';
  activity: {
    label: string;
    state: string;
    progress_milli: number;
    updated_at_ms: number;
    stale: boolean;
    is_virtual?: boolean;
  } | null;
  unread_share: boolean;
  settings: Pick<PetSettings, 'always_on_top' | 'scale_percent' | 'roam' | 'reduce_motion'>;
  reply: {
    persona_id: string;
    request_id: string;
    phase: 'sending' | 'streaming' | 'complete' | 'interrupted' | 'failed';
    preview: string;
    error: ChatFailure | null;
  } | null;
}

export const desktopPet = {
  settings: () => invoke<PetSettings>('desktop_pet_settings'),
  configure: (setting: Setting) => invoke<PetSettings>('desktop_pet_configure', { setting }),
  snapshot: () => invoke<PetSnapshot>('pet_snapshot'),
  watchMotion: (receive: (motion: PetMotion) => void) => {
    const channel = new Channel<PetMotion>();
    channel.onmessage = receive;
    return invoke<void>('pet_motion', { channel });
  },
  action: (action: PetAction) => invoke<void>('pet_action', { action }),
  send: (personaId: string, requestId: string, text: string) => invoke<void>('pet_send', { personaId, requestId, text }),
  stop: (requestId: string) => invoke<void>('pet_stop', { requestId }),
};

export function petError(error: unknown) {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') {
    return error.message;
  }
  return '桌宠操作失败。请重试或从托盘打开聊天。';
}
