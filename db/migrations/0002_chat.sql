-- Preserve every v1 event, checkpoint, tombstone, task and video table.
ALTER TABLE conversations ADD COLUMN created_at_ms INTEGER CHECK(created_at_ms>=0);
ALTER TABLE conversations ADD COLUMN updated_at_ms INTEGER CHECK(updated_at_ms>=0);
ALTER TABLE messages ADD COLUMN created_at_ms INTEGER CHECK(created_at_ms>=0);
ALTER TABLE messages ADD COLUMN model_id TEXT;
ALTER TABLE messages ADD COLUMN request_id TEXT;
CREATE INDEX messages_request ON messages(request_id);
CREATE TABLE chat_requests(
 id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id),
 user_message_id TEXT NOT NULL REFERENCES messages(id), assistant_message_id TEXT NOT NULL UNIQUE,
 input_hash TEXT NOT NULL, mode TEXT NOT NULL CHECK(mode IN ('send','regenerate')),
 state TEXT NOT NULL CHECK(state IN ('sending','generating','complete','interrupted','failed')),
 model_id TEXT NOT NULL, created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0),
 finished_at_ms INTEGER, finish_reason TEXT, error_code TEXT
);
CREATE TABLE chat_generations(
 id TEXT PRIMARY KEY, request_id TEXT NOT NULL UNIQUE REFERENCES chat_requests(id),
 user_message_id TEXT NOT NULL REFERENCES messages(id),
 content TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL CHECK(state IN ('sending','generating','complete','interrupted','failed')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 supersedes_generation_id TEXT REFERENCES chat_generations(id),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0), finished_at_ms INTEGER,
 model_id TEXT NOT NULL, finish_reason TEXT, error_code TEXT
);
CREATE INDEX generations_turn ON chat_generations(user_message_id,created_at_ms,id);
CREATE INDEX requests_running ON chat_requests(state,conversation_id);
CREATE TABLE chat_turn_choices(
 user_message_id TEXT PRIMARY KEY REFERENCES messages(id),
 selected_generation_id TEXT NOT NULL REFERENCES chat_generations(id)
);
CREATE TABLE chat_preferences(
 profile_id TEXT PRIMARY KEY REFERENCES local_profiles(id),
 settings_json TEXT NOT NULL CHECK(json_valid(settings_json)),
 revision INTEGER NOT NULL DEFAULT 1 CHECK(revision>0)
);
