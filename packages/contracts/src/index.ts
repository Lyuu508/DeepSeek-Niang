// Stage-0 IPC read models; native smoke validation is still required on both OSes.
export interface Diagnostics {
  version: string;
  sqlite_version: string;
  schema_version: number;
  device_id: string;
  profile_id: string;
  message_count: number;
  memory_count: number;
  event_count: number;
  integrity: string;
}
export interface MemoryRecord { id: string; text: string; version_lamport: number }
export type NativeCommand = 'diagnostics' | 'list_memories' | 'save_memory' | 'delete_memory';

export interface ChatFailure { code:string; message:string; retryable:boolean }
export interface ChatSettings {
  model:string; temperature:number; max_tokens:number; history_turns:number;
  timeout_seconds:number; include_history:boolean;
  character_name:string; user_title:string; personality:string;
}
export interface Persona { id:string; name:string; main_conversation_id:string; is_builtin:boolean; created_at_ms:number; revision:number }
export interface PersonaSummary { memory_count:number; activity_count:number; interest_count:number; relationship:Record<string,unknown>; mood:Record<string,unknown>; life:Record<string,unknown> }
export interface Conversation { id:string; title:string; created_at_ms:number|null; updated_at_ms:number|null; message_count:number }
export interface ChatMessage {
  id:string; conversation_id:string; role:string; content:string;
  state:'sending'|'generating'|'complete'|'interrupted'|'failed'; created_at_ms:number|null;
  model_id:string|null; request_id:string|null; user_message_id:string|null;
  revision:number; selected:boolean; error_code:string|null; finish_reason:string|null;
}
export interface StartChat { request_id:string; conversation_id:string; text:string|null; regenerate_user_id:string|null }
export interface PreparedChat { request_id:string; conversation_id:string; user_message_id:string; assistant_id:string; is_new:boolean }
export interface TokenUsage { prompt_tokens:number; completion_tokens:number; total_tokens:number }
export interface UsageSummary { requests:number; measured_requests:number; prompt_tokens:number; completion_tokens:number }
export interface KeyStatus { configured:boolean; backend:string; error:ChatFailure|null }
export interface Bootstrap { version:string; platform:string; settings:ChatSettings; personas:Persona[]; active_persona_id:string; persona_summary:PersonaSummary; key_status:KeyStatus; usage:UsageSummary; active_request:string|null; active_conversation:string|null; schema_version:number }
export type ChatEvent =
 | {type:'started';request:PreparedChat;user:ChatMessage;message:ChatMessage}
 | {type:'delta';request_id:string;conversation_id:string;message_id:string;content:string;revision:number}
 | {type:'finished';request_id:string;conversation_id:string;message:ChatMessage;usage:TokenUsage|null;error:ChatFailure|null}
 | {type:'failed';request_id:string;conversation_id:string;error:ChatFailure};
