//! Only built-in interpreters run. A manifest supplies bounded data and a capability set.
mod manifest;
use crate::{
    Error, Result,
    chat::now_ms,
    event::{Entity, Operation},
    storage::Store,
};
pub use manifest::{Config, Entry, Manifest, Permission, Preset};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Serialize)]
pub struct Preview {
    manifest: Manifest,
    compatible: bool,
    source: String,
    operation: &'static str,
}

#[derive(Serialize)]
pub struct Plugin {
    manifest: Manifest,
    source: String,
    global_enabled: bool,
    persona_enabled: Option<bool>,
    granted: Vec<Permission>,
    global_config: Config,
    persona_config: Option<Config>,
    effective_config: Config,
    status: &'static str,
    last_action: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Device,
    Persona,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Install {
        raw: String,
        source: String,
        checksum: String,
    },
    Grants {
        id: String,
        checksum: String,
        permissions: Vec<Permission>,
    },
    Settings {
        id: String,
        checksum: String,
        scope: Scope,
        enabled: Option<bool>,
        config: Option<Config>,
    },
    Uninstall {
        id: String,
        checksum: String,
    },
    CreateReminder {
        id: String,
        checksum: String,
        minutes: u16,
        label: String,
    },
    WriteNote {
        id: String,
        checksum: String,
        text: String,
    },
    DismissReminder {
        reminder_id: String,
    },
}

#[derive(Serialize)]
pub struct DueReminder {
    id: String,
    label: String,
    due_at_ms: i64,
}

fn source_label(source: &str) -> Result<String> {
    let name = source.trim();
    if name.is_empty()
        || name.len() > 240
        || name.contains(['/', '\\', ':'])
        || name.contains("..")
        || name.chars().any(char::is_control)
    {
        return Err(Error::Invalid("PLUGIN_INVALID"));
    }
    Ok(name.into())
}

fn scope(store: &Store, persona: &str) -> Result<()> {
    if store.active_persona_id()? != persona {
        return Err(Error::Invalid("PLUGIN_PERSONA_CHANGED"));
    }
    crate::persona::owner_exists(&store.conn, persona, &store.profile_id)
}

fn audit(db: &Connection, manifest: &Manifest, persona: Option<&str>, action: &str) -> Result<()> {
    db.execute("INSERT INTO app_plugin_audit(plugin_id,persona_id,checksum,action,at_ms) VALUES(?1,?2,?3,?4,?5)",
        params![manifest.id,persona,manifest.checksum,action,now_ms()])?;
    db.execute(
        "DELETE FROM app_plugin_audit WHERE seq <= (SELECT max(seq)-512 FROM app_plugin_audit)",
        [],
    )?;
    Ok(())
}

impl Store {
    fn device_platform(&self) -> Result<String> {
        Ok(self.conn.query_row(
            "SELECT platform FROM devices WHERE id=?1",
            [&self.device_id],
            |r| r.get(0),
        )?)
    }

    pub fn inspect_plugin(&self, raw: &str, source: &str) -> Result<Preview> {
        let manifest = Manifest::parse(raw)?;
        let source = source_label(source)?;
        let old: Option<String> = self
            .conn
            .query_row(
                "SELECT manifest_json FROM app_plugins WHERE id=?1",
                [&manifest.id],
                |r| r.get(0),
            )
            .optional()?;
        let operation = if let Some(previous) = old {
            let previous = Manifest::parse(&previous)?;
            if manifest::version(&manifest.version)? > manifest::version(&previous.version)? {
                "update"
            } else {
                "installed"
            }
        } else {
            "install"
        };
        Ok(Preview {
            compatible: manifest.supports(&self.device_platform()?),
            manifest,
            source,
            operation,
        })
    }

