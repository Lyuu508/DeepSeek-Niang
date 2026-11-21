use crate::{Error, Result, storage::Store};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

/// Nonsecret video settings and the latest analysis per persona. Device rows
/// do not enter LAN replication. The existing schema and tables are reused.
pub enum VideoDeviceSlot<'a> {
    Provider,
    Analysis(&'a str),
}

fn key(store: &Store, slot: VideoDeviceSlot<'_>) -> Result<String> {
    match slot {
        VideoDeviceSlot::Provider => Ok("qwen_vision".to_owned()),
        VideoDeviceSlot::Analysis(persona) => {
            store.persona(persona)?;
            Ok(format!("cloud_video:{persona}"))
        }
    }
}

impl Store {
    pub fn read_video_device(&self, slot: VideoDeviceSlot<'_>) -> Result<Option<Value>> {
        let key = key(self, slot)?;
        let raw: Option<String> = self.conn.query_row(
            "SELECT value_json FROM application_settings WHERE profile_id=?1 AND device_id=?2 AND key=?3 AND scope='device'",
            params![self.profile_id, self.device_id, key], |row|row.get(0),
        ).optional()?;
        raw.map(|value| serde_json::from_str(&value).map_err(Error::from))
            .transpose()
    }

    pub fn write_video_device(
        &self,
        slot: VideoDeviceSlot<'_>,
        value: Option<Value>,
    ) -> Result<()> {
        let key = key(self, slot)?;
        if let Some(value) = value {
            let raw = serde_json::to_string(&value)?;
            if raw.len() > 100_000 {
                return Err(Error::Invalid("video analysis size"));
            }
            self.conn.execute(
                "INSERT INTO application_settings(profile_id,key,device_id,scope,value_json) VALUES(?1,?2,?3,'device',?4)
                 ON CONFLICT(profile_id,key,device_id) DO UPDATE SET value_json=excluded.value_json, revision=application_settings.revision+1
                 WHERE application_settings.scope='device'",
                params![self.profile_id,key,self.device_id,raw],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM application_settings WHERE profile_id=?1 AND device_id=?2 AND key=?3 AND scope='device'",
                params![self.profile_id,self.device_id,key],
            )?;
        }
        Ok(())
    }
}
