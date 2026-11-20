use crate::{Error, Result, event::sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_MANIFEST_BYTES: usize = 16 * 1024;
pub type Config = BTreeMap<String, Value>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ChatStyle,
    LocalClock,
    LocalReminder,
    MemoryWrite,
    DisplayCard,
    MediaNote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Warm,
    Brief,
    Playful,
    Story,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entry {
    ChatStyle { preset: Preset },
    TimeReminder,
    MemoryNote,
    InfoCard { body: String },
    MediaNote,
}

impl Entry {
    pub fn permissions(&self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Self::ChatStyle { .. } => &[ChatStyle],
            Self::TimeReminder => &[LocalClock, LocalReminder],
            Self::MemoryNote => &[MemoryWrite],
            Self::InfoCard { .. } => &[DisplayCard],
            Self::MediaNote => &[MediaNote, MemoryWrite],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Number,
    Toggle,
    Choice,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    pub default: Value,
    pub choices: Vec<String>,
}

impl Field {
    fn accepts(&self, value: &Value) -> bool {
        match self.kind {
            FieldKind::Text => value
                .as_str()
                .is_some_and(|s| s.len() <= 600 && !s.chars().any(char::is_control)),
            FieldKind::Number => value.as_i64().is_some_and(|n| (0..=1440).contains(&n)),
            FieldKind::Toggle => value.is_boolean(),
            FieldKind::Choice => value
                .as_str()
                .is_some_and(|s| self.choices.iter().any(|c| c == s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub entry: Entry,
    pub supported_platforms: Vec<String>,
    pub permissions: Vec<Permission>,
    pub config_schema: Vec<Field>,
    pub required_api_version: u32,
    pub checksum: String,
}

fn identifier(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'-' | b'_' | b'.')
        })
        && !value.contains("..")
        && !value.ends_with('.')
}

fn text(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

pub(super) fn version(value: &str) -> Result<[u16; 3]> {
    let parts = value
        .split('.')
        .map(|p| {
            if p.is_empty()
                || !p.bytes().all(|c| c.is_ascii_digit())
                || (p.starts_with('0') && p.len() > 1)
            {
                return Err(Error::Invalid("PLUGIN_INVALID"));
            }
            p.parse::<u16>()
                .map_err(|_| Error::Invalid("PLUGIN_INVALID"))
        })
        .collect::<Result<Vec<_>>>()?;
    parts
        .try_into()
        .map_err(|_| Error::Invalid("PLUGIN_INVALID"))
}

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self> {
        if raw.len() > MAX_MANIFEST_BYTES {
            return Err(Error::Invalid("PLUGIN_INVALID"));
        }
        let manifest: Manifest =
            serde_json::from_str(raw).map_err(|_| Error::Invalid("PLUGIN_INVALID"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn calculated_checksum(&self) -> Result<String> {
        let mut payload = serde_json::to_value(self)?;
        payload
            .as_object_mut()
            .expect("a manifest serializes as an object")
            .remove("checksum");
        Ok(sha256_hex(&serde_jcs::to_vec(&payload)?))
    }

    fn validate(&self) -> Result<()> {
        version(&self.version)?;
        if !identifier(&self.id)
            || !text(&self.name, 144)
            || !text(&self.author, 144)
            || !text(&self.description, 1200)
            || !text(&self.license, 144)
            || self.required_api_version != 1
            || self.supported_platforms.is_empty()
            || self.supported_platforms.len() > 2
            || self
                .supported_platforms
                .iter()
                .any(|p| !matches!(p.as_str(), "windows" | "android"))
            || self
                .supported_platforms
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.supported_platforms.len()
            || self.permissions.iter().copied().collect::<BTreeSet<_>>()
                != self.entry.permissions().iter().copied().collect()
            || self.permissions.len() != self.entry.permissions().len()
            || self.config_schema.len() > 12
            || matches!(&self.entry,Entry::InfoCard { body } if !text(body,1800))
        {
            return Err(Error::Invalid("PLUGIN_INVALID"));
        }
        let mut keys = BTreeSet::new();
        for field in &self.config_schema {
            if !identifier(&field.key)
                || !text(&field.label, 144)
                || !keys.insert(&field.key)
                || !field.accepts(&field.default)
                || field.choices.len() > 12
                || field.choices.iter().any(|c| !text(c, 100))
                || field.choices.iter().collect::<BTreeSet<_>>().len() != field.choices.len()
                || (field.kind == FieldKind::Choice) == field.choices.is_empty()
            {
                return Err(Error::Invalid("PLUGIN_INVALID"));
            }
        }
        if self.checksum != self.calculated_checksum()? {
            return Err(Error::Invalid("PLUGIN_CHECKSUM"));
        }
        Ok(())
    }

    pub fn configuration(&self, provided: &Config) -> Result<Config> {
        if provided.len() > self.config_schema.len()
            || provided.iter().any(|(key, value)| {
                !self
                    .config_schema
                    .iter()
                    .any(|f| &f.key == key && f.accepts(value))
            })
        {
            return Err(Error::Invalid("PLUGIN_CONFIG"));
        }
        Ok(self
            .config_schema
            .iter()
            .map(|field| {
                (
                    field.key.clone(),
                    provided.get(&field.key).unwrap_or(&field.default).clone(),
                )
            })
            .collect())
    }

    pub fn supports(&self, platform: &str) -> bool {
        self.supported_platforms.iter().any(|p| p == platform)
    }
}
