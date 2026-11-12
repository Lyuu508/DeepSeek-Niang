//! Versioned deterministic rule prototype. Does not infer events or write scores from an LLM.
use crate::{Error, Result, event::MAX_CLOCK};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Ordinary,
    ThoughtfulCare,
    RememberedPreference,
    SharedActivity,
    Reconciliation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvent {
    pub id: String,
    pub origin: String,
    pub lamport: i64,
    pub evidence_hash: String,
    pub topic: String,
    pub kind: Kind,
    pub confidence_milli: u16,
    pub source_verified: bool,
    pub occurred_at_ms: i64,
    pub rule_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationshipState {
    /// Affection, trust, intimacy, comfort, initiative; integer milli-points.
    pub dimensions: [i32; 5],
    pub applied_events: usize,
    pub rule_version: u32,
}
pub fn recompute(events: &[RelationshipEvent]) -> Result<RelationshipState> {
    if events.len() > 100_000 {
        return Err(Error::Invalid("relationship replay limit"));
    }
    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| (a.lamport, &a.origin, &a.id).cmp(&(b.lamport, &b.origin, &b.id)));
    let mut evidence = BTreeSet::new();
    let mut topics: BTreeMap<String, u32> = BTreeMap::new();
    let mut cooldown: BTreeMap<(Kind, String), i64> = BTreeMap::new();
    let mut daily: BTreeMap<i64, i32> = BTreeMap::new();
    let mut state = RelationshipState {
        dimensions: [15000, 10000, 0, 30000, 10000],
        applied_events: 0,
        rule_version: 1,
    };
    for e in sorted {
        if e.rule_version != 1 {
            return Err(Error::Invalid("relationship rule version"));
        }
        if e.lamport <= 0
            || e.lamport > MAX_CLOCK
            || Uuid::parse_str(&e.id).is_err()
            || Uuid::parse_str(&e.origin).is_err()
            || e.confidence_milli > 1000
            || e.occurred_at_ms < 0
            || e.evidence_hash.len() != 64
            || !e.evidence_hash.bytes().all(|b| b.is_ascii_hexdigit())
            || e.topic.len() > 256
        {
            return Err(Error::Invalid("relationship evidence"));
        }
        if !e.source_verified
            || e.confidence_milli < 700
            || !evidence.insert(e.evidence_hash.clone())
        {
            continue;
        }
        let base = match e.kind {
            Kind::Ordinary => continue,
            Kind::ThoughtfulCare => [800, 300, 100, 1000, 200],
            Kind::RememberedPreference => [500, 1000, 600, 400, 200],
            Kind::SharedActivity => [500, 400, 600, 800, 400],
            Kind::Reconciliation => [200, 700, 200, 900, 100],
        };
        let key = (e.kind, e.topic.clone());
        if cooldown
            .get(&key)
            .is_some_and(|last| e.occurred_at_ms.saturating_sub(*last) < 1_800_000)
        {
            continue;
        }
        let used = daily.entry(e.occurred_at_ms / 86_400_000).or_default();
        let repeats = topics.entry(e.topic).or_default();
        let divisor = 1 + (*repeats).min(9) as i32;
        let delta = base.map(|d| (d / divisor).min(5000 - *used));
        let max_delta = delta.into_iter().max().unwrap_or(0);
        if max_delta <= 0 {
            continue;
        }
        for (value, d) in state.dimensions.iter_mut().zip(delta) {
            *value = (*value + d).clamp(0, 100000);
        }
        *used += max_delta;
        *repeats += 1;
        cooldown.insert(key, e.occurred_at_ms);
        state.applied_events += 1;
    }
    Ok(state)
}
