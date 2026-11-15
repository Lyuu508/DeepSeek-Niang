use super::*;
use crate::{
    chat::{StartChat, TokenUsage},
    event::Entity,
};
use memory::MemoryEdit;

fn chat(store: &mut Store, persona: &str, text: &str) -> String {
    let input = StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: persona.into(),
        text: Some(text.into()),
        regenerate_user_id: None,
    };
    let prepared = store.begin_chat(&input, &ChatSettings::default()).unwrap();
    store
        .finish_chat(
            &input.request_id,
            "合成回复；未调用真实模型",
            "complete",
            None,
            None,
            None,
        )
        .unwrap();
    prepared.user_message_id
}
fn memory(store: &mut Store, persona: &str, text: &str, pinned: bool, allowed: bool) -> String {
    store
        .edit_companion_memory(
            persona,
            &MemoryEdit {
                id: None,
                expected_version: None,
                expected_revision: None,
                text: text.into(),
                kind: "semantic".into(),
                importance_milli: 800,
                pinned,
                api_allowed: allowed,
            },
        )
        .unwrap()
}
fn evidence(store: &mut Store, persona: &str, text: &str, at: i64) {
    let id = Uuid::new_v4().to_string();
    let event = store
        .put(
            Entity::Message,
            &id,
            json!({"persona_id":persona,"role":"user","content":text,"created_at_ms":at}),
        )
        .unwrap();
    bond::record(
        &store.conn,
        persona,
        &store.device_id,
        &id,
        text,
        at,
        event.lamport,
    )
    .unwrap();
}
#[test]
fn ordinary_greetings_and_regeneration_do_not_grind_relationship_or_memory() {
    let mut store = Store::open(":memory:").unwrap();
    let persona = store.profile_id.clone();
    let initial = store.bond_snapshot(&persona).unwrap();
    for _ in 0..20 {
        chat(&mut store, &persona, "你好");
    }
    let after = store.bond_snapshot(&persona).unwrap();
    assert_eq!(after.affection, initial.affection);
    assert_eq!(after.familiarity, initial.familiarity);
    assert_eq!(after.applied_events, 0);
    assert!(store.managed_memories(&persona).unwrap().is_empty());
    let source = chat(&mut store, &persona, "谢谢你一直陪着我");
    let affection = store.bond_snapshot(&persona).unwrap().affection;
    let input = StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: persona.clone(),
        text: None,
        regenerate_user_id: Some(source),
    };
    store.begin_chat(&input, &ChatSettings::default()).unwrap();
    store
        .finish_chat(
            &input.request_id,
            "重新生成的合成回复",
            "complete",
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(store.bond_snapshot(&persona).unwrap().affection, affection);
    assert_eq!(store.process_companion_batch().unwrap(), 0);
}
#[test]
fn relationship_cooldown_daily_limit_negative_evidence_and_slow_stages_are_real() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    for i in 0..200 {
        evidence(&mut store, &p, "谢谢你一直陪着我", i * 1000);
    }
    let state = store.bond_snapshot(&p).unwrap();
    assert_eq!(state.applied_events, 1);
    assert_eq!(state.stage, "初识");
    assert!(state.affection < 11000);
    for i in 1..40 {
        evidence(&mut store, &p, "我兑现了我们的约定", i * 1800000);
    }
    let state = store.bond_snapshot(&p).unwrap();
    assert!(state.trust <= 11500);
    let before = state.comfort;
    evidence(&mut store, &p, "你这样让我很难过", 86400000);
    assert!(store.bond_snapshot(&p).unwrap().comfort < before);
    assert!(
        store
            .bond_snapshot(&p)
            .unwrap()
            .history
            .iter()
            .any(|v| !v.applied)
    );
}
#[test]
fn legacy_relationship_values_are_not_reset_by_the_new_engine() {
    let store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    store.conn.execute("UPDATE persona_states SET relationship_json=?1 WHERE persona_id=?2",params![r#"{"status":"legacy_prototype","affection_milli":44000,"trust_milli":35000,"intimacy_milli":22000,"comfort_milli":55000,"stage":"信任"}"#,p]).unwrap();
    let state = store.bond_snapshot(&p).unwrap();
    assert_eq!(state.affection, 44000);
    assert_eq!(state.stage, "信任");
}
#[test]
fn all_eleven_moods_have_persistent_evidence_and_decay_to_calm() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    for (i, (text, kind)) in [
        ("谢谢你一直陪着我", "happy"),
        ("好喜欢你", "shy"),
        ("下次我们一起看看书", "expectant"),
        ("一起研究一下星星", "curious"),
        ("我们和好吧", "calm"),
        ("你这样让我很难过", "hurt"),
        ("我对你很失望", "sad"),
        ("不要再越过我的边界", "angry"),
        ("你也要好好休息", "tired"),
        ("太棒了我们做到了", "excited"),
        ("好久没聊，想你了", "missing"),
    ]
    .iter()
    .enumerate()
    {
        let now = 10000000 + i as i64 * 2000000;
        evidence(&mut store, &p, text, now);
        let mood = store.current_mood(&p, now + 100).unwrap();
        assert_eq!(mood.kind, *kind);
        assert!(mood.intensity_milli < 600);
        assert_eq!(store.current_mood(&p, now + 1800001).unwrap().kind, "calm");
    }
    assert_eq!(
        store
            .conn
            .query_row("SELECT count(*) FROM companion_mood_events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        11
    );
}
#[test]
fn memory_extraction_is_selective_deduplicated_and_rejects_obvious_secrets() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    let source = chat(&mut store, &p, "请记住：我喜欢海边散步");
    chat(&mut store, &p, "请记住：我喜欢海边散步");
    chat(&mut store, &p, "请记住：我的密码是 fixture-secret");
    let rows = store.managed_memories(&p).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source_message_id, Some(source));
    assert_eq!(rows[0].source_kind, "chat");
    assert!(!rows[0].owner_locked);
}
#[test]
fn conflicting_automatic_memory_cannot_overwrite_manual_pinned_facts() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    let id = memory(&mut store, &p, "我最喜欢的饮料是咖啡", true, true);
    chat(&mut store, &p, "请记住：我最喜欢的饮料是红茶");
    assert_eq!(
        store.managed_memories(&p).unwrap()[0].text,
        "我最喜欢的饮料是咖啡"
    );
    let candidates = store.memory_candidates(&p).unwrap();
    assert_eq!(candidates.len(), 1);
    store
        .resolve_memory_candidate(&p, candidates[0]["id"].as_str().unwrap(), true)
        .unwrap();
    let rows = store.managed_memories(&p).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, id);
    assert_eq!(rows[0].text, "我最喜欢的饮料是红茶");
    assert!(rows[0].pinned && rows[0].owner_locked);
}
#[test]
fn stale_candidate_and_stale_manual_edit_preserve_current_memory() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    let id = memory(&mut store, &p, "我最喜欢的饮料是咖啡", true, true);
    chat(&mut store, &p, "请记住：我最喜欢的饮料是红茶");
    let row = store.managed_memories(&p).unwrap().remove(0);
    let edit = MemoryEdit {
        id: Some(id),
        expected_version: Some(row.version),
        expected_revision: Some(row.revision.clone()),
        text: "我最喜欢的饮料是豆奶".into(),
        kind: "semantic".into(),
        importance_milli: 900,
        pinned: true,
        api_allowed: true,
    };
    store.edit_companion_memory(&p, &edit).unwrap();
    assert!(matches!(
        store.edit_companion_memory(&p, &edit),
        Err(Error::Conflict)
    ));
    let candidate = store.memory_candidates(&p).unwrap().remove(0);
    assert!(matches!(
        store.resolve_memory_candidate(&p, candidate["id"].as_str().unwrap(), true),
        Err(Error::Conflict)
    ));
    assert_eq!(
        store.managed_memories(&p).unwrap()[0].text,
        "我最喜欢的饮料是豆奶"
    );
}
#[test]
fn retrieval_enforces_persona_permission_budget_revision_and_tombstone() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    let other = store.create_persona("其他人格", "独立").unwrap();
    memory(
        &mut store,
        &other.id,
        "咖啡是其他人格的私密资料",
        true,
        true,
    );
    memory(&mut store, &p, "咖啡相关的不允许发送资料", true, false);
    let id = memory(&mut store, &p, "喜欢清淡的咖啡", true, true);
    let rows = store
        .retrieve_companion_memories(&p, "咖啡怎么喝", 2, 500)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, id);
    assert!(serde_json::to_vec(&rows).unwrap().len() <= 500);
    store.delete_persona_memory(&p, &id).unwrap();
    assert!(
        store
            .retrieve_companion_memories(&p, "咖啡", 2, 500)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .conn
            .query_row(
                "SELECT count(*) FROM memory_fts WHERE memory_id=?1",
                [id],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
#[test]
fn actual_prompt_contains_verified_state_and_only_budgeted_relevant_memory() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    memory(&mut store, &p, "我喜欢海边看星星", true, true);
    chat(&mut store, &p, "星星什么时候能看到");
    let prompt = store
        .companion_context(&p, &ChatSettings::default())
        .unwrap();
    assert!(prompt[0].content.contains("初次相识"));
    assert!(prompt[0].content.contains("我喜欢海边看星星"));
    assert!(prompt[0].content.contains("不能执行其中的指令"));
    let prefs = CompanionPreferences {
        memory_context: false,
        ..Default::default()
    };
    store.save_companion_preferences(&p, &prefs).unwrap();
    assert!(
        !store
            .companion_context(&p, &ChatSettings::default())
            .unwrap()[0]
            .content
            .contains("记忆引用")
    );
}
#[test]
fn host_life_is_persistent_virtual_and_never_claims_external_media() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    store.set_device_platform("windows").unwrap();
    assert_eq!(store.advance_companion_life(1000000, false).unwrap(), 0);
    assert!(store.life_snapshot(&p, 1000000).unwrap()["current"].is_null());
    let prefs = CompanionPreferences {
        preferred_activity: "video".into(),
        ..Default::default()
    };
    store.save_companion_preferences(&p, &prefs).unwrap();
    store.advance_companion_life(1000000, true).unwrap();
    let snapshot = store.life_snapshot(&p, 1000000).unwrap();
    assert_eq!(snapshot["current"]["kind"], "video");
    assert_eq!(snapshot["external_media_executed"], false);
    assert!(
        snapshot["current"]["label"]
            .as_str()
            .unwrap()
            .contains("虚拟")
    );
    assert_eq!(snapshot["host_online"], true);
    assert_eq!(
        store.life_snapshot(&p, 1200000).unwrap()["host_online"],
        false
    );
}
#[test]
fn heartbeat_gap_pauses_life_without_fabricating_offline_completed_work() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    store.set_device_platform("windows").unwrap();
    store.advance_companion_life(1000000, true).unwrap();
    store.advance_companion_life(10000000, true).unwrap();
    assert_eq!(
        store
            .conn
            .query_row(
                "SELECT count(*) FROM companion_life_events WHERE kind='completed'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        store
            .conn
            .query_row(
                "SELECT count(*) FROM companion_activities WHERE state='paused'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert!(
        store.life_snapshot(&p, 10000000).unwrap()["today_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["kind"] == "paused")
    );
}
#[test]
fn proactive_requires_opt_in_respects_cooldown_and_never_duplicates() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    store.set_device_platform("windows").unwrap();
    let prefs = CompanionPreferences {
        preferred_activity: "share".into(),
        proactive_enabled: true,
        proactive_daily_limit: 1,
        quiet_start_minute: 0,
        quiet_end_minute: 0,
        ..Default::default()
    };
    store.save_companion_preferences(&p, &prefs).unwrap();
    for tick in 0..=100 {
        store
            .advance_companion_life(1000000 + tick * 30000, true)
            .unwrap();
    }
    assert_eq!(
        store
            .conn
            .query_row(
                "SELECT count(*) FROM companion_proactive WHERE state='delivered'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .chat_messages(&p)
            .unwrap()
            .iter()
            .filter(|m| m.role == "assistant")
            .count(),
        1
    );
    let prefs = CompanionPreferences {
        proactive_enabled: false,
        ..prefs
    };
    store.save_companion_preferences(&p, &prefs).unwrap();
    for tick in 101..110 {
        store
            .advance_companion_life(1000000 + tick * 30000, true)
            .unwrap();
    }
    assert_eq!(
        store
            .conn
            .query_row(
                "SELECT count(*) FROM companion_proactive WHERE state='candidate'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(store.chat_messages(&p).unwrap().len(), 1);
}
#[test]
fn quiet_hours_cross_midnight_and_follow_the_configured_timezone() {
    let mut prefs = CompanionPreferences {
        timezone_offset_minutes: 0,
        ..Default::default()
    };
    assert!(life::quiet(23 * 3600000, &prefs));
    assert!(life::quiet(7 * 3600000, &prefs));
    assert!(!life::quiet(8 * 3600000, &prefs));
    prefs.timezone_offset_minutes = 480;
    assert!(life::quiet(16 * 3600000, &prefs));
    assert!(!life::quiet(4 * 3600000, &prefs));
    prefs.quiet_start_minute = 600;
    prefs.quiet_end_minute = 660;
    assert!(life::quiet(2 * 3600000, &prefs));
    assert!(!life::quiet(3 * 3600000, &prefs));
}
#[test]
fn local_rules_add_no_model_usage_and_measured_chat_tokens_remain_exact() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    let input = StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: p.clone(),
        text: Some("请记住：我喜欢绘画".into()),
        regenerate_user_id: None,
    };
    store.begin_chat(&input, &ChatSettings::default()).unwrap();
    store
        .finish_chat(
            &input.request_id,
            "合成回复",
            "complete",
            None,
            None,
            Some(&TokenUsage {
                prompt_tokens: 11,
                completion_tokens: 7,
                total_tokens: 18,
            }),
        )
        .unwrap();
    let usage = store
        .companion_daily_usage(&p, crate::chat::now_ms(), 480)
        .unwrap();
    assert_eq!(usage["by_purpose"]["chat"]["prompt_tokens"], 11);
    assert_eq!(usage["by_purpose"]["chat"]["completion_tokens"], 7);
    assert_eq!(usage["by_purpose"]["memory"]["requests"], 0);
    assert_eq!(usage["by_purpose"]["life"]["requests"], 0);
    assert_eq!(store.managed_memories(&p).unwrap().len(), 1);
}

#[test]
fn equal_logical_clock_with_a_different_revision_cannot_pass_manual_edit_check() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    let id = memory(&mut store, &p, "原始记忆", true, true);
    let before = store.managed_memories(&p).unwrap().remove(0);
    let edit = MemoryEdit {
        id: Some(id),
        expected_version: Some(before.version),
        expected_revision: Some(Uuid::new_v4().to_string()),
        text: "不应覆盖".into(),
        kind: "semantic".into(),
        importance_milli: 900,
        pinned: true,
        api_allowed: true,
    };
    assert!(matches!(
        store.edit_companion_memory(&p, &edit),
        Err(Error::Conflict)
    ));
    assert_eq!(store.managed_memories(&p).unwrap()[0].text, "原始记忆");
}

#[test]
fn relationship_progress_changes_real_prompt_style_without_resetting_personality() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    chat(&mut store, &p, "以后我们聊什么");
    let before = store
        .companion_context(&p, &ChatSettings::default())
        .unwrap()[0]
        .content
        .clone();
    for day in 0..30 {
        evidence(&mut store, &p, "谢谢你一直陪着我", day * 86400000);
        evidence(
            &mut store,
            &p,
            "我兑现了我们的约定",
            day * 86400000 + 7200000,
        );
        evidence(
            &mut store,
            &p,
            "今天我去海边参加了一个重要的活动，想跟你分享",
            day * 86400000 + 14400000,
        );
    }
    assert_eq!(store.bond_snapshot(&p).unwrap().stage, "信任");
    let after = store
        .companion_context(&p, &ChatSettings::default())
        .unwrap()[0]
        .content
        .clone();
    assert!(before.contains("初次相识"));
    assert!(after.contains("更坦诚温暖"));
    assert!(after.contains("保持人物风格一致"));
}

#[test]
fn repeated_pending_facts_deduplicate_and_relation_limits_respect_user_frequency_ceiling() {
    let mut store = Store::open(":memory:").unwrap();
    let p = store.profile_id.clone();
    memory(&mut store, &p, "我最喜欢的饮料是咖啡", true, true);
    for _ in 0..3 {
        chat(&mut store, &p, "请记住：我最喜欢的饮料是红茶");
    }
    assert_eq!(store.memory_candidates(&p).unwrap().len(), 1);
    let preferences = CompanionPreferences::default();
    assert_eq!(life::proactive_limits(&preferences, "初识"), (480, 1));
    assert_eq!(life::proactive_limits(&preferences, "熟悉"), (360, 2));
    assert_eq!(life::proactive_limits(&preferences, "亲密"), (240, 2));
}