    fn plugins_for(&self, persona: &str, filter: Option<&str>) -> Result<Vec<Plugin>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let platform = self.device_platform()?;
        let mut query=self.conn.prepare(
            "SELECT p.manifest_json,p.source,p.granted_json,p.global_enabled,p.config_json,s.enabled,s.config_json,
               (SELECT action FROM app_plugin_audit a WHERE a.plugin_id=p.id AND (a.persona_id IS NULL OR a.persona_id=?1)
                 ORDER BY a.seq DESC LIMIT 1)
             FROM app_plugins p LEFT JOIN app_plugin_personas s ON s.plugin_id=p.id AND s.persona_id=?1
             WHERE ?2 IS NULL OR p.id=?2 ORDER BY p.id")?;
        let rows = query.query_map(params![persona, filter], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<bool>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?;
        rows.map(|r| {
            let (
                raw,
                source,
                grants,
                global_enabled,
                config,
                persona_enabled,
                personal,
                last_action,
            ) = r?;
            let manifest = Manifest::parse(&raw)?;
            let granted: Vec<Permission> = serde_json::from_str(&grants)?;
            let global_config = manifest.configuration(&serde_json::from_str(&config)?)?;
            let persona_config = personal
                .map(|raw| manifest.configuration(&serde_json::from_str(&raw)?))
                .transpose()?;
            let effective_config = persona_config.as_ref().unwrap_or(&global_config).clone();
            let status = if !manifest.supports(&platform) {
                "unsupported"
            } else if !persona_enabled.unwrap_or(global_enabled) {
                "disabled"
            } else if !manifest.permissions.iter().all(|p| granted.contains(p)) {
                "permission_required"
            } else {
                "enabled"
            };
            Ok(Plugin {
                manifest,
                source,
                granted,
                global_enabled,
                global_config,
                persona_enabled,
                persona_config,
                effective_config,
                status,
                last_action,
            })
        })
        .collect()
    }

    pub fn list_plugins(&self, persona: &str) -> Result<Vec<Plugin>> {
        scope(self, persona)?;
        self.plugins_for(persona, None)
    }

    pub fn due_plugin_reminders(&self, persona: &str, now: i64) -> Result<Vec<DueReminder>> {
        scope(self, persona)?;
        let ready = self
            .plugins_for(persona, None)?
            .into_iter()
            .filter(|p| p.status == "enabled" && matches!(p.manifest.entry, Entry::TimeReminder))
            .map(|p| p.manifest.id)
            .collect::<std::collections::BTreeSet<_>>();
        let mut query = self.conn.prepare(
            "SELECT plugin_id,id,label,due_at_ms FROM app_plugin_reminders WHERE persona_id=?1
             AND delivered_at_ms IS NULL AND due_at_ms<=?2 ORDER BY due_at_ms,id LIMIT 60",
        )?;
        let entries = query
            .query_map(params![persona, now], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    DueReminder {
                        id: r.get(1)?,
                        label: r.get(2)?,
                        due_at_ms: r.get(3)?,
                    },
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(entries
            .into_iter()
            .filter(|(id, _)| ready.contains(id))
            .map(|(_, r)| r)
            .collect())
    }

    fn checked_plugin(&self, persona: &str, id: &str, checksum: &str) -> Result<Plugin> {
        self.plugins_for(persona, Some(id))?
            .into_iter()
            .next()
            .filter(|p| p.manifest.checksum == checksum)
            .ok_or(Error::Invalid("PLUGIN_CHANGED"))
    }

    pub fn apply_plugin(&mut self, persona: &str, action: Action) -> Result<Value> {
        scope(self, persona)?;
        match action {
            Action::Install {
                raw,
                source,
                checksum,
            } => {
                let preview = self.inspect_plugin(&raw, &source)?;
                if !preview.compatible
                    || preview.manifest.checksum != checksum
                    || preview.operation == "installed"
                {
                    return Err(Error::Invalid("PLUGIN_CHANGED"));
                }
                let count: i64 =
                    self.conn
                        .query_row("SELECT count(*) FROM app_plugins", [], |r| r.get(0))?;
                if preview.operation == "install" && count >= 32 {
                    return Err(Error::Invalid("PLUGIN_CONFIG"));
                }
                let manifest = preview.manifest;
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                tx.execute(
                    "INSERT INTO app_plugins(id,manifest_json,source,installed_at_ms) VALUES(?1,?2,?3,?4)
                     ON CONFLICT(id) DO UPDATE SET manifest_json=excluded.manifest_json,source=excluded.source,
                     installed_at_ms=excluded.installed_at_ms,granted_json='[]',global_enabled=0,config_json='{}'",
                    params![manifest.id,serde_json::to_string(&manifest)?,preview.source,now_ms()])?;
                tx.execute(
                    "DELETE FROM app_plugin_personas WHERE plugin_id=?1",
                    [&manifest.id],
                )?;
                audit(
                    &tx,
                    &manifest,
                    None,
                    if preview.operation == "update" {
                        "updated"
                    } else {
                        "installed"
                    },
                )?;
                tx.commit()?;
            }
            Action::Grants {
                id,
                checksum,
                permissions,
            } => {
                let plugin = self.checked_plugin(persona, &id, &checksum)?;
                if permissions
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != permissions.len()
                    || !permissions
                        .iter()
                        .all(|p| plugin.manifest.permissions.contains(p))
                {
                    return Err(Error::Invalid("PLUGIN_PERMISSION"));
                }
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                tx.execute(
                    "UPDATE app_plugins SET granted_json=?2 WHERE id=?1",
                    params![id, serde_json::to_string(&permissions)?],
                )?;
                audit(&tx, &plugin.manifest, None, "permissions_updated")?;
                tx.commit()?;
            }
            Action::Settings {
                id,
                checksum,
                scope: target,
                enabled,
                config,
            } => {
                let plugin = self.checked_plugin(persona, &id, &checksum)?;
                let config = config
                    .as_ref()
                    .map(|c| plugin.manifest.configuration(c))
                    .transpose()?;
                let encoded = config.as_ref().map(serde_json::to_string).transpose()?;
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                match target {
                    Scope::Device => {
                        tx.execute(
                            "UPDATE app_plugins SET global_enabled=?2,config_json=?3 WHERE id=?1",
                            params![
                                id,
                                enabled.ok_or(Error::Invalid("PLUGIN_CONFIG"))?,
                                encoded.unwrap_or_else(|| "{}".into())
                            ],
                        )?;
                    }
                    Scope::Persona => {
                        tx.execute("INSERT INTO app_plugin_personas(plugin_id,persona_id,enabled,config_json) VALUES(?1,?2,?3,?4)
                            ON CONFLICT(plugin_id,persona_id) DO UPDATE SET enabled=excluded.enabled,config_json=excluded.config_json",
                            params![id,persona,enabled,encoded])?;
                    }
                }
                audit(
                    &tx,
                    &plugin.manifest,
                    matches!(target, Scope::Persona).then_some(persona),
                    "configuration_saved",
                )?;
                tx.commit()?;
            }
            Action::Uninstall { id, checksum } => {
                let plugin = self.checked_plugin(persona, &id, &checksum)?;
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                tx.execute("DELETE FROM app_plugins WHERE id=?1", [id])?;
                audit(&tx, &plugin.manifest, None, "uninstalled")?;
                tx.commit()?;
            }
            Action::CreateReminder {
                id,
                checksum,
                minutes,
                label,
            } => {
                let plugin = self.checked_plugin(persona, &id, &checksum)?;
                let label = label.trim();
                if plugin.status != "enabled"
                    || !matches!(plugin.manifest.entry, Entry::TimeReminder)
                {
                    return Err(Error::Invalid("PLUGIN_PERMISSION"));
                }
                if !(1..=1440).contains(&minutes)
                    || label.is_empty()
                    || label.len() > 450
                    || label.chars().any(char::is_control)
                {
                    return Err(Error::Invalid("PLUGIN_CONFIG"));
                }
                let count: i64 = self.conn.query_row(
                    "SELECT count(*) FROM app_plugin_reminders WHERE delivered_at_ms IS NULL",
                    [],
                    |r| r.get(0),
                )?;
                if count >= 60 {
                    return Err(Error::Invalid("PLUGIN_CONFIG"));
                }
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                let due = now_ms().saturating_add(i64::from(minutes) * 60_000);
                tx.execute("INSERT INTO app_plugin_reminders(id,plugin_id,persona_id,label,due_at_ms) VALUES(?1,?2,?3,?4,?5)",
                    params![Uuid::new_v4().to_string(),id,persona,label,due])?;
                audit(&tx, &plugin.manifest, Some(persona), "reminder_scheduled")?;
                tx.commit()?;
                return Ok(json!({"due_at_ms":due}));
            }
            Action::WriteNote { id, checksum, text } => {
                let plugin = self.checked_plugin(persona, &id, &checksum)?;
                if plugin.status != "enabled"
                    || !matches!(plugin.manifest.entry, Entry::MemoryNote | Entry::MediaNote)
                {
                    return Err(Error::Invalid("PLUGIN_PERMISSION"));
                }
                let label = text.trim();
                if label.is_empty() || label.len() > 2400 {
                    return Err(Error::Invalid("PLUGIN_CONFIG"));
                }
                let prefix = plugin
                    .effective_config
                    .get("prefix")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty());
                let text = prefix.map_or_else(|| label.to_owned(), |p| format!("{p}\n{label}"));
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                let profile: String = tx.query_row(
                    "SELECT profile_id FROM personas WHERE id=?1",
                    [persona],
                    |r| r.get(0),
                )?;
                let key = Uuid::new_v4().to_string();
                Self::append_in(
                    &tx,
                    &self.device_id,
                    &profile,
                    Entity::Memory,
                    &key,
                    Operation::Put,
                    json!({"text":text,"persona_id":persona}),
                )?;
                audit(&tx, &plugin.manifest, Some(persona), "memory_created")?;
                tx.commit()?;
                return Ok(json!({"saved":true}));
            }
            Action::DismissReminder { reminder_id } => {
                if Uuid::parse_str(&reminder_id).is_err() {
                    return Err(Error::Invalid("PLUGIN_CONFIG"));
                }
                self.conn.execute("UPDATE app_plugin_reminders SET delivered_at_ms=?3 WHERE id=?1 AND persona_id=?2 AND delivered_at_ms IS NULL",
                    params![reminder_id,persona,now_ms()])?;
                self.conn.execute("DELETE FROM app_plugin_reminders WHERE delivered_at_ms IS NOT NULL AND delivered_at_ms<?1",
                    [now_ms()-7*86_400_000])?;
            }
        }
        Ok(Value::Null)
    }

    pub(crate) fn append_plugin_styles(&self, persona: &str, prompt: &mut String) -> Result<()> {
        for plugin in self
            .plugins_for(persona, None)?
            .into_iter()
            .filter(|p| {
                p.status == "enabled" && matches!(p.manifest.entry, Entry::ChatStyle { .. })
            })
            .take(3)
        {
            let Entry::ChatStyle { preset } = plugin.manifest.entry else {
                continue;
            };
            let style = match preset {
                Preset::Warm => "语气温和，承接用户的感受",
                Preset::Brief => "优先回答当前问题，省略重复总结",
                Preset::Playful => "自然时点缀一点幽默",
                Preset::Story => "在明确的虚构场景里共创细节",
            };
            let concise = plugin
                .effective_config
                .get("concise")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            prompt.push_str(&format!(
                "\n已授权的表达偏好。{style}。{}",
                if concise { "控制篇幅。" } else { "" }
            ));
            audit(&self.conn, &plugin.manifest, Some(persona), "style_applied")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const STYLE: &str =
        include_str!("../../../apps/companion/src/plugins/examples/chat-style.json");
    const TIME: &str =
        include_str!("../../../apps/companion/src/plugins/examples/time-reminder.json");

    fn install(store: &mut Store, persona: &str, raw: &str) -> Manifest {
        let manifest = Manifest::parse(raw).unwrap();
        store
            .apply_plugin(
                persona,
                Action::Install {
                    raw: raw.into(),
                    source: "合成清单.json".into(),
                    checksum: manifest.checksum.clone(),
                },
            )
            .unwrap();
        manifest
    }
    fn grant(
        store: &mut Store,
        persona: &str,
        manifest: &Manifest,
        permissions: Vec<Permission>,
    ) -> Result<Value> {
        store.apply_plugin(
            persona,
            Action::Grants {
                id: manifest.id.clone(),
                checksum: manifest.checksum.clone(),
                permissions,
            },
        )
    }
    fn enable(store: &mut Store, persona: &str, m: &Manifest, scope: Scope) {
        store
            .apply_plugin(
                persona,
                Action::Settings {
                    id: m.id.clone(),
                    checksum: m.checksum.clone(),
                    scope,
                    enabled: Some(true),
                    config: None,
                },
            )
            .unwrap();
    }
    #[test]
    fn strict_manifest_rejects_unknown_entry_invalid_revision_paths_and_ungranted_capabilities() {
        let mut s = Store::open(":memory:").unwrap();
        s.set_device_platform("windows").unwrap();
        for raw in [STYLE, TIME] {
            assert!(Manifest::parse(raw).is_ok());
        }
        let mut manifest: Value = serde_json::from_str(STYLE).unwrap();
        for (key, value) in [
            ("entry", json!({"kind":"script","path":"../fixture.txt"})),
            ("id", json!("fixture/../other")),
            ("permissions", json!(["api_key"])),
            ("requiredApiVersion", json!(2)),
            ("name", json!("未重新签入校验值的文字")),
            ("checksum", json!("0")),
        ] {
            let old = manifest[key].clone();
            manifest[key] = value;
            assert!(Manifest::parse(&manifest.to_string()).is_err());
            manifest[key] = old;
        }
        assert!(s.inspect_plugin(STYLE, "../fixture.json").is_err());
    }
    #[test]
    fn settings_grants_revocation_and_update_are_scoped_to_persona() {
        let mut s = Store::open(":memory:").unwrap();
        s.set_device_platform("windows").unwrap();
        let a = s.active_persona_id().unwrap();
        let b = s.create_persona("另一人格", "喜欢图书").unwrap().id;
        let m = install(&mut s, &a, STYLE);
        assert_eq!(s.list_plugins(&a).unwrap()[0].status, "disabled");
        assert!(s.list_plugins(&b).is_err());
        assert!(grant(&mut s, &a, &m, vec![Permission::MemoryWrite]).is_err());
        enable(&mut s, &a, &m, Scope::Persona);
        assert_eq!(s.list_plugins(&a).unwrap()[0].status, "permission_required");
        grant(&mut s, &a, &m, m.permissions.clone()).unwrap();
        let mut prompt = String::new();
        s.append_plugin_styles(&a, &mut prompt).unwrap();
        assert!(prompt.contains("省略重复总结"));
        assert_eq!(
            s.list_plugins(&a).unwrap()[0].last_action.as_deref(),
            Some("style_applied")
        );
        s.select_persona(&b).unwrap();
        assert!(
            s.apply_plugin(
                &a,
                Action::Settings {
                    id: m.id.clone(),
                    checksum: m.checksum.clone(),
                    scope: Scope::Persona,
                    enabled: None,
                    config: None
                }
            )
            .is_err()
        );
        s.append_plugin_styles(&b, &mut String::new()).unwrap();
        assert_eq!(s.list_plugins(&b).unwrap()[0].status, "disabled");
        s.select_persona(&a).unwrap();
        grant(&mut s, &a, &m, Vec::new()).unwrap();
        let mut prompt = String::new();
        s.append_plugin_styles(&a, &mut prompt).unwrap();
        assert!(prompt.is_empty());
        let mut update = m;
        update.version = "1.1.0".into();
        update.checksum = update.calculated_checksum().unwrap();
        let next = install(&mut s, &a, &serde_json::to_string(&update).unwrap());
        assert_eq!(s.list_plugins(&a).unwrap()[0].status, "disabled");
        assert!(s.list_plugins(&a).unwrap()[0].granted.is_empty());
        s.apply_plugin(
            &a,
            Action::Uninstall {
                id: next.id,
                checksum: next.checksum,
            },
        )
        .unwrap();
        assert!(s.list_plugins(&a).unwrap().is_empty());
    }
    #[test]
    fn scheduled_reminder_is_persistent_and_becomes_due_once_for_its_persona() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reminder.sqlite3");
        let mut s = Store::open(&path).unwrap();
        s.set_device_platform("android").unwrap();
        let persona = s.active_persona_id().unwrap();
        let m = install(&mut s, &persona, TIME);
        let schedule = || Action::CreateReminder {
            id: m.id.clone(),
            checksum: m.checksum.clone(),
            minutes: 1,
            label: "本地提醒夹具".into(),
        };
        assert!(s.apply_plugin(&persona, schedule()).is_err());
        grant(&mut s, &persona, &m, m.permissions.clone()).unwrap();
        enable(&mut s, &persona, &m, Scope::Device);
        let due = s.apply_plugin(&persona, schedule()).unwrap()["due_at_ms"]
            .as_i64()
            .unwrap();
        assert!(
            s.due_plugin_reminders(&persona, due - 1)
                .unwrap()
                .is_empty()
        );
        drop(s);
        let mut s = Store::open(&path).unwrap();
        let reminder = s
            .due_plugin_reminders(&persona, due)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(reminder.label, "本地提醒夹具");
        s.apply_plugin(
            &persona,
            Action::DismissReminder {
                reminder_id: reminder.id,
            },
        )
        .unwrap();
        assert!(s.due_plugin_reminders(&persona, due).unwrap().is_empty());
    }
}
