use super::*;
use crate::companion::{CompanionPreferences, life::quiet};
use uuid::Uuid;
fn setup() -> (Store, String) {
    let mut s = Store::open(":memory:").unwrap();
    s.set_device_platform("windows").unwrap();
    let p = s.active_persona_id().unwrap();
    let prefs = CompanionPreferences {
        quiet_start_minute: 0,
        quiet_end_minute: 0,
        ..Default::default()
    };
    s.save_companion_preferences(&p, &prefs).unwrap();
    (s, p)
}
fn plan(s: &mut Store, p: &str, steps: i64, now: i64) -> String {
    s.create_life_plan(
        p,
        &LifePlanInput {
            name: "连续创作计划".into(),
            kind: "create".into(),
            steps,
            duration_ms: 30000,
        },
        now,
    )
    .unwrap()
}
fn step(s: &mut Store, now: i64) {
    for n in 0..=2 {
        s.advance_life_at(now + n * 30000, now + n * 30000, true)
            .unwrap();
    }
}
fn current_task(s: &Store, p: &str, now: i64) -> String {
    s.life_snapshot(p, now).unwrap()["current"]["task_id"]
        .as_str()
        .unwrap()
        .into()
}
fn candidate(s: &mut Store, p: &str) -> String {
    let prefs = CompanionPreferences {
        proactive_enabled: true,
        proactive_model_enabled: true,
        proactive_token_budget: 20000,
        quiet_start_minute: 0,
        quiet_end_minute: 0,
        ..Default::default()
    };
    s.save_companion_preferences(p, &prefs).unwrap();
    plan(s, p, 1, 1000000);
    step(s, 1000000);
    s.conn
        .query_row(
            "SELECT id FROM companion_proactive WHERE persona_id=?1",
            [p],
            |r| r.get(0),
        )
        .unwrap()
}
#[test]
fn three_day_plan_continues_without_reset_and_makes_one_important_memory() {
    let (mut s, p) = setup();
    let id = plan(&mut s, &p, 3, 1000000);
    for day in 0..3 {
        let now = 1000000 + day * (86400000 + 60000);
        if day > 0 {
            let t = current_task(&s, &p, now);
            s.control_life_task(&p, &t, "cancel", now - 30000).unwrap();
        }
        step(&mut s, now);
        let n: i64 = s
            .conn
            .query_row("SELECT next_step FROM life_plans WHERE id=?1", [&id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n, day + 1);
    }
    let state: String = s
        .conn
        .query_row("SELECT state FROM life_plans WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(state, "completed");
    assert_eq!(s.managed_memories(&p).unwrap().len(), 1);
    assert_eq!(s.managed_memories(&p).unwrap()[0].source_kind, "life");
}
#[test]
fn pause_resume_preserves_same_activity_and_checkpoint() {
    let (mut s, p) = setup();
    plan(&mut s, &p, 2, 1000000);
    s.advance_life_at(1000000, 0, true).unwrap();
    s.advance_life_at(1030000, 30000, true).unwrap();
    let task = current_task(&s, &p, 1030000);
    s.control_life_task(&p, &task, "pause", 1030001).unwrap();
    s.advance_life_at(1060000, 60000, true).unwrap();
    assert_eq!(s.task_state(&task).unwrap(), "paused");
    s.control_life_task(&p, &task, "resume", 1060001).unwrap();
    s.advance_life_at(1090000, 90000, true).unwrap();
    assert_eq!(current_task(&s, &p, 1090000), task);
    assert_eq!(s.task_state(&task).unwrap(), "running");
}
#[test]
fn cancellation_is_terminal_and_cancels_associated_plan() {
    let (mut s, p) = setup();
    let plan = plan(&mut s, &p, 2, 1000000);
    s.advance_life_at(1000000, 0, true).unwrap();
    let task = current_task(&s, &p, 1000000);
    s.control_life_task(&p, &task, "cancel", 1000001).unwrap();
    assert_eq!(s.task_state(&task).unwrap(), "cancelled");
    assert!(s.control_life_task(&p, &task, "resume", 1000002).is_err());
    assert_eq!(
        s.conn
            .query_row("SELECT state FROM life_plans WHERE id=?1", [plan], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "cancelled"
    );
}
#[test]
fn failed_task_can_retry_with_same_output_identity() {
    let (mut s, p) = setup();
    plan(&mut s, &p, 1, 1000000);
    s.advance_life_at(1000000, 0, true).unwrap();
    let t = current_task(&s, &p, 1000000);
    let lease = s.claim_task(&t, &s.device_id.clone(), 1, 60000).unwrap();
    s.fail_task(&lease, 2).unwrap();
    s.control_life_task(&p, &t, "retry", 1000002).unwrap();
    s.advance_life_at(1030000, 30000, true).unwrap();
    s.advance_life_at(1060000, 60000, true).unwrap();
    assert_eq!(s.task_state(&t).unwrap(), "completed");
}
#[test]
fn replayed_ticks_never_duplicate_outputs_or_interest_evidence() {
    let (mut s, p) = setup();
    plan(&mut s, &p, 1, 1000000);
    step(&mut s, 1000000);
    for _ in 0..5 {
        s.advance_life_at(1060000, 60000, true).unwrap();
    }
    assert_eq!(
        s.conn
            .query_row(
                "SELECT count(*) FROM companion_life_events WHERE kind='completed'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    let history: String = s
        .conn
        .query_row(
            "SELECT evidence_json FROM character_interests WHERE persona_id=?1",
            [p],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<Value>>(&history).unwrap().len(),
        1
    );
}
#[test]
fn stale_fence_cannot_commit_after_pause() {
    let (mut s, p) = setup();
    let t = s
        .queue_persona_task(&p, "fixture", "fence", &json!({}))
        .unwrap();
    let lease = s.claim_task(&t, &s.device_id.clone(), 1, 1000).unwrap();
    s.pause_task(&t).unwrap();
    assert!(s.complete_task(&lease, 2).is_err());
    assert!(s.checkpoint_task(&lease, 2, "late", &json!({})).is_err());
}
#[test]
fn epoch_cannot_be_forged_even_with_current_boot_and_fence() {
    let (mut s, p) = setup();
    let t = s
        .queue_persona_task(&p, "fixture", "epoch", &json!({}))
        .unwrap();
    let mut lease = s.claim_task(&t, &s.device_id.clone(), 1, 1000).unwrap();
    lease.authority_epoch += 1;
    assert!(s.complete_task(&lease, 2).is_err());
}
#[test]
fn lease_expiry_marks_uncertain_instead_of_replaying_external_work() {
    let (mut s, p) = setup();
    let t = s
        .queue_persona_task(&p, "external", "external", &json!({}))
        .unwrap();
    let lease = s.claim_task(&t, &s.device_id.clone(), 1, 1000).unwrap();
    s.expire_leases(1001).unwrap();
    assert_eq!(s.task_state(&t).unwrap(), "uncertain");
    assert!(s.complete_task(&lease, 1001).is_err());
}
#[test]
fn restart_recovers_original_activity_without_counting_offline_time() {
    let path = std::env::temp_dir().join(format!("life-{}.sqlite", Uuid::new_v4()));
    let (p, activity, epoch) = {
        let mut s = Store::open(&path).unwrap();
        s.set_device_platform("windows").unwrap();
        let p = s.active_persona_id().unwrap();
        plan(&mut s, &p, 2, 1000000);
        s.advance_life_at(1000000, 0, true).unwrap();
        s.advance_life_at(1030000, 30000, true).unwrap();
        let v = s.life_snapshot(&p, 1030000).unwrap();
        (
            p,
            v["current"]["id"].as_str().unwrap().to_owned(),
            s.authority_epoch,
        )
    };
    let mut s = Store::open(&path).unwrap();
    assert!(s.authority_epoch > epoch);
    s.advance_life_at(2000000, 0, true).unwrap();
    s.advance_life_at(2030000, 30000, true).unwrap();
    let v = s.life_snapshot(&p, 2030000).unwrap();
    assert_eq!(v["current"]["id"], activity);
    assert_eq!(v["current"]["elapsed_ms"], 0);
    drop(s);
    let _ = std::fs::remove_file(path);
}
#[test]
fn stale_host_is_explicit_on_android_and_no_local_executor_runs() {
    let (mut s, p) = setup();
    s.advance_life_at(1000000, 0, true).unwrap();
    let task = current_task(&s, &p, 1000000);
    s.set_device_platform("android").unwrap();
    s.advance_life_at(9000000, 0, false).unwrap();
    let v = s.life_snapshot(&p, 9000000).unwrap();
    assert_eq!(v["host_online"], false);
    assert_eq!(v["can_control"], false);
    assert_eq!(s.task_state(&task).unwrap(), "queued");
    assert_eq!(v["external_media_executed"], false);
    assert!(s.control_life_task(&p, &task, "resume", 9000000).is_err());
}
#[test]
fn cross_midnight_and_timezone_are_explicit_but_do_not_reset_rolling_quota() {
    let (mut s, p) = setup();
    let id = candidate(&mut s, &p);
    let job = s
        .prepare_proactive_share(1060000, 1060000)
        .unwrap()
        .unwrap();
    s.finish_proactive_share(
        &job,
        1060100,
        1060100,
        "合成分享：虚拟创作安排结束",
        Some(&TokenUsage {
            prompt_tokens: 3,
            completion_tokens: 7,
            total_tokens: 10,
        }),
        None,
    )
    .unwrap();
    let mut pref = s.companion_preferences(&p).unwrap();
    pref.timezone_offset_minutes = -720;
    s.save_companion_preferences(&p, &pref).unwrap();
    assert!(!life::policy(&s.conn, &p, &pref, 1060200, "初识").unwrap());
    assert_eq!(
        s.conn
            .query_row(
                "SELECT id FROM companion_proactive WHERE state='delivered'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        id
    );
    pref.quiet_start_minute = 1320;
    pref.quiet_end_minute = 480;
    pref.timezone_offset_minutes = 0;
    assert!(quiet(23 * 3600000, &pref));
    assert!(!quiet(12 * 3600000, &pref));
}
#[test]
fn disabled_shares_never_prepare_model_calls() {
    let (mut s, p) = setup();
    candidate(&mut s, &p);
    let mut prefs = s.companion_preferences(&p).unwrap();
    prefs.proactive_enabled = false;
    s.save_companion_preferences(&p, &prefs).unwrap();
    assert!(
        s.prepare_proactive_share(1060000, 1060000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        s.conn
            .query_row("SELECT count(*) FROM budget_reservations", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn token_budget_blocks_before_reserving_or_sending() {
    let (mut s, p) = setup();
    candidate(&mut s, &p);
    let mut pref = s.companion_preferences(&p).unwrap();
    pref.proactive_token_budget = 1;
    s.save_companion_preferences(&p, &pref).unwrap();
    assert!(
        s.prepare_proactive_share(1060000, 1060000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        s.conn
            .query_row(
                "SELECT count(*) FROM api_usage_records WHERE purpose='life'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
#[test]
fn cancelled_or_disabled_mid_call_never_publishes_and_keeps_actual_usage() {
    let (mut s, p) = setup();
    candidate(&mut s, &p);
    let job = s
        .prepare_proactive_share(1060000, 1060000)
        .unwrap()
        .unwrap();
    let mut pref = s.companion_preferences(&p).unwrap();
    pref.proactive_enabled = false;
    s.save_companion_preferences(&p, &pref).unwrap();
    assert!(
        !s.finish_proactive_share(
            &job,
            1060100,
            1060100,
            "合成分享",
            Some(&TokenUsage {
                prompt_tokens: 20,
                completion_tokens: 10,
                total_tokens: 30
            }),
            None
        )
        .unwrap()
    );
    assert_eq!(
        s.life_usage_window(&p, 0, 2000000).unwrap()["by_activity"][0]["prompt_tokens"],
        20
    );
}
#[test]
fn uncertain_provider_usage_stays_reserved_and_is_never_retried() {
    let (mut s, p) = setup();
    candidate(&mut s, &p);
    let job = s
        .prepare_proactive_share(1060000, 1060000)
        .unwrap()
        .unwrap();
    s.finish_proactive_share(&job, 1060100, 1060100, "", None, Some("NETWORK_ERROR"))
        .unwrap();
    assert_eq!(s.task_state(&job.task_id).unwrap(), "uncertain");
    assert!(
        s.prepare_proactive_share(2060000, 2060000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        s.life_usage_window(&p, 0, 3000000).unwrap()["by_activity"][0]["measured_requests"],
        0
    );
}
#[test]
fn source_publishes_once_even_when_finish_is_replayed() {
    let (mut s, p) = setup();
    candidate(&mut s, &p);
    let job = s
        .prepare_proactive_share(1060000, 1060000)
        .unwrap()
        .unwrap();
    assert!(
        s.finish_proactive_share(&job, 1060100, 1060100, "合成分享", None, None)
            .unwrap()
    );
    assert!(
        !s.finish_proactive_share(&job, 1060200, 1060200, "合成分享", None, None)
            .unwrap()
    );
    assert_eq!(
        s.conn
            .query_row(
                "SELECT count(*) FROM messages WHERE role='assistant'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}
#[test]
fn read_and_notification_are_distinct_idempotent_receipts() {
    let (mut s, p) = setup();
    let id = candidate(&mut s, &p);
    let job = s
        .prepare_proactive_share(1060000, 1060000)
        .unwrap()
        .unwrap();
    s.finish_proactive_share(&job, 1060100, 1060100, "合成分享", None, None)
        .unwrap();
    let mut pref = s.companion_preferences(&p).unwrap();
    pref.notifications_enabled = true;
    s.save_companion_preferences(&p, &pref).unwrap();
    assert!(s.claim_share_notification(&p, &id, 1060200).unwrap());
    assert!(!s.claim_share_notification(&p, &id, 1060300).unwrap());
    s.record_share_receipt(&p, &id, false, "shown", 1060400)
        .unwrap();
    s.record_share_receipt(&p, &id, true, "shown", 1060500)
        .unwrap();
    let v = s.life_snapshot(&p, 1060600).unwrap();
    assert_eq!(v["proactive_messages"][0]["notification_state"], "shown");
    assert_eq!(v["proactive_messages"][0]["read_at_ms"], 1060500);
}
#[test]
fn personas_diaries_plans_interests_and_usage_are_isolated() {
    let (mut s, p) = setup();
    plan(&mut s, &p, 1, 1000000);
    step(&mut s, 1000000);
    let other = s.create_persona("另一个人格", "温柔").unwrap();
    let v = s.life_snapshot(&other.id, 1060000).unwrap();
    assert!(v["plans"].as_array().unwrap().is_empty());
    assert!(v["interests"].as_array().unwrap().is_empty());
    assert!(v["diary"]["events"].as_array().unwrap().is_empty());
}
#[test]
fn own_daily_activity_never_awards_affection() {
    let (mut s, p) = setup();
    let before = s.bond_snapshot(&p).unwrap();
    plan(&mut s, &p, 1, 1000000);
    step(&mut s, 1000000);
    let after = s.bond_snapshot(&p).unwrap();
    assert_eq!(before.affection, after.affection);
    assert_eq!(after.applied_events, 0);
}
#[test]
fn structured_diary_is_reopenable_by_date_and_does_not_call_models() {
    let (mut s, p) = setup();
    plan(&mut s, &p, 1, 1000000);
    step(&mut s, 1000000);
    let day = (1060000 + 480 * 60000) / 86400000;
    let diary = s.life_diary(&p, day).unwrap();
    assert_eq!(diary["api_requests"], 0);
    assert!(
        diary["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["kind"] == "completed")
    );
    assert!(
        s.life_diary(&p, day + 1).unwrap()["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn backward_wall_clock_never_invents_progress_or_reopens_completed_work() {
    let (mut s, p) = setup();
    plan(&mut s, &p, 1, 1000000);
    s.advance_life_at(1000000, 0, true).unwrap();
    s.advance_life_at(990000, 30000, true).unwrap();
    let v = s.life_snapshot(&p, 990000).unwrap();
    assert_eq!(v["current"]["elapsed_ms"], 0);
    assert_eq!(v["host_online"], false);
}
fn v7_fixture(path: &std::path::Path) -> (String, String) {
    let c = rusqlite::Connection::open(path).unwrap();
    for (n, sql) in [
        (
            1,
            include_str!("../../../../../db/migrations/0001_initial.sql"),
        ),
        (
            2,
            include_str!("../../../../../db/migrations/0002_chat.sql"),
        ),
        (
            3,
            include_str!("../../../../../db/migrations/0003_personas.sql"),
        ),
        (
            4,
            include_str!("../../../../../db/migrations/0004_lan_sync.sql"),
        ),
        (
            5,
            include_str!("../../../../../db/migrations/0005_pairing_persona_identity.sql"),
        ),
        (
            6,
            include_str!("../../../../../db/migrations/0006_companion_foundation.sql"),
        ),
        (
            7,
            include_str!("../../../../../db/migrations/0007_chat_recall.sql"),
        ),
    ] {
        c.execute_batch(sql).unwrap();
        c.execute(
            "INSERT INTO schema_migrations(version,checksum) VALUES(?1,?2)",
            params![n, crate::event::sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    }
    c.pragma_update(None, "user_version", 7).unwrap();
    let p = Uuid::new_v4().to_string();
    let device = Uuid::new_v4().to_string();
    c.execute(
        "INSERT INTO local_profiles(id,display_name) VALUES(?1,'v7')",
        [&p],
    )
    .unwrap();
    c.execute("INSERT INTO devices(id,profile_id,name,platform,trust_state) VALUES(?1,?2,'v7','windows','local')",params![device,p]).unwrap();
    c.execute(
        "INSERT INTO conversations(id,profile_id,title) VALUES(?1,?1,'v7')",
        [&p],
    )
    .unwrap();
    crate::persona::ensure_default(&c, &p, &device).unwrap();
    c.execute(
        "INSERT INTO companion_hosts VALUES(?1,?2,'v7-boot',1000000,'read')",
        params![p, device],
    )
    .unwrap();
    let a = Uuid::new_v4().to_string();
    c.execute("INSERT INTO companion_activities VALUES(?1,?2,?3,'study','virtual','running',1000000,2000000,NULL,1000000)",params![a,p,device]).unwrap();
    c.execute(
        "INSERT INTO companion_life_events VALUES(?1,?2,?3,?4,'started','v7 source',1000000)",
        params![Uuid::new_v4().to_string(), p, a, device],
    )
    .unwrap();
    (p, a)
}
#[test]
fn schema7_upgrade_keeps_persona_activity_events_and_consistent_backup() {
    let dir = std::env::temp_dir().join(format!("life-migrate-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("old.sqlite");
    let (p, a) = v7_fixture(&path);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.active_persona_id().unwrap(), p);
    assert_eq!(s.life_snapshot(&p, 1000000).unwrap()["current"]["id"], a);
    assert_eq!(
        s.conn
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        i64::from(crate::storage::SCHEMA_VERSION)
    );
    assert_eq!(
        s.conn
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    assert!(std::fs::read_dir(&dir).unwrap().any(|p| {
        p.unwrap()
            .path()
            .to_string_lossy()
            .contains(&format!("pre-v{}-", crate::storage::SCHEMA_VERSION))
    }));
    drop(s);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn schema9_recycle_upgrade_backs_up_and_rolls_back_before_retry() {
    let dir = std::env::temp_dir().join(format!("recycle-upgrade-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("old.sqlite");
    let (persona, activity) = v7_fixture(&path);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.pragma_update(None, "foreign_keys", "OFF").unwrap();
    for (n, sql) in [
        (
            8,
            include_str!("../../../../../db/migrations/0008_life_tasks.sql"),
        ),
        (
            9,
            include_str!("../../../../../db/migrations/0009_local_video.sql"),
        ),
    ] {
        c.execute_batch(sql).unwrap();
        c.execute(
            "INSERT INTO schema_migrations(version,checksum) VALUES(?1,?2)",
            params![n, crate::event::sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    }
    let violations: i64 = c
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(violations, 0);
    c.pragma_update(None, "user_version", 9).unwrap();
    c.execute("CREATE TABLE app_plugins(block_migration INTEGER)", [])
        .unwrap();
    drop(c);
    assert!(Store::open(&path).is_err());
    let c = rusqlite::Connection::open(&path).unwrap();
    let rolled_back: bool = c
        .query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM sqlite_master WHERE name='persona_lifecycle')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(rolled_back);
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        9
    );
    c.execute("DROP TABLE app_plugins", []).unwrap();
    drop(c);
    let store = Store::open(&path).unwrap();
    assert_eq!(store.active_persona_id().unwrap(), persona);
    assert_eq!(
        store.life_snapshot(&persona, 1000000).unwrap()["current"]["id"],
        activity
    );
    assert_eq!(store.diagnostics().unwrap().schema_version, 10);
    assert_eq!(store.diagnostics().unwrap().integrity, "ok");
    let backups = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension().is_some_and(|ext| ext == "bak")).then_some(path)
        })
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 2);
    for path in backups {
        let backup = rusqlite::Connection::open(path).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            9
        );
        assert_eq!(
            backup
                .query_row("SELECT id FROM companion_activities", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            activity
        );
    }
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn failed_schema8_upgrade_rolls_back_all_rebuilds_and_retains_backup() {
    let dir = std::env::temp_dir().join(format!("life-rollback-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("old.sqlite");
    let (_, a) = v7_fixture(&path);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        "CREATE TABLE companion_receipts(block_migration INTEGER)",
        [],
    )
    .unwrap();
    drop(c);
    assert!(Store::open(&path).is_err());
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        7
    );
    assert_eq!(
        c.query_row("SELECT id FROM companion_activities", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        a
    );
    let ddl: String = c
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='companion_activities'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!ddl.contains("task_id"));
    drop(c);
    assert!(std::fs::read_dir(&dir).unwrap().any(|p| {
        p.unwrap()
            .path()
            .to_string_lossy()
            .contains(&format!("pre-v{}-", crate::storage::SCHEMA_VERSION))
    }));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn active_foreground_chat_blocks_share_even_after_recent_interaction_window() {
    let (mut s, p) = setup();
    candidate(&mut s, &p);
    let input = crate::chat::StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: p.clone(),
        text: Some("前台合成测试请求".into()),
        regenerate_user_id: None,
    };
    s.begin_chat(&input, &ChatSettings::default()).unwrap();
    let now = crate::chat::now_ms() + 180000;
    assert!(s.prepare_proactive_share(now, 1090000).unwrap().is_none());
}
