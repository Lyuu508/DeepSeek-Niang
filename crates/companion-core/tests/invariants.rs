use companion_core::{
    Error,
    event::Entity,
    relationship::{self, Kind, RelationshipEvent},
    storage::Store,
    video::{self, AnalysisCoverage, Interval},
};
use serde_json::json;
use std::{collections::BTreeMap, fs};
use uuid::Uuid;

const PROFILE: &str = "00000000-0000-4000-8000-000000000001";
const A: &str = "00000000-0000-4000-8000-00000000000a";
const B: &str = "00000000-0000-4000-8000-00000000000b";
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn pair() -> (Store, Store) {
    let mut a = Store::open_lab(":memory:", A, PROFILE).unwrap();
    let mut b = Store::open_lab(":memory:", B, PROFILE).unwrap();
    a.register_lab_peer(B).unwrap();
    b.register_lab_peer(A).unwrap();
    (a, b)
}
fn exchange(a: &mut Store, b: &mut Store) {
    for _ in 0..20 {
        let ab = a.export_after(&b.checkpoints().unwrap()).unwrap();
        let ba = b.export_after(&a.checkpoints().unwrap()).unwrap();
        if ab.is_empty() && ba.is_empty() {
            return;
        }
        b.import_lab_batch(&ab).unwrap();
        a.import_lab_batch(&ba).unwrap();
    }
    panic!("sync failed to drain");
}
#[test]
fn simultaneous_offline_messages_converge_and_deduplicate() {
    let (mut a, mut b) = pair();
    let e = a
        .put(
            Entity::Message,
            &id(),
            json!({"role":"user","content":"A offline"}),
        )
        .unwrap();
    b.put(
        Entity::Message,
        &id(),
        json!({"role":"user","content":"B offline"}),
    )
    .unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(a.message_contents().unwrap(), b.message_contents().unwrap());
    assert_eq!(a.message_contents().unwrap().len(), 2);
    assert_eq!(b.import_lab_batch(&[e.clone(), e]).unwrap(), 0);
}
#[test]
fn checkpoint_never_skips_missing_sequence() {
    let (mut a, mut b) = pair();
    let e1 = a.put(Entity::Memory, &id(), json!({"text":"one"})).unwrap();
    let e2 = a.put(Entity::Memory, &id(), json!({"text":"two"})).unwrap();
    b.import_lab_batch(&[e2]).unwrap();
    assert_eq!(b.checkpoints().unwrap().get(A), Some(&0));
    b.import_lab_batch(&[e1]).unwrap();
    assert_eq!(b.checkpoints().unwrap().get(A), Some(&2));
}
#[test]
fn incremental_pagination_drains_more_than_one_batch() {
    let (mut a, mut b) = pair();
    for n in 0..140 {
        a.put(Entity::Memory, &id(), json!({"text":format!("entry {n}")}))
            .unwrap();
    }
    exchange(&mut a, &mut b);
    assert_eq!(b.diagnostics().unwrap().memory_count, 140);
    assert!(
        a.export_after(&b.checkpoints().unwrap())
            .unwrap()
            .is_empty()
    );
}
#[test]
fn delete_wins_even_when_received_before_create() {
    let (mut a, mut b) = pair();
    let memory = id();
    let put = a
        .put(Entity::Memory, &memory, json!({"text":"private"}))
        .unwrap();
    let del = a.delete(Entity::Memory, &memory).unwrap();
    b.import_lab_batch(&[del]).unwrap();
    b.import_lab_batch(&[put]).unwrap();
    assert!(b.memories().unwrap().is_empty());
    b.put(Entity::Memory, &memory, json!({"text":"late edit"}))
        .unwrap();
    exchange(&mut a, &mut b);
    assert!(a.memories().unwrap().is_empty());
    assert!(b.memories().unwrap().is_empty());
}
#[test]
fn concurrent_memory_edits_choose_same_logical_version() {
    let (mut a, mut b) = pair();
    let m = id();
    a.put(Entity::Memory, &m, json!({"text":"A"})).unwrap();
    b.put(Entity::Memory, &m, json!({"text":"B"})).unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(a.memories().unwrap()[0].text, "B");
    assert_eq!(b.memories().unwrap()[0].text, "B");
}
#[test]
fn corrupted_batch_is_rejected_without_partial_commit() {
    let (mut a, mut b) = pair();
    let good = a
        .put(Entity::Memory, &id(), json!({"text":"good"}))
        .unwrap();
    let mut bad = a.put(Entity::Memory, &id(), json!({"text":"bad"})).unwrap();
    bad.payload = json!({"text":"tampered"});
    assert!(b.import_lab_batch(&[good, bad]).is_err());
    assert_eq!(b.diagnostics().unwrap().event_count, 0);
}
#[test]
fn identity_equivocation_rolls_back_prior_valid_events_in_same_transaction() {
    let (mut a, mut b) = pair();
    let first = a
        .put(Entity::Memory, &id(), json!({"text":"first"}))
        .unwrap();
    b.import_lab_batch(&[first]).unwrap();
    let good = a
        .put(Entity::Memory, &id(), json!({"text":"should roll back"}))
        .unwrap();
    let mut cloned = Store::open_lab(":memory:", A, PROFILE).unwrap();
    let conflict = cloned
        .put(
            Entity::Memory,
            &id(),
            json!({"text":"same origin seq; different event"}),
        )
        .unwrap();
    assert!(matches!(
        b.import_lab_batch(&[good, conflict]),
        Err(Error::Conflict)
    ));
    assert_eq!(b.diagnostics().unwrap().event_count, 1);
    assert_eq!(b.checkpoints().unwrap()[A], 1);
}
#[test]
fn untrusted_and_wrong_profile_origins_are_rejected() {
    let mut a = Store::open_lab(":memory:", A, PROFILE).unwrap();
    let mut b = Store::open_lab(":memory:", B, PROFILE).unwrap();
    let e = a
        .put(Entity::Memory, &id(), json!({"text":"not paired"}))
        .unwrap();
    assert!(matches!(b.import_lab_batch(&[e]), Err(Error::Untrusted)));
    let mut other = Store::open_lab(":memory:", &id(), &id()).unwrap();
    let e = other
        .put(Entity::Memory, &id(), json!({"text":"other profile"}))
        .unwrap();
    assert!(matches!(b.import_lab_batch(&[e]), Err(Error::Untrusted)));
}
#[test]
fn message_deletion_propagates_without_overwriting_other_messages() {
    let (mut a, mut b) = pair();
    let m = id();
    a.put(
        Entity::Message,
        &m,
        json!({"role":"user","content":"delete me"}),
    )
    .unwrap();
    b.put(
        Entity::Message,
        &id(),
        json!({"role":"user","content":"keep me"}),
    )
    .unwrap();
    exchange(&mut a, &mut b);
    b.delete(Entity::Message, &m).unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(a.message_contents().unwrap(), vec!["keep me"]);
}
#[test]
fn keyword_index_tracks_updates_and_deletion_and_short_chinese_queries() {
    let (mut a, _) = pair();
    let m = id();
    a.put(
        Entity::Memory,
        &m,
        json!({"text":"主人喜欢绘画，也喜欢蓝色。"}),
    )
    .unwrap();
    assert_eq!(a.search_memories("喜欢绘画").unwrap().len(), 1);
    assert_eq!(a.search_memories("绘画").unwrap().len(), 1);
    assert!(a.search_memories("%_").unwrap().is_empty());
    a.put(Entity::Memory, &m, json!({"text":"现在练习音乐"}))
        .unwrap();
    assert!(a.search_memories("喜欢绘画").unwrap().is_empty());
    a.delete(Entity::Memory, &m).unwrap();
    assert!(a.search_memories("练习音乐").unwrap().is_empty());
}
#[test]
fn restart_preserves_identity_history_clock_and_integrity() {
    let path = std::env::temp_dir().join(format!("companion-test-{}.sqlite3", id()));
    let device;
    {
        let mut s = Store::open(&path).unwrap();
        device = s.device_id.clone();
        s.put(Entity::Memory, &id(), json!({"text":"persisted"}))
            .unwrap();
    }
    {
        let mut s = Store::open(&path).unwrap();
        assert_eq!(s.device_id, device);
        let e = s
            .put(Entity::Memory, &id(), json!({"text":"second"}))
            .unwrap();
        assert_eq!(e.origin_seq, 2);
        assert_eq!(e.lamport, 2);
        assert_eq!(s.diagnostics().unwrap().integrity, "ok");
    }
    fs::remove_file(path).unwrap();
}
#[test]
fn one_authority_grants_one_lease_and_task_key_deduplicates() {
    let (mut a, _) = pair();
    let t = a.queue_task("local", "unique", &json!({})).unwrap();
    assert_eq!(a.queue_task("local", "unique", &json!({})).unwrap(), t);
    assert!(matches!(
        a.queue_task("local", "unique", &json!({"changed":true})),
        Err(Error::Conflict)
    ));
    let l = a.claim_task(&t, A, 0, 100).unwrap();
    assert!(a.complete_task(&l, -1).is_err());
    assert!(a.claim_task(&t, B, 0, 100).is_err());
    a.checkpoint_task(&l, 50, "step:0", &json!({"done":true}))
        .unwrap();
    a.complete_task(&l, 90).unwrap();
    assert_eq!(a.task_state(&t).unwrap(), "completed");
    assert!(a.checkpoint_task(&l, 91, "stale", &json!({})).is_err());
}
#[test]
fn expired_lease_requires_reconciliation_and_old_fence_cannot_commit() {
    let (mut a, _) = pair();
    let t = a
        .queue_task("paid-video", "video-hash/version", &json!({}))
        .unwrap();
    let old = a.claim_task(&t, A, 0, 100).unwrap();
    assert!(a.complete_task(&old, 100).is_err());
    a.expire_leases(100).unwrap();
    assert_eq!(a.task_state(&t).unwrap(), "uncertain");
    assert!(a.claim_task(&t, B, 101, 100).is_err());
    a.reconcile_safe_retry(&t).unwrap();
    let new = a.claim_task(&t, B, 110, 100).unwrap();
    assert!(new.fence > old.fence);
    assert!(a.checkpoint_task(&old, 120, "late", &json!({})).is_err());
    a.complete_task(&new, 120).unwrap();
}
#[test]
fn pause_resume_cancel_and_bounded_retry_work() {
    let (mut a, _) = pair();
    let t = a.queue_task("local", "control", &json!({})).unwrap();
    let old = a.claim_task(&t, A, 0, 100).unwrap();
    a.pause_task(&t).unwrap();
    assert!(a.complete_task(&old, 10).is_err());
    a.resume_task(&t).unwrap();
    let l = a.claim_task(&t, A, 20, 100).unwrap();
    a.fail_task(&l, 30).unwrap();
    a.retry_failed_task(&t).unwrap();
    let l = a.claim_task(&t, A, 40, 100).unwrap();
    a.fail_task(&l, 50).unwrap();
    a.retry_failed_task(&t).unwrap();
    assert!(a.claim_task(&t, A, 60, 100).is_err());
    a.cancel_task(&t).unwrap();
    assert_eq!(a.task_state(&t).unwrap(), "cancelled");
    assert!(a.resume_task(&t).is_err());
}
#[test]
fn authority_restart_invalidates_all_previous_boot_leases() {
    let path = std::env::temp_dir().join(format!("companion-lease-{}.sqlite3", id()));
    let (task, lease);
    {
        let mut s = Store::open(&path).unwrap();
        task = s.queue_task("local", "restart", &json!({})).unwrap();
        let device = s.device_id.clone();
        lease = s.claim_task(&task, &device, 0, 1000).unwrap();
    }
    {
        let mut s = Store::open(&path).unwrap();
        assert_eq!(s.task_state(&task).unwrap(), "uncertain");
        assert!(s.complete_task(&lease, 10).is_err());
    }
    fs::remove_file(path).unwrap();
}
#[test]
fn video_segmentation_covers_the_entire_timeline_including_tail() {
    let segments = video::plan_segments(605_000, 60_000, 2000).unwrap();
    assert_eq!(segments.first().unwrap().start_ms, 0);
    assert_eq!(segments.last().unwrap().end_ms, 605_000);
    assert_eq!(video::coverage_ms(605_000, &segments).unwrap(), 605_000);
    assert!(video::plan_segments(1000, 100, 100).is_err());
}
#[test]
fn video_progress_uses_union_and_joint_modal_coverage() {
    let visual = vec![
        Interval {
            start_ms: 0,
            end_ms: 60,
        },
        Interval {
            start_ms: 50,
            end_ms: 100,
        },
    ];
    assert_eq!(video::coverage_ms(100, &visual).unwrap(), 100);
    let mut c = AnalysisCoverage {
        duration_ms: 100,
        visual,
        audio: vec![Interval {
            start_ms: 0,
            end_ms: 35,
        }],
        uncertain: vec![],
        synthesis_saved: true,
    };
    assert_eq!(c.analyzed_ms().unwrap(), 35);
    assert!(!c.is_complete().unwrap());
    c.audio = vec![Interval {
        start_ms: 0,
        end_ms: 100,
    }];
    assert!(c.is_complete().unwrap());
    c.uncertain = vec![Interval {
        start_ms: 80,
        end_ms: 90,
    }];
    assert!(!c.is_complete().unwrap());
    assert!(
        video::coverage_ms(
            100,
            &[Interval {
                start_ms: 0,
                end_ms: 101
            }]
        )
        .is_err()
    );
}
fn rel(n: u64, kind: Kind) -> RelationshipEvent {
    RelationshipEvent {
        id: Uuid::from_u128(n as u128).to_string(),
        origin: A.into(),
        lamport: n as i64,
        evidence_hash: format!("{n:064x}"),
        topic: "drawing".into(),
        kind,
        confidence_milli: 900,
        source_verified: true,
        occurred_at_ms: n as i64 * 2_000_000,
        rule_version: 1,
    }
}
#[test]
fn ordinary_chat_never_mechanically_adds_affection() {
    assert_eq!(
        relationship::recompute(&[rel(1, Kind::Ordinary)]).unwrap(),
        relationship::recompute(&[]).unwrap()
    );
}
#[test]
fn relationship_events_deduplicate_and_replay_order_is_stable() {
    let one = rel(1, Kind::RememberedPreference);
    let two = rel(2, Kind::SharedActivity);
    let expected = relationship::recompute(&[one.clone(), two.clone()]).unwrap();
    assert_eq!(
        relationship::recompute(&[two, one.clone(), one]).unwrap(),
        expected
    );
}
#[test]
fn relationship_cooldown_decay_caps_and_evidence_validation_apply() {
    let first = rel(1, Kind::ThoughtfulCare);
    let mut spam = rel(2, Kind::ThoughtfulCare);
    spam.occurred_at_ms = first.occurred_at_ms + 100;
    assert_eq!(
        relationship::recompute(&[first.clone(), spam]).unwrap(),
        relationship::recompute(std::slice::from_ref(&first)).unwrap()
    );
    let many: Vec<_> = (1..100).map(|n| rel(n, Kind::ThoughtfulCare)).collect();
    let s = relationship::recompute(&many).unwrap();
    assert!(s.dimensions.iter().all(|v| (0..=100000).contains(v)));
    let mut unverified = first;
    unverified.source_verified = false;
    assert_eq!(
        relationship::recompute(&[unverified])
            .unwrap()
            .applied_events,
        0
    );
    let mut invalid = rel(1, Kind::ThoughtfulCare);
    invalid.confidence_milli = 1001;
    assert!(relationship::recompute(&[invalid]).is_err());
}
#[test]
fn empty_incremental_vector_can_bootstrap_a_new_replica() {
    let (mut a, mut b) = pair();
    a.put(Entity::Memory, &id(), json!({"text":"bootstrap"}))
        .unwrap();
    let batch = a.export_after(&BTreeMap::new()).unwrap();
    b.import_lab_batch(&batch).unwrap();
    assert_eq!(b.memories().unwrap()[0].text, "bootstrap");
}
