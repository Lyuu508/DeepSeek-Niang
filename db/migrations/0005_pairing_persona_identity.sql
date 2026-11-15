-- Pairing binds devices to an existing host identity. Original rows and IDs stay intact.
DROP TRIGGER IF EXISTS lan_personas_INSERT;
DROP TRIGGER IF EXISTS lan_personas_UPDATE;
DROP TRIGGER IF EXISTS lan_personas_DELETE;
ALTER TABLE personas ADD COLUMN avatar_resource TEXT NOT NULL DEFAULT 'deepseek-niang'
 CHECK(avatar_resource='deepseek-niang');
CREATE TABLE lan_host_binding(
 id INTEGER PRIMARY KEY CHECK(id=1),
 host_device_id TEXT NOT NULL REFERENCES lan_peers(id),
 profile_id TEXT NOT NULL,
 persona_id TEXT NOT NULL,
 paired_at_ms INTEGER NOT NULL CHECK(paired_at_ms>=0)
);
CREATE TABLE persona_links(
 alias_id TEXT PRIMARY KEY REFERENCES personas(id),
 canonical_id TEXT NOT NULL REFERENCES personas(id),
 host_device_id TEXT NOT NULL REFERENCES devices(id),
 source_device_id TEXT NOT NULL REFERENCES devices(id),
 reason TEXT NOT NULL CHECK(reason='paired_builtin'),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0),
 CHECK(alias_id<>canonical_id)
);
CREATE INDEX persona_links_target ON persona_links(canonical_id);
CREATE TRIGGER persona_link_scope BEFORE INSERT ON persona_links BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM personas a JOIN personas c
 JOIN devices s ON s.id=new.source_device_id JOIN devices h ON h.id=new.host_device_id
 WHERE a.id=new.alias_id AND c.id=new.canonical_id AND a.is_builtin=1 AND c.is_builtin=1
 AND a.profile_id=s.profile_id AND c.profile_id=h.profile_id AND a.profile_id<>c.profile_id)
 OR EXISTS(SELECT 1 FROM persona_links WHERE alias_id=new.canonical_id OR canonical_id=new.alias_id)
 THEN RAISE(ABORT,'invalid paired builtin provenance') END;
END;
CREATE TRIGGER persona_link_immutable BEFORE UPDATE ON persona_links
 WHEN new.alias_id IS NOT old.alias_id OR new.canonical_id IS NOT old.canonical_id
 OR new.host_device_id IS NOT old.host_device_id OR new.source_device_id IS NOT old.source_device_id
 OR new.reason IS NOT old.reason OR new.created_at_ms IS NOT old.created_at_ms BEGIN
 SELECT RAISE(ABORT,'pairing identity link is immutable');
END;
