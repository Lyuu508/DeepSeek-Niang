-- Source messages and the original memory rows keep their identity and event history.
CREATE TABLE companion_memory_context(
 memory_id TEXT PRIMARY KEY REFERENCES memories(id),
 persona_id TEXT NOT NULL REFERENCES personas(id),
 is_event INTEGER NOT NULL DEFAULT 0 CHECK(is_event IN (0,1)),
 shared_story INTEGER NOT NULL DEFAULT 0 CHECK(shared_story IN (0,1)),
 state TEXT NOT NULL DEFAULT 'active' CHECK(state IN ('active','completed','superseded','dismissed')),
 occurred_at_ms INTEGER NOT NULL CHECK(occurred_at_ms>=0),
 due_at_ms INTEGER CHECK(due_at_ms>=0),
 confidence_milli INTEGER NOT NULL DEFAULT 1000 CHECK(confidence_milli BETWEEN 0 AND 1000),
 recall_allowed INTEGER NOT NULL DEFAULT 0 CHECK(recall_allowed IN (0,1)),
 reminder_allowed INTEGER NOT NULL DEFAULT 0 CHECK(reminder_allowed IN (0,1)),
 shared_with_personas INTEGER NOT NULL DEFAULT 0 CHECK(shared_with_personas IN (0,1)),
 CHECK(is_event=1 OR (due_at_ms IS NULL AND reminder_allowed=0)),
 CHECK(reminder_allowed=0 OR recall_allowed=1)
);
CREATE INDEX companion_memory_context_recall ON companion_memory_context(persona_id,recall_allowed,state,due_at_ms,occurred_at_ms);
CREATE INDEX companion_memory_topic ON companion_memory_meta(persona_id,topic_key);
CREATE INDEX companion_memory_source ON companion_memory_meta(source_message_id);
CREATE INDEX companion_chat_prompt_replies ON messages(reply_to_id) WHERE is_deleted=0 AND role='assistant';
CREATE TRIGGER companion_memory_context_owner BEFORE INSERT ON companion_memory_context BEGIN
 SELECT CASE WHEN new.persona_id IS NOT (SELECT persona_id FROM memories WHERE id=new.memory_id)
 THEN RAISE(ABORT,'memory context persona mismatch') END;
END;
CREATE TRIGGER companion_memory_context_owner_update BEFORE UPDATE OF memory_id,persona_id ON companion_memory_context BEGIN
 SELECT CASE WHEN new.persona_id IS NOT (SELECT persona_id FROM memories WHERE id=new.memory_id)
 THEN RAISE(ABORT,'memory context persona mismatch') END;
END;
CREATE TABLE companion_memory_usage(
 memory_id TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
 last_used_at_ms INTEGER NOT NULL CHECK(last_used_at_ms>=0)
);
CREATE TABLE companion_device_clock(
 id INTEGER PRIMARY KEY CHECK(id=1),
 offset_minutes INTEGER NOT NULL CHECK(offset_minutes BETWEEN -840 AND 840),
 zone_name TEXT NOT NULL CHECK(length(zone_name) BETWEEN 1 AND 80)
);

CREATE TABLE companion_interest_overrides(
 interest_id TEXT PRIMARY KEY REFERENCES character_interests(id),
 persona_id TEXT NOT NULL REFERENCES personas(id),
 weight_milli INTEGER NOT NULL CHECK(weight_milli BETWEEN 0 AND 1000)
);
CREATE TRIGGER companion_interest_override_owner BEFORE INSERT ON companion_interest_overrides BEGIN
 SELECT CASE WHEN new.persona_id IS NOT (SELECT persona_id FROM character_interests WHERE id=new.interest_id)
 THEN RAISE(ABORT,'role interest persona mismatch') END;
END;
CREATE TRIGGER companion_interest_override_owner_update BEFORE UPDATE OF persona_id,interest_id ON companion_interest_overrides BEGIN
 SELECT CASE WHEN new.persona_id IS NOT (SELECT persona_id FROM character_interests WHERE id=new.interest_id)
 THEN RAISE(ABORT,'role interest persona mismatch') END;
END;

-- Extend the existing share ledger so recall uses the same notifications, quotas and message delivery.
DROP TRIGGER IF EXISTS lan_companion_proactive_INSERT;
DROP TRIGGER IF EXISTS lan_companion_proactive_UPDATE;
DROP TRIGGER IF EXISTS lan_companion_proactive_DELETE;
DROP TRIGGER IF EXISTS companion_proactive_scope;
CREATE TABLE companion_proactive_v11(
 id TEXT PRIMARY KEY, persona_id TEXT NOT NULL REFERENCES personas(id),
 source_life_event_id TEXT REFERENCES companion_life_events(id),
 host_device_id TEXT NOT NULL REFERENCES devices(id), content TEXT NOT NULL,
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0), delivered_at_ms INTEGER,
 state TEXT NOT NULL CHECK(state IN ('candidate','delivered','dismissed')), message_id TEXT REFERENCES messages(id),
 source_memory_id TEXT REFERENCES memories(id), source_memory_revision TEXT REFERENCES sync_events(event_id),
 intent TEXT CHECK(intent IN ('recall','follow_up','continue_topic','reminder','share_interest','share_life')),
 CHECK((source_life_event_id IS NOT NULL)+(source_memory_id IS NOT NULL)=1),
 CHECK((source_memory_id IS NOT NULL)=(source_memory_revision IS NOT NULL)),
 UNIQUE(persona_id,source_life_event_id), UNIQUE(persona_id,source_memory_id)
);
INSERT INTO companion_proactive_v11(id,persona_id,source_life_event_id,host_device_id,content,created_at_ms,delivered_at_ms,state,message_id)
 SELECT id,persona_id,source_life_event_id,host_device_id,content,created_at_ms,delivered_at_ms,state,message_id FROM companion_proactive;
DROP TABLE companion_proactive;
ALTER TABLE companion_proactive_v11 RENAME TO companion_proactive;
CREATE INDEX companion_share_ready ON companion_proactive(persona_id,host_device_id,state,created_at_ms);
CREATE TRIGGER companion_proactive_scope BEFORE INSERT ON companion_proactive BEGIN
 SELECT CASE WHEN new.source_life_event_id IS NOT NULL AND NOT EXISTS(
 SELECT 1 FROM companion_life_events WHERE id=new.source_life_event_id AND persona_id=new.persona_id AND host_device_id=new.host_device_id)
 THEN RAISE(ABORT,'proactive life evidence scope mismatch') END;
 SELECT CASE WHEN new.source_memory_id IS NOT NULL AND NOT EXISTS(
 SELECT 1 FROM memories m WHERE m.id=new.source_memory_id
 AND coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=m.persona_id),m.persona_id)
 =coalesce((SELECT canonical_id FROM persona_links WHERE alias_id=new.persona_id),new.persona_id))
 THEN RAISE(ABORT,'proactive memory evidence scope mismatch') END;
END;
