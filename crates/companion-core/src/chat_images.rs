use crate::{Error, Result, storage::Store};
use base64::{Engine, engine::general_purpose::STANDARD};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const MAX_IMAGE: usize = 240 * 1024;
pub const MAX_IMAGES: usize = 4;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatImage {
    pub name: String,
    pub jpeg: String,
}

impl ChatImage {
    pub(crate) fn validate(&self) -> Result<()> {
        self.jpeg_bytes().map(|_| ())
    }

    pub fn jpeg_bytes(&self) -> Result<Vec<u8>> {
        if self.name.trim().is_empty()
            || self.name.len() > 384
            || self.name.chars().any(char::is_control)
            || self.jpeg.len() > MAX_IMAGE.div_ceil(3) * 4
        {
            return Err(Error::Invalid("CHAT_IMAGE_INVALID"));
        }
        let data = STANDARD
            .decode(&self.jpeg)
            .map_err(|_| Error::Invalid("CHAT_IMAGE_INVALID"))?;
        if data.len() > MAX_IMAGE
            || !data.starts_with(&[255, 216, 255])
            || !data.ends_with(&[255, 217])
        {
            return Err(Error::Invalid("CHAT_IMAGE_INVALID"));
        }
        Ok(data)
    }
}

pub(crate) fn attach(
    db: &Connection,
    profile: &str,
    device: &str,
    message: &str,
    images: &[ChatImage],
) -> Result<()> {
    let Some((first, extras)) = images.split_first() else {
        return Ok(());
    };
    let count: i64 = db.query_row(
        "SELECT coalesce(sum(CASE WHEN key GLOB 'chat_image:*' THEN 1 ELSE json_array_length(value_json) END),0)
         FROM application_settings WHERE profile_id=?1 AND device_id=?2 AND scope='device'
         AND (key GLOB 'chat_image:*' OR key GLOB 'chat_gallery:*')",
        params![profile,device], |row|row.get(0),
    )?;
    if count + images.len() as i64 > 1500 {
        return Err(Error::Invalid("CHAT_IMAGE_LIMIT"));
    }
    // Preserve the older single-image record so a rolled-back client can still read it.
    let insert = |key: String, data: String| -> Result<()> {
        db.execute(
            "INSERT INTO application_settings(profile_id,key,device_id,scope,value_json) VALUES(?1,?2,?3,'device',?4)",
            params![profile,key,device,data],
        )?;
        Ok(())
    };
    insert(
        format!("chat_image:{message}"),
        serde_json::to_string(first)?,
    )?;
    if !extras.is_empty() {
        insert(
            format!("chat_gallery:{message}"),
            serde_json::to_string(extras)?,
        )?;
    }
    Ok(())
}

impl Store {
    pub fn local_chat_images(&self, conversation: &str) -> Result<Vec<String>> {
        let persona = self.permanent_persona(conversation)?;
        let mut statement = self.conn.prepare(
            "SELECT m.id FROM application_settings a JOIN messages m ON a.key='chat_image:'||m.id
             JOIN conversation_personas c ON c.conversation_id=m.conversation_id
             WHERE a.profile_id=?1 AND a.device_id=?2 AND a.scope='device' AND c.persona_id IN
             (SELECT ?3 UNION SELECT alias_id FROM persona_links WHERE canonical_id=?3)
             AND m.role IN ('user','assistant') AND m.is_deleted=0",
        )?;
        Ok(statement
            .query_map(
                params![self.profile_id, self.device_id, persona.id],
                |row| row.get(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn local_chat_image(&self, conversation: &str, id: &str) -> Result<Option<ChatImage>> {
        let persona = self.permanent_persona(conversation)?;
        let data: Option<String> = self.conn.query_row(
            "SELECT a.value_json FROM application_settings a JOIN messages m ON a.key='chat_image:'||m.id
             JOIN conversation_personas c ON c.conversation_id=m.conversation_id
             WHERE a.profile_id=?1 AND a.device_id=?2 AND a.scope='device' AND c.persona_id IN
             (SELECT ?3 UNION SELECT alias_id FROM persona_links WHERE canonical_id=?3)
             AND m.id=?4 AND m.role IN ('user','assistant') AND m.is_deleted=0",
            params![self.profile_id,self.device_id,persona.id,id], |row|row.get(0),
        ).optional()?;
        data.map(|value| serde_json::from_str(&value).map_err(Error::from))
            .transpose()
    }

    pub fn local_chat_gallery(&self, conversation: &str, id: &str) -> Result<Vec<ChatImage>> {
        let Some(first) = self.local_chat_image(conversation, id)? else {
            return Ok(Vec::new());
        };
        let mut images = vec![first];
        let data: Option<String> = self
            .conn
            .query_row(
                "SELECT value_json FROM application_settings
             WHERE profile_id=?1 AND device_id=?2 AND scope='device' AND key='chat_gallery:'||?3",
                params![self.profile_id, self.device_id, id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(data) = data {
            let extras: Vec<ChatImage> = serde_json::from_str(&data)?;
            if extras.len() >= MAX_IMAGES {
                return Err(Error::Invalid("CHAT_IMAGE_LIMIT"));
            }
            images.extend(extras);
        }
        Ok(images)
    }
}
