use super::*;
use crate::chat::{ChatSettings, StartChat};
use uuid::Uuid;

#[test]
fn desktop_pet_preferences_are_device_local_and_validated() {
    let store = Store::open(":memory:").unwrap();
    assert!(!store.pet_preferences().unwrap().visible);
    let previous: PetPreferences =
        serde_json::from_str(r#"{"visible":true,"scale_percent":110}"#).unwrap();
    assert!(previous.roam && previous.visible);
    let outbox: i64 = store
        .conn
        .query_row("SELECT count(*) FROM lan_outbox", [], |r| r.get(0))
        .unwrap();
    let preferences = PetPreferences {
        visible: true,
        scale_percent: 130,
        roam: false,
        ..Default::default()
    };
    store.save_pet_preferences(&preferences).unwrap();
    assert_eq!(store.pet_preferences().unwrap(), preferences);
    assert_eq!(
        store
            .conn
            .query_row("SELECT count(*) FROM lan_outbox", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        outbox
    );
    assert!(
        store
            .save_pet_preferences(&PetPreferences {
                scale_percent: u16::MAX,
                ..preferences
            })
            .is_err()
    );
}

#[test]
fn desktop_pet_uses_real_chat_phase_and_respects_persona_switch() {
    let mut store = Store::open(":memory:").unwrap();
    let first = store.personas().unwrap().remove(0);
    let second = store.create_persona("测试人格", "在这里阅读").unwrap();
    store.select_persona(&first.id).unwrap();
    let request = Uuid::new_v4().to_string();
    store
        .begin_chat(
            &StartChat {
                request_id: request.clone(),
                conversation_id: first.main_conversation_id.clone(),
                text: Some("你好".into()),
                regenerate_user_id: None,
            },
            &ChatSettings::default(),
        )
        .unwrap();
    let active = Some((request.as_str(), first.main_conversation_id.as_str()));
    let state = store.pet_snapshot(200, active).unwrap();
    assert_eq!(state.pose, PetPose::Thinking);
    assert_eq!(state.persona_id, first.id);
    store.select_persona(&second.id).unwrap();
    let state = store.pet_snapshot(201, active).unwrap();
    assert_eq!(state.chat, PetChatState::Busy);
    assert_eq!(state.pose, PetPose::Idle);
    assert_eq!(state.persona_id, second.id);
    assert!(!state.builtin_avatar);
}

#[test]
fn desktop_pet_marks_expired_records_and_uses_saved_progress() {
    let store = Store::open(":memory:").unwrap();
    let persona = store.active_persona_id().unwrap();
    store
        .conn
        .execute(
            "INSERT INTO companion_hosts VALUES(?1,?2,'test-boot',100000,'rest')",
            params![persona, store.device_id],
        )
        .unwrap();
    store.conn.execute("INSERT INTO companion_activities(id,persona_id,host_device_id,kind,state,started_at_ms,scheduled_end_at_ms,updated_at_ms,progress_milli)
         VALUES(?1,?2,?3,'rest','running',1000,900000,98000,310)", params![Uuid::new_v4().to_string(),persona,store.device_id]).unwrap();
    let fresh = store.pet_snapshot(190000, None).unwrap();
    assert_eq!(fresh.pose, PetPose::Resting);
    assert_eq!(fresh.activity.unwrap().progress_milli, 310);
    let stale = store.pet_snapshot(190001, None).unwrap();
    assert_eq!(stale.pose, PetPose::Idle);
    assert!(stale.activity.unwrap().stale);
    assert!(
        store
            .pet_snapshot(99000, None)
            .unwrap()
            .activity
            .unwrap()
            .stale
    );
}
