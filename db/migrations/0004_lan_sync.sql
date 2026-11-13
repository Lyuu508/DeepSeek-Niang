-- Production v2 wire log is independent of the immutable stage-0 lab stream.
CREATE TABLE lan_control(id INTEGER PRIMARY KEY CHECK(id=1), suppress INTEGER NOT NULL DEFAULT 0, enabled INTEGER NOT NULL DEFAULT 0, initialized INTEGER NOT NULL DEFAULT 0, seq INTEGER NOT NULL DEFAULT 0, clock INTEGER NOT NULL DEFAULT 0);
INSERT INTO lan_control(id) VALUES(1);
CREATE TABLE lan_outbox(id INTEGER PRIMARY KEY AUTOINCREMENT, table_name TEXT NOT NULL, before_json TEXT, after_json TEXT);
CREATE TABLE lan_events(id TEXT PRIMARY KEY, origin TEXT NOT NULL, seq INTEGER NOT NULL CHECK(seq>0), clock INTEGER NOT NULL CHECK(clock>0), wire TEXT NOT NULL, UNIQUE(origin,seq));
CREATE TABLE lan_cells(table_name TEXT NOT NULL, row_key TEXT NOT NULL, field TEXT NOT NULL, value_json TEXT NOT NULL, clock INTEGER NOT NULL, origin TEXT NOT NULL, event_id TEXT NOT NULL, PRIMARY KEY(table_name,row_key,field));
CREATE TABLE lan_conflicts(table_name TEXT NOT NULL, row_key TEXT NOT NULL, field TEXT NOT NULL, event_id TEXT NOT NULL, value_json TEXT NOT NULL, PRIMARY KEY(table_name,row_key,field,event_id));
CREATE TABLE lan_deleted(table_name TEXT NOT NULL, row_key TEXT NOT NULL, PRIMARY KEY(table_name,row_key));
CREATE TABLE lan_peers(id TEXT PRIMARY KEY, name TEXT NOT NULL, certificate TEXT NOT NULL, signing_key TEXT NOT NULL, revoked INTEGER NOT NULL DEFAULT 0, address TEXT, last_success_ms INTEGER, acknowledged_seq INTEGER NOT NULL DEFAULT 0);
CREATE TABLE lan_profiles(profile_id TEXT PRIMARY KEY REFERENCES local_profiles(id));
-- Local selection stays local. Imported profiles retain their original UUID and ownership.
DROP TRIGGER persona_selection_insert;
DROP TRIGGER persona_selection_update;
CREATE TRIGGER persona_selection_insert BEFORE INSERT ON persona_selection BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT (SELECT profile_id FROM devices WHERE id=new.device_id)
 AND NOT EXISTS(SELECT 1 FROM lan_profiles WHERE profile_id=(SELECT profile_id FROM personas WHERE id=new.persona_id)) THEN RAISE(ABORT,'selection profile mismatch') END;
END;
CREATE TRIGGER persona_selection_update BEFORE UPDATE ON persona_selection BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT (SELECT profile_id FROM devices WHERE id=new.device_id)
 AND NOT EXISTS(SELECT 1 FROM lan_profiles WHERE profile_id=(SELECT profile_id FROM personas WHERE id=new.persona_id)) THEN RAISE(ABORT,'selection profile mismatch') END;
END;

