-- Add character ownership without rewriting original messages or sync envelopes.
CREATE TABLE personas(
 id TEXT PRIMARY KEY,
 profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 100),
 main_conversation_id TEXT NOT NULL UNIQUE REFERENCES conversations(id),
 is_builtin INTEGER NOT NULL DEFAULT 0 CHECK(is_builtin IN (0,1)),
 created_at_ms INTEGER NOT NULL DEFAULT 0 CHECK(created_at_ms>=0),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0),
 UNIQUE(id,profile_id)
);
CREATE UNIQUE INDEX personas_one_builtin ON personas(profile_id) WHERE is_builtin=1;
CREATE TRIGGER persona_main_scope BEFORE INSERT ON personas BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM conversations WHERE id=new.main_conversation_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona main profile mismatch') END;
END;
CREATE TRIGGER persona_identity_immutable BEFORE UPDATE OF id,profile_id,main_conversation_id,is_builtin ON personas
 WHEN new.id IS NOT old.id OR new.profile_id IS NOT old.profile_id OR new.main_conversation_id IS NOT old.main_conversation_id OR new.is_builtin IS NOT old.is_builtin BEGIN
 SELECT RAISE(ABORT,'persona identity is immutable');
END;
CREATE TABLE conversation_personas(
 conversation_id TEXT PRIMARY KEY REFERENCES conversations(id),
 persona_id TEXT NOT NULL REFERENCES personas(id),
 is_legacy INTEGER NOT NULL DEFAULT 0 CHECK(is_legacy IN (0,1))
);
CREATE INDEX conversations_persona ON conversation_personas(persona_id,conversation_id);
CREATE TRIGGER persona_thread_scope BEFORE INSERT ON conversation_personas BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM conversations WHERE id=new.conversation_id) IS NOT
 (SELECT profile_id FROM personas WHERE id=new.persona_id) THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER persona_thread_immutable BEFORE UPDATE ON conversation_personas BEGIN
 SELECT RAISE(ABORT,'conversation ownership is immutable');
END;
CREATE TABLE persona_chat_preferences(
 persona_id TEXT PRIMARY KEY REFERENCES personas(id),
 settings_json TEXT NOT NULL CHECK(json_valid(settings_json)),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0)
);
CREATE TABLE persona_states(
 persona_id TEXT PRIMARY KEY REFERENCES personas(id),
 relationship_json TEXT NOT NULL DEFAULT '{"status":"not_evaluated"}' CHECK(json_valid(relationship_json)),
 mood_json TEXT NOT NULL DEFAULT '{"status":"unknown"}' CHECK(json_valid(mood_json)),
 life_json TEXT NOT NULL DEFAULT '{"status":"not_started"}' CHECK(json_valid(life_json)),
 interests_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(interests_json)),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0)
);
CREATE TABLE persona_selection(
 device_id TEXT PRIMARY KEY REFERENCES devices(id),
 persona_id TEXT NOT NULL REFERENCES personas(id)
);
CREATE TRIGGER persona_selection_insert BEFORE INSERT ON persona_selection BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT
 (SELECT profile_id FROM devices WHERE id=new.device_id) THEN RAISE(ABORT,'selection profile mismatch') END;
END;
CREATE TRIGGER persona_selection_update BEFORE UPDATE ON persona_selection BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT
 (SELECT profile_id FROM devices WHERE id=new.device_id) THEN RAISE(ABORT,'selection profile mismatch') END;
END;
INSERT OR IGNORE INTO conversations(id,profile_id,title) SELECT id,id,'DeepSeek娘 · 永久聊天' FROM local_profiles;
INSERT INTO personas(id,profile_id,name,main_conversation_id,is_builtin,created_at_ms)
 SELECT p.id,p.id,CASE WHEN coalesce(json_extract(cp.settings_json,'$.character_name'),ch.name,'DeepSeek娘')='DeepSeek 娘'
 THEN 'DeepSeek娘' ELSE coalesce(json_extract(cp.settings_json,'$.character_name'),ch.name,'DeepSeek娘') END,p.id,1,p.created_at_ms
 FROM local_profiles p LEFT JOIN chat_preferences cp ON cp.profile_id=p.id LEFT JOIN character_profiles ch ON ch.profile_id=p.id;
INSERT INTO conversation_personas(conversation_id,persona_id,is_legacy)
 SELECT id,profile_id,CASE WHEN id=profile_id THEN 0 ELSE 1 END FROM conversations;
