use crate::{Error, Result, chat::now_ms, event::sha256_hex, persona::Persona, storage::Store};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

pub(crate) const LIFECYCLE: &str = "persona_lifecycle";
pub(crate) const ERASED_HISTORY: &str = "erased_persona_history";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Decision {
    pub persona_id: String,
    pub profile_id: String,
    pub main_conversation_id: String,
    pub decision_id: String,
    pub state: String,
    pub changed_at_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecycleEntryId {
    pub persona_id: String,
    pub decision_id: String,
}

#[derive(Serialize)]
pub struct RecycledPersona {
    pub persona: Persona,
    pub personality: String,
    pub entry: RecycleEntryId,
    pub changed_at_ms: i64,
    pub sync_state: &'static str,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecycleAction {
    Trash { persona_id: String },
    Restore { entry: RecycleEntryId },
    Purge { entries: Vec<RecycleEntryId> },
}

fn uuid(value: &str) -> Result<()> {
    Uuid::parse_str(value).map_err(|_| Error::Invalid("persona recycle identity"))?;
    Ok(())
}

pub(crate) fn read_decision(db: &Connection, id: &str) -> Result<Option<Decision>> {
    Ok(db
        .query_row(
            "SELECT profile_id,main_conversation_id,decision_id,state,changed_at_ms
         FROM persona_lifecycle WHERE persona_id=?1",
            [id],
            |r| {
                Ok(Decision {
                    persona_id: id.into(),
                    profile_id: r.get(0)?,
                    main_conversation_id: r.get(1)?,
                    decision_id: r.get(2)?,
                    state: r.get(3)?,
                    changed_at_ms: r.get(4)?,
                })
            },
        )
        .optional()?)
}

pub(crate) fn validate_decision(db: &Connection, decision: &Decision) -> Result<()> {
    for value in [
        &decision.persona_id,
        &decision.profile_id,
        &decision.main_conversation_id,
        &decision.decision_id,
    ] {
        uuid(value)?;
    }
    if decision.persona_id == decision.profile_id
        || decision.changed_at_ms < 0
        || !matches!(
            decision.state.as_str(),
            "active" | "trashed" | "permanently_deleted"
        )
    {
        return Err(Error::Invalid("persona lifecycle"));
    }
    let profile: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM local_profiles p WHERE p.id=?1 AND
         (EXISTS(SELECT 1 FROM lan_profiles l WHERE l.profile_id=p.id)
          OR EXISTS(SELECT 1 FROM devices d WHERE d.profile_id=p.id AND d.trust_state='local')))",
        [&decision.profile_id],
        |r| r.get(0),
    )?;
    let live: Option<(String, String, bool)> = db
        .query_row(
            "SELECT profile_id,main_conversation_id,is_builtin FROM personas WHERE id=?1",
            [&decision.persona_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let previous = read_decision(db, &decision.persona_id)?;
    if !profile
        || live.as_ref().is_some_and(|(p,c,b)| *b || p != &decision.profile_id || c != &decision.main_conversation_id)
        || previous.is_some_and(|p| p.profile_id != decision.profile_id || p.main_conversation_id != decision.main_conversation_id)
        || db.query_row(
            "SELECT EXISTS(SELECT 1 FROM personas WHERE main_conversation_id=?1 AND id<>?2)
                 OR EXISTS(SELECT 1 FROM conversation_personas WHERE conversation_id=?1 AND persona_id<>?2)
                 OR EXISTS(SELECT 1 FROM conversations WHERE id=?1 AND profile_id<>?3)",
            params![decision.main_conversation_id, decision.persona_id, decision.profile_id], |r| r.get::<_,bool>(0),
        )?
    {
        return Err(Error::Untrusted);
    }
    Ok(())
}

fn record(db: &Connection, decision: &Decision) -> Result<()> {
    db.execute(
        "INSERT INTO persona_lifecycle(persona_id,profile_id,main_conversation_id,decision_id,state,changed_at_ms)
         VALUES(?1,?2,?3,?4,?5,?6)
         ON CONFLICT(persona_id) DO UPDATE SET decision_id=excluded.decision_id,
                 state=excluded.state,changed_at_ms=excluded.changed_at_ms",
        params![decision.persona_id, decision.profile_id, decision.main_conversation_id,
                decision.decision_id, decision.state, decision.changed_at_ms],
    )?;
    Ok(())
}

pub(crate) fn enact(db: &Connection, decision: &Decision) -> Result<()> {
    validate_decision(db, decision)?;
    record(db, decision)?;
    if decision.state == "active" {
        return Ok(());
    }
    let id = &decision.persona_id;
    // Protect only the matching identity. The fallback uses the same local or paired policy as persona selection.
    let selected: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM persona_selection WHERE persona_id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if selected {
        let fallback: String = db.query_row(
            "SELECT p.id FROM personas p WHERE p.id<>?1 AND NOT EXISTS(SELECT 1 FROM persona_links WHERE alias_id=p.id)
             AND NOT EXISTS(SELECT 1 FROM persona_lifecycle l WHERE l.persona_id=p.id AND l.state<>'active')
             AND (p.profile_id IN (SELECT profile_id FROM devices WHERE trust_state='local') OR p.profile_id IN (SELECT profile_id FROM lan_profiles))
             ORDER BY p.is_builtin DESC,p.created_at_ms,p.id LIMIT 1", [id], |r| r.get(0),
        ).optional()?.ok_or(Error::Invalid("PERSONA_PROTECTED"))?;
        db.execute(
            "UPDATE persona_selection SET persona_id=?2 WHERE persona_id=?1",
            params![id, fallback],
        )?;
    }
    let suppressed: i64 = db.query_row("SELECT suppress FROM lan_control WHERE id=1", [], |r| {
        r.get(0)
    })?;
    db.execute("UPDATE lan_control SET suppress=1 WHERE id=1", [])?;
    if decision.state == "permanently_deleted" {
        purge(db, decision)?;
    } else {
        pause(db, id)?;
    }
    db.execute(
        "UPDATE lan_control SET suppress=?1 WHERE id=1",
        [suppressed],
    )?;
    Ok(())
}

fn pause(db: &Connection, id: &str) -> Result<()> {
    db.execute(
        "UPDATE chat_generations SET state='interrupted',error_code='PERSONA_TRASHED',revision=revision+1
         WHERE state IN ('sending','generating') AND request_id IN
           (SELECT r.id FROM chat_requests r JOIN conversation_personas p ON p.conversation_id=r.conversation_id WHERE p.persona_id=?1)", [id],
    )?;
    db.execute(
        "UPDATE chat_requests SET state='interrupted',error_code='PERSONA_TRASHED'
         WHERE state IN ('sending','generating') AND conversation_id IN
           (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1)",
        [id],
    )?;
    db.execute(
        "UPDATE budget_reservations SET state='uncertain' WHERE state='reserved' AND task_id IN
           (SELECT id FROM task_queue WHERE persona_id=?1 AND state='running')",
        [id],
    )?;
    db.execute(
        "UPDATE task_queue SET state=CASE WHEN state='running' AND kind='companion_proactive' THEN 'uncertain' ELSE 'paused' END,
           fence=fence+1,lease_boot_id=NULL,lease_until_tick=NULL,last_error='PERSONA_TRASHED'
         WHERE persona_id=?1 AND state IN ('queued','running')", [id],
    )?;
    db.execute("UPDATE companion_activities SET state='paused' WHERE persona_id=?1 AND state IN ('queued','running')", [id])?;
    db.execute("UPDATE video_progress SET analysis_state='paused',playback_state='stopped' WHERE analysis_state IN ('queued','running','analyzing','decoding','organizing') AND task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)", [id])?;
    Ok(())
}

fn target(
    db: &Connection,
    profile: &str,
    id: &str,
    state: &str,
    expected: Option<&str>,
) -> Result<Decision> {
    uuid(id)?;
    let (owner, conversation, builtin): (String, String, bool) = db
        .query_row(
            "SELECT profile_id,main_conversation_id,is_builtin FROM personas WHERE id=?1 AND
          (profile_id=?2 OR profile_id IN (SELECT profile_id FROM lan_profiles))",
            params![id, profile],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(Error::Invalid("PERSONA_RECYCLE_CHANGED"))?;
    if builtin {
        return Err(Error::Invalid("PERSONA_PROTECTED"));
    }
    if state == "trashed" && !db.query_row(
        "SELECT EXISTS(SELECT 1 FROM personas p WHERE p.id<>?1
            AND (p.profile_id=?2 OR p.profile_id IN (SELECT profile_id FROM lan_profiles))
            AND NOT EXISTS(SELECT 1 FROM persona_links WHERE alias_id=p.id)
            AND NOT EXISTS(SELECT 1 FROM persona_lifecycle WHERE persona_id=p.id AND state<>'active'))",
        params![id,profile], |r| r.get::<_,bool>(0),
    )? {
        return Err(Error::Invalid("PERSONA_PROTECTED"));
    }
    let current = read_decision(db, id)?;
    match expected {
        Some(token)
            if !current
                .as_ref()
                .is_some_and(|p| p.state == "trashed" && p.decision_id == token) =>
        {
            return Err(Error::Invalid("PERSONA_RECYCLE_CHANGED"));
        }
        None if current.as_ref().is_some_and(|p| p.state != "active") => {
            return Err(Error::Invalid("PERSONA_RECYCLE_CHANGED"));
        }
        _ => {}
    }
    Ok(Decision {
        persona_id: id.into(),
        profile_id: owner,
        main_conversation_id: conversation,
        decision_id: Uuid::new_v4().to_string(),
        state: state.into(),
        changed_at_ms: now_ms(),
    })
}

impl Store {
    pub fn recycled_personas(&self) -> Result<Vec<RecycledPersona>> {
        let peers = self
            .lan_peers()?
            .into_iter()
            .filter(|p| !p.revoked)
            .collect::<Vec<_>>();
        let mut query = self.conn.prepare(
            "SELECT p.id,p.name,p.main_conversation_id,p.created_at_ms,p.revision,p.avatar_resource,
                    l.decision_id,l.changed_at_ms,coalesce(json_extract(c.settings_json,'$.personality'),'')
             FROM persona_lifecycle l JOIN personas p ON p.id=l.persona_id
             LEFT JOIN persona_chat_preferences c ON c.persona_id=p.id
             WHERE l.state='trashed' AND p.is_builtin=0 AND
                 (p.profile_id=?1 OR p.profile_id IN (SELECT profile_id FROM lan_profiles))
             ORDER BY l.changed_at_ms DESC,p.id",
        )?;
        let rows = query.query_map([&self.profile_id], |row| {
            let id: String = row.get(0)?;
            Ok(RecycledPersona {
                persona: Persona {
                    id: id.clone(),
                    name: row.get(1)?,
                    main_conversation_id: row.get(2)?,
                    is_builtin: false,
                    created_at_ms: row.get(3)?,
                    revision: row.get(4)?,
                    avatar_resource: row.get(5)?,
                    personality: row.get(8)?,
                },
                entry: RecycleEntryId {
                    persona_id: id,
                    decision_id: row.get(6)?,
                },
                changed_at_ms: row.get(7)?,
                personality: row.get(8)?,
                sync_state: "local",
            })
        })?;
        rows.map(|row| {
            let mut item = row?;
            if !peers.is_empty() {
                let event: Option<(String,i64)> = self.conn.query_row(
                    "SELECT origin,seq FROM lan_events WHERE json_extract(wire,'$.table')='persona_lifecycle'
                     AND json_extract(wire,'$.row.decision_id')=?1 LIMIT 1",
                    [&item.entry.decision_id], |row| Ok((row.get(0)?,row.get(1)?)),
                ).optional()?;
                item.sync_state = match event {
                    Some((origin,seq)) if origin == self.device_id && peers.iter().all(|p| p.acknowledged_seq >= seq) => "acknowledged",
                    Some((origin,_)) if origin != self.device_id => "received",
                    _ => "pending",
                };
            }
            Ok(item)
        }).collect()
    }

    pub fn change_recycle(&mut self, action: RecycleAction) -> Result<Vec<String>> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let decisions = match action {
            RecycleAction::Trash { persona_id } => {
                vec![target(&tx, &self.profile_id, &persona_id, "trashed", None)?]
            }
            RecycleAction::Restore { entry } => vec![target(
                &tx,
                &self.profile_id,
                &entry.persona_id,
                "active",
                Some(&entry.decision_id),
            )?],
            RecycleAction::Purge { entries } => {
                if entries.is_empty() || entries.len() > 300 {
                    return Err(Error::Invalid("PERSONA_RECYCLE_CHANGED"));
                }
                let mut seen = std::collections::BTreeSet::new();
                entries
                    .iter()
                    .map(|entry| {
                        if !seen.insert(&entry.persona_id) {
                            return Err(Error::Invalid("PERSONA_RECYCLE_CHANGED"));
                        }
                        target(
                            &tx,
                            &self.profile_id,
                            &entry.persona_id,
                            "permanently_deleted",
                            Some(&entry.decision_id),
                        )
                    })
                    .collect::<Result<Vec<_>>>()?
            }
        };
        for decision in &decisions {
            enact(&tx, decision)?;
        }
        tx.commit()?;
        Ok(decisions
            .into_iter()
            .map(|d| d.main_conversation_id)
            .collect())
    }
}