-- Preserve rows while extending prototype natural keys to include persona ownership.
-- Child references retain the original final table names; constraints are checked before commit.
-- Store::connect temporarily disables FK actions on its private connection;
-- foreign_key_check runs before commit and enforcement is restored before use.
CREATE TABLE lan_migrate_relationship_events(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 evidence_hash TEXT NOT NULL, rule_version INTEGER NOT NULL CHECK(rule_version>0),
 kind TEXT NOT NULL, evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),
 occurred_at_ms INTEGER NOT NULL, lamport INTEGER NOT NULL CHECK(lamport>0),
 origin_device_id TEXT NOT NULL REFERENCES devices(id),
 source_event_id TEXT REFERENCES sync_events(event_id),
 persona_id TEXT REFERENCES personas(id),
 UNIQUE(profile_id,persona_id,evidence_hash,rule_version)
);
INSERT INTO lan_migrate_relationship_events SELECT * FROM relationship_events;
DROP TABLE relationship_events;
ALTER TABLE lan_migrate_relationship_events RENAME TO relationship_events;
CREATE INDEX relationship_events_persona_scope ON relationship_events(persona_id);
CREATE TRIGGER relationship_events_persona_insert BEFORE INSERT ON relationship_events WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER relationship_events_persona_update BEFORE UPDATE OF persona_id,profile_id ON relationship_events BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TABLE lan_migrate_character_interests(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), topic TEXT NOT NULL,
 weight_milli INTEGER NOT NULL CHECK(weight_milli BETWEEN 0 AND 1000),
 evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)), revision INTEGER NOT NULL DEFAULT 1,
 persona_id TEXT REFERENCES personas(id),
 UNIQUE(profile_id,persona_id,topic)
);
INSERT INTO lan_migrate_character_interests SELECT * FROM character_interests;
DROP TABLE character_interests;
ALTER TABLE lan_migrate_character_interests RENAME TO character_interests;
CREATE INDEX character_interests_persona_scope ON character_interests(persona_id);
CREATE TRIGGER character_interests_persona_insert BEFORE INSERT ON character_interests WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER character_interests_persona_update BEFORE UPDATE OF persona_id,profile_id ON character_interests BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TABLE lan_migrate_task_queue(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 kind TEXT NOT NULL, dedupe_key TEXT NOT NULL,
 authority_device_id TEXT NOT NULL REFERENCES devices(id),
 state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','paused','completed','cancelled','failed','uncertain')),
 priority INTEGER NOT NULL DEFAULT 0 CHECK(priority BETWEEN 0 AND 100),
 payload_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(payload_json)),
 owner_device_id TEXT REFERENCES devices(id), fence INTEGER NOT NULL DEFAULT 0 CHECK(fence>=0),
 lease_boot_id TEXT, lease_until_tick INTEGER, attempt INTEGER NOT NULL DEFAULT 0 CHECK(attempt>=0),
 max_attempts INTEGER NOT NULL DEFAULT 3 CHECK(max_attempts BETWEEN 1 AND 10),
 last_error TEXT, persona_id TEXT REFERENCES personas(id),
 UNIQUE(profile_id,persona_id,dedupe_key)
);
INSERT INTO lan_migrate_task_queue SELECT * FROM task_queue;
DROP TABLE task_queue;
ALTER TABLE lan_migrate_task_queue RENAME TO task_queue;
CREATE INDEX task_queue_persona_scope ON task_queue(persona_id);
CREATE TRIGGER task_queue_persona_insert BEFORE INSERT ON task_queue WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER task_queue_persona_update BEFORE UPDATE OF persona_id,profile_id ON task_queue BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TABLE lan_migrate_life_events(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id), activity_id TEXT REFERENCES daily_activities(id),
 occurrence_key TEXT NOT NULL, event_json TEXT NOT NULL CHECK(json_valid(event_json)),
 source_event_id TEXT REFERENCES sync_events(event_id), persona_id TEXT REFERENCES personas(id),
 UNIQUE(profile_id,persona_id,occurrence_key)
);
INSERT INTO lan_migrate_life_events SELECT * FROM life_events;
DROP TABLE life_events;
ALTER TABLE lan_migrate_life_events RENAME TO life_events;
CREATE INDEX life_events_persona_scope ON life_events(persona_id);
CREATE TRIGGER life_events_persona_insert BEFORE INSERT ON life_events WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER life_events_persona_update BEFORE UPDATE OF persona_id,profile_id ON life_events BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TABLE lan_migrate_proactive_messages(
 id TEXT PRIMARY KEY, profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 trigger_key TEXT NOT NULL, evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),
 content TEXT NOT NULL, state TEXT NOT NULL CHECK(state IN ('candidate','approved','delivered','dismissed')),
 source_event_id TEXT REFERENCES sync_events(event_id), persona_id TEXT REFERENCES personas(id),
 UNIQUE(profile_id,persona_id,trigger_key)
);
INSERT INTO lan_migrate_proactive_messages SELECT * FROM proactive_messages;
DROP TABLE proactive_messages;
ALTER TABLE lan_migrate_proactive_messages RENAME TO proactive_messages;
CREATE INDEX proactive_messages_persona_scope ON proactive_messages(persona_id);
CREATE TRIGGER proactive_messages_persona_insert BEFORE INSERT ON proactive_messages WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER proactive_messages_persona_update BEFORE UPDATE OF persona_id,profile_id ON proactive_messages BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE INDEX relationship_replay ON relationship_events(profile_id,lamport,origin_device_id,id);
CREATE INDEX task_schedule ON task_queue(authority_device_id,state,priority DESC);
UPDATE relationship_events SET persona_id=profile_id WHERE persona_id IS NULL;
UPDATE character_interests SET persona_id=profile_id WHERE persona_id IS NULL;
UPDATE task_queue SET persona_id=profile_id WHERE persona_id IS NULL;
UPDATE life_events SET persona_id=profile_id WHERE persona_id IS NULL;
UPDATE proactive_messages SET persona_id=profile_id WHERE persona_id IS NULL;
