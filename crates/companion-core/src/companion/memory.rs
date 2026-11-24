use super::{
    MEMBERS, excerpt,
    intelligence::{self, MemoryContext, MemoryState},
    stable_id,
};
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
    pub last_used_at_ms: Option<i64>,
    pub context: MemoryContext,
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
    #[serde(default)]
    pub context: Option<MemoryContext>,
}
#[derive(Debug, Serialize)]
pub struct RetrievedMemory {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub source_message_id: Option<String>,
    pub occurred_at_ms: i64,
    pub due_at_ms: Option<i64>,
    pub event_state: MemoryState,
    pub evidence_kind: String,
}
pub fn normalized(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn topic_key(kind: &str, content: &str) -> String {
    if intelligence::observed_topic(content) == Some("routine/sleep")
        && ["我习惯", "我现在", "我以后", "我的作息"]
            .iter()
            .any(|v| content.starts_with(v))
    {
        return "routine/sleep".into();
    }
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
    if !intelligence::clear_statement(text) {
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
    offset: i64,
) -> Result<()> {
    let canonical = crate::pairing::resolve(db, owner)?;
    let text = text.trim();
    let topic_hint = intelligence::observed_topic(text);
    if let Some(topic) = topic_hint {
        let do_not_mention = ["别再提", "不要再提", "不用再提醒", "不要提醒我"]
            .iter()
            .any(|mark| text.contains(mark));
        if intelligence::completed(text) || do_not_mention {
            let state = if do_not_mention {
                "dismissed"
            } else {
                "completed"
            };
            db.execute(&format!("UPDATE companion_memory_context SET state=?3,recall_allowed=0,reminder_allowed=0 WHERE persona_id IN ({MEMBERS}) AND is_event=1 AND state='active' AND memory_id IN(SELECT d.memory_id FROM companion_memory_meta d JOIN memories m ON m.id=d.memory_id WHERE d.topic_key=?2 AND m.is_deleted=0) AND (SELECT count(*) FROM companion_memory_context x JOIN companion_memory_meta d ON d.memory_id=x.memory_id JOIN memories m ON m.id=d.memory_id WHERE x.persona_id IN ({MEMBERS}) AND x.is_event=1 AND x.state='active' AND m.is_deleted=0 AND d.topic_key=?2)=1"), params![canonical, topic, state])?;
            return Ok(());
        }
    }
    let event = intelligence::event_context(text, at, offset);
    let correction =
        intelligence::explicit_correction(text) && topic_hint.is_some() && text.starts_with('我');
    let classified = if let Some(context) = &event {
        Some((
            if context.shared_story {
                "relationship"
            } else {
                "episodic"
            }
            .into(),
            text.into(),
            topic_hint.unwrap_or_default().into(),
            760,
        ))
    } else if correction {
        Some((
            "semantic".into(),
            text.into(),
            topic_hint.unwrap_or_default().into(),
            880,
        ))
    } else {
        classify(text)
    };
    let Some((kind, content, topic, importance)) = classified else {
        return Ok(());
    };
    if !(3..=1000).contains(&content.chars().count()) {
        return Ok(());
    }
    let id = stable_id(
        "automatic-memory",
        &format!("{owner}/{}", sha256_hex(normalized(&content).as_bytes())),
    );
    let known: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM memories WHERE id=?1)",
        [&id],
        |r| r.get(0),
    )?;
    if known {
        return Ok(());
    }
    let shared_story =
        kind == "relationship" || (kind == "episodic" && content.starts_with("我们一起"));
    let context = event.unwrap_or_else(|| MemoryContext {
        occurred_at_ms: at,
        shared_story,
        confidence_milli: 900,
        recall_allowed: shared_story || text.starts_with("请记住") || text.starts_with("记住，"),
        ..Default::default()
    });
    let mut q=db.prepare(&format!("SELECT m.id,m.text,m.version_lamport,coalesce(d.topic_key,''),m.version_event_id,coalesce(d.owner_locked,1) FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id LEFT JOIN companion_memory_context x ON x.memory_id=m.id WHERE m.persona_id IN ({MEMBERS}) AND m.is_deleted=0 AND coalesce(x.state,'active')='active' AND (d.topic_key=?2 OR m.text=?3) ORDER BY coalesce(d.updated_at_ms,0) DESC,m.id LIMIT 128"))?;
    let rows = q
        .query_map(params![canonical, topic, content], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, bool>(5)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if rows
        .iter()
        .any(|(_, value, _, _, _, _)| normalized(value) == normalized(&content))
    {
        return Ok(());
    }
    let conflict = rows.iter().find(|(_, _, _, key, _, _)| key == &topic);
    if let Some((target, _, version, _, revision, locked)) = conflict
        && !(!locked && correction && rows.len() == 1)
    {
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

        db.execute("INSERT OR IGNORE INTO companion_memory_candidates(id,persona_id,source_message_id,kind,topic_key,content,importance_milli,created_at_ms,conflicting_memory_id,expected_memory_version,expected_memory_revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![stable_id("memory-candidate",&format!("{source}/{topic}")),owner,source,kind,topic,content,importance,at,target,version,revision])?;
    } else {
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
        intelligence::save_context(db, &id, owner, &context)?;
        if let Some((old, _, _, _, _, _)) = conflict {
            db.execute("INSERT INTO companion_memory_context(memory_id,persona_id,occurred_at_ms,state) SELECT id,persona_id,?2,'superseded' FROM memories WHERE id=?1 ON CONFLICT(memory_id) DO UPDATE SET state='superseded',recall_allowed=0,reminder_allowed=0", params![old, at])?;
            db.execute("INSERT OR IGNORE INTO memory_relations(from_memory_id,to_memory_id,relation) VALUES(?1,?2,'supersedes')", params![id, old])?;
        }
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn write(
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
    pub fn collect_shared_memory(&mut self, persona: &str, message_id: &str) -> Result<String> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let tx = self.conn.transaction()?;
        let (owner, text, at) = tx.query_row(&format!("SELECT c.persona_id,m.content,m.created_at_ms FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE c.persona_id IN ({MEMBERS}) AND m.id=?2 AND m.role='user' AND m.is_deleted=0"), params![persona,message_id], |r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<i64>>(2)?)))?;
        if !intelligence::safe_source(&text) {
            return Err(Error::Invalid("private memory source"));
        }
        let at = at.ok_or(Error::Invalid("undated memory source"))?;
        let id = stable_id("shared-chat-memory", &format!("{owner}/{message_id}"));
        let deleted: Option<bool> = tx
            .query_row("SELECT is_deleted FROM memories WHERE id=?1", [&id], |r| {
                r.get(0)
            })
            .optional()?;
        if deleted == Some(true) {
            return Err(Error::Conflict);
        }
        if deleted.is_none() {
            let content = excerpt(
                &text.replace(intelligence::USER_FRAGMENT_SEPARATOR, "\n"),
                1200,
            );
            write(
                &tx,
                &self.device_id,
                &owner,
                &id,
                &content,
                "episodic",
                800,
                false,
                false,
                Some(message_id),
                "chat",
                "",
                at,
                true,
            )?;
            intelligence::save_context(
                &tx,
                &id,
                &owner,
                &MemoryContext {
                    shared_story: true,
                    occurred_at_ms: at,
                    ..Default::default()
                },
            )?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn managed_memories(&self, persona: &str) -> Result<Vec<ManagedMemory>> {
        self.search_managed_memories(persona, "")
    }

    pub fn search_managed_memories(
        &self,
        persona: &str,
        search: &str,
    ) -> Result<Vec<ManagedMemory>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let search = excerpt(search.trim(), 96);
        let mut q=self.conn.prepare(&format!("SELECT m.id,m.text,m.version_lamport,coalesce(d.kind,'semantic'),m.importance_milli,coalesce(d.pinned,0),m.sensitivity='api_allowed',coalesce(d.owner_locked,1),coalesce(d.source_kind,'legacy'),d.source_message_id,d.created_at_ms,d.updated_at_ms,m.version_event_id,coalesce((SELECT content FROM messages WHERE id=d.source_message_id AND is_deleted=0),(SELECT description FROM companion_life_events WHERE id=d.source_life_event_id)),d.source_life_event_id,u.last_used_at_ms,coalesce(x.is_event,0),coalesce(x.shared_story,d.kind='relationship' AND d.source_message_id IS NOT NULL,0),coalesce(x.state,'active'),coalesce(x.occurred_at_ms,d.created_at_ms,0),x.due_at_ms,coalesce(x.confidence_milli,1000),coalesce(x.recall_allowed,0),coalesce(x.reminder_allowed,0),coalesce(x.shared_with_personas,0) FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id LEFT JOIN companion_memory_context x ON x.memory_id=m.id LEFT JOIN companion_memory_usage u ON u.memory_id=m.id WHERE m.persona_id IN ({MEMBERS}) AND m.is_deleted=0 AND (?2='' OR instr(lower(m.text),lower(?2))>0 OR EXISTS(SELECT 1 FROM messages src WHERE src.id=d.source_message_id AND src.is_deleted=0 AND instr(lower(src.content),lower(?2))>0)) ORDER BY coalesce(d.pinned,0) DESC,m.version_lamport DESC,m.id LIMIT 500"))?;
        Ok(q.query_map(params![persona, search], |r| {
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
                last_used_at_ms: r.get(15)?,
                context: intelligence::context_row(r, 16)?,
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
        if let Some(context) = &edit.context {
            if (context.recall_allowed || context.shared_with_personas) && !edit.api_allowed {
                return Err(Error::Invalid(
                    "memory context requires permission for model use",
                ));
            }
            if context.shared_story && !tx.query_row::<bool, _, _>("SELECT EXISTS(SELECT 1 FROM companion_memory_meta d JOIN messages m ON m.id=d.source_message_id WHERE d.memory_id=?1 AND m.is_deleted=0 AND m.role='user')", [&id], |r| r.get(0))? {
                return Err(Error::Invalid("shared memory must have an original chat message"));
            }
            intelligence::save_context(&tx, &id, &owner, context)?;
        } else if !edit.api_allowed {
            tx.execute("UPDATE companion_memory_context SET recall_allowed=0,reminder_allowed=0,shared_with_personas=0 WHERE memory_id=?1", [&id])?;
        }
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
        let topic = intelligence::observed_topic(query);
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
        terms.truncate(48);
        for alias in intelligence::query_aliases(topic) {
            if !terms.iter().any(|v| v == alias) {
                terms.push((*alias).into());
            }
        }
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let predicates = terms
            .iter()
            .enumerate()
            .map(|(i, _)| format!("instr(lower(m.text),?{})>0", i + 5))
            .collect::<Vec<_>>()
            .join(" OR ");
        let fts = terms
            .iter()
            .filter(|term| term.chars().count() >= 3)
            .take(16)
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut parameters = vec![
            SqlValue::Text(persona.into()),
            SqlValue::Text(topic.unwrap_or_default().into()),
            SqlValue::Text(fts),
            SqlValue::Text(self.profile_id.clone()),
        ];
        parameters.extend(terms.iter().cloned().map(SqlValue::Text));
        let mut q=self.conn.prepare(&format!(
            "WITH search_hits AS (SELECT memory_id FROM memory_fts WHERE ?3<>'' AND memory_fts MATCH ?3 LIMIT 256)
             SELECT m.id,m.text,coalesce(d.kind,'semantic'),d.source_message_id,m.importance_milli,coalesce(d.pinned,0),
               coalesce(x.occurred_at_ms,d.created_at_ms,0),x.due_at_ms,coalesce(x.state,'active'),coalesce(d.source_kind,'legacy'),
               coalesce(x.confidence_milli,800),u.last_used_at_ms,coalesce(d.topic_key,''),m.id IN(SELECT memory_id FROM search_hits),
               m.persona_id IN ({MEMBERS})
             FROM memories m LEFT JOIN companion_memory_meta d ON d.memory_id=m.id
             LEFT JOIN companion_memory_context x ON x.memory_id=m.id LEFT JOIN companion_memory_usage u ON u.memory_id=m.id
             WHERE (m.persona_id IN ({MEMBERS}) OR (x.shared_with_personas=1 AND (m.profile_id=?4 OR m.profile_id IN (SELECT profile_id FROM lan_profiles))))
             AND m.is_deleted=0 AND m.sensitivity='api_allowed' AND coalesce(x.state,'active') IN ('active','completed')
             AND NOT EXISTS(SELECT 1 FROM persona_lifecycle l WHERE l.persona_id=m.persona_id AND l.state<>'active')
             AND (d.source_message_id IS NULL OR EXISTS(SELECT 1 FROM messages src WHERE src.id=d.source_message_id AND src.is_deleted=0))
             AND ((?2<>'' AND d.topic_key=?2) OR m.id IN(SELECT memory_id FROM search_hits) OR {predicates})
             ORDER BY (?2<>'' AND d.topic_key=?2) DESC,coalesce(d.pinned,0) DESC,m.importance_milli DESC,m.version_lamport DESC LIMIT 128"))?;
        let mut candidates = q
            .query_map(rusqlite::params_from_iter(parameters), |r| {
                let owner: bool = r.get(14)?;
                let state: String = r.get(8)?;
                Ok((
                    RetrievedMemory {
                        id: r.get(0)?,
                        text: r.get(1)?,
                        kind: r.get(2)?,
                        source_message_id: if owner { r.get(3)? } else { None },
                        occurred_at_ms: r.get(6)?,
                        due_at_ms: r.get(7)?,
                        event_state: if state == "completed" {
                            MemoryState::Completed
                        } else {
                            MemoryState::Active
                        },
                        evidence_kind: if owner {
                            r.get(9)?
                        } else {
                            "explicitly_shared".into()
                        },
                    },
                    r.get::<_, i64>(4)?,
                    r.get::<_, bool>(5)?,
                    r.get::<_, i64>(10)?,
                    r.get::<_, Option<i64>>(11)?,
                    r.get::<_, String>(12)?,
                    r.get::<_, bool>(13)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let now = crate::chat::now_ms();
        candidates.sort_by_key(
            |(m, importance, pinned, confidence, last_used, key, full_text)| {
                let match_text = normalized(&m.text);
                std::cmp::Reverse(
                    terms
                        .iter()
                        .filter(|t| match_text.contains(t.as_str()))
                        .count() as i64
                        * 100
                        + importance / 10
                        + confidence / 25
                        + if *pinned { 200 } else { 0 }
                        + if Some(key.as_str()) == topic { 350 } else { 0 }
                        + if *full_text { 100 } else { 0 }
                        + (30 - now.saturating_sub(m.occurred_at_ms).max(0) / intelligence::DAY_MS)
                            .max(0)
                        - if last_used.is_some_and(|v| now.saturating_sub(v) < 600_000) {
                            10
                        } else {
                            0
                        },
                )
            },
        );
        let mut result = Vec::new();
        let mut used = 2;
        let mut seen = std::collections::BTreeSet::new();
        for (mut memory, ..) in candidates {
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
