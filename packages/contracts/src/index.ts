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
export interface Dialogue {
  mode:'standard'|'free'|'custom'; tone:'natural'|'gentle'|'calm'|'playful';
  reply_length:'short'|'balanced'|'detailed'; humor:number; creativity:number; expression:number;
  roleplay:'daily'|'in_character'|'co_write'; extra_style:string;
}
export interface ChatSettings {
  dialogue?:Dialogue;
  model:string; temperature:number; max_tokens:number; history_turns:number;
  timeout_seconds:number; include_history:boolean;
  character_name:string; user_title:string; personality:string;
}
export interface Persona { id:string; name:string; main_conversation_id:string; is_builtin:boolean; created_at_ms:number; revision:number; avatar_resource?:string; personality?:string }
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
export interface Bootstrap { font_scale?:number; version:string; platform:string; settings:ChatSettings; personas:Persona[]; active_persona_id:string; persona_summary:PersonaSummary; key_status:KeyStatus; usage:UsageSummary; active_request:string|null; active_conversation:string|null; schema_version:number }
export type ChatEvent =
 | {type:'started';request:PreparedChat;user:ChatMessage;message:ChatMessage}
 | {type:'delta';request_id:string;conversation_id:string;message_id:string;content:string;revision:number}
 | {type:'finished';request_id:string;conversation_id:string;message:ChatMessage;usage:TokenUsage|null;error:ChatFailure|null}
 | {type:'failed';request_id:string;conversation_id:string;error:ChatFailure};

export interface CompanionPreferences {
 relationship_enabled:boolean;automatic_memory:boolean;memory_context:boolean;memory_limit:number;
 memory_budget_bytes:number;context_budget_bytes:number;life_enabled:boolean;preferred_activity:string;
 proactive_enabled:boolean;proactive_mode:"off"|"quiet"|"natural"|"active"|"custom";proactive_model_enabled:boolean;proactive_token_budget:number;notifications_enabled:boolean;proactive_interval_minutes:number;proactive_daily_limit:number;
 quiet_start_minute:number;quiet_end_minute:number;timezone_offset_minutes:number;
}
export interface BondHistory {id:string;source_message_id:string;kind:string;explanation:string;occurred_at_ms:number;delta:number[];applied:boolean;decision:string}
export interface BondSnapshot {familiarity:number;trust:number;affection:number;intimacy:number;comfort:number;stage:string;rule_version:number;applied_events: number;distinct_days:number;history:BondHistory[]}
export interface MoodSnapshot {kind:string;label:string;intensity_milli:number;source:string;started_at_ms:number|null;expires_at_ms:number|null}
export type MemoryState = 'active'|'completed'|'superseded'|'dismissed';
export interface MemoryContext {
 is_event:boolean;shared_story:boolean;state:MemoryState;occurred_at_ms:number;due_at_ms:number|null;
 confidence_milli:number;recall_allowed:boolean;reminder_allowed:boolean;shared_with_personas:boolean;
}
export interface ManagedMemory {id:string;text:string;version:number;revision:string;source_excerpt:string|null;source_life_event_id?:string|null;kind:string;importance_milli:number;pinned:boolean;api_allowed:boolean;owner_locked:boolean;source_kind:string;source_message_id:string|null;created_at_ms:number|null;updated_at_ms:number|null;last_used_at_ms?:number|null;context?:MemoryContext}
export interface MemoryEdit {id:string|null;expected_version:number|null;expected_revision:string|null;text:string;kind:string;importance_milli:number;pinned:boolean;api_allowed:boolean;context?:MemoryContext}
export interface MemoryCandidate {id:string;content:string;kind:string;source_message_id:string;created_at_ms:number;conflicting_memory_id:string|null}
export interface LifeSnapshot {
 host_device_id:string|null;heartbeat_at_ms:number|null;host_online:boolean;
 current:null|{id:string;kind:string;label:string;state:string;started_at_ms:number;scheduled_end_at_ms:number;updated_at_ms:number;execution_kind:'virtual';elapsed_ms?:number;progress_milli?:number;task_id?:string|null;plan_id?:string|null;error_code?:string|null};
 next_activity:string|null;today_events:{id:string;description:string;occurred_at_ms:number;kind:string}[];
 proactive_messages:{id:string;content:string;state:string;created_at_ms:number;delivered_at_ms:number|null;message_id:string|null;read_at_ms?:number|null;notification_state?:string|null;source_life_event_id?:string|null;source_memory_id?:string|null;intent?:string|null}[];
 can_control?:boolean;completed_today?:number;offline_notice?:string;plans?:LifePlan[];interests?:LifeInterest[];diary?:LifeDiary;daily_cost?:LifeCost;
 day_start_ms:number;execution_kind:'virtual';external_media_executed:false;
}
export interface LifePlanInput {name:string;kind:string;steps:number;duration_ms:number}
export interface LifePlan {id:string;name:string;state:string;next_step:number;steps:number;next_due_at_ms:number}
export interface LifeInterest {id:string;name:string;strength:number;customized?:boolean;history:{source_event_id:string;at_ms:number;reason:string;from:number;to:number}[]}
export interface LifeDiary {day:number;timezone_offset_minutes:number;events:{id:string;text:string;at_ms:number;source_id:string;source_kind:string;kind:string}[];chats:{source_id:string;text:string;at_ms:number;source_kind:string}[];moods:{source_id:string;mood:string;at_ms:number;source_kind:string}[];generator:string;api_requests:number}
export interface LifeCost {by_activity:{activity:string;requests:number;measured_requests:number;prompt_tokens:number;completion_tokens:number;cost_micros:number|null;priced_requests:number}[];local_rule_tokens:number}
export interface DailyUsage {weekly?:LifeCost;day_start_ms:number;timezone_offset_minutes:number;by_purpose:Record<'chat'|'memory'|'life',{requests:number;measured_requests:number;unknown_requests:number;prompt_tokens:number;completion_tokens:number}>;memory_and_life_provider:string}
export interface CompanionSnapshot {recall?:{stored_messages:number;strategy:string;additional_model_requests:number};worker?:{error_code:string|null;failures:number};persona_id:string;relationship:BondSnapshot;mood:MoodSnapshot;memories:ManagedMemory[];memory_candidates:MemoryCandidate[];preferences:CompanionPreferences;life:LifeSnapshot;daily_usage:DailyUsage}
