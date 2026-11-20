//! Production replication: durable transaction outbox and per-field logical registers.
//! The network crate authenticates both the TLS peer and every origin signature.
//! Legacy laboratory import is never called by this adapter.
use crate::{
    Error, Result,
    event::sha256_hex,
    recycle::{self, Decision, ERASED_HISTORY, LIFECYCLE},
    storage::Store,
};
use rusqlite::{Connection, OptionalExtension, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const PAGE: usize = 64;
pub const MAX_WIRE: usize = 4 * 1024 * 1024;
// Dependency order for initial bootstrap. Device rows carry public metadata only.
pub(crate) const TABLES: &[(&str, &str)] = &[
    ("local_profiles", "id"),
    ("devices", "id"),
    ("conversations", "id"),
    ("personas", "id"),
    ("conversation_personas", "conversation_id"),
    (LIFECYCLE, "persona_id"),
    ("persona_chat_preferences", "persona_id"),
    ("persona_states", "persona_id"),
    ("sync_events", "event_id"),
    ("messages", "id"),
    ("memories", "id"),
    ("data_tombstones", "profile_id,entity_type,entity_id"),
    ("memory_relations", "from_memory_id,to_memory_id,relation"),
    ("chat_requests", "id"),
    ("chat_generations", "id"),
    ("chat_turn_choices", "user_message_id"),
    ("relationship_events", "id"),
    ("character_moods", "id"),
    ("task_queue", "id"),
    ("task_checkpoints", "task_id,step_key"),
    ("life_plans", "id"),
    ("daily_activities", "id"),
    ("life_events", "id"),
    ("activity_states", "id"),
    ("proactive_messages", "id"),
    ("api_usage_records", "id"),
    ("application_settings", "profile_id,key,device_id"),
    ("persona_links", "alias_id"),
    ("companion_preferences", "persona_id"),
    ("companion_memory_candidates", "id"),
    ("companion_hosts", "persona_id"),
    ("companion_activities", "id"),
    ("companion_life_events", "id"),
    ("character_interests", "id"),
    ("companion_memory_meta", "memory_id"),
    ("companion_bond_events", "id"),
    ("companion_mood_events", "id"),
    ("companion_proactive", "id"),
    ("companion_receipts", "proactive_id,device_id"),
];
pub(crate) fn key_fields(table: &str) -> Result<Vec<&str>> {
    Ok(TABLES
        .iter()
        .find(|(t, _)| *t == table)
        .ok_or(Error::Invalid("sync entity type"))?
        .1
        .split(',')
        .collect())
}
fn columns(db: &Connection, table: &str) -> Result<Vec<String>> {
    key_fields(table)?;
    let mut s = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let all = s
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(all
        .into_iter()
        .filter(|c| match table {
            "devices" => ["id", "profile_id", "name", "platform"].contains(&c.as_str()),
            "task_queue" => ![
                "owner_device_id",
                "lease_boot_id",
                "lease_until_tick",
                "fence",
                "attempt",
                "authority_epoch",
            ]
            .contains(&c.as_str()),
            _ => c != "origin_signature",
        })
        .collect())
}
fn row_key(table: &str, row: &Value) -> Result<String> {
    let keys = key_fields(table)?
        .iter()
        .map(|k| {
            row.get(*k)
                .filter(|v| v.is_string())
                .cloned()
                .ok_or(Error::Invalid("sync key"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::to_string(&keys)?)
}
fn object_expr(cols: &[String], prefix: &str) -> String {
    format!(
        "json_object({})",
        cols.iter()
            .map(|c| format!("'{c}',{prefix}.{c}"))
            .collect::<Vec<_>>()
            .join(",")
    )
}
pub(crate) fn install_capture(db: &Connection) -> Result<()> {
    for (table, _) in TABLES {
        let cols = columns(db, table)?;
        let new = object_expr(&cols, "new");
        let old = object_expr(&cols, "old");
        // Streaming drafts are local. Only completed/interrupted/failed generations leave the device.
        let ready = match *table {
            "application_settings" => {
                " AND new.scope='profile' AND new.device_id='' AND new.key IN ('theme','language')"
            }
            "chat_requests" | "chat_generations" => {
                " AND new.state NOT IN ('sending','generating')"
            }
            "chat_turn_choices" => {
                " AND EXISTS(SELECT 1 FROM chat_generations WHERE id=new.selected_generation_id AND state NOT IN ('sending','generating'))"
            }
            _ => "",
        };
        for (op, before, after) in [
            ("INSERT", "NULL", new.as_str()),
            ("UPDATE", old.as_str(), new.as_str()),
            ("DELETE", old.as_str(), "NULL"),
        ] {
            let prefix = if op == "DELETE" { "old" } else { "new" };
            let task_guard = match *table {
                "task_queue" => format!(" AND {prefix}.kind<>'local_video'"),
                "task_checkpoints" => format!(
                    " AND EXISTS(SELECT 1 FROM task_queue WHERE id={prefix}.task_id AND kind<>'local_video')"
                ),
                _ => String::new(),
            };
            let guard = if op == "DELETE" {
                if *table == "application_settings" {
                    " AND old.scope='profile' AND old.device_id='' AND old.key IN ('theme','language')"
                } else {
                    ""
                }
            } else {
                ready
            };
            db.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS lan_{table}_{op} AFTER {op} ON {table} WHEN (SELECT suppress=0 FROM lan_control WHERE id=1){guard}{task_guard} BEGIN INSERT INTO lan_outbox(table_name,before_json,after_json) VALUES('{table}',{before},{after}); END;"))?;
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub version: u32,
    pub id: String,
    pub origin: String,
    pub seq: i64,
    pub clock: i64,
    pub table: String,
    pub key: String,
    pub row: Value,
    pub fields: BTreeMap<String, Value>,
    pub deleted: bool,
    pub signature: String,
}
impl Change {
    pub fn signing_bytes(&self) -> Result<Vec<u8>> {
        // Fixed, versioned tuple. All dynamic maps use JCS in the signing adapter.
        Ok(serde_jcs::to_vec(&json!([
            "DeepSeek-Niang/LAN/event/v2",
            self.version,
            self.id,
            self.origin,
            self.seq,
            self.clock,
            self.table,
            self.key,
            self.row,
            self.fields,
            self.deleted
        ]))?)
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != 2
            || !(1..=(1_i64 << 52)).contains(&self.seq)
            || !(1..=(1_i64 << 52)).contains(&self.clock)
        {
            return Err(Error::Invalid("sync version/clock"));
        }
        for id in [&self.id, &self.origin] {
            Uuid::parse_str(id).map_err(|_| Error::Invalid("sync UUID"))?;
        }
        let key = if self.table == ERASED_HISTORY {
            let id = self.row["persona_id"]
                .as_str()
                .ok_or(Error::Invalid("erased persona"))?;
            let digest = self.row["wire_sha256"]
                .as_str()
                .ok_or(Error::Invalid("erased digest"))?;
            Uuid::parse_str(id).map_err(|_| Error::Invalid("erased persona"))?;
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || self.row.as_object().map_or(0, |r| r.len()) != 2
                || self.deleted
                || !self.fields.is_empty()
            {
                return Err(Error::Invalid("erased history shape"));
            }
            json!([id]).to_string()
        } else {
            row_key(&self.table, &self.row)?
        };
        if self.key != key
            || self.fields.len() > 100
            || serde_json::to_vec(self)?.len() > 256 * 1024
        {
            return Err(Error::Invalid("sync shape/size"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub certificate: String,
    pub signing_key: String,
    pub revoked: bool,
    pub address: Option<String>,
    pub last_success_ms: Option<i64>,
    pub acknowledged_seq: i64,
}
impl Store {
    pub fn lan_outbox_count(&self) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT (SELECT count(*) FROM lan_outbox)+(SELECT count(*) FROM lan_erasure_queue)",
            [],
            |r| r.get(0),
        )?)
    }
    pub fn lan_enabled(&self) -> Result<bool> {
        Ok(self
            .conn
            .query_row("SELECT enabled FROM lan_control WHERE id=1", [], |r| {
                r.get(0)
            })?)
    }
    pub fn lan_set_enabled(&mut self, enabled: bool) -> Result<()> {
        self.conn
            .execute("UPDATE lan_control SET enabled=?1 WHERE id=1", [enabled])?;
        Ok(())
    }
    pub fn conversation_profile(&self, id: &str) -> Result<String> {
        crate::persona::owner_of_thread(&self.conn, id, &self.profile_id)?;
        Ok(self.conn.query_row(
            "SELECT profile_id FROM conversations WHERE id=?1",
            [id],
            |r| r.get(0),
        )?)
    }
    pub fn lan_peers(&self) -> Result<Vec<Peer>> {
        let mut s=self.conn.prepare("SELECT id,name,certificate,signing_key,revoked,address,last_success_ms,acknowledged_seq FROM lan_peers ORDER BY name,id")?;
        Ok(s.query_map([], |r| {
            Ok(Peer {
                id: r.get(0)?,
                name: r.get(1)?,
                certificate: r.get(2)?,
                signing_key: r.get(3)?,
                revoked: r.get(4)?,
                address: r.get(5)?,
                last_success_ms: r.get(6)?,
                acknowledged_seq: r.get(7)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
    }
    pub fn lan_trust(&mut self, peer: &Peer) -> Result<()> {
        Uuid::parse_str(&peer.id).map_err(|_| Error::Invalid("peer UUID"))?;
        if peer.id == self.device_id || peer.name.is_empty() || peer.name.len() > 100 {
            return Err(Error::Untrusted);
        }
        // Explicit re-pairing may replace a revoked identity; ordinary connections cannot.
        self.conn.execute("INSERT INTO lan_peers(id,name,certificate,signing_key,address) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET name=excluded.name,certificate=excluded.certificate,signing_key=excluded.signing_key,address=excluded.address,revoked=0",params![peer.id,peer.name,peer.certificate,peer.signing_key,peer.address])?;
        Ok(())
    }
    pub fn lan_revoke(&mut self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE lan_peers SET revoked=1,address=NULL WHERE id=?1",
            [id],
        )?;
        Ok(())
    }
    pub fn lan_ack(&mut self, id: &str, ack: i64) -> Result<()> {
        self.conn.execute("UPDATE lan_peers SET acknowledged_seq=max(acknowledged_seq,?1) WHERE id=?2 AND revoked=0",params![ack,id])?;
        Ok(())
    }
    pub fn lan_success(&mut self, id: &str, ack: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE lan_peers SET last_success_ms=?1,acknowledged_seq=max(acknowledged_seq,?2) WHERE id=?3 AND revoked=0",
            params![crate::chat::now_ms(), ack, id],
        )?;
        Ok(())
    }
    pub fn lan_address(&mut self, id: &str, address: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE lan_peers SET address=?1 WHERE id=?2 AND revoked=0",
            params![address, id],
        )?;
        Ok(())
    }
    pub fn lan_pending(&self, peer: &Peer) -> Result<i64> {
        Ok(self.conn.query_row("SELECT (SELECT count(*) FROM lan_events WHERE origin=?1 AND seq>?2)+(SELECT count(*) FROM lan_outbox)",params![self.device_id,peer.acknowledged_seq],|r|r.get(0))?)
    }
    /// Sign only durable local changes. Both sequence assignment and outbox removal are atomic.
    pub fn lan_capture(&mut self, sign: impl Fn(&[u8]) -> Result<String>) -> Result<()> {
        let tx = self.conn.transaction()?;
        let erasures = {
            let mut s = tx.prepare(
                "SELECT e.id,e.seq,e.clock,q.persona_id,q.wire_sha256 FROM lan_erasure_queue q
                JOIN lan_events e ON e.id=q.event_id WHERE e.origin=?1 ORDER BY e.seq LIMIT 512",
            )?;
            s.query_map([&self.device_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (id, seq, clock, persona, digest) in erasures {
            let mut masked = Change {
                version: 2,
                id,
                origin: self.device_id.clone(),
                seq,
                clock,
                table: ERASED_HISTORY.into(),
                key: json!([persona]).to_string(),
                row: json!({"persona_id":persona,"wire_sha256":digest}),
                fields: BTreeMap::new(),
                deleted: false,
                signature: String::new(),
            };
            masked.validate()?;
            masked.signature = sign(&masked.signing_bytes()?)?;
            tx.execute(
                "UPDATE lan_events SET wire=?2 WHERE id=?1",
                params![masked.id, serde_json::to_string(&masked)?],
            )?;
            tx.execute(
                "DELETE FROM lan_erasure_queue WHERE event_id=?1",
                [&masked.id],
            )?;
        }
        if !tx.query_row("SELECT initialized FROM lan_control WHERE id=1", [], |r| {
            r.get::<_, bool>(0)
        })? {
            tx.execute("DELETE FROM lan_outbox", [])?;
            for (table, _) in TABLES {
                let cols = columns(&tx, table)?;
                let expr = object_expr(&cols, "t");
                let filter = match *table {
                    "application_settings" => {
                        " WHERE t.scope='profile' AND t.device_id='' AND t.key IN ('theme','language')"
                    }
                    "chat_requests" | "chat_generations" => {
                        " WHERE t.state NOT IN ('sending','generating')"
                    }
                    "chat_turn_choices" => {
                        " WHERE EXISTS(SELECT 1 FROM chat_generations WHERE id=t.selected_generation_id AND state NOT IN ('sending','generating'))"
                    }
                    "task_queue" => " WHERE t.kind<>'local_video'",
                    "task_checkpoints" => {
                        " WHERE EXISTS(SELECT 1 FROM task_queue WHERE id=t.task_id AND kind<>'local_video')"
                    }
                    _ => "",
                };
                let mut query = tx.prepare(&format!(
                    "SELECT {expr} FROM {table} t{filter} ORDER BY {}",
                    key_fields(table)?
                        .iter()
                        .map(|k| format!("t.{k}"))
                        .collect::<Vec<_>>()
                        .join(",")
                ))?;
                let mut rows = query
                    .query_map([], |r| r.get::<_, String>(0))?
                    .map(|r| Ok(serde_json::from_str::<Value>(&r?)?))
                    .collect::<Result<std::collections::VecDeque<_>>>()?;
                let mut remaining = rows
                    .iter()
                    .filter_map(|r| r["id"].as_str().map(str::to_owned))
                    .collect::<std::collections::BTreeSet<_>>();
                let references: &[&str] = match *table {
                    "messages" => &["reply_to_id", "supersedes_id"],
                    "chat_generations" => &["supersedes_generation_id"],
                    _ => &[],
                };
                let mut deferred = 0;
                while let Some(row) = rows.pop_front() {
                    if references
                        .iter()
                        .any(|k| row[*k].as_str().is_some_and(|id| remaining.contains(id)))
                    {
                        rows.push_back(row);
                        deferred += 1;
                        if deferred > rows.len() {
                            return Err(Error::Invalid("cyclic bootstrap dependency"));
                        }
                        continue;
                    }
                    deferred = 0;
                    if let Some(id) = row["id"].as_str() {
                        remaining.remove(id);
                    }
                    tx.execute(
                        "INSERT INTO lan_outbox(table_name,after_json) VALUES(?1,?2)",
                        params![table, serde_json::to_string(&row)?],
                    )?;
                }
            }
            tx.execute("UPDATE lan_control SET initialized=1 WHERE id=1", [])?;
        }
        let pending = {
            let mut s = tx.prepare(
                "SELECT id,table_name,before_json,after_json FROM lan_outbox ORDER BY id LIMIT 512",
            )?;
            s.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (outbox, table, old, new) in pending {
            let before = old
                .map(|s| serde_json::from_str::<Value>(&s))
                .transpose()?
                .unwrap_or(Value::Null);
            let after = new.map(|s| serde_json::from_str::<Value>(&s)).transpose()?;
            let mut row = after.as_ref().unwrap_or(&before).clone();
            if after.is_none() && table == "application_settings" {
                row["value_json"] = json!("null");
            }
            let deleted = (after.is_none() && table != "application_settings")
                || row.get("is_deleted") == Some(&json!(1));
            let previous = flatten(&before)?;
            let current = flatten(&row)?;
            let fields = current
                .into_iter()
                .filter(|(k, v)| table == LIFECYCLE || previous.get(k) != Some(v))
                .collect::<BTreeMap<_, _>>();
            if !fields.is_empty() || deleted {
                let (seq, clock): (i64, i64) = tx.query_row(
                    "SELECT seq+1,clock+1 FROM lan_control WHERE id=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                let mut event = Change {
                    version: 2,
                    id: Uuid::new_v4().to_string(),
                    origin: self.device_id.clone(),
                    seq,
                    clock,
                    table: table.clone(),
                    key: row_key(&table, &row)?,
                    row,
                    fields,
                    deleted,
                    signature: String::new(),
                };
                event.validate()?;
                event.signature = sign(&event.signing_bytes()?)?;
                save_event(&tx, &event)?;
                merge_cells(&tx, &event)?;
                if event.table == "relationship_events" {
                    tx.execute("UPDATE lan_control SET suppress=1 WHERE id=1", [])?;
                    recompute_relationship(
                        &tx,
                        event.row["persona_id"]
                            .as_str()
                            .ok_or(Error::Invalid("relationship persona"))?,
                    )?;
                    tx.execute("UPDATE lan_control SET suppress=0 WHERE id=1", [])?;
                }

                tx.execute(
                    "UPDATE lan_control SET seq=?1,clock=?2 WHERE id=1",
                    params![seq, clock],
                )?;
            }
            tx.execute("DELETE FROM lan_outbox WHERE id=?1", [outbox])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn lan_cursor(&self, origin: &str) -> Result<i64> {
        let mut n = 0;
        let mut s = self
            .conn
            .prepare("SELECT seq FROM lan_events WHERE origin=?1 ORDER BY seq")?;
        for v in s.query_map([origin], |r| r.get::<_, i64>(0))? {
            let v = v?;
            if v == n + 1 {
                n = v;
            } else if v > n + 1 {
                break;
            }
        }
        Ok(n)
    }
    pub fn lan_export(&self, after: i64) -> Result<Vec<Change>> {
        let mut s = self.conn.prepare(
            "SELECT wire FROM lan_events WHERE origin=?1 AND seq>?2 ORDER BY seq LIMIT ?3",
        )?;
        let rows = s
            .query_map(params![self.device_id, after, PAGE as i64], |r| {
                r.get::<_, String>(0)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut total = 0;
        let mut page = Vec::new();
        for wire in rows {
            if total + wire.len() > MAX_WIRE / 2 {
                break;
            }
            total += wire.len();
            match serde_json::from_str(&wire) {
                Ok(event) => page.push(event),
                Err(_) if erased_digest(&wire).is_some() => break,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(page)
    }
    /// The caller supplies the authenticated direct origin, after verifying every signature.
    /// Forwarded events from unpaired origins are deliberately not accepted.
    pub fn lan_import(&mut self, peer: &str, events: &[Change]) -> Result<usize> {
        if events.len() > PAGE || serde_json::to_vec(events)?.len() > MAX_WIRE {
            return Err(Error::Invalid("sync batch limit"));
        }
        if !self.lan_peers()?.iter().any(|p| p.id == peer && !p.revoked) {
            return Err(Error::Untrusted);
        }
        let tx = self.conn.transaction()?;
        tx.execute("UPDATE lan_control SET suppress=1 WHERE id=1", [])?;
        let mut count = 0;
        for e in events {
            e.validate()?;
            if e.origin != peer || e.origin == self.device_id {
                return Err(Error::Untrusted);
            }
            let old: Option<(String,String,i64,i64,String)> = tx
                .query_row(
                    "SELECT id,origin,seq,clock,wire FROM lan_events WHERE id=?1 OR (origin=?2 AND seq=?3)",
                    params![e.id, e.origin, e.seq],
                    |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
                )
                .optional()?;
            if let Some((id, origin, seq, clock, old)) = old {
                if (id.as_str(), origin.as_str(), seq, clock)
                    != (e.id.as_str(), e.origin.as_str(), e.seq, e.clock)
                {
                    return Err(Error::Conflict);
                }
                let incoming = serde_json::to_string(e)?;
                if incoming == old {
                    continue;
                }
                let known_digest = erased_digest(&old);
                let incoming_digest = e.row["wire_sha256"].as_str();
                if known_digest.as_deref().is_some_and(|d| {
                    if e.table == ERASED_HISTORY {
                        incoming_digest == Some(d)
                    } else {
                        sha256_hex(incoming.as_bytes()) == d
                    }
                }) {
                    continue;
                }
                if e.table == ERASED_HISTORY
                    && incoming_digest.is_some_and(|d| d == sha256_hex(old.as_bytes()))
                {
                    // A signed replacement keeps the original origin, sequence and checksum.
                    tx.execute(
                        "UPDATE lan_events SET wire=?2 WHERE id=?1",
                        params![e.id, incoming],
                    )?;
                    continue;
                }
                return Err(Error::Conflict);
            }
            let next: i64 = tx.query_row(
                "SELECT coalesce(max(seq),0)+1 FROM lan_events WHERE origin=?1",
                [peer],
                |r| r.get(0),
            )?;
            if e.seq != next {
                return Err(Error::Invalid("CHECKPOINT_GAP"));
            }
            if e.table == ERASED_HISTORY {
                save_event(&tx, e)?;
            } else if e.table == LIFECYCLE {
                validate_row(&tx, e, &self.device_id)?;
                import_lifecycle(&tx, e)?;
                save_event(&tx, e)?;
            } else if let Some(persona) = recycle::erased_owner(&tx, &e.table, &e.key, &e.row)? {
                recycle::remember_key(&tx, &e.table, &e.key, &persona)?;
                let digest = sha256_hex(serde_json::to_string(e)?.as_bytes());
                tx.execute(
                    "INSERT INTO lan_events(id,origin,seq,clock,wire) VALUES(?1,?2,?3,?4,?5)",
                    params![
                        e.id,
                        e.origin,
                        e.seq,
                        e.clock,
                        recycle::erased_wire(&digest)
                    ],
                )?;
            } else {
                validate_row(&tx, e, &self.device_id)?;
                merge_cells(&tx, e)?;
                project(&tx, e)?;
                save_event(&tx, e)?;
            }
            let observed = e
                .clock
                .max(
                    e.row
                        .get("created_lamport")
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                )
                .max(e.row.get("lamport").and_then(Value::as_i64).unwrap_or(0));
            tx.execute(
                "UPDATE local_clock SET lamport=max(lamport,?1) WHERE device_id=?2",
                params![observed, self.device_id],
            )?;
            tx.execute(
                "UPDATE lan_control SET clock=max(clock,?1) WHERE id=1",
                [e.clock],
            )?;
            count += 1;
        }
        tx.execute("UPDATE lan_control SET suppress=0 WHERE id=1", [])?;
        if tx.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get::<_, i64>(0)
        })? != 0
        {
            return Err(Error::Schema);
        }
        tx.commit()?;
        self.reconcile_paired_persona()?;
        Ok(count)
    }
    pub fn lan_conflicts(&self) -> Result<Vec<Value>> {
        let mut s=self.conn.prepare("SELECT table_name,row_key,field,event_id,value_json FROM lan_conflicts ORDER BY table_name,row_key LIMIT 100")?;
        Ok(s.query_map([],|r|Ok(json!({"entity":r.get::<_,String>(0)?,"key":r.get::<_,String>(1)?,"field":r.get::<_,String>(2)?,"event":r.get::<_,String>(3)?,"value":r.get::<_,String>(4)?})))?.collect::<std::result::Result<_,_>>()?)
    }
}
fn erased_digest(wire: &str) -> Option<String> {
    let value: Value = serde_json::from_str(wire).ok()?;
    if value["table"] == ERASED_HISTORY {
        value["row"]["wire_sha256"].as_str().map(str::to_owned)
    } else {
        value["erased_digest"].as_str().map(str::to_owned)
    }
}

fn import_lifecycle(db: &Connection, event: &Change) -> Result<()> {
    let next: Decision = serde_json::from_value(event.row.clone())?;
    recycle::validate_decision(db, &next)?;
    let latest = recycle::read_decision(db, &next.persona_id)?;
    let terminal = next.state == "permanently_deleted";
    let already_deleted = latest.is_some_and(|d| d.state == "permanently_deleted");
    if already_deleted && !terminal {
        return Ok(());
    }
    let previous: Option<(i64,String,String)> = db.query_row(
        "SELECT clock,origin,event_id FROM lan_cells WHERE table_name=?1 AND row_key=?2 AND field='state'",
        params![LIFECYCLE,event.key], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).optional()?;
    // Compare the decision as a unit, independently of concurrent edits to the persona row.
    if (!terminal || already_deleted)
        && previous.is_some_and(|p| {
            (p.0, p.1.as_str(), p.2.as_str())
                >= (event.clock, event.origin.as_str(), event.id.as_str())
        })
    {
        return Ok(());
    }
    db.execute(
        "DELETE FROM lan_cells WHERE table_name=?1 AND row_key=?2",
        params![LIFECYCLE, event.key],
    )?;
    merge_cells(db, event)?;
    recycle::enact(db, &next)
}

fn flatten(row: &Value) -> Result<BTreeMap<String, Value>> {
    let mut fields = BTreeMap::new();
    if let Some(obj) = row.as_object() {
        for (k, v) in obj {
            if k == "settings_json" {
                let value: Value =
                    serde_json::from_str(v.as_str().ok_or(Error::Invalid("settings JSON"))?)?;
                let allowed = [
                    "dialogue",
                    "model",
                    "temperature",
                    "max_tokens",
                    "history_turns",
                    "timeout_seconds",
                    "include_history",
                    "character_name",
                    "user_title",
                    "personality",
                ];
                if value
                    .as_object()
                    .ok_or(Error::Invalid("settings object"))?
                    .keys()
                    .any(|k| !allowed.contains(&k.as_str()))
                {
                    return Err(Error::Invalid("unknown synchronized preference"));
                }
                for (name, val) in value.as_object().ok_or(Error::Invalid("settings object"))? {
                    fields.insert(format!("settings_json/{name}"), val.clone());
                }
            } else {
                fields.insert(k.clone(), v.clone());
            }
        }
    }
    Ok(fields)
}
fn save_event(db: &Connection, e: &Change) -> Result<()> {
    db.execute(
        "INSERT INTO lan_events(id,origin,seq,clock,wire) VALUES(?1,?2,?3,?4,?5)",
        params![e.id, e.origin, e.seq, e.clock, serde_json::to_string(e)?],
    )?;
    Ok(())
}
fn merge_cells(db: &Connection, e: &Change) -> Result<()> {
    if e.deleted {
        db.execute(
            "INSERT OR IGNORE INTO lan_deleted VALUES(?1,?2)",
            params![e.table, e.key],
        )?;
    }
    for (field, value) in &e.fields {
        let old:Option<(i64,String,String,String)>=db.query_row("SELECT clock,origin,event_id,value_json FROM lan_cells WHERE table_name=?1 AND row_key=?2 AND field=?3",params![e.table,e.key,field],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        let encoded = serde_json::to_string(value)?;
        if let Some((clock, origin, id, prior)) = &old {
            if prior != &encoded && origin != &e.origin {
                // Keep both candidate values, including the losing register, for explicit review.
                for (eid, val) in [
                    (id.as_str(), prior.as_str()),
                    (e.id.as_str(), encoded.as_str()),
                ] {
                    db.execute(
                        "INSERT OR IGNORE INTO lan_conflicts VALUES(?1,?2,?3,?4,?5)",
                        params![e.table, e.key, field, eid, val],
                    )?;
                }
            }
            if (*clock, origin, id) >= (e.clock, &e.origin, &e.id) {
                continue;
            }
        }
        db.execute("INSERT INTO lan_cells VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(table_name,row_key,field) DO UPDATE SET value_json=excluded.value_json,clock=excluded.clock,origin=excluded.origin,event_id=excluded.event_id",params![e.table,e.key,field,encoded,e.clock,e.origin,e.id])?;
    }
    Ok(())
}
fn legacy_default(table: &str, column: &str) -> Option<Value> {
    match (table, column) {
        ("personas", "avatar_resource") => Some(json!("deepseek-niang")),
        ("api_usage_records", "purpose") => Some(json!("chat")),
        ("life_plans", "host_device_id")
        | ("companion_activities", "task_id" | "plan_id")
        | ("companion_memory_meta", "source_life_event_id") => Some(Value::Null),
        ("life_plans", "state") => Some(json!("active")),
        ("life_plans", "created_at_ms" | "next_due_at_ms")
        | ("companion_activities", "elapsed_ms" | "progress_milli" | "checkpoint_seq")
        | ("companion_life_events", "sequence") => Some(json!(0)),
        ("companion_activities", "duration_ms") => Some(json!(1200000)),
        _ => None,
    }
}
fn validate_row(db: &Connection, e: &Change, local: &str) -> Result<()> {
    let cols = columns(db, &e.table)?;
    let obj = e.row.as_object().ok_or(Error::Invalid("row object"))?;
    if obj.keys().any(|k| !cols.contains(k))
        || cols
            .iter()
            .any(|column| !obj.contains_key(column) && legacy_default(&e.table, column).is_none())
    {
        return Err(Error::Invalid("unknown sync field"));
    }
    for (name, value) in obj {
        let uuid_column = (name == "id" || name.ends_with("_id") || name == "version_origin")
            && name != "model_id";
        if uuid_column
            && !value.is_null()
            && !(e.table == "application_settings" && name == "device_id")
        {
            Uuid::parse_str(value.as_str().ok_or(Error::Invalid("entity UUID"))?)
                .map_err(|_| Error::Invalid("entity UUID"))?;
        }
    }
    let flat = flatten(&e.row)?;
    if e.fields.iter().any(|(k, v)| flat.get(k) != Some(v)) {
        return Err(Error::Invalid("invalid change fields"));
    }
    if e.table == LIFECYCLE && (e.deleted || e.fields != flat) {
        return Err(Error::Invalid("incomplete persona lifecycle decision"));
    }
    if e.table == "personas"
        && let Some(id) = e.row["id"].as_str()
        && let Some(decision) = recycle::read_decision(db, id)?
        && (e.row["profile_id"] != decision.profile_id
            || e.row["main_conversation_id"] != decision.main_conversation_id
            || e.row["is_builtin"] != json!(0))
    {
        return Err(Error::Untrusted);
    }
    if e.table == "devices" && e.row["id"] == local {
        return Err(Error::Untrusted);
    }
    if e.table == "persona_links"
        && e.row["host_device_id"] != e.origin
        && e.row["source_device_id"] != e.origin
    {
        return Err(Error::Untrusted);
    }
    if e.table == "sync_events" {
        let event: crate::event::Event = serde_json::from_str(
            e.row["envelope_json"]
                .as_str()
                .ok_or(Error::Invalid("legacy envelope"))?,
        )?;
        event.validate()?;
        let operation = if event.operation == crate::event::Operation::Put {
            "put"
        } else {
            "delete"
        };
        if e.row["event_id"] != event.event_id
            || e.row["origin_device_id"] != event.origin_device_id
            || e.row["profile_id"] != event.profile_id
            || e.row["origin_seq"] != event.origin_seq
            || e.row["lamport"] != event.lamport
            || e.row["checksum"] != event.checksum
            || e.row["entity_id"] != event.entity_id
            || e.row["entity_type"] != event.entity.as_str()
            || e.row["operation"] != operation
        {
            return Err(Error::Conflict);
        }
        if event.origin_device_id == local {
            return Err(Error::Untrusted);
        }
    }
    if e.table == "persona_chat_preferences" {
        let settings: crate::chat::ChatSettings = serde_json::from_str(
            e.row["settings_json"]
                .as_str()
                .ok_or(Error::Invalid("preferences"))?,
        )?;
        settings.validate()?;
    }
    if e.table == "companion_preferences" {
        let settings: crate::companion::CompanionPreferences = serde_json::from_str(
            e.row["config_json"]
                .as_str()
                .ok_or(Error::Invalid("companion config"))?,
        )?;
        settings.validate()?;
    }
    if e.table == "character_interests" {
        let evidence: Value = serde_json::from_str(
            e.row["evidence_json"]
                .as_str()
                .ok_or(Error::Invalid("interest evidence"))?,
        )?;
        if let Some(history) = evidence.as_array() {
            for item in history
                .iter()
                .filter(|v| v.get("source_event_id").is_some())
            {
                let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM companion_life_events WHERE id=?1 AND persona_id=?2 AND host_device_id=?3 AND kind='completed')",params![item["source_event_id"].as_str(),to_sql(&e.row["persona_id"]),e.origin],|r|r.get(0))?;
                if !valid {
                    return Err(Error::Untrusted);
                }
            }
        }
    }
    if e.table == "companion_receipts" && e.row["device_id"] != e.origin {
        return Err(Error::Untrusted);
    }
    if e.table == "life_plans"
        && e.row.get("host_device_id").is_some_and(|v| !v.is_null())
        && e.row["host_device_id"] != e.origin
    {
        return Err(Error::Untrusted);
    }
    if e.table == "companion_memory_meta" && e.row["source_kind"] == "life" {
        let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM companion_life_events WHERE id=?1 AND persona_id=?2 AND host_device_id=?3 AND kind='completed')",params![to_sql(&e.row["source_life_event_id"]),to_sql(&e.row["persona_id"]),e.origin],|r|r.get(0))?;
        if !valid {
            return Err(Error::Untrusted);
        }
    }
    if e.table == "companion_hosts" {
        let previous: Option<String> = db
            .query_row(
                "SELECT device_id FROM companion_hosts WHERE persona_id=?1",
                [to_sql(&e.row["persona_id"])],
                |r| r.get(0),
            )
            .optional()?;
        if previous.is_some_and(|v| Some(v.as_str()) != e.row["device_id"].as_str()) {
            return Err(Error::Conflict);
        }

        if e.row["device_id"] != e.origin {
            return Err(Error::Untrusted);
        }
        let platform: String = db.query_row(
            "SELECT platform FROM devices WHERE id=?1",
            [&e.origin],
            |r| r.get(0),
        )?;
        if platform != "windows" {
            return Err(Error::Untrusted);
        }
    }
    if matches!(
        e.table.as_str(),
        "companion_activities" | "companion_life_events" | "companion_proactive"
    ) {
        let host: String = db.query_row(
            "SELECT device_id FROM companion_hosts WHERE persona_id=?1",
            [to_sql(&e.row["persona_id"])],
            |r| r.get(0),
        )?;
        if e.row["host_device_id"] != e.origin || host != e.origin {
            return Err(Error::Untrusted);
        }
    }
    if matches!(
        e.table.as_str(),
        "companion_bond_events"
            | "companion_mood_events"
            | "companion_memory_candidates"
            | "companion_memory_meta"
    ) && e.row.get("source_message_id").is_some_and(|v| !v.is_null())
    {
        let (owner,origin,role,text,at,clock):(String,String,String,String,i64,i64)=db.query_row("SELECT c.persona_id,m.origin_device_id,m.role,m.content,coalesce(m.created_at_ms,0),m.created_lamport FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE m.id=?1",[to_sql(&e.row["source_message_id"])],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
        if role != "user"
            || crate::pairing::resolve(db, &owner)?
                != crate::pairing::resolve(
                    db,
                    e.row["persona_id"]
                        .as_str()
                        .ok_or(Error::Invalid("evidence persona"))?,
                )?
        {
            return Err(Error::Untrusted);
        }
        if e.table == "companion_bond_events" {
            if origin != e.origin
                || e.row["origin_device_id"] != origin
                || e.row["occurred_at_ms"] != at
                || e.row["lamport"] != clock
            {
                return Err(Error::Untrusted);
            }
            if crate::companion::bond::evidence_kind(&text) != e.row["kind"].as_str() {
                return Err(Error::Untrusted);
            }
        }
        if e.table == "companion_mood_events"
            && (origin != e.origin
                || e.row["origin_device_id"] != origin
                || e.row["started_at_ms"] != at
                || e.row["lamport"] != clock
                || crate::companion::bond::inferred_mood(&text) != e.row["mood"].as_str())
        {
            return Err(Error::Untrusted);
        }
    }
    if e.table == "companion_mood_events" && !e.row["activity_id"].is_null() {
        let (host, persona): (String, String) = db.query_row(
            "SELECT host_device_id,persona_id FROM companion_activities WHERE id=?1",
            [to_sql(&e.row["activity_id"])],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if host != e.origin || e.row["origin_device_id"] != host || e.row["persona_id"] != persona {
            return Err(Error::Untrusted);
        }
    }
    if e.table == "application_settings"
        && (e.row["scope"] != "profile"
            || e.row["device_id"] != ""
            || !matches!(e.row["key"].as_str(), Some("theme" | "language")))
    {
        return Err(Error::Untrusted);
    }
    if e.table == "task_checkpoints" {
        let (authority, kind): (String, String) = db.query_row(
            "SELECT authority_device_id,kind FROM task_queue WHERE id=?1",
            [e.row["task_id"]
                .as_str()
                .ok_or(Error::Invalid("task UUID"))?],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if authority != e.origin || kind == "local_video" {
            return Err(Error::Untrusted);
        }
    }
    if e.table == "task_queue"
        && (e.row["authority_device_id"] != e.origin || e.row["kind"] == "local_video")
    {
        return Err(Error::Untrusted);
    }
    if obj.values().any(|v| {
        v.as_i64()
            .is_some_and(|n| !(-crate::event::MAX_CLOCK..=crate::event::MAX_CLOCK).contains(&n))
    }) {
        return Err(Error::Invalid("sync scalar bounds"));
    }
    if obj
        .values()
        .any(|v| !(v.is_null() || v.is_string() || v.is_i64()))
    {
        return Err(Error::Invalid("non-scalar sync column"));
    }
    // Identity and ownership can never be changed by field conflict resolution.
    let keys = key_fields(&e.table)?;
    let predicate = keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k}=?{}", i + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    for field in cols.iter().filter(|f| {
        obj.contains_key(f.as_str())
            && (keys.contains(&f.as_str())
                || [
                    "profile_id",
                    "persona_id",
                    "main_conversation_id",
                    "is_builtin",
                    "conversation_id",
                    "user_message_id",
                    "origin_device_id",
                    "source_event_id",
                    "host_device_id",
                    "task_id",
                    "plan_id",
                    "source_life_event_id",
                    "authority_device_id",
                    "host_device_id",
                    "source_device_id",
                    "canonical_id",
                ]
                .contains(&f.as_str()))
    }) {
        let existing: Option<SqlValue> = db
            .query_row(
                &format!("SELECT {field} FROM {} WHERE {predicate}", e.table),
                rusqlite::params_from_iter(keys.iter().map(|k| to_sql(&e.row[*k]))),
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing
            && existing != to_sql(&e.row[field])
        {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}
fn to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::String(s) => SqlValue::Text(s.clone()),
        _ => SqlValue::Integer(v.as_i64().unwrap_or(0)),
    }
}
fn project(db: &Connection, e: &Change) -> Result<()> {
    let cols = columns(db, &e.table)?;
    let keys = key_fields(&e.table)?;
    let predicate = keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k}=?{}", i + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let exists: bool = db.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {} WHERE {predicate})", e.table),
        rusqlite::params_from_iter(keys.iter().map(|k| to_sql(&e.row[*k]))),
        |r| r.get(0),
    )?;
    let mut row = e.row.clone();
    // v4 queues are already signed. Supply only the two known additive SQL
    // defaults in the projection; never change the stored wire/signature.
    for column in &cols {
        if row.get(column).is_none()
            && let Some(value) = legacy_default(&e.table, column)
        {
            row[column] = value;
        }
    }
    let mut settings = serde_json::Map::new();
    let mut s =
        db.prepare("SELECT field,value_json FROM lan_cells WHERE table_name=?1 AND row_key=?2")?;
    for v in s.query_map(params![e.table, e.key], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (field, value) = v?;
        let value: Value = serde_json::from_str(&value)?;
        if let Some(name) = field.strip_prefix("settings_json/") {
            settings.insert(name.into(), value);
        } else {
            row[field] = value;
        }
    }
    if !settings.is_empty() {
        let prefs = Value::Object(settings);
        let parsed: crate::chat::ChatSettings = serde_json::from_value(prefs.clone())?;
        parsed.validate()?;
        row["settings_json"] = json!(serde_json::to_string(&prefs)?);
    }
    let deleted: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM lan_deleted WHERE table_name=?1 AND row_key=?2)",
        params![e.table, e.key],
        |r| r.get(0),
    )?;
    if deleted {
        if e.table == "messages" || e.table == "memories" {
            row["is_deleted"] = json!(1);
            row[if e.table == "messages" {
                "content"
            } else {
                "text"
            }] = json!("");
        } else {
            return Err(Error::Invalid("unsupported physical delete"));
        }
    }
    // Append-only message/evidence identity cannot be replaced with different content.
    if exists
        && [
            "messages",
            "relationship_events",
            "life_events",
            "sync_events",
            "local_profiles",
            "devices",
            "conversation_personas",
            "api_usage_records",
            "persona_links",
            "companion_bond_events",
            "companion_mood_events",
            "companion_life_events",
        ]
        .contains(&e.table.as_str())
    {
        for c in &cols {
            if ["is_deleted", "content", "text", "name", "platform"].contains(&c.as_str()) {
                continue;
            }
            let value: SqlValue = db.query_row(
                &format!("SELECT {c} FROM {} WHERE {predicate}", e.table),
                rusqlite::params_from_iter(keys.iter().map(|k| to_sql(&row[*k]))),
                |r| r.get(0),
            )?;
            if value != to_sql(&row[c]) {
                return Err(Error::Conflict);
            }
        }
        if e.table == "messages" && !deleted {
            let content: String = db.query_row(
                &format!("SELECT content FROM messages WHERE {predicate}"),
                rusqlite::params_from_iter(keys.iter().map(|k| to_sql(&row[*k]))),
                |r| r.get(0),
            )?;
            if content != row["content"].as_str().unwrap_or("") {
                return Err(Error::Conflict);
            }
        }
        if e.table == "conversation_personas" {
            return Ok(());
        }
    }
    if e.table == "relationship_events" {
        let existing:Option<(i64,String,String)>=db.query_row("SELECT lamport,origin_device_id,id FROM relationship_events WHERE profile_id=?1 AND persona_id=?2 AND evidence_hash=?3 AND rule_version=?4",params![to_sql(&row["profile_id"]),to_sql(&row["persona_id"]),to_sql(&row["evidence_hash"]),to_sql(&row["rule_version"])],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some(old) = existing {
            let incoming = (
                row["lamport"]
                    .as_i64()
                    .ok_or(Error::Invalid("relationship clock"))?,
                row["origin_device_id"]
                    .as_str()
                    .ok_or(Error::Invalid("relationship origin"))?
                    .to_owned(),
                row["id"]
                    .as_str()
                    .ok_or(Error::Invalid("relationship id"))?
                    .to_owned(),
            );
            if old.2 != incoming.2 {
                if old <= incoming {
                    return Ok(());
                }
                db.execute("DELETE FROM relationship_events WHERE id=?1", [old.2])?;
            }
        }
    }
    if e.table == "activity_states" {
        row["state"] = json!("remote_snapshot");
        row["error_code"] = json!("REMOTE_SNAPSHOT_REQUIRES_LIVE_HEARTBEAT");
    }
    let mut insert_cols = cols.clone();
    let mut values: Vec<SqlValue> = cols.iter().map(|c| to_sql(&row[c])).collect();
    if e.table == "devices" {
        insert_cols.push("trust_state".into());
        values.push(SqlValue::Text("revoked".into()));
    }
    let update = cols
        .iter()
        .filter(|c| {
            !keys.contains(&c.as_str())
                && ![
                    "profile_id",
                    "persona_id",
                    "main_conversation_id",
                    "is_builtin",
                ]
                .contains(&c.as_str())
        })
        .map(|c| format!("{c}=excluded.{c}"))
        .collect::<Vec<_>>()
        .join(",");
    let action = if update.is_empty() {
        "DO NOTHING".into()
    } else {
        format!("DO UPDATE SET {update}")
    };
    db.execute(
        &format!(
            "INSERT INTO {}({}) VALUES({}) ON CONFLICT({}) {action}",
            e.table,
            insert_cols.join(","),
            (1..=values.len())
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(","),
            keys.join(",")
        ),
        rusqlite::params_from_iter(values),
    )?;
    if e.table == "personas" {
        db.execute(
            "INSERT OR IGNORE INTO persona_states(persona_id) VALUES(?1)",
            [row["id"].as_str().ok_or(Error::Invalid("persona id"))?],
        )?;
    }
    if e.table == "data_tombstones" {
        let entity = row["entity_type"]
            .as_str()
            .ok_or(Error::Invalid("tombstone type"))?;
        let table = match entity {
            "message" => "messages",
            "memory" => "memories",
            _ => return Err(Error::Invalid("tombstone type")),
        };
        let id = row["entity_id"]
            .as_str()
            .ok_or(Error::Invalid("tombstone UUID"))?;
        let actual: Option<String> = db
            .query_row(
                &format!("SELECT profile_id FROM {table} WHERE id=?1"),
                [id],
                |r| r.get(0),
            )
            .optional()?;
        if actual
            .as_deref()
            .is_some_and(|p| Some(p) != row["profile_id"].as_str())
        {
            return Err(Error::Untrusted);
        }
        db.execute(
            "INSERT OR IGNORE INTO lan_deleted VALUES(?1,?2)",
            params![table, serde_json::to_string(&json!([id]))?],
        )?;
        let field = if table == "messages" {
            "content"
        } else {
            "text"
        };
        db.execute(
            &format!("UPDATE {table} SET is_deleted=1,{field}='' WHERE id=?1"),
            [id],
        )?;
        if table == "memories" {
            db.execute("DELETE FROM memory_embeddings WHERE memory_id=?1", [id])?;
        }
    }
    if e.table == "relationship_events" {
        recompute_relationship(
            db,
            row["persona_id"]
                .as_str()
                .ok_or(Error::Invalid("relationship persona"))?,
        )?;
    }
    if e.table == "local_profiles" {
        db.execute(
            "INSERT OR IGNORE INTO lan_profiles VALUES(?1)",
            [row["id"].as_str().ok_or(Error::Invalid("profile id"))?],
        )?;
    }
    Ok(())
}

fn recompute_relationship(db: &Connection, persona: &str) -> Result<()> {
    let raw = {
        let mut s=db.prepare("SELECT evidence_json FROM relationship_events WHERE persona_id=?1 ORDER BY lamport,origin_device_id,id")?;
        s.query_map([persona], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let parsed = raw
        .iter()
        .map(|s| serde_json::from_str::<crate::relationship::RelationshipEvent>(s))
        .collect::<std::result::Result<Vec<_>, _>>();
    if let Ok(events) = parsed {
        let state = crate::relationship::recompute(&events)?;
        db.execute(
            "UPDATE persona_states SET relationship_json=?1 WHERE persona_id=?2",
            params![serde_json::to_string(&state)?, persona],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use crate::event::sha256_hex;
    fn v3(path: &std::path::Path) -> Connection {
        let db = Connection::open(path).unwrap();
        for (number, sql) in [
            (1, include_str!("../../../db/migrations/0001_initial.sql")),
            (2, include_str!("../../../db/migrations/0002_chat.sql")),
            (3, include_str!("../../../db/migrations/0003_personas.sql")),
        ] {
            db.execute_batch(sql).unwrap();
            db.execute(
                "INSERT INTO schema_migrations VALUES(?1,?2)",
                params![number, sha256_hex(sql.as_bytes())],
            )
            .unwrap();
            db.pragma_update(None, "user_version", number).unwrap();
        }
        db
    }
    fn paired(a: &mut Store, b: &Store) {
        a.lan_trust(&Peer {
            id: b.device_id.clone(),
            name: "fixture".into(),
            certificate: "00".into(),
            signing_key: "00".into(),
            revoked: false,
            address: None,
            last_success_ms: None,
            acknowledged_seq: 0,
        })
        .unwrap();
    }
    fn capture(store: &mut Store) {
        loop {
            store.lan_capture(|b| Ok(sha256_hex(b))).unwrap();
            if store.lan_outbox_count().unwrap() == 0 {
                break;
            }
        }
    }
    fn replicate(a: &mut Store, b: &mut Store) {
        capture(a);
        loop {
            let page = a.lan_export(b.lan_cursor(&a.device_id).unwrap()).unwrap();
            if page.is_empty() {
                break;
            }
            b.lan_import(&a.device_id, &page).unwrap();
        }
    }
    #[test]
    fn recycle_bidirectional_restore_and_terminal_erasure_fence_offline_edits() {
        use crate::recycle::RecycleAction;
        let mut desktop = Store::open(":memory:").unwrap();
        let mut phone = Store::open(":memory:").unwrap();
        desktop.set_device_platform("windows").unwrap();
        phone.set_device_platform("android").unwrap();
        paired(&mut desktop, &phone);
        paired(&mut phone, &desktop);
        let original = desktop.create_persona("临时人格螺壳", "喜欢河流").unwrap();
        let survivor = desktop.create_persona("保留人格", "喜欢看云").unwrap();
        let p = &original.id;
        let memory = Uuid::new_v4().to_string();
        let keep = Uuid::new_v4().to_string();
        desktop
            .put_persona_memory(p, &memory, "合成删除标识螺壳")
            .unwrap();
        desktop
            .put_persona_memory(&survivor.id, &keep, "保留性格夹具")
            .unwrap();
        let chat = crate::chat::StartChat {
            request_id: Uuid::new_v4().to_string(),
            conversation_id: original.main_conversation_id.clone(),
            text: Some("谢谢你陪我练习。螺壳夹具聊天".into()),
            regenerate_user_id: None,
        };
        let settings = desktop.persona_settings(p).unwrap();
        desktop.begin_chat(&chat, &settings).unwrap();
        desktop
            .finish_chat(&chat.request_id, "夹具回复。", "complete", None, None, None)
            .unwrap();
        let bond = desktop.bond_snapshot(p).unwrap().applied_events;
        let before = desktop.chat_messages(&chat.conversation_id).unwrap();
        capture(&mut phone);
        replicate(&mut desktop, &mut phone);
        replicate(&mut phone, &mut desktop);
        desktop.select_persona(p).unwrap();
        phone.select_persona(p).unwrap();
        desktop
            .change_recycle(RecycleAction::Trash {
                persona_id: p.clone(),
            })
            .unwrap();
        assert!(desktop.persona(p).is_err());
        assert_ne!(desktop.active_persona_id().unwrap(), *p);
        assert!(desktop.begin_chat(&chat, &settings).is_err());
        let mut offline = phone.persona_settings(p).unwrap();
        offline.character_name = "离线修改的名字".into();
        phone.save_persona_settings(p, &offline).unwrap();
        replicate(&mut phone, &mut desktop);
        assert!(desktop.persona(p).is_err());
        replicate(&mut desktop, &mut phone);
        assert_ne!(phone.active_persona_id().unwrap(), *p);
        let trash = phone.recycled_personas().unwrap().pop().unwrap();
        assert_eq!(trash.persona.name, "离线修改的名字");
        phone
            .change_recycle(RecycleAction::Restore { entry: trash.entry })
            .unwrap();
        replicate(&mut phone, &mut desktop);
        assert_eq!(desktop.persona(p).unwrap().id, *p);
        let after = desktop.chat_messages(&chat.conversation_id).unwrap();
        assert_eq!(
            before.iter().map(|m| &m.id).collect::<Vec<_>>(),
            after.iter().map(|m| &m.id).collect::<Vec<_>>()
        );
        assert_eq!(desktop.bond_snapshot(p).unwrap().applied_events, bond);
        assert_eq!(desktop.memories_for_persona(p).unwrap()[0].id, memory);
        phone
            .change_recycle(RecycleAction::Trash {
                persona_id: p.clone(),
            })
            .unwrap();
        replicate(&mut phone, &mut desktop);
        let both = desktop.recycled_personas().unwrap().pop().unwrap();
        let peer = phone.recycled_personas().unwrap().pop().unwrap();
        desktop
            .change_recycle(RecycleAction::Purge {
                entries: vec![both.entry],
            })
            .unwrap();
        // The phone makes a new restore and edit before learning about deletion.
        phone
            .change_recycle(RecycleAction::Restore { entry: peer.entry })
            .unwrap();
        phone
            .put_persona_memory(p, &Uuid::new_v4().to_string(), "离线新写入螺壳")
            .unwrap();
        replicate(&mut phone, &mut desktop);
        assert!(desktop.persona(p).is_err());
        replicate(&mut desktop, &mut phone);
        replicate(&mut phone, &mut desktop);
        replicate(&mut desktop, &mut phone);
        for store in [&desktop, &phone] {
            assert!(store.persona(p).is_err() && store.recycled_personas().unwrap().is_empty());
            assert_eq!(
                store.memories_for_persona(&survivor.id).unwrap()[0].id,
                keep
            );
            assert_eq!(store.diagnostics().unwrap().integrity, "ok");
            let leaked: i64 = store
                .conn
                .query_row(
                    "SELECT count(*) FROM lan_events WHERE wire LIKE '%螺壳%'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(leaked, 0);
        }
        let mut newcomer = Store::open(":memory:").unwrap();
        paired(&mut newcomer, &desktop);
        replicate(&mut desktop, &mut newcomer);
        assert!(newcomer.persona(p).is_err());
        for event in desktop.lan_export(0).unwrap() {
            assert!(!serde_json::to_string(&event).unwrap().contains("螺壳"));
        }
        assert!(newcomer.persona(&survivor.id).is_ok());
    }

    #[test]
    fn recycle_batch_is_transactional_and_confirmation_is_revision_bound() {
        use crate::recycle::RecycleAction;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("v10.sqlite3");
        let mut s = Store::open(&path).unwrap();
        let builtin = s.active_persona_id().unwrap();
        assert!(
            s.change_recycle(RecycleAction::Trash {
                persona_id: builtin
            })
            .is_err()
        );
        let a = s.create_persona("合成甲", "好奇").unwrap();
        let b = s.create_persona("合成乙", "温柔").unwrap();
        for p in [&a, &b] {
            s.change_recycle(RecycleAction::Trash {
                persona_id: p.id.clone(),
            })
            .unwrap();
        }
        s.set_font_scale(140).unwrap();
        drop(s);
        let mut s = Store::open(&path).unwrap();
        let entries = s
            .recycled_personas()
            .unwrap()
            .into_iter()
            .map(|p| p.entry)
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        assert_eq!(s.font_scale().unwrap(), 140);
        assert!(s.set_font_scale(151).is_err());
        let before = s.lan_outbox_count().unwrap();
        s.conn
            .execute_batch(&format!(
                "CREATE TRIGGER inject_purge_failure BEFORE DELETE ON personas
           WHEN old.id='{}' BEGIN SELECT RAISE(ABORT,'fixture write failure'); END",
                entries[1].persona_id
            ))
            .unwrap();
        assert!(
            s.change_recycle(RecycleAction::Purge {
                entries: entries.clone()
            })
            .is_err()
        );
        assert_eq!(s.recycled_personas().unwrap().len(), 2);
        assert_eq!(s.lan_outbox_count().unwrap(), before);
        s.conn
            .execute_batch("DROP TRIGGER inject_purge_failure")
            .unwrap();
        s.change_recycle(RecycleAction::Restore {
            entry: entries[1].clone(),
        })
        .unwrap();
        assert!(
            s.change_recycle(RecycleAction::Purge {
                entries: entries.clone()
            })
            .is_err()
        );
        assert_eq!(
            s.change_recycle(RecycleAction::Purge {
                entries: vec![entries[0].clone()]
            })
            .unwrap()
            .len(),
            1
        );
        assert!(
            s.change_recycle(RecycleAction::Purge {
                entries: vec![entries[0].clone()]
            })
            .is_err()
        );
        assert!(s.persona(&entries[1].persona_id).is_ok());
        assert_eq!(s.diagnostics().unwrap().integrity, "ok");
    }
    #[test]
    fn evidence_life_tasks_and_public_preferences_sync_without_credentials_or_live_leases() {
        let mut a = Store::open(":memory:").unwrap();
        let mut b = Store::open(":memory:").unwrap();
        paired(&mut a, &b);
        paired(&mut b, &a);
        capture(&mut a);
        capture(&mut b);
        let task = a
            .queue_task("virtual", "one-occurrence", &json!({"virtual":true}))
            .unwrap();
        let plan = Uuid::new_v4().to_string();
        let activity = Uuid::new_v4().to_string();
        let occurrence = Uuid::new_v4().to_string();
        a.conn.execute("INSERT INTO life_plans(id,profile_id,name,plan_json,persona_id) VALUES(?1,?2,'fixture','{}',?2)",params![plan,a.profile_id]).unwrap();
        a.conn.execute("INSERT INTO daily_activities(id,profile_id,plan_id,task_id,kind,execution_kind,started_at_ms,ended_at_ms,persona_id) VALUES(?1,?2,?3,?4,'fixture','virtual',0,1,?2)",params![activity,a.profile_id,plan,task]).unwrap();
        a.conn.execute("INSERT INTO life_events(id,profile_id,activity_id,occurrence_key,event_json,persona_id) VALUES(?1,?2,?3,'occurrence','{}',?2)",params![occurrence,a.profile_id,activity]).unwrap();
        a.conn.execute("UPDATE persona_states SET life_json='{\"status\":\"virtual\"}' WHERE persona_id=?1",[&a.profile_id]).unwrap();
        for (key, scope, device, value) in [
            ("theme", "profile", "", "\"dark\""),
            ("api_key", "profile", "", "\"fixture-private-not-synced\""),
            (
                "theme",
                "device",
                "device-local",
                "\"fixture-device-not-synced\"",
            ),
        ] {
            a.conn.execute("INSERT INTO application_settings(profile_id,key,value_json,scope,device_id) VALUES(?1,?2,?3,?4,?5)",params![a.profile_id,key,value,scope,device]).unwrap();
        }
        a.conn
            .execute("DELETE FROM application_settings WHERE scope='device'", [])
            .unwrap();
        let evidence = crate::relationship::RelationshipEvent {
            id: Uuid::new_v4().to_string(),
            origin: a.device_id.clone(),
            lamport: 1,
            evidence_hash: "ab".repeat(32),
            topic: "care".into(),
            kind: crate::relationship::Kind::ThoughtfulCare,
            confidence_milli: 900,
            source_verified: true,
            occurred_at_ms: 0,
            rule_version: 1,
        };
        a.conn.execute("INSERT INTO relationship_events(id,profile_id,evidence_hash,rule_version,kind,evidence_json,occurred_at_ms,lamport,origin_device_id,persona_id) VALUES(?1,?2,?3,1,'ThoughtfulCare',?4,0,1,?5,?2)",params![evidence.id,a.profile_id,evidence.evidence_hash,serde_json::to_string(&evidence).unwrap(),a.device_id]).unwrap();
        replicate(&mut a, &mut b);
        replicate(&mut a, &mut b);
        assert_eq!(
            b.conn
                .query_row("SELECT count(*) FROM life_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            b.conn
                .query_row("SELECT count(*) FROM task_queue", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            b.conn
                .query_row("SELECT count(*) FROM application_settings", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            a.persona_summary(&a.profile_id).unwrap().relationship,
            b.persona_summary(&a.profile_id).unwrap().relationship
        );
        assert_eq!(
            b.persona_summary(&a.profile_id).unwrap().life,
            json!({"status":"virtual"})
        );
        let wire = serde_json::to_string(&a.lan_export(0).unwrap()).unwrap();
        assert!(!wire.contains("fixture-private-not-synced"));
        assert!(!wire.contains("fixture-device-not-synced"));
    }
    #[test]
    fn logical_message_order_survives_device_wall_clock_skew() {
        let mut a = Store::open(":memory:").unwrap();
        let mut b = Store::open(":memory:").unwrap();
        paired(&mut a, &b);
        paired(&mut b, &a);
        capture(&mut a);
        capture(&mut b);
        let persona = a.profile_id.clone();
        let first = Uuid::new_v4().to_string();
        let second = Uuid::new_v4().to_string();
        a.put(
            crate::event::Entity::Message,
            &first,
            json!({"role":"user","content":"先发生","created_at_ms":9999999999999_i64}),
        )
        .unwrap();
        replicate(&mut a, &mut b);
        b.select_persona(&persona).unwrap();
        b.put(
            crate::event::Entity::Message,
            &second,
            json!({"role":"user","content":"后发生但时钟慢","created_at_ms":1}),
        )
        .unwrap();
        replicate(&mut b, &mut a);
        for s in [&a, &b] {
            assert_eq!(
                s.chat_messages(&persona)
                    .unwrap()
                    .iter()
                    .map(|m| m.id.clone())
                    .collect::<Vec<_>>(),
                vec![first.clone(), second.clone()]
            );
        }
    }
    #[test]
    fn v3_migration_backup_and_restart_preserve_identity_and_existing_content() {
        let folder = std::env::temp_dir().join(format!("deepseek-v4-{}", Uuid::new_v4()));
        std::fs::create_dir(&folder).unwrap();
        let file = folder.join("fixture.sqlite3");
        let db = v3(&file);
        let profile = Uuid::new_v4().to_string();
        let device = Uuid::new_v4().to_string();
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
        db.execute(
            "INSERT INTO conversations(id,profile_id,title) VALUES(?1,?1,'fixture')",
            [&profile],
        )
        .unwrap();
        crate::persona::ensure_default(&db, &profile, &device).unwrap();
        let task = Uuid::new_v4().to_string();
        db.execute("INSERT INTO task_queue(id,profile_id,kind,dedupe_key,authority_device_id) VALUES(?1,?2,'fixture','legacy-child',?3)",params![task,profile,device]).unwrap();
        db.execute("INSERT INTO task_checkpoints(task_id,step_key,fence,checkpoint_json,checksum) VALUES(?1,'preserved',1,'{}','fixture')",[&task]).unwrap();
        drop(db);
        {
            let s = Store::open(&file).unwrap();
            assert_eq!(s.device_id, device);
            assert_eq!(s.profile_id, profile);
            assert_eq!(s.personas().unwrap()[0].id, profile);
            assert_eq!(
                s.diagnostics().unwrap().schema_version,
                crate::storage::SCHEMA_VERSION
            );
            assert_eq!(
                s.conn
                    .query_row(
                        "SELECT count(*) FROM task_checkpoints WHERE task_id=?1",
                        [&task],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
        }
        let backups = std::fs::read_dir(&folder)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "bak"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let backup = Connection::open(&backups[0]).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        drop(backup);
        {
            let s = Store::open(&file).unwrap();
            assert_eq!(s.personas().unwrap().len(), 1);
            assert_eq!(s.diagnostics().unwrap().integrity, "ok");
        }
        assert_eq!(
            std::fs::read_dir(&folder)
                .unwrap()
                .filter(|e| e
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|e| e == "bak"))
                .count(),
            1
        );
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn failed_v4_ddl_rolls_back_and_retains_the_usable_v3_backup() {
        let folder = std::env::temp_dir().join(format!("deepseek-rollback-{}", Uuid::new_v4()));
        std::fs::create_dir(&folder).unwrap();
        let file = folder.join("fixture.sqlite3");
        let db = v3(&file);
        db.execute_batch("CREATE TABLE lan_control(deliberate_schema_collision TEXT)")
            .unwrap();
        drop(db);
        assert!(Store::open(&file).is_err());
        let db = Connection::open(&file).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM schema_migrations WHERE version=4",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        drop(db);
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn legacy_defaults_never_accept_other_missing_or_extra_signed_fields() {
        let mut store = Store::open(":memory:").unwrap();
        capture(&mut store);
        let mut event = store
            .lan_export(0)
            .unwrap()
            .into_iter()
            .find(|e| e.table == "personas")
            .unwrap();
        event.row.as_object_mut().unwrap().remove("avatar_resource");
        event.fields.remove("avatar_resource");
        validate_row(&store.conn, &event, &store.device_id).unwrap();
        let mut unknown = event.clone();
        unknown.row["unsupported_field"] = json!("fixture-only");
        assert!(validate_row(&store.conn, &unknown, &store.device_id).is_err());
        let mut missing = event;
        missing.row.as_object_mut().unwrap().remove("name");
        missing.fields.remove("name");
        assert!(validate_row(&store.conn, &missing, &store.device_id).is_err());
    }
}