INSERT INTO persona_chat_preferences(persona_id,settings_json,revision)
 SELECT c.profile_id,json_set(c.settings_json,'$.character_name',p.name),c.revision
 FROM chat_preferences c JOIN personas p ON p.id=c.profile_id;
INSERT INTO persona_states(persona_id) SELECT id FROM personas;
UPDATE persona_states SET relationship_json=(SELECT json_object('status','legacy_prototype','stage',r.stage,
 'affection_milli',r.affection_milli,'trust_milli',r.trust_milli,'intimacy_milli',r.intimacy_milli,
 'comfort_milli',r.comfort_milli,'initiative_milli',r.initiative_milli,'rule_version',r.rule_version)
 FROM relationship_states r WHERE r.profile_id=persona_states.persona_id)
 WHERE EXISTS(SELECT 1 FROM relationship_states r WHERE r.profile_id=persona_states.persona_id);
INSERT INTO persona_selection(device_id,persona_id) SELECT id,profile_id FROM devices WHERE trust_state='local';

ALTER TABLE memories ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE memories SET persona_id=profile_id;
CREATE INDEX memories_persona_scope ON memories(persona_id);
CREATE TRIGGER memories_persona_insert BEFORE INSERT ON memories WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER memories_persona_update BEFORE UPDATE OF persona_id,profile_id ON memories BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE relationship_events ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE relationship_events SET persona_id=profile_id;
CREATE INDEX relationship_events_persona_scope ON relationship_events(persona_id);
CREATE TRIGGER relationship_events_persona_insert BEFORE INSERT ON relationship_events WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER relationship_events_persona_update BEFORE UPDATE OF persona_id,profile_id ON relationship_events BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE character_moods ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE character_moods SET persona_id=profile_id;
CREATE INDEX character_moods_persona_scope ON character_moods(persona_id);
CREATE TRIGGER character_moods_persona_insert BEFORE INSERT ON character_moods WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER character_moods_persona_update BEFORE UPDATE OF persona_id,profile_id ON character_moods BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE character_interests ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE character_interests SET persona_id=profile_id;
CREATE INDEX character_interests_persona_scope ON character_interests(persona_id);
CREATE TRIGGER character_interests_persona_insert BEFORE INSERT ON character_interests WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER character_interests_persona_update BEFORE UPDATE OF persona_id,profile_id ON character_interests BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE life_plans ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE life_plans SET persona_id=profile_id;
CREATE INDEX life_plans_persona_scope ON life_plans(persona_id);
CREATE TRIGGER life_plans_persona_insert BEFORE INSERT ON life_plans WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER life_plans_persona_update BEFORE UPDATE OF persona_id,profile_id ON life_plans BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE daily_activities ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE daily_activities SET persona_id=profile_id;
CREATE INDEX daily_activities_persona_scope ON daily_activities(persona_id);
CREATE TRIGGER daily_activities_persona_insert BEFORE INSERT ON daily_activities WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER daily_activities_persona_update BEFORE UPDATE OF persona_id,profile_id ON daily_activities BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE life_events ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE life_events SET persona_id=profile_id;
CREATE INDEX life_events_persona_scope ON life_events(persona_id);
CREATE TRIGGER life_events_persona_insert BEFORE INSERT ON life_events WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER life_events_persona_update BEFORE UPDATE OF persona_id,profile_id ON life_events BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE proactive_messages ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE proactive_messages SET persona_id=profile_id;
CREATE INDEX proactive_messages_persona_scope ON proactive_messages(persona_id);
CREATE TRIGGER proactive_messages_persona_insert BEFORE INSERT ON proactive_messages WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER proactive_messages_persona_update BEFORE UPDATE OF persona_id,profile_id ON proactive_messages BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE video_sources ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE video_sources SET persona_id=profile_id;
CREATE INDEX video_sources_persona_scope ON video_sources(persona_id);
CREATE TRIGGER video_sources_persona_insert BEFORE INSERT ON video_sources WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER video_sources_persona_update BEFORE UPDATE OF persona_id,profile_id ON video_sources BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE meme_knowledge ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE meme_knowledge SET persona_id=profile_id;
CREATE INDEX meme_knowledge_persona_scope ON meme_knowledge(persona_id);
CREATE TRIGGER meme_knowledge_persona_insert BEFORE INSERT ON meme_knowledge WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER meme_knowledge_persona_update BEFORE UPDATE OF persona_id,profile_id ON meme_knowledge BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE task_queue ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE task_queue SET persona_id=profile_id;
CREATE INDEX task_queue_persona_scope ON task_queue(persona_id);
CREATE TRIGGER task_queue_persona_insert BEFORE INSERT ON task_queue WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER task_queue_persona_update BEFORE UPDATE OF persona_id,profile_id ON task_queue BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE activity_states ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE activity_states SET persona_id=profile_id;
CREATE INDEX activity_states_persona_scope ON activity_states(persona_id);
CREATE TRIGGER activity_states_persona_insert BEFORE INSERT ON activity_states WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER activity_states_persona_update BEFORE UPDATE OF persona_id,profile_id ON activity_states BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

