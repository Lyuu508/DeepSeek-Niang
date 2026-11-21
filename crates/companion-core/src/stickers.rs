//! Local persona sticker catalog. No network, provider or plugin execution.
use crate::{
    Error, Result,
    chat_images::{self, ChatImage},
    storage::Store,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const CATALOG_SIZE: usize = 24;
const SHARE_INTERVAL_MS: i64 = 10 * 60 * 1000;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sticker {
    pub id: String,
    pub tag: String,
    pub image: ChatImage,
}

#[derive(Serialize)]
pub struct StickerCatalog {
    pub entries: Vec<Sticker>,
    pub automatic: bool,
}

fn get<T: serde::de::DeserializeOwned>(
    db: &Connection,
    profile: &str,
    device: &str,
    key: &str,
) -> Result<Option<T>> {
    let data: Option<String> = db.query_row(
        "SELECT value_json FROM application_settings WHERE profile_id=?1 AND device_id=?2 AND key=?3 AND scope='device'",
        params![profile, device, key], |row| row.get(0),
    ).optional()?;
    data.map(|value| serde_json::from_str(&value).map_err(Error::from))
        .transpose()
}

fn write<T: Serialize>(
    db: &Connection,
    profile: &str,
    device: &str,
    key: &str,
    value: &T,
) -> Result<()> {
    db.execute(
        "INSERT INTO application_settings(profile_id,device_id,key,scope,value_json) VALUES(?1,?2,?3,'device',?4)
         ON CONFLICT(profile_id,key,device_id) DO UPDATE SET value_json=excluded.value_json,revision=application_settings.revision+1
         WHERE application_settings.scope='device'",
        params![profile,device,key,serde_json::to_string(value)?],
    )?;
    Ok(())
}

impl Store {
    pub fn stickers(&self, persona: &str) -> Result<StickerCatalog> {
        self.persona(persona)?;
        let key = format!("sticker:{persona}:*");
        let mut select = self.conn.prepare(
            "SELECT value_json FROM application_settings WHERE profile_id=?1 AND device_id=?2 AND scope='device'
             AND key GLOB ?3 ORDER BY key LIMIT ?4",
        )?;
        let rows = select.query_map(
            params![self.profile_id, self.device_id, key, CATALOG_SIZE as i64],
            |row| row.get::<_, String>(0),
        )?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(serde_json::from_str(&row?)?);
        }
        let automatic = get(
            &self.conn,
            &self.profile_id,
            &self.device_id,
            &format!("sticker-auto:{persona}"),
        )?
        .unwrap_or(false);
        Ok(StickerCatalog { entries, automatic })
    }

    pub fn add_sticker(&self, persona: &str, tag: String, image: ChatImage) -> Result<()> {
        let tag = tag.trim();
        if tag.is_empty() || tag.chars().count() > 20 || tag.chars().any(char::is_control) {
            return Err(Error::Invalid("sticker tag"));
        }
        image.validate()?;
        if self.stickers(persona)?.entries.len() >= CATALOG_SIZE {
            return Err(Error::Invalid("sticker catalog limit"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        write(
            &self.conn,
            &self.profile_id,
            &self.device_id,
            &format!("sticker:{persona}:{id}"),
            &Sticker {
                id,
                tag: tag.into(),
                image,
            },
        )
    }

    pub fn remove_sticker(&self, persona: &str, id: &str) -> Result<()> {
        self.persona(persona)?;
        uuid::Uuid::parse_str(id).map_err(|_| Error::Invalid("sticker id"))?;
        self.conn.execute(
            "DELETE FROM application_settings WHERE profile_id=?1 AND device_id=?2 AND scope='device' AND key=?3",
            params![self.profile_id,self.device_id,format!("sticker:{persona}:{id}")],
        )?;
        Ok(())
    }

    pub fn set_automatic_stickers(&self, persona: &str, enabled: bool) -> Result<()> {
        self.persona(persona)?;
        write(
            &self.conn,
            &self.profile_id,
            &self.device_id,
            &format!("sticker-auto:{persona}"),
            &enabled,
        )
    }

    pub fn maybe_attach_sticker(&mut self, conversation: &str, message: &str) -> Result<bool> {
        let persona = self.permanent_persona(conversation)?.id;
        let auto_key = format!("sticker-auto:{persona}");
        let enabled: bool =
            get(&self.conn, &self.profile_id, &self.device_id, &auto_key)?.unwrap_or(false);
        if !enabled {
            return Ok(false);
        }
        let now = crate::chat::now_ms();
        let last_key = format!("sticker-last:{persona}");
        let last: i64 = get(&self.conn, &self.profile_id, &self.device_id, &last_key)?.unwrap_or(0);
        if now.saturating_sub(last) < SHARE_INTERVAL_MS {
            return Ok(false);
        }
        let mood = self.current_mood(&persona, now)?.label;
        let matches: Vec<_> = self
            .stickers(&persona)?
            .entries
            .into_iter()
            .filter(|sticker| sticker.tag == mood)
            .collect();
        if matches.is_empty() {
            return Ok(false);
        }
        let available = uuid::Uuid::new_v4().as_u128() as usize % matches.len();
        let tx = self.conn.transaction()?;
        chat_images::attach(
            &tx,
            &self.profile_id,
            &self.device_id,
            message,
            std::slice::from_ref(&matches[available].image),
        )?;
        write(&tx, &self.profile_id, &self.device_id, &last_key, &now)?;
        tx.commit()?;
        Ok(true)
    }
}
