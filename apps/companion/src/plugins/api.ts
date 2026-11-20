import { invoke } from '@tauri-apps/api/core';

export type Config = Record<string, string | number | boolean>;
export interface Manifest {
  id: string; name: string; version: string; description: string; author: string; license: string;
  entry: { kind: 'chat_style' | 'time_reminder' | 'memory_note' | 'info_card' | 'media_note'; body?: string };
  supportedPlatforms: string[]; permissions: string[];
  configSchema: { key: string; label: string; kind: 'text' | 'number' | 'toggle' | 'choice'; default: Config[string]; choices: string[] }[];
  requiredApiVersion: number; checksum: string;
}
export interface Plugin {
  manifest: Manifest; source: string; global_enabled: boolean; persona_enabled: boolean | null;
  granted: string[]; global_config: Config; persona_config: Config | null; effective_config: Config;
  status: 'enabled' | 'disabled' | 'permission_required' | 'unsupported';
  last_action: string | null;
}
export interface Preview { manifest: Manifest; compatible: boolean; source: string; operation: 'install' | 'update' | 'installed' }
export interface DueReminder { id: string; label: string; due_at_ms: number }
export const pluginAPI = {
  list: (persona: string) => invoke<Plugin[]>('list_plugins', { persona }),
  inspect: (raw: string, source: string) => invoke<Preview>('inspect_plugin', { raw, source }),
  change: <T = unknown>(persona: string, action: Record<string, unknown>) => invoke<T>('change_plugin', { persona, action }),
  due: (persona: string) => invoke<DueReminder[]>('due_plugin_reminders', { persona }),
};