// Child rows precede parents so membership subqueries remain available. Only fixed predicates
// enter SQL. User identifiers are bound at every query.
const CLEANUP: &[(&str, &str)] = &[
    (
        "companion_receipts",
        "proactive_id IN (SELECT id FROM companion_proactive WHERE persona_id=?1)",
    ),
    ("companion_proactive", "persona_id=?1"),
    ("companion_mood_events", "persona_id=?1"),
    ("companion_bond_events", "persona_id=?1"),
    ("companion_memory_meta", "persona_id=?1"),
    ("companion_memory_candidates", "persona_id=?1"),
    ("companion_life_events", "persona_id=?1"),
    ("companion_activities", "persona_id=?1"),
    ("companion_hosts", "persona_id=?1"),
    ("companion_preferences", "persona_id=?1"),
    (
        "companion_processed_sources",
        "message_id IN (SELECT id FROM messages WHERE conversation_id IN (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1))",
    ),
    (
        "video_memories",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1) OR source_id IN (SELECT id FROM video_sources WHERE persona_id=?1) OR memory_id IN (SELECT id FROM memories WHERE persona_id=?1)",
    ),
    (
        "video_analysis_results",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "video_timeline_events",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "video_segments",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "video_progress",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "video_tasks",
        "id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "video_local_media",
        "source_id IN (SELECT id FROM video_sources WHERE persona_id=?1)",
    ),
    (
        "media_interests",
        "source_id IN (SELECT id FROM video_sources WHERE persona_id=?1) OR interest_id IN (SELECT id FROM character_interests WHERE persona_id=?1)",
    ),
    ("video_sources", "persona_id=?1"),
    ("meme_knowledge", "persona_id=?1"),
    ("character_interests", "persona_id=?1"),
    (
        "notification_deliveries",
        "proactive_message_id IN (SELECT id FROM proactive_messages WHERE persona_id=?1)",
    ),
    ("proactive_messages", "persona_id=?1"),
    ("activity_states", "persona_id=?1"),
    ("life_events", "persona_id=?1"),
    ("daily_activities", "persona_id=?1"),
    ("life_plans", "persona_id=?1"),
    ("api_usage_records", "persona_id=?1"),
    (
        "task_checkpoints",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "task_logs",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    (
        "budget_reservations",
        "task_id IN (SELECT id FROM task_queue WHERE persona_id=?1)",
    ),
    ("task_queue", "persona_id=?1"),
    ("relationship_events", "persona_id=?1"),
    ("character_moods", "persona_id=?1"),
    (
        "memory_embeddings",
        "memory_id IN (SELECT id FROM memories WHERE persona_id=?1)",
    ),
    (
        "memory_relations",
        "from_memory_id IN (SELECT id FROM memories WHERE persona_id=?1) OR to_memory_id IN (SELECT id FROM memories WHERE persona_id=?1)",
    ),
    ("memories", "persona_id=?1"),
    (
        "chat_turn_choices",
        "user_message_id IN (SELECT id FROM messages WHERE conversation_id IN (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1))",
    ),
    (
        "chat_generations",
        "request_id IN (SELECT id FROM chat_requests WHERE conversation_id IN (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1))",
    ),
    (
        "chat_requests",
        "conversation_id IN (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1)",
    ),
    (
        "messages",
        "conversation_id IN (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1) OR conversation_id=(SELECT main_conversation_id FROM persona_lifecycle WHERE persona_id=?1)",
    ),
    ("persona_states", "persona_id=?1"),
    ("persona_chat_preferences", "persona_id=?1"),
    ("conversation_personas", "persona_id=?1"),
    ("personas", "id=?1 AND is_builtin=0"),
    (
        "conversations",
        "json_array(id) IN (SELECT row_key FROM persona_erased_keys WHERE persona_id=?1 AND table_name='conversations')",
    ),
];

