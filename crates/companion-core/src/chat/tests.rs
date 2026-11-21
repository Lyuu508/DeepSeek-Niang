use super::*;
use crate::event::Event;
use rusqlite::Connection;
use std::{fs, path::PathBuf};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("companion-chat-test-{}", Uuid::new_v4()));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn db(&self) -> PathBuf {
        self.0.join("chat.sqlite3")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn second_instance_cannot_recover_a_live_generation() {
    let f = Fixture::new();
    let mut first = Store::open(f.db()).unwrap();
    let c = first.create_conversation("实例锁测试").unwrap().id;
    let p = first
        .begin_chat(&send(&c, "正在运行"), &ChatSettings::default())
        .unwrap();
    assert!(matches!(Store::open(f.db()), Err(Error::DatabaseInUse)));
    assert_eq!(first.generation(&p.assistant_id).unwrap().state, "sending");
    drop(first);
    let recovered = Store::open(f.db()).unwrap();
    assert_eq!(
        recovered.generation(&p.assistant_id).unwrap().state,
        "interrupted"
    );
}

#[test]
fn an_attached_image_is_atomic_with_the_user_message_and_stays_out_of_context() {
    let fixture = Fixture::new();
    let mut store = Store::open(fixture.db()).unwrap();
    let persona = store.active_persona_id().unwrap();
    let thread = store.persona(&persona).unwrap().main_conversation_id;
    let settings = store.chat_settings().unwrap();
    let input = send(&thread, "只有这段文字进入对话");
    let bad = crate::chat_images::ChatImage {
        name: "invalid.jpg".into(),
        jpeg: "bad".into(),
    };
    assert!(
        store
            .begin_chat_with_image(&input, &settings, Some(&bad))
            .is_err()
    );
    assert!(store.chat_messages(&thread).unwrap().is_empty());
    let good = crate::chat_images::ChatImage {
        name: "fixture.jpg".into(),
        jpeg: "/9j/Af/Z".into(),
    };
    let request = store
        .begin_chat_with_image(&input, &settings, Some(&good))
        .unwrap();
    assert_eq!(
        store.local_chat_images(&thread).unwrap().as_slice(),
        std::slice::from_ref(&request.user_message_id)
    );
    assert!(
        !serde_json::to_string(&store.companion_context(&thread, &settings).unwrap())
            .unwrap()
            .contains(&good.jpeg)
    );
    let other = store.create_persona("另一个人格", "冷静").unwrap();
    assert!(
        store
            .local_chat_image(&other.main_conversation_id, &request.user_message_id)
            .unwrap()
            .is_none()
    );
    drop(store);
    let store = Store::open(fixture.db()).unwrap();
    assert_eq!(
        store
            .local_chat_image(&thread, &request.user_message_id)
            .unwrap()
            .unwrap()
            .jpeg,
        good.jpeg
    );
}

#[test]
fn escaped_output_limit_cannot_corrupt_the_recovery_checkpoint() {
    let (mut s, c) = setup();
    let p = s
        .begin_chat(&send(&c, "控制字符测试"), &ChatSettings::default())
        .unwrap();
    s.checkpoint_chat(&p.request_id, "已保存内容").unwrap();
    assert!(
        s.checkpoint_chat(&p.request_id, &"\0".repeat(9000))
            .is_err()
    );
    assert_eq!(s.generation(&p.assistant_id).unwrap().content, "已保存内容");
    s.finish_chat(
        &p.request_id,
        "已保存内容",
        "interrupted",
        Some("OUTPUT_LIMIT"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(s.generation(&p.assistant_id).unwrap().state, "interrupted");
}
fn setup() -> (Store, String) {
    let mut s = Store::open(":memory:").unwrap();
    let c = s.create_conversation("新对话").unwrap();
    (s, c.id)
}
fn send(conversation: &str, text: &str) -> StartChat {
    StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: conversation.into(),
        text: Some(text.into()),
        regenerate_user_id: None,
    }
}
fn finish(store: &mut Store, p: &PreparedChat, text: &str) {
    store
        .finish_chat(&p.request_id, text, "complete", None, Some("stop"), None)
        .unwrap();
}

#[test]
fn send_is_transactional_idempotent_and_preserves_sync_fields() {
    let (mut s, c) = setup();
    let input = send(&c, "中文消息 ' ; 不会成为 SQL");
    let p = s.begin_chat(&input, &ChatSettings::default()).unwrap();
    assert!(p.is_new);
    assert!(
        !s.begin_chat(&input, &ChatSettings::default())
            .unwrap()
            .is_new
    );
    let rows = s.chat_messages(&c).unwrap();
    assert_eq!(rows.iter().filter(|m| m.role == "user").count(), 1);
    assert_eq!(rows[0].content, "中文消息 ' ; 不会成为 SQL");
    assert_eq!(s.diagnostics().unwrap().event_count, 1);
    assert!(s.checkpoints().unwrap()[&s.device_id] > 0);
    let mut changed = input;
    changed.text = Some("different request body".into());
    assert!(matches!(
        s.begin_chat(&changed, &ChatSettings::default()),
        Err(Error::Conflict)
    ));
}
#[test]
fn stream_checkpoints_finalize_once_and_ignore_late_tokens() {
    let (mut s, c) = setup();
    let p = s
        .begin_chat(&send(&c, "你好"), &ChatSettings::default())
        .unwrap();
    assert!(s.checkpoint_chat(&p.request_id, "你好，主人🌊").unwrap());
    assert_eq!(s.generation(&p.assistant_id).unwrap().state, "generating");
    let usage = TokenUsage {
        prompt_tokens: 6,
        completion_tokens: 4,
        total_tokens: 10,
    };
    assert!(
        s.finish_chat(
            &p.request_id,
            "你好，主人🌊",
            "complete",
            None,
            Some("stop"),
            Some(&usage)
        )
        .unwrap()
    );
    assert!(
        !s.checkpoint_chat(&p.request_id, "late corrupt text")
            .unwrap()
    );
    assert!(
        !s.finish_chat(
            &p.request_id,
            "duplicate",
            "complete",
            None,
            Some("stop"),
            Some(&usage)
        )
        .unwrap()
    );
    assert_eq!(
        s.generation(&p.assistant_id).unwrap().content,
        "你好，主人🌊"
    );
    assert_eq!(s.diagnostics().unwrap().event_count, 2);
    let counts = s.usage_summary().unwrap();
    assert_eq!(
        (
            counts.requests,
            counts.measured_requests,
            counts.prompt_tokens,
            counts.completion_tokens
        ),
        (1, 1, 6, 4)
    );
}
#[test]
fn regeneration_keeps_one_user_and_both_reply_variants() {
    let (mut s, c) = setup();
    let settings = ChatSettings::default();
    let first = s.begin_chat(&send(&c, "一个问题"), &settings).unwrap();
    finish(&mut s, &first, "旧回复");
    let second = s
        .begin_chat(
            &StartChat {
                request_id: Uuid::new_v4().to_string(),
                conversation_id: c.clone(),
                text: None,
                regenerate_user_id: Some(first.user_message_id.clone()),
            },
            &settings,
        )
        .unwrap();
    finish(&mut s, &second, "新回复");
    let rows = s.chat_messages(&c).unwrap();
    assert_eq!(rows.iter().filter(|m| m.role == "user").count(), 1);
    assert_eq!(rows.iter().filter(|m| m.role == "assistant").count(), 2);
    assert!(
        !rows
            .iter()
            .find(|m| m.id == first.assistant_id)
            .unwrap()
            .selected
    );
    assert!(
        rows.iter()
            .find(|m| m.id == second.assistant_id)
            .unwrap()
            .selected
    );
    let context = s
        .chat_context(&c, &settings, &ContextInjection::default())
        .unwrap();
    assert!(context.iter().any(|m| m.content == "新回复"));
    assert!(!context.iter().any(|m| m.content == "旧回复"));
    let next = s.begin_chat(&send(&c, "第二个问题"), &settings).unwrap();
    finish(&mut s, &next, "第二个回复");
    assert!(
        s.begin_chat(
            &StartChat {
                request_id: Uuid::new_v4().to_string(),
                conversation_id: c,
                text: None,
                regenerate_user_id: Some(first.user_message_id)
            },
            &settings
        )
        .is_err()
    );
}
#[test]
fn conversations_and_history_scope_are_isolated() {
    let (mut s, c) = setup();
    let settings = ChatSettings::default();
    let p = s.begin_chat(&send(&c, "私人话题甲"), &settings).unwrap();
    finish(&mut s, &p, "甲的回复");
    let b = s.create_conversation("乙会话").unwrap().id;
    let p = s.begin_chat(&send(&b, "话题乙"), &settings).unwrap();
    finish(&mut s, &p, "乙的回复");
    let ctx = s
        .chat_context(&b, &settings, &ContextInjection::default())
        .unwrap();
    assert!(!ctx.iter().any(|m| m.content.contains("私人话题甲")));
    let p = s.begin_chat(&send(&b, "只发这一轮"), &settings).unwrap();
    let mut current = settings;
    current.include_history = false;
    let ctx = s
        .chat_context(&b, &current, &ContextInjection::default())
        .unwrap();
    assert_eq!(ctx.len(), 2);
    assert_eq!(ctx[1].content, "只发这一轮");
    finish(&mut s, &p, "完成");
}
#[test]
fn empty_failure_is_durable_without_fake_assistant_message_event() {
    let (mut s, c) = setup();
    let p = s
        .begin_chat(&send(&c, "无网络"), &ChatSettings::default())
        .unwrap();
    s.finish_chat(
        &p.request_id,
        "",
        "failed",
        Some("NETWORK_ERROR"),
        None,
        None,
    )
    .unwrap();
    let m = s.generation(&p.assistant_id).unwrap();
    assert_eq!(m.state, "failed");
    assert_eq!(m.content, "");
    assert_eq!(m.error_code.as_deref(), Some("NETWORK_ERROR"));
    assert_eq!(s.diagnostics().unwrap().event_count, 1);
    assert_eq!(s.usage_summary().unwrap().measured_requests, 0);
}
#[test]
fn restart_recovers_partial_reply_and_keeps_history_and_personality() {
    let f = Fixture::new();
    let (c, assistant, user);
    {
        let mut s = Store::open(f.db()).unwrap();
        c = s.create_conversation("恢复测试").unwrap().id;
        let mut settings = s.chat_settings().unwrap();
        settings.user_title = "旅行者".into();
        s.save_chat_settings(&settings).unwrap();
        let p = s.begin_chat(&send(&c, "中途退出"), &settings).unwrap();
        assistant = p.assistant_id;
        user = p.user_message_id;
        s.checkpoint_chat(&p.request_id, "已保存的半句").unwrap();
    }
    {
        let s = Store::open(f.db()).unwrap();
        let m = s.generation(&assistant).unwrap();
        assert_eq!(m.state, "interrupted");
        assert_eq!(m.error_code.as_deref(), Some("APP_RESTARTED"));
        assert_eq!(m.content, "已保存的半句");
        assert!(s.chat_messages(&c).unwrap().iter().any(|m| m.id == user));
        assert_eq!(s.chat_settings().unwrap().user_title, "旅行者");
        assert_eq!(s.diagnostics().unwrap().integrity, "ok");
    }
    {
        let s = Store::open(f.db()).unwrap();
        assert_eq!(s.diagnostics().unwrap().event_count, 2);
    }
}
#[test]
fn character_is_editable_with_optional_verified_state_and_no_fixed_affection() {
    let settings = ChatSettings::default();
    let prompt = settings.system_prompt(&ContextInjection::default());
    assert!(prompt.contains("主人"));
    assert!(!prompt.contains("喜欢程度："));
    let injection = ContextInjection {
        relationship: Some("根据已核验事件计算的状态".into()),
        mood: Some("当前平静".into()),
    };
    assert!(settings.system_prompt(&injection).contains("当前平静"));
    let mut invalid = settings;
    invalid.max_tokens = 999999;
    assert!(invalid.validate().is_err());
    let mut value = serde_json::to_value(ChatSettings::default()).unwrap();
    value["api_key"] = json!("fixture-never-in-preferences");
    assert!(serde_json::from_value::<ChatSettings>(value).is_err());
}
#[test]
fn invalid_usage_cannot_partially_commit_a_response() {
    let (mut s, c) = setup();
    let p = s
        .begin_chat(&send(&c, "测试统计"), &ChatSettings::default())
        .unwrap();
    let u = TokenUsage {
        prompt_tokens: -1,
        completion_tokens: 2,
        total_tokens: 1,
    };
    assert!(
        s.finish_chat(
            &p.request_id,
            "正文",
            "complete",
            None,
            Some("stop"),
            Some(&u)
        )
        .is_err()
    );
    assert_eq!(s.generation(&p.assistant_id).unwrap().state, "sending");
    assert_eq!(s.usage_summary().unwrap().requests, 0);
}
#[test]
fn v1_migration_preserves_raw_events_sequence_and_creates_consistent_backup() {
    let f = Fixture::new();
    let profile = Uuid::new_v4().to_string();
    let device = Uuid::new_v4().to_string();
    let message = Uuid::new_v4().to_string();
    let event = Event {
        schema_version: 1,
        profile_id: profile.clone(),
        event_id: Uuid::new_v4().to_string(),
        origin_device_id: device.clone(),
        origin_seq: 1,
        lamport: 1,
        entity: Entity::Message,
        entity_id: message.clone(),
        operation: Operation::Put,
        payload: json!({"role":"user","content":"来自阶段零"}),
        checksum: String::new(),
    }
    .seal()
    .unwrap();
    let raw = serde_json::to_string(&event).unwrap();
    {
        let db = Connection::open(f.db()).unwrap();
        let ddl = include_str!("../../../../db/migrations/0001_initial.sql");
        db.execute_batch(ddl).unwrap();
        db.execute(
            "INSERT INTO schema_migrations VALUES(1,?1)",
            [sha256_hex(ddl.as_bytes())],
        )
        .unwrap();
        db.pragma_update(None, "user_version", 1).unwrap();
        db.execute(
            "INSERT INTO local_profiles(id,display_name) VALUES(?1,'fixture')",
            [&profile],
        )
        .unwrap();
        db.execute(
            "INSERT INTO devices(id,profile_id,name,trust_state) VALUES(?1,?2,'fixture','local')",
            params![device, profile],
        )
        .unwrap();
        db.execute("INSERT INTO local_clock VALUES(?1,1,1)", [&device])
            .unwrap();
        db.execute(
            "INSERT INTO conversations(id,profile_id,title) VALUES(?1,?1,'旧会话')",
            [&profile],
        )
        .unwrap();
        db.execute("INSERT INTO sync_events(event_id,profile_id,origin_device_id,origin_seq,lamport,schema_version,entity_type,entity_id,operation,checksum,envelope_json) VALUES(?1,?2,?3,1,1,1,'message',?4,'put',?5,?6)",params![event.event_id,profile,device,message,event.checksum,raw]).unwrap();
        db.execute("INSERT INTO messages(id,profile_id,conversation_id,role,content,origin_device_id,created_lamport,source_event_id) VALUES(?1,?2,?2,'user','来自阶段零',?3,1,?4)",params![message,profile,device,event.event_id]).unwrap();
        db.execute("INSERT INTO sync_checkpoints VALUES(?1,1)", [&device])
            .unwrap();
    }
    let mut s = Store::open(f.db()).unwrap();
    assert_eq!(
        s.diagnostics().unwrap().schema_version,
        crate::storage::SCHEMA_VERSION
    );
    assert_eq!(s.message_contents().unwrap(), vec!["来自阶段零"]);
    assert_eq!(s.checkpoints().unwrap()[&device], 1);
    let saved: String = s
        .conn
        .query_row("SELECT envelope_json FROM sync_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(saved, raw);
    let next = s
        .put(
            Entity::Memory,
            &Uuid::new_v4().to_string(),
            json!({"text":"升级之后"}),
        )
        .unwrap();
    assert_eq!(next.origin_seq, 2);
    let backups: Vec<_> = fs::read_dir(&f.0)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "bak"))
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(&backups[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn gallery_and_upgrade_backup_keep_the_original_message_and_images() {
    use crate::chat_images::ChatImage;
    let fixture = Fixture::new();
    let mut store = Store::open(fixture.db()).unwrap();
    let persona = store.active_persona_id().unwrap();
    let thread = store.persona(&persona).unwrap().main_conversation_id;
    let input = send(&thread, "升级前保留的聊天");
    let settings = store.chat_settings().unwrap();
    let images = ["第一张.jpg", "第二张.jpg"].map(|name| ChatImage {
        name: name.into(),
        jpeg: "/9j/Af/Z".into(),
    });
    let request = store
        .begin_chat_with_images(&input, &settings, &images)
        .unwrap();
    assert!(
        !store
            .begin_chat_with_images(&input, &settings, &images)
            .unwrap()
            .is_new
    );
    store
        .finish_chat(
            &request.request_id,
            "已收到本机附件。",
            "complete",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        store
            .local_chat_image(&thread, &request.user_message_id)
            .unwrap()
            .unwrap()
            .name,
        images[0].name
    );
    let path = store
        .backup_for_upgrade(&fixture.0.join("before-update"))
        .unwrap();
    let restored = Store::open(path).unwrap();
    assert_eq!(restored.active_persona_id().unwrap(), persona);
    assert!(
        restored
            .chat_messages(&thread)
            .unwrap()
            .iter()
            .any(|message| message.id == request.user_message_id)
    );
    let gallery = restored
        .local_chat_gallery(&thread, &request.user_message_id)
        .unwrap();
    assert_eq!(
        gallery
            .iter()
            .map(|image| image.name.as_str())
            .collect::<Vec<_>>(),
        ["第一张.jpg", "第二张.jpg"]
    );
}
