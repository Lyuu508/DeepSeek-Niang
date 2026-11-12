use super::*;
use crate::{
    chat::{StartChat, TokenUsage},
    event::{Entity, Event, Operation, sha256_hex},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("persona-test-{}", Uuid::new_v4()));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn db(&self) -> PathBuf {
        self.0.join("fixture.sqlite3")
    }
    fn backups(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "bak"))
            .collect()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn send(c: &str, text: &str) -> StartChat {
    StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: c.into(),
        text: Some(text.into()),
        regenerate_user_id: None,
    }
}
fn complete(s: &mut Store, c: &str, text: &str) {
    let settings = s.settings_for_conversation(c).unwrap();
    let p = s.begin_chat(&send(c, text), &settings).unwrap();
    s.finish_chat(
        &p.request_id,
        "已完成的测试回复",
        "complete",
        None,
        Some("stop"),
        Some(&TokenUsage {
            prompt_tokens: 7,
            completion_tokens: 3,
            total_tokens: 10,
        }),
    )
    .unwrap();
}
#[test]
fn one_main_thread_and_last_persona_survive_reopen_without_changing_device_identity() {
    let f = Files::new();
    let (profile, device, main, other);
    {
        let mut s = Store::open(f.db()).unwrap();
        assert_eq!(s.personas().unwrap().len(), 1);
        let p = s.personas().unwrap().remove(0);
        assert_eq!(p.name, "DeepSeek娘");
        profile = s.profile_id.clone();
        device = s.device_id.clone();
        main = p.main_conversation_id;
        let new = s
            .create_persona("原创角色B", "安静、理性，喜欢天文")
            .unwrap();
        other = new.id.clone();
        s.select_persona(&other).unwrap();
        complete(&mut s, &new.main_conversation_id, "B的第一条消息");
    }
    let s = Store::open(f.db()).unwrap();
    assert_eq!(s.active_persona_id().unwrap(), other);
    assert_eq!(s.profile_id, profile);
    assert_eq!(s.device_id, device);
    assert_eq!(s.persona(&profile).unwrap().main_conversation_id, main);
    assert!(s.chat_messages(&main).unwrap().is_empty());
    assert_eq!(s.chat_messages(&other).unwrap().len(), 2);
}
#[test]
fn history_memory_personality_states_and_tokens_are_isolated() {
    let mut s = Store::open(":memory:").unwrap();
    let a = s.active_persona_id().unwrap();
    let b = s.create_persona("B", "只属于B的性格").unwrap();
    s.put_persona_memory(&a, &Uuid::new_v4().to_string(), "A的保密绘画记忆")
        .unwrap();
    complete(&mut s, &a, "A的私密话题");
    s.select_persona(&b.id).unwrap();
    assert!(s.memories().unwrap().is_empty());
    assert!(s.search_memories("绘画").unwrap().is_empty());
    assert!(s.search_memories("保密绘画").unwrap().is_empty());
    assert_eq!(s.usage_summary().unwrap().requests, 0);
    let mut config = s.chat_settings().unwrap();
    config.user_title = "伙伴B".into();
    s.save_chat_settings(&config).unwrap();
    assert_eq!(s.persona_settings(&a).unwrap().user_title, "主人");
    assert_eq!(s.persona_settings(&b.id).unwrap().user_title, "伙伴B");
    complete(&mut s, &b.id, "B的话题");
    let context = s.chat_context(&b.id, &config, &Default::default()).unwrap();
    let text = serde_json::to_string(&context).unwrap();
    assert!(!text.contains("A的私密"));
    assert!(!text.contains("绘画记忆"));
    assert!(text.contains("伙伴B"));
    assert_eq!(s.usage_for_persona(&a).unwrap().prompt_tokens, 7);
    assert_eq!(s.usage_for_persona(&b.id).unwrap().prompt_tokens, 7);
    assert_eq!(s.persona_summary(&a).unwrap().memory_count, 1);
    assert_eq!(s.persona_summary(&b.id).unwrap().memory_count, 0);
    s.conn.execute("UPDATE persona_states SET mood_json='{\"status\":\"test_fixture\",\"mood\":\"好奇\"}' WHERE persona_id=?1",[&a]).unwrap();
    assert_eq!(s.persona_summary(&b.id).unwrap().mood["status"], "unknown");
    assert_eq!(
        s.persona_summary(&b.id).unwrap().life["status"],
        "not_started"
    );
}
#[test]
fn switching_mid_generation_does_not_move_the_reply_or_usage() {
    let mut s = Store::open(":memory:").unwrap();
    let a = s.active_persona_id().unwrap();
    let b = s.create_persona("B", "B的独立设定").unwrap();
    let settings = s.persona_settings(&a).unwrap();
    let task = s.begin_chat(&send(&a, "A正在生成"), &settings).unwrap();
    s.select_persona(&b.id).unwrap();
    s.checkpoint_chat(&task.request_id, "A的部分回复").unwrap();
    s.finish_chat(
        &task.request_id,
        "A的部分回复",
        "complete",
        None,
        Some("stop"),
        Some(&TokenUsage {
            prompt_tokens: 2,
            completion_tokens: 1,
            total_tokens: 3,
        }),
    )
    .unwrap();
    assert_eq!(s.chat_messages(&a).unwrap().len(), 2);
    assert!(s.chat_messages(&b.id).unwrap().is_empty());
    assert_eq!(s.usage_summary().unwrap().requests, 0);
    assert_eq!(s.usage_for_persona(&a).unwrap().requests, 1);
}
#[test]
fn memory_owner_cannot_be_changed_or_deleted_from_another_persona() {
    let mut s = Store::open(":memory:").unwrap();
    let a = s.active_persona_id().unwrap();
    let b = s.create_persona("B", "B的设定").unwrap();
    let m = Uuid::new_v4().to_string();
    s.put_persona_memory(&a, &m, "A的记忆").unwrap();
    let count = s.diagnostics().unwrap().event_count;
    assert!(s.put_persona_memory(&b.id, &m, "越界修改").is_err());
    assert!(s.delete_persona_memory(&b.id, &m).is_err());
    s.select_persona(&b.id).unwrap();
    assert!(s.delete(Entity::Memory, &m).is_err());
    assert_eq!(s.diagnostics().unwrap().event_count, count);
    assert_eq!(s.memories_for_persona(&a).unwrap()[0].text, "A的记忆");
}
#[test]
fn sql_guards_block_cross_persona_relations_and_profile_reassignment() {
    let mut s = Store::open(":memory:").unwrap();
    let a = s.active_persona_id().unwrap();
    let b = s.create_persona("B", "B的设定").unwrap();
    let ids: Vec<_> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
    s.put_persona_memory(&a, &ids[0], "A的第一条").unwrap();
    s.put_persona_memory(&a, &ids[1], "A的第二条").unwrap();
    s.put_persona_memory(&b.id, &ids[2], "B的第一条").unwrap();
    s.conn.execute("INSERT INTO memory_relations(from_memory_id,to_memory_id,relation) VALUES(?1,?2,'test')", params![ids[0],ids[1]]).unwrap();
    assert!(
        s.conn
            .execute("UPDATE memory_relations SET to_memory_id=?1", [&ids[2]])
            .is_err()
    );
    assert!(s.conn.execute("INSERT INTO memory_relations(from_memory_id,to_memory_id,relation) VALUES(?1,?2,'test')", params![ids[0],ids[2]]).is_err());
    let other = Uuid::new_v4().to_string();
    s.conn
        .execute(
            "INSERT INTO local_profiles(id,display_name) VALUES(?1,'另一个档案')",
            [&other],
        )
        .unwrap();
    assert!(
        s.conn
            .execute(
                "UPDATE memories SET profile_id=?1 WHERE id=?2",
                params![other, ids[0]]
            )
            .is_err()
    );
    assert!(
        s.conn
            .execute(
                "UPDATE personas SET main_conversation_id=?1 WHERE id=?2",
                params![b.main_conversation_id, a]
            )
            .is_err()
    );
    assert_eq!(s.memories_for_persona(&a).unwrap().len(), 2);
    assert_eq!(s.memories_for_persona(&b.id).unwrap().len(), 1);
    assert!(
        s.put(
            Entity::Memory,
            &Uuid::new_v4().to_string(),
            json!("not an object")
        )
        .is_err()
    );
}
fn old_v2(path: &Path) -> (String, String, Vec<String>, Vec<String>) {
    let db = Connection::open(path).unwrap();
    db.pragma_update(None, "foreign_keys", "ON").unwrap();
    for (v, ddl) in [
        (
            1,
            include_str!("../../../../db/migrations/0001_initial.sql"),
        ),
        (2, include_str!("../../../../db/migrations/0002_chat.sql")),
    ] {
        db.execute_batch(ddl).unwrap();
        db.execute(
            "INSERT INTO schema_migrations VALUES(?1,?2)",
            params![v, sha256_hex(ddl.as_bytes())],
        )
        .unwrap();
    }
    db.pragma_update(None, "user_version", 2).unwrap();
    let profile = Uuid::new_v4().to_string();
    let device = Uuid::new_v4().to_string();
    let second = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO local_profiles(id,display_name) VALUES(?1,'测试档案')",
        [&profile],
    )
    .unwrap();
    db.execute(
        "INSERT INTO devices(id,profile_id,name,trust_state) VALUES(?1,?2,'测试设备','local')",
        params![device, profile],
    )
    .unwrap();
    db.execute("INSERT INTO local_clock VALUES(?1,2,2)", [&device])
        .unwrap();
    let mut ids = Vec::new();
    let mut raw = Vec::new();
    for (i, conversation) in [&profile, &second].into_iter().enumerate() {
        db.execute(
            "INSERT INTO conversations(id,profile_id,title) VALUES(?1,?2,'旧会话')",
            params![conversation, profile],
        )
        .unwrap();
        let id = Uuid::new_v4().to_string();
        let e=Event{schema_version:1,profile_id:profile.clone(),event_id:Uuid::new_v4().to_string(),origin_device_id:device.clone(),origin_seq:(i+1) as i64,lamport:(i+1) as i64,entity:Entity::Message,entity_id:id.clone(),operation:Operation::Put,payload:json!({"role":"user","content":format!("旧消息{i}"),"conversation_id":conversation,"created_at_ms":100+(i as i64)*100}),checksum:String::new()}.seal().unwrap();
        let envelope = serde_json::to_string(&e).unwrap();
        db.execute("INSERT INTO sync_events(event_id,profile_id,origin_device_id,origin_seq,lamport,schema_version,entity_type,entity_id,operation,checksum,envelope_json) VALUES(?1,?2,?3,?4,?4,1,'message',?5,'put',?6,?7)",params![e.event_id,profile,device,e.origin_seq,id,e.checksum,envelope]).unwrap();
        db.execute("INSERT INTO messages(id,profile_id,conversation_id,role,content,origin_device_id,created_lamport,source_event_id,created_at_ms,status) VALUES(?1,?2,?3,'user',?4,?5,?6,?7,?8,'complete')",params![id,profile,conversation,e.payload["content"].as_str(),device,e.lamport,e.event_id,e.payload["created_at_ms"].as_i64()]).unwrap();
        ids.push(id);
        raw.push(envelope);
    }
    db.execute("INSERT INTO sync_checkpoints VALUES(?1,2)", [&device])
        .unwrap();
    let settings = ChatSettings {
        character_name: "DeepSeek 娘".into(),
        user_title: "旅行者".into(),
        ..ChatSettings::default()
    };
    db.execute(
        "INSERT INTO chat_preferences(profile_id,settings_json) VALUES(?1,?2)",
        params![profile, serde_json::to_string(&settings).unwrap()],
    )
    .unwrap();
    db.execute("INSERT INTO api_usage_records(id,profile_id,request_key,provider,model_id,input_tokens,output_tokens,currency,usage_state,created_at_ms,price_snapshot_json) VALUES(?1,?2,?3,'deepseek','deepseek-flash',11,9,'USD','measured',300,'{}')",params![Uuid::new_v4().to_string(),profile,Uuid::new_v4().to_string()]).unwrap();
    (profile, device, ids, raw)
}
#[test]
fn v2_backup_merges_legacy_views_without_touching_ids_events_or_timestamps() {
    let f = Files::new();
    let (profile, device, ids, events) = old_v2(&f.db());
    {
        let s = Store::open(f.db()).unwrap();
        assert_eq!(s.diagnostics().unwrap().schema_version, 3);
        assert_eq!(s.profile_id, profile);
        assert_eq!(s.device_id, device);
        let rows = s.chat_messages(&profile).unwrap();
        assert_eq!(rows.iter().map(|m| m.id.clone()).collect::<Vec<_>>(), ids);
        assert_eq!(
            rows.iter()
                .map(|m| m.created_at_ms.unwrap())
                .collect::<Vec<_>>(),
            vec![100, 200]
        );
        assert!(rows.iter().all(|m| m.state == "complete"));
        assert_eq!(s.usage_summary().unwrap().prompt_tokens, 11);
        assert_eq!(s.chat_settings().unwrap().user_title, "旅行者");
        assert_eq!(s.checkpoints().unwrap()[&device], 2);
        let saved = s
            .conn
            .prepare("SELECT envelope_json FROM sync_events ORDER BY origin_seq")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(saved, events);
        assert_eq!(
            s.conn
                .query_row("SELECT count(*) FROM conversations", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    assert_eq!(f.backups().len(), 1);
    let before = Connection::open(&f.backups()[0]).unwrap();
    assert_eq!(
        before
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        before
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    drop(before);
    let s = Store::open(f.db()).unwrap();
    assert_eq!(s.personas().unwrap().len(), 1);
    assert_eq!(s.chat_messages(&profile).unwrap().len(), 2);
    assert_eq!(f.backups().len(), 1);
}
#[test]
fn migrated_last_turn_can_regenerate_in_the_permanent_space() {
    let f = Files::new();
    let (profile, _, ids, _) = old_v2(&f.db());
    let mut s = Store::open(f.db()).unwrap();
    let settings = s.chat_settings().unwrap();
    let task = s
        .begin_chat(
            &StartChat {
                request_id: Uuid::new_v4().to_string(),
                conversation_id: profile.clone(),
                text: None,
                regenerate_user_id: Some(ids[1].clone()),
            },
            &settings,
        )
        .unwrap();
    s.finish_chat(
        &task.request_id,
        "旧话题的新回复",
        "complete",
        None,
        Some("stop"),
        None,
    )
    .unwrap();
    let rows = s.chat_messages(&profile).unwrap();
    assert_eq!(rows.iter().filter(|m| m.role == "user").count(), 2);
    assert_eq!(rows.last().unwrap().content, "旧话题的新回复");
    assert_eq!(rows.last().unwrap().user_message_id.as_ref(), Some(&ids[1]));
}
#[test]
fn failed_cross_profile_migration_rolls_back_the_whole_schema_upgrade() {
    let f = Files::new();
    let (profile, _, _, _) = old_v2(&f.db());
    let other = Uuid::new_v4().to_string();
    {
        let c = Connection::open(f.db()).unwrap();
        c.execute(
            "INSERT INTO local_profiles(id,display_name) VALUES(?1,'另一个档案')",
            [&other],
        )
        .unwrap();
        c.execute(
            "UPDATE conversations SET profile_id=?1 WHERE id<>?2",
            params![other, profile],
        )
        .unwrap();
    }
    assert!(Store::open(f.db()).is_err());
    let c = Connection::open(f.db()).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='personas'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(f.backups().len(), 1);
}
#[test]
fn renaming_keeps_main_thread_and_refuses_unknown_personas() {
    let mut s = Store::open(":memory:").unwrap();
    let a = s.active_persona_id().unwrap();
    let before = s.persona(&a).unwrap();
    let mut config = s.chat_settings().unwrap();
    config.character_name = "可编辑的人格名称".into();
    s.save_chat_settings(&config).unwrap();
    assert_eq!(
        s.persona(&a).unwrap().main_conversation_id,
        before.main_conversation_id
    );
    assert_eq!(s.personas().unwrap().len(), 1);
    assert!(s.select_persona(&Uuid::new_v4().to_string()).is_err());
    assert!(s.create_persona("", "无效").is_err());
}
