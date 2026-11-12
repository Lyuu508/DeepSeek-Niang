-- Schema v1. All timestamps are UTC epoch milliseconds for display/audit only.
-- Logical ordering uses Lamport + origin + event UUID. Never copy database files for sync.
CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, checksum TEXT NOT NULL);
CREATE TABLE local_profiles(
 id TEXT PRIMARY KEY, display_name TEXT NOT NULL, created_at_ms INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE devices(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 name TEXT NOT NULL, platform TEXT NOT NULL DEFAULT 'unknown',
 trust_state TEXT NOT NULL CHECK(trust_state IN ('local','trusted','revoked')),
 identity_public_key BLOB, certificate_fingerprint TEXT, secret_ref TEXT,
 last_seen_ms INTEGER, revoked_at_ms INTEGER, sync_epoch INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX devices_profile ON devices(profile_id,trust_state);
CREATE TABLE local_clock(
 device_id TEXT PRIMARY KEY REFERENCES devices(id),
 origin_seq INTEGER NOT NULL DEFAULT 0 CHECK(origin_seq>=0),
 lamport INTEGER NOT NULL DEFAULT 0 CHECK(lamport>=0)
);
CREATE TABLE conversations(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 title TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX conversations_profile ON conversations(profile_id);
CREATE TABLE sync_events(
 event_id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 origin_device_id TEXT NOT NULL REFERENCES devices(id),
 origin_seq INTEGER NOT NULL CHECK(origin_seq>0), lamport INTEGER NOT NULL CHECK(lamport>0),
 schema_version INTEGER NOT NULL CHECK(schema_version=1),
 entity_type TEXT NOT NULL CHECK(entity_type IN ('message','memory')),
 entity_id TEXT NOT NULL, operation TEXT NOT NULL CHECK(operation IN ('put','delete')),
 checksum TEXT NOT NULL CHECK(length(checksum)=64),
 envelope_json TEXT NOT NULL CHECK(json_valid(envelope_json)),
 origin_signature BLOB, UNIQUE(origin_device_id,origin_seq)
);
CREATE INDEX sync_entity ON sync_events(entity_type,entity_id,lamport);
CREATE INDEX sync_profile ON sync_events(profile_id,origin_device_id,origin_seq);
CREATE TABLE sync_checkpoints(
 origin_device_id TEXT PRIMARY KEY REFERENCES devices(id),
 contiguous_seq INTEGER NOT NULL DEFAULT 0 CHECK(contiguous_seq>=0)
);
CREATE TABLE sync_sessions(
 id TEXT PRIMARY KEY, peer_device_id TEXT NOT NULL REFERENCES devices(id),
 started_at_ms INTEGER NOT NULL, completed_at_ms INTEGER,
 state TEXT NOT NULL CHECK(state IN ('connecting','syncing','succeeded','failed')),
 error_code TEXT, protocol_version INTEGER NOT NULL
);
CREATE TABLE data_tombstones(
 profile_id TEXT NOT NULL REFERENCES local_profiles(id), entity_type TEXT NOT NULL,
 entity_id TEXT NOT NULL, delete_event_id TEXT NOT NULL REFERENCES sync_events(event_id),
 lamport INTEGER NOT NULL CHECK(lamport>0), origin_device_id TEXT NOT NULL REFERENCES devices(id),
 PRIMARY KEY(profile_id,entity_type,entity_id)
);
CREATE TABLE messages(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 conversation_id TEXT NOT NULL REFERENCES conversations(id),
 role TEXT NOT NULL CHECK(role IN ('user','assistant','tool')),
 content TEXT NOT NULL, origin_device_id TEXT NOT NULL REFERENCES devices(id),
 created_lamport INTEGER NOT NULL CHECK(created_lamport>0),
 source_event_id TEXT NOT NULL UNIQUE REFERENCES sync_events(event_id),
 reply_to_id TEXT REFERENCES messages(id) DEFERRABLE INITIALLY DEFERRED,
 generation_id TEXT, supersedes_id TEXT REFERENCES messages(id),
 status TEXT NOT NULL DEFAULT 'complete' CHECK(status IN ('streaming','complete','interrupted','failed')),
 is_deleted INTEGER NOT NULL DEFAULT 0 CHECK(is_deleted IN (0,1))
);
CREATE INDEX messages_history ON messages(conversation_id,created_lamport,origin_device_id,id);
CREATE TABLE character_profiles(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL UNIQUE REFERENCES local_profiles(id),
 name TEXT NOT NULL, base_personality_json TEXT NOT NULL CHECK(json_valid(base_personality_json)),
 revision INTEGER NOT NULL CHECK(revision>0), source_event_id TEXT REFERENCES sync_events(event_id)
);
CREATE TABLE relationship_states(
 profile_id TEXT PRIMARY KEY REFERENCES local_profiles(id),
 affection_milli INTEGER NOT NULL DEFAULT 0 CHECK(affection_milli BETWEEN 0 AND 100000),
 trust_milli INTEGER NOT NULL DEFAULT 0 CHECK(trust_milli BETWEEN 0 AND 100000),
 intimacy_milli INTEGER NOT NULL DEFAULT 0 CHECK(intimacy_milli BETWEEN 0 AND 100000),
 comfort_milli INTEGER NOT NULL DEFAULT 0 CHECK(comfort_milli BETWEEN 0 AND 100000),
 initiative_milli INTEGER NOT NULL DEFAULT 0 CHECK(initiative_milli BETWEEN 0 AND 100000),
 stage TEXT NOT NULL DEFAULT '陌生', rule_version INTEGER NOT NULL DEFAULT 1,
 source_digest TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE relationship_events(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 evidence_hash TEXT NOT NULL, rule_version INTEGER NOT NULL CHECK(rule_version>0),
 kind TEXT NOT NULL, evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),
 occurred_at_ms INTEGER NOT NULL, lamport INTEGER NOT NULL CHECK(lamport>0),
 origin_device_id TEXT NOT NULL REFERENCES devices(id),
 source_event_id TEXT REFERENCES sync_events(event_id),
 UNIQUE(profile_id,evidence_hash,rule_version)
);
CREATE INDEX relationship_replay ON relationship_events(profile_id,lamport,origin_device_id,id);
CREATE TABLE character_moods(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 mood TEXT NOT NULL, intensity_milli INTEGER NOT NULL CHECK(intensity_milli BETWEEN 0 AND 1000),
 started_at_ms INTEGER NOT NULL, expires_at_ms INTEGER NOT NULL CHECK(expires_at_ms>=started_at_ms),
 source_event_id TEXT REFERENCES sync_events(event_id)
);
CREATE TABLE memories(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 text TEXT NOT NULL, category TEXT NOT NULL DEFAULT 'explicit',
 importance_milli INTEGER NOT NULL DEFAULT 500 CHECK(importance_milli BETWEEN 0 AND 1000),
 sensitivity TEXT NOT NULL DEFAULT 'private' CHECK(sensitivity IN ('private','api_allowed','blocked')),
 version_lamport INTEGER NOT NULL CHECK(version_lamport>0),
 version_origin TEXT NOT NULL REFERENCES devices(id), version_event_id TEXT NOT NULL REFERENCES sync_events(event_id),
 is_deleted INTEGER NOT NULL DEFAULT 0 CHECK(is_deleted IN (0,1))
);
CREATE INDEX memories_profile ON memories(profile_id,is_deleted,category);
CREATE VIRTUAL TABLE memory_fts USING fts5(memory_id UNINDEXED,text,tokenize='trigram');
CREATE TRIGGER memories_ai AFTER INSERT ON memories WHEN new.is_deleted=0 BEGIN
 INSERT INTO memory_fts(memory_id,text) VALUES(new.id,new.text);
END;
CREATE TRIGGER memories_au AFTER UPDATE ON memories BEGIN
 DELETE FROM memory_fts WHERE memory_id=old.id;
 INSERT INTO memory_fts(memory_id,text) SELECT new.id,new.text WHERE new.is_deleted=0;
END;
CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
 DELETE FROM memory_fts WHERE memory_id=old.id;
END;
CREATE TABLE memory_relations(
 from_memory_id TEXT NOT NULL REFERENCES memories(id), to_memory_id TEXT NOT NULL REFERENCES memories(id),
 relation TEXT NOT NULL, source_event_id TEXT REFERENCES sync_events(event_id),
 PRIMARY KEY(from_memory_id,to_memory_id,relation), CHECK(from_memory_id<>to_memory_id)
);
CREATE TABLE memory_embeddings(
 memory_id TEXT NOT NULL REFERENCES memories(id), model_id TEXT NOT NULL,
 model_checksum TEXT NOT NULL, dimensions INTEGER NOT NULL CHECK(dimensions>0),
 vector BLOB NOT NULL, content_checksum TEXT NOT NULL,
 PRIMARY KEY(memory_id,model_id,model_checksum)
);
CREATE TABLE character_interests(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), topic TEXT NOT NULL,
 weight_milli INTEGER NOT NULL CHECK(weight_milli BETWEEN 0 AND 1000),
 evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)), revision INTEGER NOT NULL DEFAULT 1,
 UNIQUE(profile_id,topic)
);
CREATE TABLE task_queue(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 kind TEXT NOT NULL, dedupe_key TEXT NOT NULL,
 authority_device_id TEXT NOT NULL REFERENCES devices(id),
 state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','paused','completed','cancelled','failed','uncertain')),
 priority INTEGER NOT NULL DEFAULT 0 CHECK(priority BETWEEN 0 AND 100),
 payload_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(payload_json)),
 owner_device_id TEXT REFERENCES devices(id), fence INTEGER NOT NULL DEFAULT 0 CHECK(fence>=0),
 lease_boot_id TEXT, lease_until_tick INTEGER, attempt INTEGER NOT NULL DEFAULT 0 CHECK(attempt>=0),
 max_attempts INTEGER NOT NULL DEFAULT 3 CHECK(max_attempts BETWEEN 1 AND 10),
 last_error TEXT, UNIQUE(profile_id,dedupe_key)
);
CREATE INDEX task_schedule ON task_queue(authority_device_id,state,priority DESC);
CREATE TABLE task_checkpoints(
 task_id TEXT NOT NULL REFERENCES task_queue(id), step_key TEXT NOT NULL,
 fence INTEGER NOT NULL CHECK(fence>0), checkpoint_json TEXT NOT NULL CHECK(json_valid(checkpoint_json)),
 checksum TEXT NOT NULL, PRIMARY KEY(task_id,step_key)
);
CREATE TABLE task_logs(
 id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL REFERENCES task_queue(id),
 fence INTEGER NOT NULL, code TEXT NOT NULL, detail_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(detail_json))
);
CREATE TABLE budget_reservations(
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES task_queue(id), request_key TEXT NOT NULL UNIQUE,
 currency TEXT NOT NULL, reserved_micros INTEGER NOT NULL CHECK(reserved_micros>=0),
 actual_micros INTEGER CHECK(actual_micros>=0), state TEXT NOT NULL CHECK(state IN ('reserved','settled','uncertain','released'))
);
CREATE TABLE life_plans(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), name TEXT NOT NULL,
 plan_json TEXT NOT NULL CHECK(json_valid(plan_json)), revision INTEGER NOT NULL DEFAULT 1,
 next_step INTEGER NOT NULL DEFAULT 0 CHECK(next_step>=0), source_event_id TEXT REFERENCES sync_events(event_id)
);
CREATE TABLE daily_activities(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 plan_id TEXT REFERENCES life_plans(id), task_id TEXT REFERENCES task_queue(id),
 kind TEXT NOT NULL, execution_kind TEXT NOT NULL CHECK(execution_kind IN ('real','virtual')),
 started_at_ms INTEGER NOT NULL, ended_at_ms INTEGER,
 progress_milli INTEGER NOT NULL DEFAULT 0 CHECK(progress_milli BETWEEN 0 AND 1000),
 source_event_id TEXT REFERENCES sync_events(event_id), CHECK(ended_at_ms IS NULL OR ended_at_ms>=started_at_ms)
);
CREATE TABLE life_events(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), activity_id TEXT REFERENCES daily_activities(id),
 occurrence_key TEXT NOT NULL, event_json TEXT NOT NULL CHECK(json_valid(event_json)),
 source_event_id TEXT REFERENCES sync_events(event_id), UNIQUE(profile_id,occurrence_key)
);
CREATE TABLE proactive_settings(
 profile_id TEXT PRIMARY KEY REFERENCES local_profiles(id),
 mode TEXT NOT NULL DEFAULT 'natural' CHECK(mode IN ('off','quiet','natural','active','custom')),
 timezone TEXT NOT NULL DEFAULT 'UTC', quiet_start TEXT NOT NULL DEFAULT '22:00', quiet_end TEXT NOT NULL DEFAULT '08:00',
 cooldown_seconds INTEGER NOT NULL DEFAULT 7200 CHECK(cooldown_seconds>=0),
 notifications_enabled INTEGER NOT NULL DEFAULT 0 CHECK(notifications_enabled IN (0,1)),
 revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE proactive_messages(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 trigger_key TEXT NOT NULL, evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),
 content TEXT NOT NULL, state TEXT NOT NULL CHECK(state IN ('candidate','approved','delivered','dismissed')),
 source_event_id TEXT REFERENCES sync_events(event_id), UNIQUE(profile_id,trigger_key)
);
CREATE TABLE notification_deliveries(
 proactive_message_id TEXT NOT NULL REFERENCES proactive_messages(id), device_id TEXT NOT NULL REFERENCES devices(id),
 delivery_token TEXT NOT NULL UNIQUE, delivered_at_ms INTEGER,
 PRIMARY KEY(proactive_message_id,device_id)
);
CREATE TABLE video_sources(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 source_kind TEXT NOT NULL CHECK(source_kind IN ('local_file','authorized_bilibili')),
 external_video_id TEXT, title TEXT NOT NULL, source_url TEXT, uploader TEXT,
 media_checksum TEXT, duration_ms INTEGER NOT NULL CHECK(duration_ms>0),
 authorization_json TEXT NOT NULL CHECK(json_valid(authorization_json)),
 UNIQUE(profile_id,source_kind,external_video_id)
);
CREATE TABLE video_tasks(
 id TEXT PRIMARY KEY REFERENCES task_queue(id), source_id TEXT NOT NULL REFERENCES video_sources(id),
 analysis_version TEXT NOT NULL, capability_snapshot_json TEXT NOT NULL CHECK(json_valid(capability_snapshot_json)),
 model_id TEXT NOT NULL, media_available INTEGER NOT NULL DEFAULT 0 CHECK(media_available IN (0,1)),
 UNIQUE(source_id,analysis_version)
);
CREATE TABLE video_progress(
 task_id TEXT PRIMARY KEY REFERENCES video_tasks(id),
 playback_position_ms INTEGER NOT NULL DEFAULT 0 CHECK(playback_position_ms>=0),
 decoded_until_ms INTEGER NOT NULL DEFAULT 0 CHECK(decoded_until_ms>=0),
 playback_state TEXT NOT NULL DEFAULT 'stopped', analysis_state TEXT NOT NULL DEFAULT 'queued',
 visual_coverage_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(visual_coverage_json)),
 audio_coverage_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(audio_coverage_json)),
 uncertain_intervals_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(uncertain_intervals_json)),
 updated_at_ms INTEGER NOT NULL DEFAULT 0, fence INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE video_segments(
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES video_tasks(id),
 start_ms INTEGER NOT NULL CHECK(start_ms>=0), end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),
 media_checksum TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed','uncertain')),
 request_key TEXT NOT NULL UNIQUE, sampling_json TEXT NOT NULL CHECK(json_valid(sampling_json)),
 UNIQUE(task_id,start_ms,end_ms)
);
CREATE TABLE video_timeline_events(
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES video_tasks(id),
 start_ms INTEGER NOT NULL CHECK(start_ms>=0), end_ms INTEGER NOT NULL CHECK(end_ms>=start_ms),
 kind TEXT NOT NULL, description TEXT NOT NULL, confidence_milli INTEGER NOT NULL CHECK(confidence_milli BETWEEN 0 AND 1000),
 evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json))
);
CREATE INDEX video_timeline ON video_timeline_events(task_id,start_ms);
CREATE TABLE video_analysis_results(
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES video_tasks(id), segment_id TEXT REFERENCES video_segments(id),
 revision INTEGER NOT NULL CHECK(revision>0), result_json TEXT NOT NULL CHECK(json_valid(result_json)),
 model_id TEXT NOT NULL, prompt_version TEXT NOT NULL, checksum TEXT NOT NULL,
 UNIQUE(task_id,segment_id,revision)
);
CREATE TABLE video_memories(
 id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES video_sources(id),
 memory_id TEXT NOT NULL REFERENCES memories(id), task_id TEXT NOT NULL REFERENCES video_tasks(id),
 reaction_json TEXT NOT NULL CHECK(json_valid(reaction_json)), share_worthy INTEGER NOT NULL CHECK(share_worthy IN (0,1)),
 UNIQUE(task_id,memory_id)
);
CREATE TABLE meme_knowledge(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), phrase TEXT NOT NULL,
 meaning TEXT NOT NULL, provenance_json TEXT NOT NULL CHECK(json_valid(provenance_json)),
 confidence_milli INTEGER NOT NULL CHECK(confidence_milli BETWEEN 0 AND 1000), UNIQUE(profile_id,phrase)
);
CREATE TABLE media_interests(
 interest_id TEXT NOT NULL REFERENCES character_interests(id), source_id TEXT NOT NULL REFERENCES video_sources(id),
 evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)), PRIMARY KEY(interest_id,source_id)
);
CREATE TABLE external_accounts(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), provider TEXT NOT NULL,
 external_user_id TEXT NOT NULL, credential_ref TEXT NOT NULL,
 scopes_json TEXT NOT NULL CHECK(json_valid(scopes_json)), UNIQUE(profile_id,provider,external_user_id)
);
CREATE TABLE application_settings(
 profile_id TEXT NOT NULL REFERENCES local_profiles(id), key TEXT NOT NULL,
 value_json TEXT NOT NULL CHECK(json_valid(value_json)),
 scope TEXT NOT NULL CHECK(scope IN ('profile','device')), device_id TEXT NOT NULL DEFAULT '',
 revision INTEGER NOT NULL DEFAULT 1, PRIMARY KEY(profile_id,key,device_id)
);
CREATE TABLE activity_states(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 task_id TEXT REFERENCES task_queue(id), activity_id TEXT REFERENCES daily_activities(id),
 executor_device_id TEXT NOT NULL REFERENCES devices(id), fence INTEGER NOT NULL DEFAULT 0,
 kind TEXT NOT NULL, state TEXT NOT NULL, progress_milli INTEGER NOT NULL CHECK(progress_milli BETWEEN 0 AND 1000),
 started_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL, heartbeat_ttl_ms INTEGER NOT NULL CHECK(heartbeat_ttl_ms>0),
 mood TEXT NOT NULL, next_plan TEXT, source_json TEXT NOT NULL CHECK(json_valid(source_json)), error_code TEXT,
 CHECK(task_id IS NOT NULL OR activity_id IS NOT NULL)
);
CREATE INDEX activity_latest ON activity_states(profile_id,updated_at_ms DESC);
CREATE TABLE api_usage_records(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), task_id TEXT REFERENCES task_queue(id),
 request_key TEXT NOT NULL UNIQUE, provider TEXT NOT NULL, model_id TEXT NOT NULL,
 input_tokens INTEGER CHECK(input_tokens>=0), output_tokens INTEGER CHECK(output_tokens>=0),
 cost_micros INTEGER CHECK(cost_micros>=0), currency TEXT NOT NULL,
 usage_state TEXT NOT NULL CHECK(usage_state IN ('measured','estimated','unknown')),
 created_at_ms INTEGER NOT NULL, price_snapshot_json TEXT NOT NULL CHECK(json_valid(price_snapshot_json))
);
