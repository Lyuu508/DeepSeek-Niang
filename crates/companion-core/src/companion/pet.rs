//! A narrow snapshot for the desktop window and local window preferences.
//! The desktop window does not need access to the full companion snapshot or chat history.
use super::{MEMBERS, bond::mood_label, life};
use crate::{Error, Result, storage::Store};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PetPlacement {
    pub monitor: Option<String>,
    /// Position within the available travel of the window, from 0 to 10,000.
    pub horizontal: u16,
    pub vertical: u16,
}

impl Default for PetPlacement {
    fn default() -> Self {
        Self {
            monitor: None,
            horizontal: 10_000,
            vertical: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PetPreferences {
    pub visible: bool,
    pub always_on_top: bool,
    pub scale_percent: u16,
    pub roam: bool,
    pub reduce_motion: bool,
    pub placement: PetPlacement,
}

impl Default for PetPreferences {
    fn default() -> Self {
        Self {
            visible: false,
            always_on_top: true,
            scale_percent: 100,
            roam: true,
            reduce_motion: false,
            placement: PetPlacement::default(),
        }
    }
}

impl PetPreferences {
    pub fn validate(&self) -> Result<()> {
        if !(75..=150).contains(&self.scale_percent)
            || self.placement.horizontal > 10_000
            || self.placement.vertical > 10_000
            || self
                .placement
                .monitor
                .as_ref()
                .is_some_and(|name| name.len() > 256)
        {
            return Err(Error::Invalid("desktop pet preferences"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PetChatState {
    Idle,
    Busy,
    Thinking,
    Replying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PetPose {
    Idle,
    Happy,
    Shy,
    Thinking,
    Annoyed,
    Replying,
    Resting,
    Reading,
}

#[derive(Debug, Serialize)]
pub struct PetActivity {
    pub label: String,
    pub state: String,
    pub progress_milli: i64,
    pub updated_at_ms: i64,
    pub stale: bool,
    pub is_virtual: bool,
}

#[derive(Debug, Serialize)]
pub struct PetSnapshot {
    pub persona_id: String,
    pub name: String,
    pub builtin_avatar: bool,
    pub mood_label: &'static str,
    pub pose: PetPose,
    pub chat: PetChatState,
    pub activity: Option<PetActivity>,
    pub unread_share: bool,
}

impl Store {
    pub fn pet_preferences(&self) -> Result<PetPreferences> {
        let saved: Option<String> = self
            .conn
            .query_row(
                "SELECT value_json FROM application_settings
             WHERE profile_id=?1 AND device_id=?2 AND key='desktop_pet' AND scope='device'",
                params![self.profile_id, self.device_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(saved
            .and_then(|json| serde_json::from_str::<PetPreferences>(&json).ok())
            .filter(|prefs| prefs.validate().is_ok())
            .unwrap_or_default())
    }

    pub fn save_pet_preferences(&self, preferences: &PetPreferences) -> Result<()> {
        preferences.validate()?;
        self.conn.execute(
            "INSERT INTO application_settings (profile_id,key,value_json,scope,device_id)
             VALUES (?1,'desktop_pet',?2,'device',?3)
             ON CONFLICT(profile_id,key,device_id) DO UPDATE SET
                 value_json=excluded.value_json, revision=application_settings.revision+1
             WHERE application_settings.scope='device' AND application_settings.value_json<>excluded.value_json",
            params![self.profile_id, serde_json::to_string(preferences)?, self.device_id],
        )?;
        Ok(())
    }

    pub fn pet_snapshot(&self, now: i64, generation: Option<(&str, &str)>) -> Result<PetSnapshot> {
        let selected = self.active_persona_id()?;
        let persona = self
            .personas()?
            .into_iter()
            .find(|p| p.id == selected)
            .ok_or(Error::Untrusted)?;
        let heartbeat: Option<i64> = self
            .conn
            .query_row(
                "SELECT heartbeat_at_ms FROM companion_hosts WHERE persona_id=?1",
                [&persona.id],
                |row| row.get(0),
            )
            .optional()?;
        let fresh = heartbeat.is_some_and(|at| life::heartbeat_is_fresh(now, at));
        let recent: Option<(String, String, i64, i64)> = self
            .conn
            .query_row(
                "SELECT kind,state,progress_milli,updated_at_ms FROM companion_activities
             WHERE persona_id=?1 ORDER BY started_at_ms DESC,id DESC LIMIT 1",
                [&persona.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let life_enabled = self.companion_preferences(&persona.id)?.life_enabled;
        let mut activity = recent
            .as_ref()
            .map(|(kind, state, progress, updated)| PetActivity {
                label: format!("虚拟{}", life::activity_label(kind)),
                state: if !life_enabled && matches!(state.as_str(), "running" | "queued") {
                    "paused".into()
                } else {
                    state.clone()
                },
                progress_milli: (*progress).clamp(0, 1000),
                updated_at_ms: *updated,
                stale: !fresh,
                is_virtual: true,
            });
        if let Some(video) = self.current_video_activity(&persona.id)? {
            activity = Some(PetActivity {
                label: video.label.into(),
                state: "running".into(),
                progress_milli: video.progress_milli,
                updated_at_ms: video.updated_at_ms,
                stale: false,
                is_virtual: false,
            });
        }
        let mood: Option<(String, i64, i64, bool)> = self
            .conn
            .query_row(
                &format!(
                    "SELECT mood,started_at_ms,expires_at_ms,source_message_id IS NOT NULL
                FROM companion_mood_events WHERE persona_id IN ({MEMBERS})
                ORDER BY lamport DESC,origin_device_id DESC,id DESC LIMIT 1"
                ),
                [&persona.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let mood = mood
            .as_ref()
            .filter(|(_, since, until, from_chat)| {
                now >= *since && now < *until && (*from_chat || fresh && life_enabled)
            })
            .map(|(kind, _, _, _)| kind.as_str())
            .unwrap_or("calm");
        let chat = match generation {
            Some((request, conversation)) if conversation == persona.main_conversation_id => {
                let status: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT state FROM chat_requests WHERE id=?1 AND conversation_id=?2",
                        params![request, conversation],
                        |row| row.get(0),
                    )
                    .optional()?;
                match status.as_deref() {
                    Some("sending") => PetChatState::Thinking,
                    Some("generating") => PetChatState::Replying,
                    _ => PetChatState::Idle,
                }
            }
            Some(_) => PetChatState::Busy,
            None => PetChatState::Idle,
        };
        let running = activity
            .as_ref()
            .is_some_and(|value| !value.stale && value.state == "running");
        let kind = recent
            .as_ref()
            .map(|row| row.0.as_str())
            .filter(|_| running && activity.as_ref().is_some_and(|a| a.is_virtual));
        let pose = match (chat, mood, kind) {
            (PetChatState::Thinking, _, _) => PetPose::Thinking,
            (PetChatState::Replying, _, _) => PetPose::Replying,
            (_, "happy" | "excited", _) => PetPose::Happy,
            (_, "shy", _) => PetPose::Shy,
            (_, "angry" | "hurt", _) => PetPose::Annoyed,
            (_, _, Some("rest")) => PetPose::Resting,
            (_, _, Some("read" | "study" | "organize" | "recall" | "journal")) => PetPose::Reading,
            _ => PetPose::Idle,
        };
        let unread_share = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM companion_proactive s
             JOIN messages m ON m.id=s.message_id AND m.is_deleted=0
             LEFT JOIN companion_receipts r ON r.proactive_id=s.id AND r.device_id=?2
             WHERE s.persona_id=?1 AND s.state='delivered' AND r.read_at_ms IS NULL)",
            params![persona.id, self.device_id],
            |row| row.get(0),
        )?;
        Ok(PetSnapshot {
            persona_id: persona.id,
            name: persona.name,
            builtin_avatar: persona.avatar_resource == "deepseek-niang" && persona.is_builtin,
            mood_label: mood_label(mood),
            pose,
            chat,
            activity,
            unread_share,
        })
    }
}

#[cfg(test)]
mod tests;
