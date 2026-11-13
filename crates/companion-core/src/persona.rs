//! Character ownership is independent of the paired local profile/device identity.
//! Legacy conversations remain immutable archives and are read through one main thread.
use crate::{
    Error, Result,
    chat::{ChatSettings, UsageSummary, now_ms},
    storage::{MemoryRecord, Store},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub main_conversation_id: String,
    pub is_builtin: bool,
    pub created_at_ms: i64,
    pub revision: i64,
}
#[derive(Debug, Serialize)]
pub struct PersonaSummary {
    pub memory_count: i64,
    pub activity_count: i64,
    pub interest_count: i64,
    pub relationship: Value,
    pub mood: Value,
    pub life: Value,
}
pub(crate) fn ensure_default(db: &Connection, profile: &str, device: &str) -> Result<()> {
    db.execute("INSERT OR IGNORE INTO personas(id,profile_id,name,main_conversation_id,is_builtin,created_at_ms) VALUES(?1,?1,'DeepSeek娘',?1,1,?2)",params![profile,now_ms()])?;
    db.execute("INSERT OR IGNORE INTO conversation_personas(conversation_id,persona_id,is_legacy) SELECT id,?1,id<>?1 FROM conversations WHERE profile_id=?1",[profile])?;
    db.execute(
        "INSERT OR IGNORE INTO persona_states(persona_id) VALUES(?1)",
        [profile],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO persona_selection(device_id,persona_id) VALUES(?1,?2)",
        params![device, profile],
    )?;
    Ok(())
}
pub(crate) fn owner_of_thread(
    db: &Connection,
    conversation: &str,
    profile: &str,
) -> Result<String> {
    db.query_row("SELECT p.id FROM conversation_personas c JOIN personas p ON p.id=c.persona_id WHERE c.conversation_id=?1 AND (p.profile_id=?2 OR p.profile_id IN (SELECT profile_id FROM lan_profiles))",params![conversation,profile],|r|r.get(0)).optional()?.ok_or(Error::Untrusted)
}
pub(crate) fn owner_exists(db: &Connection, persona: &str, profile: &str) -> Result<()> {
    Uuid::parse_str(persona).map_err(|_| Error::Invalid("persona UUID"))?;
    if !db.query_row(
        "SELECT EXISTS(SELECT 1 FROM personas WHERE id=?1 AND (profile_id=?2 OR profile_id IN (SELECT profile_id FROM lan_profiles)))",
        params![persona, profile],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Untrusted);
    }
    Ok(())
}
impl Store {
    pub fn personas(&self) -> Result<Vec<Persona>> {
        let mut s=self.conn.prepare("SELECT id,name,main_conversation_id,is_builtin,created_at_ms,revision FROM personas WHERE (profile_id=?1 OR profile_id IN (SELECT profile_id FROM lan_profiles)) ORDER BY is_builtin DESC,created_at_ms,id")?;
        Ok(s.query_map([&self.profile_id], |r| {
            Ok(Persona {
                id: r.get(0)?,
                name: r.get(1)?,
                main_conversation_id: r.get(2)?,
                is_builtin: r.get(3)?,
                created_at_ms: r.get(4)?,
                revision: r.get(5)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
    }
    pub fn permanent_persona(&self, thread: &str) -> Result<Persona> {
        self.personas()?
            .into_iter()
            .find(|p| p.main_conversation_id == thread)
            .ok_or(Error::Untrusted)
    }
    pub fn persona(&self, id: &str) -> Result<Persona> {
        self.personas()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or(Error::Untrusted)
    }
    pub fn active_persona_id(&self) -> Result<String> {
        Ok(self.conn.query_row("SELECT s.persona_id FROM persona_selection s JOIN personas p ON p.id=s.persona_id WHERE s.device_id=?1 AND (p.profile_id=?2 OR p.profile_id IN (SELECT profile_id FROM lan_profiles))",params![self.device_id,self.profile_id],|r|r.get(0))?)
    }
    pub fn select_persona(&mut self, id: &str) -> Result<()> {
        owner_exists(&self.conn, id, &self.profile_id)?;
        self.conn.execute("INSERT INTO persona_selection(device_id,persona_id) VALUES(?1,?2) ON CONFLICT(device_id) DO UPDATE SET persona_id=excluded.persona_id",params![self.device_id,id])?;
        Ok(())
    }
    pub fn create_persona(&mut self, name: &str, personality: &str) -> Result<Persona> {
        let settings = ChatSettings {
            character_name: name.trim().into(),
            personality: personality.trim().into(),
            ..ChatSettings::default()
        };
        settings.validate()?;
        if self.personas()?.len() >= 100 {
            return Err(Error::Invalid("persona limit"));
        }
        let id = Uuid::new_v4().to_string();
        let now = now_ms();
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO conversations(id,profile_id,title,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,?4)",params![id,self.profile_id,format!("{} · 永久聊天",settings.character_name),now])?;
        tx.execute("INSERT INTO personas(id,profile_id,name,main_conversation_id,created_at_ms) VALUES(?1,?2,?3,?1,?4)",params![id,self.profile_id,settings.character_name,now])?;
        tx.execute(
            "INSERT INTO conversation_personas(conversation_id,persona_id) VALUES(?1,?1)",
            [&id],
        )?;
        tx.execute(
            "INSERT INTO persona_chat_preferences(persona_id,settings_json) VALUES(?1,?2)",
            params![id, serde_json::to_string(&settings)?],
        )?;
        tx.execute("INSERT INTO persona_states(persona_id) VALUES(?1)", [&id])?;
        tx.commit()?;
        self.persona(&id)
    }
    pub fn persona_settings(&self, id: &str) -> Result<ChatSettings> {
        let persona = self.persona(id)?;
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT settings_json FROM persona_chat_preferences WHERE persona_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        let mut settings = raw
            .map(|s| serde_json::from_str::<ChatSettings>(&s))
            .transpose()?
            .unwrap_or_default();
        settings.character_name = persona.name;
        settings.validate()?;
        Ok(settings)
    }
    pub fn settings_for_conversation(&self, id: &str) -> Result<ChatSettings> {
        self.persona_settings(&owner_of_thread(&self.conn, id, &self.profile_id)?)
    }
    pub fn save_persona_settings(&mut self, id: &str, settings: &ChatSettings) -> Result<()> {
        owner_exists(&self.conn, id, &self.profile_id)?;
        settings.validate()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO persona_chat_preferences(persona_id,settings_json) VALUES(?1,?2) ON CONFLICT(persona_id) DO UPDATE SET settings_json=excluded.settings_json,revision=revision+1",params![id,serde_json::to_string(settings)?])?;
        tx.execute(
            "UPDATE personas SET name=?1,revision=revision+1 WHERE id=?2",
            params![settings.character_name, id],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// A main thread includes its immutable legacy archives; a legacy inspection
    /// request still reads only that original conversation for compatibility.
    pub(crate) fn thread_ids(&self, id: &str) -> Result<Vec<String>> {
        let main: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM personas WHERE main_conversation_id=?1 AND (profile_id=?2 OR profile_id IN (SELECT profile_id FROM lan_profiles))",
                params![id, self.profile_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(persona) = main {
            let mut s=self.conn.prepare("SELECT conversation_id FROM conversation_personas WHERE persona_id=?1 ORDER BY conversation_id")?;
            return Ok(s
                .query_map([persona], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?);
        }
        // Legacy API is kept for existing tests/import adapters, not exposed in UI.
        let owned: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE id=?1 AND (profile_id=?2 OR profile_id IN (SELECT profile_id FROM lan_profiles)))",
            params![id, self.profile_id],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(Error::Untrusted);
        }
        Ok(vec![id.into()])
    }
    pub fn usage_for_persona(&self, id: &str) -> Result<UsageSummary> {
        owner_exists(&self.conn, id, &self.profile_id)?;
        Ok(self.conn.query_row("SELECT count(*),coalesce(sum(usage_state='measured'),0),coalesce(sum(input_tokens),0),coalesce(sum(output_tokens),0) FROM api_usage_records WHERE (profile_id=?1 OR profile_id IN (SELECT profile_id FROM lan_profiles)) AND persona_id=?2 AND provider='deepseek'",params![self.profile_id,id],|r|Ok(UsageSummary{requests:r.get(0)?,measured_requests:r.get(1)?,prompt_tokens:r.get(2)?,completion_tokens:r.get(3)?}))?)
    }
    pub fn persona_summary(&self, id: &str) -> Result<PersonaSummary> {
        owner_exists(&self.conn, id, &self.profile_id)?;
        let (r, m, l): (String, String, String) = self.conn.query_row(
            "SELECT relationship_json,mood_json,life_json FROM persona_states WHERE persona_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let count = |table: &str| -> Result<i64> {
            Ok(self.conn.query_row(
                &format!("SELECT count(*) FROM {table} WHERE (profile_id=?1 OR profile_id IN (SELECT profile_id FROM lan_profiles)) AND persona_id=?2"),
                params![self.profile_id, id],
                |r| r.get(0),
            )?)
        };
        Ok(PersonaSummary {
            memory_count: self.conn.query_row("SELECT count(*) FROM memories WHERE (profile_id=?1 OR profile_id IN (SELECT profile_id FROM lan_profiles)) AND persona_id=?2 AND is_deleted=0",params![self.profile_id,id],|r|r.get(0))?,
            activity_count: count("daily_activities")?,
            interest_count: count("character_interests")?,
            relationship: serde_json::from_str(&r)?,
            mood: serde_json::from_str(&m)?,
            life: serde_json::from_str(&l)?,
        })
    }
    pub fn memories_for_persona(&self, id: &str) -> Result<Vec<MemoryRecord>> {
        owner_exists(&self.conn, id, &self.profile_id)?;
        let mut s=self.conn.prepare("SELECT id,text,version_lamport FROM memories WHERE (profile_id=?1 OR profile_id IN (SELECT profile_id FROM lan_profiles)) AND persona_id=?2 AND is_deleted=0 ORDER BY version_lamport DESC,id LIMIT 100")?;
        Ok(s.query_map(params![self.profile_id, id], |r| {
            Ok(MemoryRecord {
                id: r.get(0)?,
                text: r.get(1)?,
                version_lamport: r.get(2)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
    }
    pub fn put_persona_memory(&mut self, persona: &str, id: &str, text: &str) -> Result<()> {
        owner_exists(&self.conn, persona, &self.profile_id)?;
        self.put(
            crate::event::Entity::Memory,
            id,
            json!({"text":text,"persona_id":persona}),
        )?;
        Ok(())
    }
    pub fn delete_persona_memory(&mut self, persona: &str, id: &str) -> Result<()> {
        owner_exists(&self.conn, persona, &self.profile_id)?;
        let owner: Option<String> = self
            .conn
            .query_row(
                "SELECT persona_id FROM memories WHERE id=?1 AND (profile_id=?2 OR profile_id IN (SELECT profile_id FROM lan_profiles))",
                params![id, self.profile_id],
                |r| r.get(0),
            )
            .optional()?;
        if owner.as_deref() != Some(persona) {
            return Err(Error::Untrusted);
        }
        self.delete_unscoped(crate::event::Entity::Memory, id)?;
        Ok(())
    }
}
