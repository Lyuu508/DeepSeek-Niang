use crate::{Error, Result, storage::Store};
use rusqlite::{OptionalExtension, params};

impl Store {
    pub fn font_scale(&self) -> Result<u8> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value_json FROM application_settings
             WHERE profile_id=?1 AND device_id=?2 AND scope='device' AND key='font_scale'",
                params![self.profile_id, self.device_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value
            .and_then(|v| v.parse().ok())
            .filter(|v| (85..=150).contains(v))
            .unwrap_or(100))
    }

    pub fn set_font_scale(&self, percent: u8) -> Result<()> {
        if !(85..=150).contains(&percent) {
            return Err(Error::Invalid("font scale"));
        }
        self.conn.execute(
            "INSERT INTO application_settings(profile_id,key,value_json,scope,device_id)
             VALUES(?1,'font_scale',?2,'device',?3)
             ON CONFLICT(profile_id,key,device_id) DO UPDATE SET
               scope='device', value_json=excluded.value_json, revision=application_settings.revision+1",
            params![self.profile_id, percent.to_string(), self.device_id],
        )?;
        Ok(())
    }
}
