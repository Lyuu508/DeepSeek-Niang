//! Select real, authorized memories before taking a lease or spending model tokens.
use super::{MEMBERS, excerpt, intelligence::DAY_MS, life, stable_id};
use crate::{Result, storage::Store};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};

const MAX_PENDING_AGE: i64 = 15 * 60_000;

impl Store {
    pub fn stage_memory_recollection(&mut self, now: i64) -> Result<()> {
        for persona in self.personas()? {
            let p = &persona.id;
            if !self.owns_life(p)? {
                continue;
            }
            let pref = self.companion_preferences(p)?;
            let host: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM companion_hosts WHERE persona_id=?1 AND device_id=?2)",
                params![p, self.device_id],
                |r| r.get(0),
            )?;
            if !host {
                continue;
            }
            let pending = {
                let mut q = self.conn.prepare("SELECT id,created_at_ms FROM companion_proactive WHERE persona_id=?1 AND host_device_id=?2 AND source_memory_id IS NOT NULL AND state='candidate'")?;
                q.query_map(params![p, self.device_id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
            };
            let mut retain = false;
            for (id, created) in pending {
                if pref.proactive_enabled
                    && pref.proactive_model_enabled
                    && pref.memory_context
                    && pref.proactive_mode != "off"
                    && (0..=MAX_PENDING_AGE).contains(&now.saturating_sub(created))
                    && eligible(&self.conn, &id, now)?
                {
                    retain = true;
                } else {
                    self.conn.execute("UPDATE companion_proactive SET state='dismissed' WHERE id=?1 AND state='candidate'", [&id])?;
                }
            }
            if retain
                || !pref.proactive_model_enabled
                || !pref.memory_context
                || !life::policy(&self.conn, p, &pref, now, &self.bond_snapshot(p)?.stage)?
            {
                continue;
            }
            let picked: Option<(String, String)> = self.conn.query_row(&format!(
                "SELECT m.id,m.version_event_id FROM companion_memory_context x JOIN memories m ON m.id=x.memory_id
                 JOIN companion_memory_meta d ON d.memory_id=m.id
                 WHERE x.persona_id IN ({MEMBERS}) AND x.state='active' AND x.recall_allowed=1 AND x.confidence_milli>=700
                 AND m.sensitivity='api_allowed' AND m.is_deleted=0 AND m.importance_milli>=500
                 AND x.occurred_at_ms BETWEEN 1 AND ?2-1800000
                 AND (x.is_event=0 OR (x.due_at_ms IS NULL AND x.occurred_at_ms>=?2-?3)
                      OR x.due_at_ms BETWEEN ?2-?4 AND ?2)
                 AND (d.source_message_id IS NULL OR EXISTS(SELECT 1 FROM messages src WHERE src.id=d.source_message_id AND src.is_deleted=0 AND src.role='user'))
                 AND NOT EXISTS (SELECT 1 FROM companion_proactive prev
                   LEFT JOIN companion_memory_meta prior ON prior.memory_id=prev.source_memory_id
                   WHERE prev.persona_id IN ({MEMBERS}) AND prev.source_memory_id IS NOT NULL
                   AND (prev.source_memory_id=m.id OR prev.created_at_ms>?2-?5
                   OR (d.topic_key<>'' AND prior.topic_key=d.topic_key AND prev.created_at_ms>?2-?3)))
                 ORDER BY CASE WHEN x.due_at_ms IS NOT NULL THEN 2 WHEN x.is_event=1 THEN 1 ELSE 0 END DESC,
                   coalesce(d.pinned,0) DESC,m.importance_milli DESC,x.confidence_milli DESC,x.occurred_at_ms DESC,m.id
                 LIMIT 1"), params![p,now,30*DAY_MS,7*DAY_MS,pref.proactive_interval_minutes*60_000], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
            if let Some((memory, revision)) = picked {
                let share = stable_id("memory-recollection", &format!("{p}/{memory}"));
                self.conn.execute("INSERT OR IGNORE INTO companion_proactive(id,persona_id,source_memory_id,source_memory_revision,host_device_id,content,created_at_ms,state) VALUES(?1,?2,?3,?4,?5,'',?6,'candidate')", params![share,p,memory,revision,self.device_id,now])?;
            }
        }
        Ok(())
    }

    pub(crate) fn recollection_evidence(
        &self,
        id: &str,
        persona: &str,
        now: i64,
    ) -> Result<Option<Value>> {
        if !eligible(&self.conn, id, now)? {
            return Ok(None);
        }
        let source = self.conn.query_row("SELECT m.id,m.text,d.source_message_id,d.source_kind,x.is_event,x.shared_story,x.occurred_at_ms,x.due_at_ms,x.reminder_allowed FROM companion_proactive p JOIN memories m ON m.id=p.source_memory_id JOIN companion_memory_context x ON x.memory_id=m.id JOIN companion_memory_meta d ON d.memory_id=m.id WHERE p.id=?1 AND p.persona_id=?2", params![id,persona], |r| Ok(json!({
            "id":r.get::<_,String>(0)?,"statement":excerpt(&r.get::<_,String>(1)?,160),
            "source_message_id":r.get::<_,Option<String>>(2)?,"source_kind":r.get::<_,String>(3)?,
            "is_event":r.get::<_,bool>(4)?,"shared_story":r.get::<_,bool>(5)?,
            "occurred_at_ms":r.get::<_,i64>(6)?,"due_at_ms":r.get::<_,Option<i64>>(7)?,"reminder_allowed":r.get::<_,bool>(8)?,
        })))?;
        let mut time = self.time_evidence(persona, now)?;
        if let Some(obj) = time.as_object_mut() {
            obj.remove("yesterday");
            obj.remove("tomorrow");
        }
        Ok(Some(
            json!({"memory":source,"device_time":time,"role_interests":self.persona_interest_evidence(persona)?}),
        ))
    }
}

pub(crate) fn eligible(db: &Connection, id: &str, now: i64) -> Result<bool> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM companion_proactive p JOIN memories m ON m.id=p.source_memory_id JOIN companion_memory_context x ON x.memory_id=m.id JOIN companion_memory_meta d ON d.memory_id=m.id WHERE p.id=?1 AND p.state='candidate' AND m.is_deleted=0 AND m.sensitivity='api_allowed' AND m.version_event_id=p.source_memory_revision AND x.state='active' AND x.recall_allowed=1 AND x.occurred_at_ms<=?2 AND p.created_at_ms BETWEEN ?2-?3 AND ?2 AND (x.due_at_ms IS NULL OR x.due_at_ms BETWEEN ?2-?4 AND ?2) AND (d.source_message_id IS NULL OR EXISTS(SELECT 1 FROM messages src WHERE src.id=d.source_message_id AND src.is_deleted=0)))",params![id,now,MAX_PENDING_AGE,7*DAY_MS], |r|r.get(0))?)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    send: bool,
    #[serde(default)]
    intent: String,
    #[serde(default)]
    message: String,
}

pub(crate) fn read_draft(content: &str) -> Option<(String, String)> {
    let value = content.trim();
    let json = value
        .strip_prefix("```json")
        .and_then(|v| v.strip_suffix("```"))
        .unwrap_or(value);
    let draft: Draft = serde_json::from_str(json).ok()?;
    let message = draft.message.trim();
    if !draft.send
        || !matches!(
            draft.intent.as_str(),
            "recall" | "follow_up" | "continue_topic" | "reminder" | "share_interest"
        )
        || message.is_empty()
        || message.chars().count() > 180
        || message.contains("```")
        || message.split(super::intelligence::BUBBLE_SEPARATOR).count() > 3
    {
        return None;
    }
    Some((draft.intent, message.into()))
}
