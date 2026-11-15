#!/usr/bin/env python3
"""Independent SQLite DDL/FK/constraint/FTS smoke validation, entirely in memory."""
import json
import sqlite3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
required = set("""local_profiles devices conversations messages character_profiles relationship_states
relationship_events character_moods memories memory_relations character_interests daily_activities
life_events life_plans proactive_messages proactive_settings video_tasks video_sources video_progress
video_timeline_events video_analysis_results video_memories meme_knowledge media_interests
external_accounts application_settings task_queue task_checkpoints activity_states sync_events
sync_checkpoints data_tombstones api_usage_records""".split())
db = sqlite3.connect(":memory:")
db.execute("PRAGMA foreign_keys=ON")
for migration in sorted((root / "db/migrations").glob("*.sql")):
    db.executescript(migration.read_text(encoding="utf-8"))
tables = {r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
assert required <= tables, required - tables
chat_tables = {"chat_requests", "chat_generations", "chat_turn_choices", "chat_preferences"}
assert chat_tables <= tables
persona_tables = {"personas", "conversation_personas", "persona_chat_preferences", "persona_states", "persona_selection"}
assert persona_tables <= tables
companion_tables=set('persona_links lan_host_binding companion_preferences companion_bond_events companion_mood_events companion_memory_meta companion_memory_candidates companion_hosts companion_activities companion_life_events companion_proactive companion_processed_sources companion_worker_status'.split())
assert companion_tables <= tables

db.execute("INSERT INTO local_profiles(id,display_name) VALUES('lab','schema fixture')")
try:
    db.execute("INSERT INTO devices(id,profile_id,name,trust_state) VALUES('bad','missing','x','trusted')")
except sqlite3.IntegrityError:
    pass
else:
    raise AssertionError("foreign key not enforced")
try:
    db.execute("INSERT INTO relationship_states(profile_id,affection_milli,source_digest) VALUES('lab',100001,'x')")
except sqlite3.IntegrityError:
    pass
else:
    raise AssertionError("relationship bounds not enforced")
db.execute("INSERT INTO local_profiles(id,display_name) VALUES('other','independent fixture')")
db.execute("INSERT INTO devices(id,profile_id,name,trust_state) VALUES('device','lab','fixture','local')")
for persona, profile in [('a', 'lab'), ('b', 'lab'), ('c', 'other')]:
    db.execute("INSERT INTO conversations(id,profile_id,title) VALUES(?,?,'fixture')", (persona, profile))
    db.execute("INSERT INTO personas(id,profile_id,name,main_conversation_id) VALUES(?,?,?,?)", (persona, profile, persona, persona))
    db.execute("INSERT INTO conversation_personas(conversation_id,persona_id) VALUES(?,?)", (persona, persona))


def rejects(sql, parameters=()):
    try:
        db.execute(sql, parameters)
    except sqlite3.IntegrityError:
        return
    raise AssertionError("ownership constraint was bypassed: " + sql)


db.execute("INSERT INTO conversations(id,profile_id,title) VALUES('unmapped-other','other','fixture')")
rejects("INSERT INTO conversation_personas(conversation_id,persona_id) VALUES('unmapped-other','a')")
rejects("INSERT INTO personas(id,profile_id,name,main_conversation_id) VALUES('wrong','lab','wrong','unmapped-other')")
rejects("INSERT INTO persona_selection(device_id,persona_id) VALUES('device','c')")
db.execute("INSERT INTO persona_selection(device_id,persona_id) VALUES('device','a')")
rejects("UPDATE persona_selection SET persona_id='c'")
rejects("UPDATE personas SET main_conversation_id='b' WHERE id='a'")
rejects("UPDATE personas SET profile_id='other' WHERE id='a'")
rejects("UPDATE conversation_personas SET persona_id='b' WHERE conversation_id='a'")
scoped_tables = '''memories relationship_events character_moods character_interests daily_activities life_events
life_plans proactive_messages video_sources meme_knowledge task_queue activity_states api_usage_records'''.split()
for table in scoped_tables:
    assert 'persona_id' in {r[1] for r in db.execute('PRAGMA table_info(' + table + ')')}
db.execute("INSERT INTO memory_fts(memory_id,text) VALUES('fixture','主人喜欢绘画练习')")
assert db.execute("SELECT count(*) FROM memory_fts WHERE memory_fts MATCH ?", ('"喜欢绘画"',)).fetchone()[0] == 1
# Exercise the independent full-chat index, including an edit and a tombstone.
db.execute("INSERT INTO sync_events(event_id,profile_id,origin_device_id,origin_seq,lamport,schema_version,entity_type,entity_id,operation,checksum,envelope_json) VALUES('recall-event','lab','device',1,1,1,'message','recall-fixture','put',?,'{}')", ('0'*64,))
db.execute("INSERT INTO messages(id,conversation_id,profile_id,role,content,status,origin_device_id,created_lamport,source_event_id) VALUES('recall-fixture','a','lab','user','去年一起去了黄山旅行','complete','device',1,'recall-event')")
assert db.execute("SELECT count(*) FROM chat_recall_fts WHERE chat_recall_fts MATCH ?", ('"黄山旅行"',)).fetchone()[0] == 1
db.execute("UPDATE messages SET content='今年一起去了青岛旅行' WHERE id='recall-fixture'")
assert db.execute("SELECT count(*) FROM chat_recall_fts WHERE chat_recall_fts MATCH ?", ('"黄山旅行"',)).fetchone()[0] == 0
assert db.execute("SELECT count(*) FROM chat_recall_fts WHERE chat_recall_fts MATCH ?", ('"青岛旅行"',)).fetchone()[0] == 1
db.execute("UPDATE messages SET is_deleted=1 WHERE id='recall-fixture'")
assert db.execute("SELECT count(*) FROM chat_recall_fts WHERE chat_recall_fts MATCH ?", ('"青岛旅行"',)).fetchone()[0] == 0
assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
assert list(db.execute("PRAGMA foreign_key_check")) == []
print(json.dumps({"sqlite_version": sqlite3.sqlite_version, "required_tables": len(required),
    "all_required_tables_exist": True, "chat_tables": len(chat_tables), "persona_tables": len(persona_tables), "companion_tables": len(companion_tables),
    "persona_scoped_tables": len(scoped_tables), "persona_constraints": "passed", "foreign_keys": "passed", "bounds": "passed",
    "chinese_trigram_search": "passed", "full_chat_index_insert_edit_delete": "passed", "integrity": "ok"}, ensure_ascii=False, indent=2))
