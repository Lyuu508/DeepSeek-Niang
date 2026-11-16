-- Stage 4: extend the existing life ledger, queue and plans. Original migrations stay immutable.
DROP TRIGGER IF EXISTS lan_companion_activities_INSERT;
DROP TRIGGER IF EXISTS lan_companion_activities_UPDATE;
DROP TRIGGER IF EXISTS lan_companion_activities_DELETE;
DROP TRIGGER IF EXISTS lan_companion_life_events_INSERT;
DROP TRIGGER IF EXISTS lan_companion_life_events_UPDATE;
DROP TRIGGER IF EXISTS lan_companion_life_events_DELETE;
DROP TRIGGER IF EXISTS lan_companion_memory_meta_INSERT;
DROP TRIGGER IF EXISTS lan_companion_memory_meta_UPDATE;
DROP TRIGGER IF EXISTS lan_companion_memory_meta_DELETE;
DROP TRIGGER IF EXISTS lan_life_plans_INSERT;
DROP TRIGGER IF EXISTS lan_life_plans_UPDATE;
DROP TRIGGER IF EXISTS lan_life_plans_DELETE;
DROP TRIGGER IF EXISTS lan_character_interests_INSERT;
DROP TRIGGER IF EXISTS lan_character_interests_UPDATE;
DROP TRIGGER IF EXISTS lan_character_interests_DELETE;
DROP TRIGGER IF EXISTS companion_activity_scope;
DROP TRIGGER IF EXISTS companion_life_event_scope;
DROP TRIGGER IF EXISTS companion_proactive_scope;
DROP TRIGGER IF EXISTS companion_mood_scope;
DROP TRIGGER IF EXISTS companion_memory_owner;
CREATE TABLE companion_activities_v8(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), host_device_id TEXT NOT NULL REFERENCES devices(id),
 kind TEXT NOT NULL CHECK(kind IN ('rest','study','video','music','organize','recall','wait','share','wake','read','interest','create','journal')),
 execution_kind TEXT NOT NULL DEFAULT 'virtual' CHECK(execution_kind='virtual'),
 state TEXT NOT NULL CHECK(state IN ('queued','running','paused','completed','cancelled','failed','uncertain')),
 started_at_ms INTEGER NOT NULL CHECK(started_at_ms>=0), scheduled_end_at_ms INTEGER NOT NULL CHECK(scheduled_end_at_ms>started_at_ms),
 ended_at_ms INTEGER CHECK(ended_at_ms>=started_at_ms), updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=started_at_ms),
 task_id TEXT REFERENCES task_queue(id), plan_id TEXT REFERENCES life_plans(id),
 elapsed_ms INTEGER NOT NULL DEFAULT 0 CHECK(elapsed_ms>=0),
 duration_ms INTEGER NOT NULL DEFAULT 1200000 CHECK(duration_ms BETWEEN 30000 AND 7200000),
 progress_milli INTEGER NOT NULL DEFAULT 0 CHECK(progress_milli BETWEEN 0 AND 1000),
 checkpoint_seq INTEGER NOT NULL DEFAULT 0 CHECK(checkpoint_seq>=0)
);
INSERT INTO companion_activities_v8(id,persona_id,host_device_id,kind,execution_kind,state,started_at_ms,scheduled_end_at_ms,ended_at_ms,updated_at_ms) SELECT id,persona_id,host_device_id,kind,execution_kind,state,started_at_ms,scheduled_end_at_ms,ended_at_ms,updated_at_ms FROM companion_activities;
DROP TABLE companion_activities;
ALTER TABLE companion_activities_v8 RENAME TO companion_activities;
CREATE TABLE companion_life_events_v8(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id), activity_id TEXT NOT NULL REFERENCES companion_activities(id),
 host_device_id TEXT NOT NULL REFERENCES devices(id), kind TEXT NOT NULL CHECK(kind IN ('started','completed','paused','resumed','cancelled','failed','recovered')),
 description TEXT NOT NULL, occurred_at_ms INTEGER NOT NULL CHECK(occurred_at_ms>=0), sequence INTEGER NOT NULL DEFAULT 0 CHECK(sequence>=0), UNIQUE(activity_id,kind,sequence)
);
INSERT INTO companion_life_events_v8(id,persona_id,activity_id,host_device_id,kind,description,occurred_at_ms) SELECT id,persona_id,activity_id,host_device_id,kind,description,occurred_at_ms FROM companion_life_events;
DROP TABLE companion_life_events;
ALTER TABLE companion_life_events_v8 RENAME TO companion_life_events;
CREATE TABLE companion_memory_meta_v8(
 memory_id TEXT PRIMARY KEY REFERENCES memories(id), persona_id TEXT NOT NULL REFERENCES personas(id),
 kind TEXT NOT NULL CHECK(kind IN ('episodic','semantic','relationship','knowledge')),
 source_message_id TEXT REFERENCES messages(id), source_kind TEXT NOT NULL CHECK(source_kind IN ('manual','chat','legacy','life')),
 topic_key TEXT NOT NULL DEFAULT '', pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0,1)),
 owner_locked INTEGER NOT NULL DEFAULT 1 CHECK(owner_locked IN (0,1)),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0), updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=created_at_ms),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
 source_life_event_id TEXT REFERENCES companion_life_events(id)
);
INSERT INTO companion_memory_meta_v8(memory_id,persona_id,kind,source_message_id,source_kind,topic_key,pinned,owner_locked,created_at_ms,updated_at_ms,revision) SELECT memory_id,persona_id,kind,source_message_id,source_kind,topic_key,pinned,owner_locked,created_at_ms,updated_at_ms,revision FROM companion_memory_meta;
DROP TABLE companion_memory_meta;
ALTER TABLE companion_memory_meta_v8 RENAME TO companion_memory_meta;
CREATE UNIQUE INDEX companion_one_activity ON companion_activities(persona_id) WHERE state='running';
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
CREATE TRIGGER companion_memory_owner BEFORE INSERT ON companion_memory_meta BEGIN
 SELECT CASE WHEN new.persona_id IS NOT (SELECT persona_id FROM memories WHERE id=new.memory_id)
 THEN RAISE(ABORT,'memory metadata persona mismatch') END;
