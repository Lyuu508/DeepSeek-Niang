-- Lifecycle outlives the persona row. Keep only immutable identity and state here.
CREATE TABLE persona_lifecycle(
 persona_id TEXT PRIMARY KEY,
 profile_id TEXT NOT NULL REFERENCES local_profiles(id),
 main_conversation_id TEXT NOT NULL UNIQUE,
 decision_id TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('active','trashed','permanently_deleted')),
 changed_at_ms INTEGER NOT NULL CHECK(changed_at_ms>=0),
 CHECK(persona_id<>profile_id)
);
CREATE INDEX persona_lifecycle_state ON persona_lifecycle(state,changed_at_ms);
CREATE TABLE persona_erased_keys(
 table_name TEXT NOT NULL,
 row_key TEXT NOT NULL,
 persona_id TEXT NOT NULL REFERENCES persona_lifecycle(persona_id),
 PRIMARY KEY(table_name,row_key)
);
CREATE INDEX persona_erased_owner ON persona_erased_keys(persona_id);
-- Local mask work retains sequence and the digest required to recognize duplicate events.
CREATE TABLE lan_erasure_queue(
 event_id TEXT PRIMARY KEY REFERENCES lan_events(id),
 persona_id TEXT NOT NULL REFERENCES persona_lifecycle(persona_id),
 wire_sha256 TEXT NOT NULL CHECK(length(wire_sha256)=64)
);
-- Declarative extensions and their grants are local to this installation.
CREATE TABLE app_plugins(
 id TEXT PRIMARY KEY,
 manifest_json TEXT NOT NULL CHECK(json_valid(manifest_json)),
 source TEXT NOT NULL,
 granted_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(granted_json)),
 global_enabled INTEGER NOT NULL DEFAULT 0 CHECK(global_enabled IN (0,1)),
 config_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(config_json)),
 installed_at_ms INTEGER NOT NULL CHECK(installed_at_ms>=0)
);
CREATE TABLE app_plugin_personas(
 plugin_id TEXT NOT NULL REFERENCES app_plugins(id) ON DELETE CASCADE,
 persona_id TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
 enabled INTEGER CHECK(enabled IN (0,1)),
 config_json TEXT CHECK(config_json IS NULL OR json_valid(config_json)),
 PRIMARY KEY(plugin_id,persona_id)
);
CREATE TABLE app_plugin_reminders(
 id TEXT PRIMARY KEY,
 plugin_id TEXT NOT NULL REFERENCES app_plugins(id) ON DELETE CASCADE,
 persona_id TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
 label TEXT NOT NULL,
 due_at_ms INTEGER NOT NULL,
 delivered_at_ms INTEGER
);
CREATE INDEX app_plugin_due ON app_plugin_reminders(persona_id,delivered_at_ms,due_at_ms);
CREATE TABLE app_plugin_audit(
 seq INTEGER PRIMARY KEY,
 plugin_id TEXT NOT NULL,
 persona_id TEXT REFERENCES personas(id) ON DELETE CASCADE,
 checksum TEXT NOT NULL,
 action TEXT NOT NULL,
 at_ms INTEGER NOT NULL
);