ALTER TABLE api_usage_records ADD COLUMN persona_id TEXT REFERENCES personas(id);
UPDATE api_usage_records SET persona_id=profile_id;
CREATE INDEX api_usage_records_persona_scope ON api_usage_records(persona_id);
CREATE TRIGGER api_usage_records_persona_insert BEFORE INSERT ON api_usage_records WHEN new.persona_id IS NOT NULL BEGIN
 SELECT CASE WHEN (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;
CREATE TRIGGER api_usage_records_persona_update BEFORE UPDATE OF persona_id,profile_id ON api_usage_records BEGIN
 SELECT CASE WHEN old.persona_id IS NOT NULL AND old.persona_id IS NOT new.persona_id THEN RAISE(ABORT,'persona ownership is immutable') END;
 SELECT CASE WHEN new.persona_id IS NOT NULL AND (SELECT profile_id FROM personas WHERE id=new.persona_id) IS NOT new.profile_id THEN RAISE(ABORT,'persona profile mismatch') END;
END;

CREATE TRIGGER memory_relation_persona_insert BEFORE INSERT ON memory_relations BEGIN
 SELECT CASE WHEN (SELECT persona_id FROM memories WHERE id=new.from_memory_id) IS NOT
 (SELECT persona_id FROM memories WHERE id=new.to_memory_id) THEN RAISE(ABORT,'memory persona mismatch') END;
END;

CREATE TRIGGER memory_relation_persona_update BEFORE UPDATE OF from_memory_id,to_memory_id ON memory_relations BEGIN
 SELECT CASE WHEN (SELECT persona_id FROM memories WHERE id=new.from_memory_id) IS NOT
 (SELECT persona_id FROM memories WHERE id=new.to_memory_id) THEN RAISE(ABORT,'memory persona mismatch') END;
END;

CREATE TRIGGER memories_default_persona AFTER INSERT ON memories WHEN new.persona_id IS NULL BEGIN
 UPDATE memories SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER relationship_events_default_persona AFTER INSERT ON relationship_events WHEN new.persona_id IS NULL BEGIN
 UPDATE relationship_events SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER character_moods_default_persona AFTER INSERT ON character_moods WHEN new.persona_id IS NULL BEGIN
 UPDATE character_moods SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER character_interests_default_persona AFTER INSERT ON character_interests WHEN new.persona_id IS NULL BEGIN
 UPDATE character_interests SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER life_plans_default_persona AFTER INSERT ON life_plans WHEN new.persona_id IS NULL BEGIN
 UPDATE life_plans SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER daily_activities_default_persona AFTER INSERT ON daily_activities WHEN new.persona_id IS NULL BEGIN
 UPDATE daily_activities SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER life_events_default_persona AFTER INSERT ON life_events WHEN new.persona_id IS NULL BEGIN
 UPDATE life_events SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER proactive_messages_default_persona AFTER INSERT ON proactive_messages WHEN new.persona_id IS NULL BEGIN
 UPDATE proactive_messages SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER video_sources_default_persona AFTER INSERT ON video_sources WHEN new.persona_id IS NULL BEGIN
 UPDATE video_sources SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER meme_knowledge_default_persona AFTER INSERT ON meme_knowledge WHEN new.persona_id IS NULL BEGIN
 UPDATE meme_knowledge SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER task_queue_default_persona AFTER INSERT ON task_queue WHEN new.persona_id IS NULL BEGIN
 UPDATE task_queue SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER activity_states_default_persona AFTER INSERT ON activity_states WHEN new.persona_id IS NULL BEGIN
 UPDATE activity_states SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;

CREATE TRIGGER api_usage_records_default_persona AFTER INSERT ON api_usage_records WHEN new.persona_id IS NULL BEGIN
 UPDATE api_usage_records SET persona_id=new.profile_id WHERE rowid=new.rowid;
END;
