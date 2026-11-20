use crate::event::sha256_hex;
use crate::{
    Error, Result,
    event::{Entity, Event, MAX_BATCH, MAX_CLOCK, Operation},
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, time::Duration};
use uuid::Uuid;

const MIGRATION: &str = include_str!("../../../db/migrations/0001_initial.sql");

const CHAT_MIGRATION: &str = include_str!("../../../db/migrations/0002_chat.sql");
const LAN_MIGRATION: &str = include_str!("../../../db/migrations/0004_lan_sync.sql");
const PERSONA_MIGRATION: &str = include_str!("../../../db/migrations/0003_personas.sql");
const PAIRING_MIGRATION: &str =
    include_str!("../../../db/migrations/0005_pairing_persona_identity.sql");
const RECALL_MIGRATION: &str = include_str!("../../../db/migrations/0007_chat_recall.sql");
const COMPANION_MIGRATION: &str =
    include_str!("../../../db/migrations/0006_companion_foundation.sql");

pub struct Store {
    pub(crate) conn: Connection,
    pub device_id: String,
    pub profile_id: String,
    pub(crate) boot_id: String,
    pub(crate) authority_epoch: i64,
    _file_lock: Option<std::fs::File>,
}
pub const SCHEMA_VERSION: u32 = 9;
#[derive(Debug, Serialize)]
pub struct MemoryRecord {
    pub id: String,
    pub text: String,
    pub version_lamport: i64,
}
#[derive(Debug, Serialize)]
pub struct Diagnostics {
    pub version: &'static str,
    pub sqlite_version: String,
    pub schema_version: u32,
    pub device_id: String,
    pub profile_id: String,
    pub message_count: i64,
    pub memory_count: i64,
    pub event_count: i64,
    pub integrity: String,
}
impl Store {
    /// Application entry: reuse the local identity already stored in this DB.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let guard = Self::lock_file(path.as_ref())?;
        let conn = Self::connect(path)?;
        let identity: Option<(String, String)> = conn
            .query_row(
                "SELECT id,profile_id FROM devices WHERE trust_state='local' LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (device, profile) =
            identity.unwrap_or_else(|| (Uuid::new_v4().to_string(), Uuid::new_v4().to_string()));
        Self::initialize(conn, &device, &profile, guard)
    }
    /// Explicit identities for the isolated two-replica protocol laboratory.
    #[cfg(feature = "lab")]
    pub fn open_lab(path: impl AsRef<Path>, device: &str, profile: &str) -> Result<Self> {
        let guard = Self::lock_file(path.as_ref())?;
        Self::initialize(Self::connect(path)?, device, profile, guard)
    }
    fn lock_file(path: &Path) -> Result<Option<std::fs::File>> {
        if path == Path::new(":memory:") {
            return Ok(None);
        }
        let name = path.as_os_str().to_os_string();
        let mut lock_name = name;
        lock_name.push(".lock");
        let guard = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_name)
            .map_err(|_| Error::Invalid("database lock unavailable"))?;
        guard.try_lock().map_err(|_| Error::DatabaseInUse)?;
        Ok(Some(guard))
    }
    fn connect(path: impl AsRef<Path>) -> Result<Connection> {
        let mut conn = Connection::open(path.as_ref())?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(Error::Schema);
        }
        let migrations = [
            (1, MIGRATION),
            (2, CHAT_MIGRATION),
            (3, PERSONA_MIGRATION),
            (4, LAN_MIGRATION),
            (5, PAIRING_MIGRATION),
            (6, COMPANION_MIGRATION),
            (7, RECALL_MIGRATION),
            (
                8,
                include_str!("../../../db/migrations/0008_life_tasks.sql"),
            ),
            (
                9,
                include_str!("../../../db/migrations/0009_local_video.sql"),
            ),
        ];
        for (number, sql) in migrations {
            let digest = sha256_hex(sql.as_bytes());
            if number <= version {
                let saved: String = conn.query_row(
                    "SELECT checksum FROM schema_migrations WHERE version=?1",
                    [number],
                    |r| r.get(0),
                )?;
                if saved != digest {
                    return Err(Error::Schema);
                }
            }
        }
        // A consistent SQLite backup, including committed WAL, before upgrading old data.
        let path = path.as_ref();
        if (1..SCHEMA_VERSION).contains(&version) && path != Path::new(":memory:") {
            let backup =
                path.with_extension(format!("pre-v{SCHEMA_VERSION}-{}.bak", Uuid::new_v4()));
            conn.backup("main", backup, rusqlite::backup::NO_PROGRESS)?;
        }
        if version < SCHEMA_VERSION {
            let preserved = if version > 0 {
                [
                    "messages",
                    "sync_events",
                    "memories",
                    "api_usage_records",
                    "task_queue",
                    "personas",
                    "devices",
                    "lan_peers",
                    "persona_links",
                    "companion_activities",
                    "companion_life_events",
                    "companion_bond_events",
                    "companion_mood_events",
                    "companion_proactive",
                    "video_sources",
                    "video_tasks",
                    "video_segments",
                    "video_analysis_results",
                    "video_memories",
                ]
                .iter()
                .filter(|table| {
                    conn.query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                        [table],
                        |r| r.get::<_, bool>(0),
                    )
                    .unwrap_or(false)
                })
                .map(|table| {
                    Ok((
                        *table,
                        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                            r.get::<_, i64>(0)
                        })?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            // SQLite's documented table-rebuild procedure: this connection is private,
            // guarded by the application file lock and has not been exposed to the app.
            // Validate all FKs explicitly before commit, then restore enforcement.
            conn.pragma_update(None, "foreign_keys", "OFF")?;
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            for (number, sql) in migrations {
                if number > version {
                    tx.execute_batch(sql)?;
                    tx.execute(
                        "INSERT INTO schema_migrations(version,checksum) VALUES(?1,?2)",
                        params![number, sha256_hex(sql.as_bytes())],
                    )?;
                    tx.pragma_update(None, "user_version", number)?;
                }
            }
            for (table, count) in preserved {
                if tx.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                    r.get::<_, i64>(0)
                })? != count
                {
                    return Err(Error::Schema);
                }
            }
            let invalid:i64=tx.query_row("SELECT count(*) FROM messages m JOIN conversations c ON c.id=m.conversation_id WHERE m.profile_id<>c.profile_id",[],|r|r.get(0))?;
            let fk: i64 =
                tx.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                    r.get(0)
                })?;
            if invalid != 0 || fk != 0 {
                return Err(Error::Schema);
            }
            tx.commit()?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
            if !conn.pragma_query_value(None, "foreign_keys", |r| r.get::<_, bool>(0))? {
                return Err(Error::Schema);
            }
        }
        Ok(conn)
    }
    fn initialize(
        mut conn: Connection,
        device: &str,
        profile: &str,
        guard: Option<std::fs::File>,
    ) -> Result<Self> {
        if Uuid::parse_str(device).is_err() || Uuid::parse_str(profile).is_err() {
            return Err(Error::Invalid("identity UUID"));
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let other: u32 = tx.query_row(
            "SELECT count(*) FROM devices WHERE trust_state='local' AND id<>?1",
            [device],
            |r| r.get(0),
        )?;
        if other > 0 {
            return Err(Error::Invalid("database belongs to another local device"));
        }
        tx.execute(
            "INSERT OR IGNORE INTO local_profiles(id,display_name) VALUES(?1,'本地角色档案')",
            [profile],
        )?;
        tx.execute("INSERT OR IGNORE INTO devices(id,profile_id,name,trust_state) VALUES(?1,?2,'本机','local')",params![device,profile])?;
        let saved: (String, String) = tx.query_row(
            "SELECT profile_id,trust_state FROM devices WHERE id=?1",
            [device],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if saved != (profile.to_owned(), "local".to_owned()) {
            return Err(Error::Untrusted);
        }
        tx.execute(
            "INSERT OR IGNORE INTO local_clock(device_id) VALUES(?1)",
            [device],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO conversations(id,profile_id,title) VALUES(?1,?1,'默认对话')",
            [profile],
        )?;
        crate::persona::ensure_default(&tx, profile, device)?;
        // A process restart invalidates every old monotonic deadline and executor token.
        tx.execute("UPDATE task_queue SET state='uncertain',fence=fence+1,lease_boot_id=NULL,lease_until_tick=NULL,last_error='AUTHORITY_RESTART_RECONCILE' WHERE authority_device_id=?1 AND state='running'",[device])?;
        tx.execute("UPDATE budget_reservations SET state='uncertain' WHERE state='reserved' AND task_id IN (SELECT id FROM task_queue WHERE authority_device_id=?1 AND state='uncertain')",[device])?;
        tx.execute("INSERT OR IGNORE INTO api_usage_records(id,profile_id,persona_id,task_id,request_key,provider,model_id,currency,usage_state,created_at_ms,price_snapshot_json,purpose) SELECT b.request_key,t.profile_id,t.persona_id,t.id,b.request_key,'deepseek',coalesce(json_extract(c.settings_json,'$.model'),'unknown'),'unknown','unknown',b.created_at_ms,'{}','life' FROM budget_reservations b JOIN task_queue t ON t.id=b.task_id LEFT JOIN persona_chat_preferences c ON c.persona_id=t.persona_id WHERE b.state='uncertain' AND b.currency='TOKEN' AND t.kind='companion_proactive' AND t.authority_device_id=?1",[device])?;
        let epoch: i64 = tx.query_row("SELECT coalesce(max(authority_epoch),0)+1 FROM task_queue WHERE authority_device_id=?1",[device],|r|r.get(0))?;
        tx.execute(
            "UPDATE task_queue SET authority_epoch=?1 WHERE authority_device_id=?2",
            params![epoch, device],
        )?;
        tx.commit()?;
        let mut store = Self {
            conn,
            device_id: device.into(),
            profile_id: profile.into(),
            boot_id: Uuid::new_v4().to_string(),
            authority_epoch: epoch,
            _file_lock: guard,
        };
        for persona in store.personas()? {
            let settings = store.persona_settings(&persona.id)?;
            store.conn.execute("INSERT OR IGNORE INTO persona_chat_preferences(persona_id,settings_json) VALUES(?1,?2)",params![persona.id,serde_json::to_string(&settings)?])?;
        }
        store.recover_chat_requests()?;
        crate::replication::install_capture(&store.conn)?;
        Ok(store)
    }
    /// Lab-only trust fixture, not a pairing implementation. Never expose via IPC/network.
    #[cfg(feature = "lab")]
    pub fn register_lab_peer(&mut self, device: &str) -> Result<()> {
        if Uuid::parse_str(device).is_err() || device == self.device_id {
            return Err(Error::Invalid("peer UUID"));
        }
        self.conn.execute("INSERT INTO devices(id,profile_id,name,trust_state) VALUES(?1,?2,'实验节点','trusted') ON CONFLICT(id) DO NOTHING",params![device,self.profile_id])?;
        Ok(())
    }
    pub fn put(&mut self, entity: Entity, id: &str, mut payload: Value) -> Result<Event> {
        if !payload.is_object() {
            return Err(Error::Invalid("payload object required"));
        }
        if payload.get("persona_id").is_none() {
            payload["persona_id"] = Value::String(self.active_persona_id()?);
        }
        if entity == Entity::Message && payload.get("conversation_id").is_none() {
            payload["conversation_id"] = payload["persona_id"].clone();
        }
        self.append(entity, id, Operation::Put, payload)
    }
    pub fn delete(&mut self, entity: Entity, id: &str) -> Result<Event> {
        let owner:Option<String>=match entity {
            Entity::Memory=>self.conn.query_row("SELECT persona_id FROM memories WHERE id=?1",[id],|r|r.get(0)).optional()?,
            Entity::Message=>self.conn.query_row("SELECT c.persona_id FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE m.id=?1",[id],|r|r.get(0)).optional()?,
        };
        if owner.is_some_and(|p| Some(p) != self.active_persona_id().ok()) {
            return Err(Error::Untrusted);
        }
        self.delete_unscoped(entity, id)
    }
    pub(crate) fn delete_unscoped(&mut self, entity: Entity, id: &str) -> Result<Event> {
        self.append(entity, id, Operation::Delete, Value::Null)
    }
    fn append(
        &mut self,
        entity: Entity,
        id: &str,
        operation: Operation,
        payload: Value,
    ) -> Result<Event> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let data_profile: String = match payload.get("persona_id").and_then(Value::as_str) {
            Some(id) => tx.query_row("SELECT profile_id FROM personas WHERE id=?1", [id], |r| {
                r.get(0)
            })?,
            None => {
                let table = if entity == Entity::Message {
                    "messages"
                } else {
                    "memories"
                };
                tx.query_row(
                    &format!("SELECT profile_id FROM {table} WHERE id=?1"),
                    [id],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or_else(|| self.profile_id.clone())
            }
        };
        let event = Self::append_in(
            &tx,
            &self.device_id,
            &data_profile,
            entity,
            id,
            operation,
            payload,
        )?;
        tx.commit()?;
        Ok(event)
    }
    pub(crate) fn append_in(
        tx: &Transaction<'_>,
        device: &str,
        profile: &str,
        entity: Entity,
        id: &str,
        operation: Operation,
        payload: Value,
    ) -> Result<Event> {
        let (seq, clock): (i64, i64) = tx.query_row(
            "SELECT origin_seq,lamport FROM local_clock WHERE device_id=?1",
            [device],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if seq >= MAX_CLOCK || clock >= MAX_CLOCK {
            return Err(Error::Invalid("clock exhausted"));
        }
        let event = Event {
            schema_version: 1,
            profile_id: profile.to_owned(),
            event_id: Uuid::new_v4().to_string(),
            origin_device_id: device.to_owned(),
            origin_seq: seq + 1,
            lamport: clock + 1,
            entity,
            entity_id: id.into(),
            operation,
            payload,
            checksum: String::new(),
        }
        .seal()?;
        Self::apply(tx, &event)?;
        tx.execute(
            "UPDATE local_clock SET origin_seq=?1,lamport=?2 WHERE device_id=?3",
            params![seq + 1, clock + 1, device],
        )?;
        Self::update_checkpoint(tx, device)?;
        Ok(event)
    }
    /// Transport-free merge harness. No origin signature validation at this stage.
    /// A future mTLS adapter must NOT pass arbitrary peer JSON straight to this method.
    #[cfg(feature = "lab")]
    pub fn import_lab_batch(&mut self, events: &[Event]) -> Result<usize> {
        if events.len() > MAX_BATCH {
            return Err(Error::Invalid("batch too large"));
        }
        for event in events {
            event.validate()?;
            if event.profile_id != self.profile_id {
                return Err(Error::Untrusted);
            }
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut count = 0;
        for event in events {
            let trust: Option<String> = tx
                .query_row(
                    "SELECT trust_state FROM devices WHERE id=?1 AND profile_id=?2",
                    params![event.origin_device_id, self.profile_id],
                    |r| r.get(0),
                )
                .optional()?;
            if !matches!(trust.as_deref(), Some("local" | "trusted")) {
                return Err(Error::Untrusted);
            }
            // New events purporting to originate here indicate identity cloning/divergence.
            if event.origin_device_id == self.device_id && !Self::known_identical(&tx, event)? {
                return Err(Error::Conflict);
            }
            count += usize::from(Self::apply(&tx, event)?);
            tx.execute(
                "UPDATE local_clock SET lamport=max(lamport,?1) WHERE device_id=?2",
                params![event.lamport, self.device_id],
            )?;
            Self::update_checkpoint(&tx, &event.origin_device_id)?;
        }
        tx.commit()?;
        Ok(count)
    }
    fn known_identical(tx: &Transaction<'_>, e: &Event) -> Result<bool> {
        let mut stmt=tx.prepare("SELECT envelope_json FROM sync_events WHERE event_id=?1 OR (origin_device_id=?2 AND origin_seq=?3)")?;
        let rows = stmt.query_map(params![e.event_id, e.origin_device_id, e.origin_seq], |r| {
            r.get::<_, String>(0)
        })?;
        let mut found = false;
        for row in rows {
            let old: Event = serde_json::from_str(&row?)?;
            if old != *e {
                return Err(Error::Conflict);
            }
            found = true;
        }
        Ok(found)
    }
    fn apply(tx: &Transaction<'_>, e: &Event) -> Result<bool> {
        if Self::known_identical(tx, e)? {
            return Ok(false);
        }
        tx.execute("INSERT INTO sync_events(event_id,profile_id,origin_device_id,origin_seq,lamport,schema_version,entity_type,entity_id,operation,checksum,envelope_json) VALUES(?1,?2,?3,?4,?5,1,?6,?7,?8,?9,?10)",params![e.event_id,e.profile_id,e.origin_device_id,e.origin_seq,e.lamport,e.entity.as_str(),e.entity_id,if e.operation==Operation::Put {"put"} else {"delete"},e.checksum,serde_json::to_string(e)?])?;
        if e.operation == Operation::Delete {
            tx.execute("INSERT INTO data_tombstones(profile_id,entity_type,entity_id,delete_event_id,lamport,origin_device_id) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(profile_id,entity_type,entity_id) DO UPDATE SET delete_event_id=excluded.delete_event_id,lamport=excluded.lamport,origin_device_id=excluded.origin_device_id WHERE (excluded.lamport,excluded.origin_device_id,excluded.delete_event_id)>(data_tombstones.lamport,data_tombstones.origin_device_id,data_tombstones.delete_event_id)",params![e.profile_id,e.entity.as_str(),e.entity_id,e.event_id,e.lamport,e.origin_device_id])?;
            match e.entity {
                Entity::Message => {
                    tx.execute(
                        "UPDATE messages SET is_deleted=1,content='' WHERE id=?1",
                        [&e.entity_id],
                    )?;
                }
                Entity::Memory => {
                    tx.execute(
                        "UPDATE memories SET is_deleted=1,text='' WHERE id=?1",
                        [&e.entity_id],
                    )?;
                    tx.execute(
                        "DELETE FROM memory_embeddings WHERE memory_id=?1",
                        [&e.entity_id],
                    )?;
                }
            }
            return Ok(true);
        }
        let tombstoned: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM data_tombstones WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3)",params![e.profile_id,e.entity.as_str(),e.entity_id],|r|r.get(0))?;
        if tombstoned {
            return Ok(true);
        }
        match e.entity {
            Entity::Message => {
                let old: Option<String> = tx
                    .query_row(
                        "SELECT source_event_id FROM messages WHERE id=?1",
                        [&e.entity_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if old.is_some() {
                    return Err(Error::Conflict);
                }
                let conversation = e
                    .payload
                    .get("conversation_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&e.profile_id);
                let title = e
                    .payload
                    .get("conversation_title")
                    .and_then(Value::as_str)
                    .unwrap_or("同步会话");
                tx.execute(
                    "INSERT OR IGNORE INTO conversations(id,profile_id,title) VALUES(?1,?2,?3)",
                    params![conversation, e.profile_id, title],
                )?;
                let existing_owner: Option<String> = tx
                    .query_row(
                        "SELECT persona_id FROM conversation_personas WHERE conversation_id=?1",
                        [conversation],
                        |r| r.get(0),
                    )
                    .optional()?;
                let persona = e
                    .payload
                    .get("persona_id")
                    .and_then(Value::as_str)
                    .or(existing_owner.as_deref())
                    .unwrap_or(&e.profile_id);
                crate::persona::owner_exists(tx, persona, &e.profile_id)?;
                if existing_owner.as_deref().is_some_and(|p| p != persona) {
                    return Err(Error::Untrusted);
                }
                tx.execute("INSERT OR IGNORE INTO conversation_personas(conversation_id,persona_id,is_legacy) VALUES(?1,?2,?1<>?2)",params![conversation,persona])?;
                tx.execute("INSERT INTO messages(id,profile_id,conversation_id,role,content,origin_device_id,created_lamport,source_event_id,created_at_ms,model_id,request_id,reply_to_id,generation_id,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![e.entity_id,e.profile_id,conversation,e.payload["role"].as_str(),e.payload["content"].as_str(),e.origin_device_id,e.lamport,e.event_id,e.payload.get("created_at_ms").and_then(Value::as_i64),e.payload.get("model_id").and_then(Value::as_str),e.payload.get("request_id").and_then(Value::as_str),e.payload.get("reply_to_id").and_then(Value::as_str),e.payload.get("generation_id").and_then(Value::as_str),e.payload.get("status").and_then(Value::as_str).unwrap_or("complete")])?;
            }
            Entity::Memory => {
                let persona = e
                    .payload
                    .get("persona_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&e.profile_id);
                crate::persona::owner_exists(tx, persona, &e.profile_id)?;
                let old_owner: Option<String> = tx
                    .query_row(
                        "SELECT persona_id FROM memories WHERE id=?1",
                        [&e.entity_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if old_owner.as_deref().is_some_and(|p| p != persona) {
                    return Err(Error::Untrusted);
                }
                tx.execute("INSERT INTO memories(id,profile_id,persona_id,text,version_lamport,version_origin,version_event_id,importance_milli,sensitivity) VALUES(?1,?2,?3,?4,?5,?6,?7,coalesce(?8,500),coalesce(?9,'private')) ON CONFLICT(id) DO UPDATE SET text=excluded.text,version_lamport=excluded.version_lamport,version_origin=excluded.version_origin,version_event_id=excluded.version_event_id,importance_milli=coalesce(?8,memories.importance_milli),sensitivity=coalesce(?9,memories.sensitivity) WHERE (excluded.version_lamport,excluded.version_origin,excluded.version_event_id)>(memories.version_lamport,memories.version_origin,memories.version_event_id)",params![e.entity_id,e.profile_id,persona,e.payload["text"].as_str(),e.lamport,e.origin_device_id,e.event_id,e.payload.get("importance_milli").and_then(Value::as_i64),e.payload.get("sensitivity").and_then(Value::as_str)])?;
            }
        }
        Ok(true)
    }
    fn update_checkpoint(tx: &Transaction<'_>, origin: &str) -> Result<()> {
        let current: i64 = tx
            .query_row(
                "SELECT contiguous_seq FROM sync_checkpoints WHERE origin_device_id=?1",
                [origin],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        let mut next = current;
        let mut stmt=tx.prepare("SELECT origin_seq FROM sync_events WHERE origin_device_id=?1 AND origin_seq>?2 ORDER BY origin_seq")?;
        for row in stmt.query_map(params![origin, current], |r| r.get::<_, i64>(0))? {
            let seq = row?;
            if seq == next + 1 {
                next = seq;
            } else {
                break;
            }
        }
        tx.execute("INSERT INTO sync_checkpoints(origin_device_id,contiguous_seq) VALUES(?1,?2) ON CONFLICT(origin_device_id) DO UPDATE SET contiguous_seq=excluded.contiguous_seq",params![origin,next])?;
        Ok(())
    }
    pub fn checkpoints(&self) -> Result<BTreeMap<String, i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT origin_device_id,contiguous_seq FROM sync_checkpoints")?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn export_after(&self, peer: &BTreeMap<String, i64>) -> Result<Vec<Event>> {
        // Per-origin capped batches prevent one large origin from starving another.
        let mut result = Vec::new();
        let mut origins = self.conn.prepare(
            "SELECT DISTINCT origin_device_id FROM sync_events ORDER BY origin_device_id",
        )?;
        let ids = origins
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for id in ids {
            let limit = (MAX_BATCH - result.len()).min(64);
            if limit == 0 {
                break;
            }
            let mut stmt=self.conn.prepare("SELECT envelope_json FROM sync_events WHERE origin_device_id=?1 AND origin_seq>?2 ORDER BY origin_seq LIMIT ?3")?;
            for row in stmt.query_map(
                params![id, peer.get(&id).copied().unwrap_or(0), limit as i64],
                |r| r.get::<_, String>(0),
            )? {
                result.push(serde_json::from_str(&row?)?);
            }
        }
        Ok(result)
    }
    pub fn memories(&self) -> Result<Vec<MemoryRecord>> {
        self.memories_for_persona(&self.active_persona_id()?)
    }
    pub fn search_memories(&self, query: &str) -> Result<Vec<String>> {
        if query.is_empty() || query.len() > 256 {
            return Err(Error::Invalid("search query length"));
        }
        if query.chars().count() < 3 {
            let escaped = query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let mut stmt=self.conn.prepare("SELECT text FROM memories WHERE profile_id=?1 AND persona_id=?3 AND is_deleted=0 AND text LIKE ?2 ESCAPE '\\' LIMIT 20")?;
            return Ok(stmt
                .query_map(
                    params![
                        self.profile_id,
                        format!("%{escaped}%"),
                        self.active_persona_id()?
                    ],
                    |r| r.get(0),
                )?
                .collect::<std::result::Result<_, _>>()?);
        }
        let phrase = format!("\"{}\"", query.replace('"', "\"\""));
        let mut stmt=self.conn.prepare("SELECT m.text FROM memory_fts f JOIN memories m ON m.id=f.memory_id WHERE memory_fts MATCH ?1 AND m.profile_id=?2 AND m.persona_id=?3 AND m.is_deleted=0 ORDER BY bm25(memory_fts) LIMIT 20")?;
        Ok(stmt
            .query_map(
                params![phrase, self.profile_id, self.active_persona_id()?],
                |r| r.get(0),
            )?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn message_contents(&self) -> Result<Vec<String>> {
        let mut stmt=self.conn.prepare("SELECT content FROM messages WHERE profile_id=?1 AND is_deleted=0 ORDER BY created_lamport,origin_device_id,id")?;
        Ok(stmt
            .query_map([&self.profile_id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn diagnostics(&self) -> Result<Diagnostics> {
        let integrity: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let foreign_errors: i64 =
            self.conn
                .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                    r.get(0)
                })?;
        Ok(Diagnostics {
            version: env!("CARGO_PKG_VERSION"),
            sqlite_version: rusqlite::version().to_owned(),
            schema_version: self
                .conn
                .pragma_query_value(None, "user_version", |r| r.get(0))?,
            device_id: self.device_id.clone(),
            profile_id: self.profile_id.clone(),
            message_count: self.conn.query_row(
                "SELECT count(*) FROM messages WHERE is_deleted=0",
                [],
                |r| r.get(0),
            )?,
            memory_count: self.conn.query_row(
                "SELECT count(*) FROM memories WHERE is_deleted=0",
                [],
                |r| r.get(0),
            )?,
            event_count: self
                .conn
                .query_row("SELECT count(*) FROM sync_events", [], |r| r.get(0))?,
            integrity: if foreign_errors == 0 {
                integrity
            } else {
                "foreign_key_violation".into()
            },
        })
    }
}