const LEGACY_ENTITIES: &str = "(entity_type,entity_id) IN
    (SELECT CASE table_name WHEN 'messages' THEN 'message' ELSE 'memory' END,json_extract(row_key,'$[0]')
     FROM persona_erased_keys WHERE table_name IN ('messages','memories') AND persona_id=?1)";

pub(crate) fn remember_key(db: &Connection, table: &str, key: &str, id: &str) -> Result<()> {
    db.execute(
        "INSERT OR IGNORE INTO persona_erased_keys VALUES(?1,?2,?3)",
        params![table, key, id],
    )?;
    Ok(())
}

fn inventory(db: &Connection, table: &str, predicate: &str, id: &str) -> Result<()> {
    if let Ok(keys) = crate::replication::key_fields(table) {
        db.execute(
            &format!(
                "INSERT OR IGNORE INTO persona_erased_keys
           SELECT '{table}',json_array({}),?1 FROM {table} WHERE {predicate}",
                keys.join(",")
            ),
            [id],
        )?;
    }
    Ok(())
}

fn purge(db: &Connection, decision: &Decision) -> Result<()> {
    let id = &decision.persona_id;
    db.pragma_update(None, "defer_foreign_keys", "ON")?;
    remember_key(db, "personas", &json!([id]).to_string(), id)?;
    remember_key(
        db,
        "conversations",
        &json!([decision.main_conversation_id]).to_string(),
        id,
    )?;
    inventory(
        db,
        "conversations",
        "id IN (SELECT conversation_id FROM conversation_personas WHERE persona_id=?1)",
        id,
    )?;
    for (table, predicate) in CLEANUP {
        inventory(db, table, predicate, id)?;
    }
    for table in ["data_tombstones", "sync_events"] {
        inventory(db, table, LEGACY_ENTITIES, id)?;
    }
    redact_history(db, id)?;
    for (table, predicate) in CLEANUP {
        db.execute(&format!("DELETE FROM {table} WHERE {predicate}"), [id])?;
    }
    for table in ["data_tombstones", "sync_events"] {
        db.execute(
            &format!("DELETE FROM {table} WHERE {LEGACY_ENTITIES}"),
            [id],
        )?;
    }
    db.execute(
        "DELETE FROM application_settings WHERE scope='device' AND key='voice:'||?1",
        [id],
    )?;
    Ok(())
}

