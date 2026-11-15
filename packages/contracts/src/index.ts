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
export interface Persona { id:string; name:string; main_conversation_id:string; is_builtin:boolean; created_at_ms:number; revision:number; avatar_resource?:string }
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

export interface CompanionPreferences {
 relationship_enabled:boolean;automatic_memory:boolean;memory_context:boolean;memory_limit:number;
 memory_budget_bytes:number;context_budget_bytes:number;life_enabled:boolean;preferred_activity:string;
 proactive_enabled:boolean;proactive_interval_minutes:number;proactive_daily_limit:number;
 quiet_start_minute:number;quiet_end_minute:number;timezone_offset_minutes:number;
}
export interface BondHistory {id:string;source_message_id:string;kind:string;explanation:string;occurred_at_ms:number;delta:number[];applied:boolean;decision:string}
export interface BondSnapshot {familiarity:number;trust:number;affection:number;intimacy:number;comfort:number;stage:string;rule_version:number;applied_events: number;distinct_days:number;history:BondHistory[]}
export interface MoodSnapshot {kind:string;label:string;intensity_milli:number;source:string;started_at_ms:number|null;expires_at_ms:number|null}
export interface ManagedMemory {id:string;text:string;version:number;revision:string;source_excerpt:string|null;kind:string;importance_milli:number;pinned:boolean;api_allowed:boolean;owner_locked:boolean;source_kind:string;source_message_id:string|null;created_at_ms:number|null;updated_at_ms:number|null}
export interface MemoryEdit {id:string|null;expected_version:number|null;expected_revision:string|null;text:string;kind:string;importance_milli:number;pinned:boolean;api_allowed:boolean}
export interface MemoryCandidate {id:string;content:string;kind:string;source_message_id:string;created_at_ms:number;conflicting_memory_id:string|null}
export interface LifeSnapshot {
 host_device_id:string|null;heartbeat_at_ms:number|null;host_online:boolean;
 current:null|{id:string;kind:string;label:string;state:string;started_at_ms:number;scheduled_end_at_ms:number;updated_at_ms:number;execution_kind:'virtual'};
 next_activity:string|null;today_events:{id:string;description:string;occurred_at_ms:number;kind:string}[];
 proactive_messages:{id:string;content:string;state:string;created_at_ms:number;delivered_at_ms:number|null;message_id:string|null}[];
 day_start_ms:number;execution_kind:'virtual';external_media_executed:false;
}
export interface DailyUsage {day_start_ms:number;timezone_offset_minutes:number;by_purpose:Record<'chat'|'memory'|'life',{requests:number;measured_requests:number;unknown_requests:number;prompt_tokens:number;completion_tokens:number}>;memory_and_life_provider:string}
export interface CompanionSnapshot {worker?:{error_code:string|null;failures:number};persona_id:string;relationship:BondSnapshot;mood:MoodSnapshot;memories:ManagedMemory[];memory_candidates:MemoryCandidate[];preferences:CompanionPreferences;life:LifeSnapshot;daily_usage:DailyUsage}
