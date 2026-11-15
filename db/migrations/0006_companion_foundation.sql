-- Additive stage-3 data. Never rewrite original messages, memories or personas.
CREATE TABLE companion_preferences(
 persona_id TEXT PRIMARY KEY REFERENCES personas(id),
 config_json TEXT NOT NULL CHECK(json_valid(config_json)), revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE companion_bond_events(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id),
 origin_device_id TEXT NOT NULL REFERENCES devices(id),
 source_message_id TEXT NOT NULL REFERENCES messages(id),
 kind TEXT NOT NULL CHECK(kind IN ('care','shared_day','recalled_experience','promise_kept','reconciliation','disappointment','broken_promise')),
 topic TEXT NOT NULL CHECK(length(topic)<=120), explanation TEXT NOT NULL CHECK(length(explanation)<=500),
 occurred_at_ms INTEGER NOT NULL CHECK(occurred_at_ms>=0), lamport INTEGER NOT NULL CHECK(lamport>0),
 rule_version INTEGER NOT NULL DEFAULT 2 CHECK(rule_version=2),
 UNIQUE(persona_id,source_message_id,kind)
);
CREATE INDEX companion_bond_replay ON companion_bond_events(persona_id,lamport,origin_device_id,id);
CREATE TABLE companion_memory_meta(
 memory_id TEXT PRIMARY KEY REFERENCES memories(id), persona_id TEXT NOT NULL REFERENCES personas(id),
 kind TEXT NOT NULL CHECK(kind IN ('episodic','semantic','relationship','knowledge')),
 source_message_id TEXT REFERENCES messages(id), source_kind TEXT NOT NULL CHECK(source_kind IN ('manual','chat','legacy')),
 topic_key TEXT NOT NULL DEFAULT '', pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0,1)),
 owner_locked INTEGER NOT NULL DEFAULT 1 CHECK(owner_locked IN (0,1)),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0), updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=created_at_ms),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0)
);
CREATE TRIGGER companion_memory_owner BEFORE INSERT ON companion_memory_meta BEGIN
 SELECT CASE WHEN new.persona_id IS NOT (SELECT persona_id FROM memories WHERE id=new.memory_id)
 THEN RAISE(ABORT,'memory metadata persona mismatch') END;
END;
CREATE TABLE companion_memory_candidates(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), source_message_id TEXT NOT NULL REFERENCES messages(id),
 kind TEXT NOT NULL CHECK(kind IN ('episodic','semantic','relationship','knowledge')),
 topic_key TEXT NOT NULL, content TEXT NOT NULL CHECK(length(content) BETWEEN 1 AND 2000),
 importance_milli INTEGER NOT NULL CHECK(importance_milli BETWEEN 0 AND 1000),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0),
 state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','accepted','rejected')),
 conflicting_memory_id TEXT REFERENCES memories(id), expected_memory_version INTEGER,
 expected_memory_revision TEXT REFERENCES sync_events(event_id),
 resolved_memory_id TEXT REFERENCES memories(id), UNIQUE(persona_id,source_message_id,topic_key)
);
CREATE TABLE companion_hosts(
 persona_id TEXT PRIMARY KEY REFERENCES personas(id), device_id TEXT NOT NULL REFERENCES devices(id),
 boot_id TEXT NOT NULL, heartbeat_at_ms INTEGER NOT NULL CHECK(heartbeat_at_ms>=0),
 next_activity TEXT NOT NULL
);
CREATE TABLE companion_activities(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), host_device_id TEXT NOT NULL REFERENCES devices(id),
 kind TEXT NOT NULL CHECK(kind IN ('rest','study','video','music','organize','recall','wait','share')),
 execution_kind TEXT NOT NULL DEFAULT 'virtual' CHECK(execution_kind='virtual'),
 state TEXT NOT NULL CHECK(state IN ('running','completed','paused')),
 started_at_ms INTEGER NOT NULL CHECK(started_at_ms>=0), scheduled_end_at_ms INTEGER NOT NULL CHECK(scheduled_end_at_ms>started_at_ms),
 ended_at_ms INTEGER CHECK(ended_at_ms>=started_at_ms), updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=started_at_ms)
);
CREATE UNIQUE INDEX companion_one_activity ON companion_activities(persona_id) WHERE state='running';
CREATE TABLE companion_life_events(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), activity_id TEXT NOT NULL REFERENCES companion_activities(id),
 host_device_id TEXT NOT NULL REFERENCES devices(id), kind TEXT NOT NULL CHECK(kind IN ('started','completed','paused')),
 description TEXT NOT NULL, occurred_at_ms INTEGER NOT NULL CHECK(occurred_at_ms>=0), UNIQUE(activity_id,kind)
);
CREATE TABLE companion_mood_events(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), origin_device_id TEXT NOT NULL REFERENCES devices(id),
 source_message_id TEXT REFERENCES messages(id), activity_id TEXT REFERENCES companion_activities(id),
 mood TEXT NOT NULL CHECK(mood IN ('happy','shy','expectant','curious','calm','hurt','sad','angry','tired','excited','missing')),
 intensity_milli INTEGER NOT NULL CHECK(intensity_milli BETWEEN 0 AND 1000),
 started_at_ms INTEGER NOT NULL CHECK(started_at_ms>=0), expires_at_ms INTEGER NOT NULL CHECK(expires_at_ms>=started_at_ms),
 lamport INTEGER NOT NULL CHECK(lamport>0),
 CHECK((source_message_id IS NOT NULL)+(activity_id IS NOT NULL)=1)
);
CREATE TABLE companion_proactive(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), source_life_event_id TEXT NOT NULL REFERENCES companion_life_events(id),
 host_device_id TEXT NOT NULL REFERENCES devices(id), content TEXT NOT NULL,
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0), delivered_at_ms INTEGER,
 state TEXT NOT NULL CHECK(state IN ('candidate','delivered','dismissed')), message_id TEXT REFERENCES messages(id),
 UNIQUE(persona_id,source_life_event_id)
);
-- Local work cursors are neither authoritative shared state nor cloud tasks.
CREATE TABLE companion_processed_sources(message_id TEXT PRIMARY KEY REFERENCES messages(id),processed_at_ms INTEGER NOT NULL);
CREATE TABLE companion_worker_status(id INTEGER PRIMARY KEY CHECK(id=1),error_code TEXT,failures INTEGER NOT NULL DEFAULT 0);
INSERT INTO companion_worker_status(id) VALUES(1);
INSERT INTO companion_processed_sources SELECT id,0 FROM messages WHERE role='user';
DROP TRIGGER IF EXISTS lan_api_usage_records_INSERT;
DROP TRIGGER IF EXISTS lan_api_usage_records_UPDATE;
DROP TRIGGER IF EXISTS lan_api_usage_records_DELETE;
ALTER TABLE api_usage_records ADD COLUMN purpose TEXT NOT NULL DEFAULT 'chat' CHECK(purpose IN ('chat','memory','life'));

