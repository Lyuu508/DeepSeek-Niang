use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MAX_BATCH: usize = 256;
pub const MAX_PAYLOAD: usize = 64 * 1024;
pub const MAX_CLOCK: i64 = (1i64 << 53) - 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Entity {
    Message,
    Memory,
}
impl Entity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Memory => "memory",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Put,
    Delete,
}

/// Stage-0 lab envelope. SHA256 is integrity only, NOT authentication.
/// Production import must be behind paired, pinned mTLS plus origin signatures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub schema_version: u32,
    pub profile_id: String,
    pub event_id: String,
    pub origin_device_id: String,
    pub origin_seq: i64,
    pub lamport: i64,
    pub entity: Entity,
    pub entity_id: String,
    pub operation: Operation,
    pub payload: Value,
    pub checksum: String,
}
impl Event {
    pub(crate) fn seal(mut self) -> Result<Self> {
        self.checksum = self.digest()?;
        self.validate()?;
        Ok(self)
    }
    fn digest(&self) -> Result<String> {
        // Fixed tuple and serde_json's default sorted object keys are canonical v1.
        let bytes = serde_json::to_vec(&(
            self.schema_version,
            &self.profile_id,
            &self.event_id,
            &self.origin_device_id,
            self.origin_seq,
            self.lamport,
            self.entity,
            &self.entity_id,
            self.operation,
            &self.payload,
        ))?;
        Ok(sha256_hex(&bytes))
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(Error::Schema);
        }
        for id in [
            &self.event_id,
            &self.entity_id,
            &self.profile_id,
            &self.origin_device_id,
        ] {
            if Uuid::parse_str(id).is_err() {
                return Err(Error::Invalid("UUID required"));
            }
        }
        if self.origin_seq <= 0
            || self.lamport <= 0
            || self.origin_seq > MAX_CLOCK
            || self.lamport > MAX_CLOCK
        {
            return Err(Error::Invalid("logical clock out of range"));
        }
        if serde_json::to_vec(&self.payload)?.len() > MAX_PAYLOAD {
            return Err(Error::Invalid("payload too large"));
        }
        if self.checksum != self.digest()? {
            return Err(Error::Invalid("checksum mismatch"));
        }
        if self.operation == Operation::Delete {
            if self.payload != Value::Null {
                return Err(Error::Invalid("delete payload must be null"));
            }
        } else {
            if self.entity == Entity::Memory
                && (self
                    .payload
                    .get("importance_milli")
                    .is_some_and(|v| v.as_i64().is_none_or(|n| !(0..=1000).contains(&n)))
                    || self.payload.get("sensitivity").is_some_and(|v| {
                        !matches!(v.as_str(), Some("private" | "api_allowed" | "blocked"))
                    }))
            {
                return Err(Error::Invalid("memory metadata"));
            }
            if let Some(owner) = self.payload.get("persona_id")
                && owner.as_str().is_none_or(|id| Uuid::parse_str(id).is_err())
            {
                return Err(Error::Invalid("persona UUID"));
            }
            if self.entity == Entity::Message {
                for key in [
                    "conversation_id",
                    "reply_to_id",
                    "generation_id",
                    "request_id",
                ] {
                    if let Some(value) = self.payload.get(key)
                        && value.as_str().is_none_or(|s| Uuid::parse_str(s).is_err())
                    {
                        return Err(Error::Invalid("message reference UUID"));
                    }
                }
                if self
                    .payload
                    .get("created_at_ms")
                    .is_some_and(|v| v.as_i64().is_none_or(|n| !(0..=MAX_CLOCK).contains(&n)))
                {
                    return Err(Error::Invalid("message timestamp"));
                }
                if self.payload.get("status").is_some_and(|v| {
                    !matches!(v.as_str(), Some("complete" | "interrupted" | "failed"))
                }) {
                    return Err(Error::Invalid("immutable message status"));
                }
                if self
                    .payload
                    .get("conversation_title")
                    .is_some_and(|v| v.as_str().is_none_or(|s| s.is_empty() || s.len() > 200))
                {
                    return Err(Error::Invalid("conversation title"));
                }
            }
            let key = if self.entity == Entity::Message {
                "content"
            } else {
                "text"
            };
            let text = self
                .payload
                .get(key)
                .and_then(Value::as_str)
                .ok_or(Error::Invalid("text required"))?;
            if text.trim().is_empty() || text.len() > 32_768 {
                return Err(Error::Invalid("text length"));
            }
            if self.entity == Entity::Message
                && !matches!(
                    self.payload.get("role").and_then(Value::as_str),
                    Some("user" | "assistant" | "tool")
                )
            {
                return Err(Error::Invalid("message role"));
            }
        }
        Ok(())
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
