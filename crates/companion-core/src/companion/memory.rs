use super::{MEMBERS, excerpt, stable_id};
use crate::{
    Error, Result,
    event::{Entity, Operation, sha256_hex},
    storage::Store,
};
use rusqlite::{OptionalExtension, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct ManagedMemory {
    pub id: String,
    pub text: String,
    pub version: i64,
    pub revision: String,
    pub kind: String,
    pub importance_milli: i64,
    pub pinned: bool,
    pub api_allowed: bool,
    pub owner_locked: bool,
    pub source_kind: String,
    pub source_message_id: Option<String>,
    pub source_life_event_id: Option<String>,
    pub source_excerpt: Option<String>,
    pub created_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryEdit {
    pub id: Option<String>,
    pub expected_version: Option<i64>,
    #[serde(default)]
    pub expected_revision: Option<String>,
    pub text: String,
    pub kind: String,
    pub importance_milli: i64,
    pub pinned: bool,
    pub api_allowed: bool,
}
#[derive(Debug, Serialize)]
pub struct RetrievedMemory {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub source_message_id: Option<String>,
}
pub fn normalized(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn topic_key(kind: &str, content: &str) -> String {
    let topic = if let Some(value) = content
        .strip_prefix("我不喜欢")
        .or_else(|| content.strip_prefix("我喜欢"))
    {
        format!("preference:{}", normalized(value))
    } else if let Some((subject, _)) = content.split_once('是') {
        format!("fact:{}", normalized(subject))
    } else {
        format!("{kind}:{}", normalized(content))
    };
    excerpt(&topic, 120)
}
fn classify(text: &str) -> Option<(String, String, String, i64)> {
    if [
        "密码",
        "私钥",
        "API Key",
        "api_key",
        "银行卡",
        "不要记住",
        "别记住",
    ]
    .iter()
    .any(|s| text.contains(s))
    {
        return None;
    }
    let explicit = text
        .strip_prefix("请记住")
        .or_else(|| text.strip_prefix("记住，"));
    let content = explicit
        .unwrap_or(text)
        .trim_start_matches(['：', ':', '，', ',', ' '])
        .trim();
    if explicit.is_none()
        && (content.starts_with("我叫你")
            || ["？", "?", "什么", "哪里", "怎么"]
                .iter()
                .any(|mark| content.contains(mark)))
    {
        return None;
    }
    let kind = if content.contains("这个梗的意思是") || content.starts_with("网络梗：") {
        "knowledge"
    } else if content.contains("我们的约定") {
        "relationship"
    } else if (explicit.is_some() && content.starts_with("我们一起"))
        || (content.starts_with("今天我")
            && ["第一次", "重要", "毕业"]
                .iter()
                .any(|v| content.contains(v)))
    {
        "episodic"
    } else if explicit.is_some()
        || content.starts_with("我喜欢")
        || content.starts_with("我不喜欢")
        || content.starts_with("我最喜欢")
        || content.starts_with("我的工作是")
        || content.starts_with("我的职业是")
        || content.starts_with("我习惯")
        || content.starts_with("我叫")
        || content.starts_with("我的名字是")
        || content.starts_with("我住在")
        || content.starts_with("我的生日是")
    {
        "semantic"
    } else {
        return None;
    };
    if !(3..=1000).contains(&content.chars().count()) {
        return None;
    }
    Some((
        kind.into(),
        content.into(),
        topic_key(kind, content),
        if explicit.is_some() { 800 } else { 650 },
    ))
}
pub(crate) fn extract(
    db: &rusqlite::Transaction<'_>,
    owner: &str,
    device: &str,
    source: &str,
    text: &str,
    at: i64,
) -> Result<()> {
    let Some((kind, content, topic, importance)) = classify(text) else {
        return Ok(());
    };
    let canonical = crate::pairing::resolve(db, owner)?;
    let mut q=db.prepare(&format!("SELECT m.id,m.text,m.version_lamport,coalesce(d.topic_key,''),m.version_event_id FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id WHERE m.persona_id IN ({MEMBERS}) AND m.is_deleted=0 LIMIT 10000"))?;
    let rows = q
        .query_map([&canonical], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if rows
        .iter()
        .any(|(_, value, _, _, _)| normalized(value) == normalized(&content))
    {
        return Ok(());
    }
    let conflict = rows.iter().find(|(_, _, _, key, _)| key == &topic);
    if let Some((id, _, version, _, revision)) = conflict {
        let mut pending=db.prepare(&format!("SELECT content FROM companion_memory_candidates WHERE persona_id IN ({MEMBERS}) AND state='pending' LIMIT 1000"))?;
        let pending = pending
            .query_map([&canonical], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if pending
            .iter()
            .any(|value| normalized(value) == normalized(&content))
        {
            return Ok(());
        }

        db.execute("INSERT OR IGNORE INTO companion_memory_candidates(id,persona_id,source_message_id,kind,topic_key,content,importance_milli,created_at_ms,conflicting_memory_id,expected_memory_version,expected_memory_revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![stable_id("memory-candidate",source),owner,source,kind,topic,content,importance,at,id,version,revision])?;
    } else {
        let id = stable_id(
            "automatic-memory",
            &format!("{owner}/{}", sha256_hex(normalized(&content).as_bytes())),
        );
        write(
            db,
            device,
            owner,
            &id,
            &content,
            &kind,
            importance,
            false,
            true,
            Some(source),
            "chat",
            &topic,
            at,
            false,
        )?;
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn write(
    db: &rusqlite::Transaction<'_>,
    device: &str,
    owner: &str,
    id: &str,
    text: &str,
    kind: &str,
    importance: i64,
    pinned: bool,
    allowed: bool,
    source: Option<&str>,
    source_kind: &str,
    topic: &str,
    at: i64,
    locked: bool,
) -> Result<()> {
    let profile: String = db.query_row(
        "SELECT profile_id FROM personas WHERE id=?1",
        [owner],
        |r| r.get(0),
    )?;
    Store::append_in(
        db,
        device,
        &profile,
        Entity::Memory,
        id,
        Operation::Put,
        json!({"text":text,"persona_id":owner,"importance_milli":importance,"sensitivity":if allowed{"api_allowed"}else{"private"}}),
    )?;
    db.execute("INSERT INTO companion_memory_meta(memory_id,persona_id,kind,source_message_id,source_kind,topic_key,pinned,owner_locked,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9) ON CONFLICT(memory_id) DO UPDATE SET kind=excluded.kind,source_message_id=coalesce(excluded.source_message_id,source_message_id),topic_key=CASE WHEN excluded.topic_key='' THEN topic_key ELSE excluded.topic_key END,pinned=excluded.pinned,owner_locked=excluded.owner_locked,updated_at_ms=max(updated_at_ms,excluded.updated_at_ms),revision=revision+1",params![id,owner,kind,source,source_kind,topic,pinned,locked,at])?;
    Ok(())
}
struct PendingMemoryRevision {
    owner: String,
    source: String,
    text: String,
    kind: String,
    topic: String,
    importance: i64,
    state: String,
    target: Option<String>,
    expected: Option<i64>,
    expected_revision: Option<String>,
}
impl Store {
    pub fn managed_memories(&self, persona: &str) -> Result<Vec<ManagedMemory>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let mut q=self.conn.prepare(&format!("SELECT m.id,m.text,m.version_lamport,coalesce(d.kind,'semantic'),m.importance_milli,coalesce(d.pinned,0),m.sensitivity='api_allowed',coalesce(d.owner_locked,1),coalesce(d.source_kind,'legacy'),d.source_message_id,d.created_at_ms,d.updated_at_ms,m.version_event_id,coalesce((SELECT content FROM messages WHERE id=d.source_message_id),(SELECT description FROM companion_life_events WHERE id=d.source_life_event_id)),d.source_life_event_id FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id WHERE m.persona_id IN ({MEMBERS}) AND m.is_deleted=0 ORDER BY coalesce(d.pinned,0) DESC,m.version_lamport DESC,m.id LIMIT 500"))?;
        Ok(q.query_map([persona], |r| {
            Ok(ManagedMemory {
                id: r.get(0)?,
                text: r.get(1)?,
                version: r.get(2)?,
                kind: r.get(3)?,
                importance_milli: r.get(4)?,
                pinned: r.get(5)?,
                api_allowed: r.get(6)?,
                owner_locked: r.get(7)?,
                source_kind: r.get(8)?,
                source_message_id: r.get(9)?,
                created_at_ms: r.get(10)?,
                updated_at_ms: r.get(11)?,
                revision: r.get(12)?,
                source_excerpt: r.get::<_, Option<String>>(13)?.map(|v| excerpt(&v, 160)),
                source_life_event_id: r.get(14)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn edit_companion_memory(&mut self, persona: &str, edit: &MemoryEdit) -> Result<String> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        if edit.text.trim().is_empty()
            || edit.text.len() > 8000
            || !matches!(
                edit.kind.as_str(),
                "semantic" | "episodic" | "relationship" | "knowledge"
            )
            || !(0..=1000).contains(&edit.importance_milli)
        {
            return Err(Error::Invalid("memory edit"));
        }
        let id = edit
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        Uuid::parse_str(&id).map_err(|_| Error::Invalid("memory UUID"))?;
        let tx = self.conn.transaction()?;
        let old: Option<(String, i64, bool, String)> = tx
            .query_row(
                "SELECT persona_id,version_lamport,is_deleted,version_event_id FROM memories WHERE id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?,r.get(3)?)),
            )
            .optional()?;
        let owner = if let Some((owner, version, deleted, revision)) = old {
            if crate::pairing::resolve(&tx, &owner)? != crate::pairing::resolve(&tx, persona)? {
                return Err(Error::Untrusted);
            }
            if deleted
                || edit.expected_version != Some(version)
                || edit.expected_revision.as_deref() != Some(revision.as_str())
            {
                return Err(Error::Conflict);
            }
            owner
        } else {
            if edit.id.is_some()
                || edit.expected_version.is_some()
                || edit.expected_revision.is_some()
            {
                return Err(Error::Conflict);
            }
            persona.into()
        };
        write(
            &tx,
            &self.device_id,
            &owner,
            &id,
            edit.text.trim(),
            &edit.kind,
            edit.importance_milli,
            edit.pinned,
            edit.api_allowed,
            None,
            "manual",
            &topic_key(&edit.kind, edit.text.trim()),
            crate::chat::now_ms(),
            true,
        )?;
        tx.commit()?;
        Ok(id)
    }
    pub fn memory_candidates(&self, persona: &str) -> Result<Vec<Value>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let mut q=self.conn.prepare(&format!("SELECT id,content,kind,source_message_id,created_at_ms,conflicting_memory_id FROM companion_memory_candidates WHERE persona_id IN ({MEMBERS}) AND state='pending' ORDER BY created_at_ms DESC,id LIMIT 100"))?;
        Ok(q.query_map([persona],|r|Ok(json!({"id":r.get::<_,String>(0)?,"content":r.get::<_,String>(1)?,"kind":r.get::<_,String>(2)?,"source_message_id":r.get::<_,String>(3)?,"created_at_ms":r.get::<_,i64>(4)?,"conflicting_memory_id":r.get::<_,Option<String>>(5)?})))?.collect::<std::result::Result<Vec<_>,_>>()?)
    }
    pub fn resolve_memory_candidate(
        &mut self,
        persona: &str,
        id: &str,
        accept: bool,
    ) -> Result<()> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let tx = self.conn.transaction()?;
        let PendingMemoryRevision { owner,source,text,kind,topic,importance,state,target,expected,expected_revision }=tx.query_row(
            "SELECT persona_id,source_message_id,content,kind,topic_key,importance_milli,state,conflicting_memory_id,expected_memory_version,expected_memory_revision FROM companion_memory_candidates WHERE id=?1",[id],|r|Ok(PendingMemoryRevision{
                owner:r.get(0)?,source:r.get(1)?,text:r.get(2)?,kind:r.get(3)?,topic:r.get(4)?,importance:r.get(5)?,state:r.get(6)?,target:r.get(7)?,expected:r.get(8)?,expected_revision:r.get(9)?
            }))?;
        if crate::pairing::resolve(&tx, &owner)? != crate::pairing::resolve(&tx, persona)? {
            return Err(Error::Untrusted);
        }
        if state != "pending" {
            return Ok(());
        }
        if accept {
            let memory = target
                .clone()
                .unwrap_or_else(|| stable_id("accepted-memory", id));
            let actual:Option<(String,i64,bool,bool,String)>=tx.query_row("SELECT m.persona_id,m.version_lamport,coalesce(d.pinned,0),m.is_deleted,m.version_event_id FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id WHERE m.id=?1",[&memory],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
            let (memory_owner, pinned) = if let Some((o, v, p, d, revision)) = actual {
                if d || expected != Some(v)
                    || expected_revision.as_deref() != Some(revision.as_str())
                {
                    return Err(Error::Conflict);
                }
                if crate::pairing::resolve(&tx, &o)? != crate::pairing::resolve(&tx, persona)? {
                    return Err(Error::Untrusted);
                }
                (o, p)
            } else {
                if target.is_some() {
                    return Err(Error::Conflict);
                }
                (persona.to_owned(), false)
            };
            write(
                &tx,
                &self.device_id,
                &memory_owner,
                &memory,
                &text,
                &kind,
                importance,
                pinned,
                true,
                Some(&source),
                "chat",
                &topic,
                crate::chat::now_ms(),
                true,
            )?;
            tx.execute("UPDATE companion_memory_candidates SET state='accepted',resolved_memory_id=?1 WHERE id=?2",params![memory,id])?;
        } else {
            tx.execute(
                "UPDATE companion_memory_candidates SET state='rejected' WHERE id=?1",
                [id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn retrieve_companion_memories(
        &self,
        persona: &str,
        query: &str,
        limit: usize,
        budget: usize,
    ) -> Result<Vec<RetrievedMemory>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        if !(1..=12).contains(&limit) || budget > 6000 {
            return Err(Error::Invalid("memory retrieval budget"));
        }
        let chars = normalized(&excerpt(query, 256)).chars().collect::<Vec<_>>();
        let mut terms = chars
            .windows(2)
            .map(|v| v.iter().collect::<String>())
            .collect::<Vec<_>>();
        terms.extend(
            query
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.len() >= 2)
                .take(12)
                .map(|s| s.to_lowercase()),
        );
        terms.sort();
        terms.dedup();
        terms.truncate(64);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let predicates = terms
            .iter()
            .enumerate()
            .map(|(i, _)| format!("instr(lower(m.text),?{})>0", i + 2))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut parameters = vec![SqlValue::Text(persona.into())];
        parameters.extend(terms.iter().cloned().map(SqlValue::Text));
        let mut q=self.conn.prepare(&format!("SELECT m.id,m.text,coalesce(d.kind,'semantic'),d.source_message_id,m.importance_milli,coalesce(d.pinned,0) FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id WHERE m.persona_id IN ({MEMBERS}) AND m.is_deleted=0 AND m.sensitivity='api_allowed' AND ({predicates}) ORDER BY coalesce(d.pinned,0) DESC,m.importance_milli DESC,m.version_lamport DESC LIMIT 128"))?;
        let mut candidates = q
            .query_map(rusqlite::params_from_iter(parameters), |r| {
                Ok((
                    RetrievedMemory {
                        id: r.get(0)?,
                        text: r.get(1)?,
                        kind: r.get(2)?,
                        source_message_id: r.get(3)?,
                    },
                    r.get::<_, i64>(4)?,
                    r.get::<_, bool>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        candidates.sort_by_key(|(m, importance, pinned)| {
            std::cmp::Reverse(
                terms
                    .iter()
                    .filter(|t| normalized(&m.text).contains(t.as_str()))
                    .count() as i64
                    * 100
                    + importance / 10
                    + if *pinned { 200 } else { 0 },
            )
        });
        let mut result = Vec::new();
        let mut used = 2;
        let mut seen = std::collections::BTreeSet::new();
        for (mut memory, _, _) in candidates {
            if !seen.insert(normalized(&memory.text)) {
                continue;
            }
            memory.text = excerpt(&memory.text, 180);
            let size = serde_json::to_vec(&memory)?.len() + 1;
            if used + size > budget {
                continue;
            }
            used += size;
            result.push(memory);
            if result.len() == limit {
                break;
            }
        }
        Ok(result)
    }
}