CREATE TRIGGER companion_bond_scope BEFORE INSERT ON companion_bond_events BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id
 WHERE m.id=new.source_message_id AND m.role='user' AND m.origin_device_id=new.origin_device_id
 AND coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=c.persona_id),c.persona_id)
 =coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=new.persona_id),new.persona_id))
 THEN RAISE(ABORT,'relationship evidence scope mismatch') END;
END;
CREATE TRIGGER companion_candidate_scope BEFORE INSERT ON companion_memory_candidates BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id
 WHERE m.id=new.source_message_id AND m.role='user'
 AND coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=c.persona_id),c.persona_id)
 =coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=new.persona_id),new.persona_id))
 THEN RAISE(ABORT,'memory candidate scope mismatch') END;
END;
CREATE TRIGGER companion_activity_scope BEFORE INSERT ON companion_activities BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM companion_hosts WHERE persona_id=new.persona_id AND device_id=new.host_device_id)
 THEN RAISE(ABORT,'life host scope mismatch') END;
END;
CREATE TRIGGER companion_life_event_scope BEFORE INSERT ON companion_life_events BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM companion_activities WHERE id=new.activity_id AND persona_id=new.persona_id AND host_device_id=new.host_device_id)
 THEN RAISE(ABORT,'life evidence scope mismatch') END;
END;
CREATE TRIGGER companion_proactive_scope BEFORE INSERT ON companion_proactive BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM companion_life_events WHERE id=new.source_life_event_id AND persona_id=new.persona_id AND host_device_id=new.host_device_id)
 THEN RAISE(ABORT,'proactive evidence scope mismatch') END;
END;
CREATE TRIGGER companion_mood_scope BEFORE INSERT ON companion_mood_events BEGIN
 SELECT CASE WHEN new.activity_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM companion_activities WHERE id=new.activity_id AND persona_id=new.persona_id AND host_device_id=new.origin_device_id)
 THEN RAISE(ABORT,'mood activity scope mismatch') END;
 SELECT CASE WHEN new.source_message_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id
 WHERE m.id=new.source_message_id AND m.role='user' AND m.origin_device_id=new.origin_device_id
 AND coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=c.persona_id),c.persona_id)
 =coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=new.persona_id),new.persona_id))
 THEN RAISE(ABORT,'mood chat scope mismatch') END;
END;