fn redact_history(db: &Connection, id: &str) -> Result<()> {
    for (table, _) in crate::replication::TABLES {
        let key = crate::replication::key_fields(table)?
            .iter()
            .map(|field| format!("json_extract(coalesce(after_json,before_json),'$.{field}')"))
            .collect::<Vec<_>>()
            .join(",");
        db.execute(&format!(
            "DELETE FROM lan_outbox WHERE table_name=?1 AND EXISTS(SELECT 1 FROM persona_erased_keys k
             WHERE k.table_name=?1 AND k.row_key=json_array({key}) AND k.persona_id=?2)"), params![table,id])?;
    }
    for table in ["lan_cells", "lan_conflicts", "lan_deleted"] {
        db.execute(
            &format!(
                "DELETE FROM {table} WHERE (table_name,row_key) IN
           (SELECT table_name,row_key FROM persona_erased_keys WHERE persona_id=?1)"
            ),
            [id],
        )?;
    }
    loop {
        let mut query = db.prepare(
            "SELECT e.id,e.wire,d.trust_state='local' FROM lan_events e LEFT JOIN devices d ON d.id=e.origin
             WHERE (json_extract(e.wire,'$.table'),json_extract(e.wire,'$.key')) IN
               (SELECT table_name,row_key FROM persona_erased_keys WHERE persona_id=?1) LIMIT 128")?;
        let page = query
            .query_map([id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<bool>>(2)?.unwrap_or(false),
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if page.is_empty() {
            break;
        }
        for (event, wire, local) in page {
            let digest = sha256_hex(wire.as_bytes());
            if local {
                db.execute(
                    "INSERT OR IGNORE INTO lan_erasure_queue VALUES(?1,?2,?3)",
                    params![event, id, digest],
                )?;
            }
            db.execute(
                "UPDATE lan_events SET wire=?2 WHERE id=?1",
                params![event, erased_wire(&digest)],
            )?;
        }
    }
    Ok(())
}

pub(crate) fn erased_wire(digest: &str) -> String {
    json!({"erased_digest":digest}).to_string()
}

pub(crate) fn erased_owner(
    db: &Connection,
    table: &str,
    key: &str,
    row: &Value,
) -> Result<Option<String>> {
    fn owner(db: &Connection, table: &str, key: &str) -> Result<Option<String>> {
        Ok(db
            .query_row(
                "SELECT persona_id FROM persona_erased_keys WHERE table_name=?1 AND row_key=?2",
                params![table, key],
                |r| r.get(0),
            )
            .optional()?)
    }
    if let Some(id) = owner(db, table, key)? {
        return Ok(Some(id));
    }
    if let Some(id) = row["persona_id"].as_str()
        && read_decision(db, id)?.is_some_and(|p| p.state == "permanently_deleted")
    {
        return Ok(Some(id.into()));
    }
    for (field, parent) in [
        ("conversation_id", "conversations"),
        ("task_id", "task_queue"),
        ("request_id", "chat_requests"),
        ("user_message_id", "messages"),
        ("source_message_id", "messages"),
        ("proactive_id", "companion_proactive"),
        ("from_memory_id", "memories"),
        ("to_memory_id", "memories"),
        ("source_event_id", "sync_events"),
        ("delete_event_id", "sync_events"),
        ("version_event_id", "sync_events"),
    ] {
        if let Some(reference) = row[field].as_str()
            && let Some(id) = owner(db, parent, &json!([reference]).to_string())?
        {
            return Ok(Some(id));
        }
    }
    if matches!(table, "sync_events" | "data_tombstones") {
        let parent = match row["entity_type"].as_str() {
            Some("message") => "messages",
            Some("memory") => "memories",
            _ => return Ok(None),
        };
        if let Some(id) = owner(db, parent, &json!([row["entity_id"]]).to_string())? {
            return Ok(Some(id));
        }
        // A newly created offline message's legacy envelope comes before its SQL row.
        if table == "sync_events"
            && let Some(envelope) = row["envelope_json"].as_str()
            && let Ok(Value::Object(event)) = serde_json::from_str::<Value>(envelope)
        {
            let payload = &event["payload"];
            if let Some(thread) = payload["conversation_id"].as_str()
                && let Some(id) = owner(db, "conversations", &json!([thread]).to_string())?
            {
                return Ok(Some(id));
            }
            if let Some(persona) = payload["persona_id"].as_str()
                && read_decision(db, persona)?.is_some_and(|p| p.state == "permanently_deleted")
            {
                return Ok(Some(persona.into()));
            }
        }
    }
    Ok(None)
}