END;
ALTER TABLE task_queue ADD COLUMN authority_epoch INTEGER NOT NULL DEFAULT 1 CHECK(authority_epoch>0);
ALTER TABLE life_plans ADD COLUMN host_device_id TEXT REFERENCES devices(id);
ALTER TABLE life_plans ADD COLUMN state TEXT NOT NULL DEFAULT 'active' CHECK(state IN ('active','paused','completed','cancelled'));
ALTER TABLE life_plans ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0 CHECK(created_at_ms>=0);
ALTER TABLE life_plans ADD COLUMN next_due_at_ms INTEGER NOT NULL DEFAULT 0 CHECK(next_due_at_ms>=0);
CREATE TABLE companion_receipts(
 proactive_id TEXT NOT NULL REFERENCES companion_proactive(id), device_id TEXT NOT NULL REFERENCES devices(id),
 read_at_ms INTEGER CHECK(read_at_ms>=0), notification_state TEXT NOT NULL DEFAULT 'pending'
 CHECK(notification_state IN ('pending','sending','shown','disabled','failed')),
 updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=0), PRIMARY KEY(proactive_id,device_id)
);
CREATE INDEX companion_activity_tasks ON companion_activities(task_id);
CREATE INDEX companion_life_day ON companion_life_events(persona_id,occurred_at_ms);
CREATE TRIGGER companion_task_scope BEFORE INSERT ON companion_activities WHEN new.task_id IS NOT NULL BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM task_queue WHERE id=new.task_id AND persona_id=new.persona_id AND authority_device_id=new.host_device_id AND kind='companion_life')
 THEN RAISE(ABORT,'activity task scope mismatch') END;
END;
CREATE TRIGGER companion_plan_scope BEFORE INSERT ON companion_activities WHEN new.plan_id IS NOT NULL BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM life_plans WHERE id=new.plan_id AND persona_id=new.persona_id AND host_device_id=new.host_device_id)
 THEN RAISE(ABORT,'activity plan scope mismatch') END;
END;

ALTER TABLE budget_reservations ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0 CHECK(created_at_ms>=0);

-- Local UI presence is telemetry, never a shared task ownership claim.
CREATE TABLE companion_presence(id INTEGER PRIMARY KEY CHECK(id=1),focused INTEGER NOT NULL CHECK(focused IN (0,1)),updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=0));
