//! Non-destructive identity reconciliation after an authenticated host pairing.
//! Neither QR parsing nor pairing creates a persona. Only auto-provisioned builtins
//! may be linked; their original preferences, message IDs and memories remain intact.
use crate::{Error, Result, chat::now_ms, storage::Store};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Entity, sha256_hex};
    use serde_json::json;

    #[test]
    fn v4_upgrade_is_backed_up_preserves_rows_and_rolls_back_on_failure() {
        for inject_failure in [false, true] {
            let dir = std::env::temp_dir().join(format!("pairing-migration-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("fixture.sqlite3");
            let persona = Uuid::new_v4().to_string();
            let device = Uuid::new_v4().to_string();
            let message = Uuid::new_v4().to_string();
            let mut db = Connection::open(&path).unwrap();
            for (number, sql) in [
                (1, include_str!("../../../db/migrations/0001_initial.sql")),
                (2, include_str!("../../../db/migrations/0002_chat.sql")),
                (3, include_str!("../../../db/migrations/0003_personas.sql")),
                (4, include_str!("../../../db/migrations/0004_lan_sync.sql")),
            ] {
                db.execute_batch(sql).unwrap();
                db.execute(
                    "INSERT INTO schema_migrations VALUES(?1,?2)",
                    params![number, sha256_hex(sql.as_bytes())],
                )
                .unwrap();
            }
            db.execute_batch("PRAGMA user_version=4; PRAGMA foreign_keys=ON;")
                .unwrap();
            db.execute(
                "INSERT INTO local_profiles(id,display_name) VALUES(?1,'合成旧库')",
                [&persona],
            )
            .unwrap();
            db.execute("INSERT INTO devices(id,profile_id,name,trust_state) VALUES(?1,?2,'合成本机','local')",params![device,persona]).unwrap();
            db.execute("INSERT INTO local_clock(device_id) VALUES(?1)", [&device])
                .unwrap();
            db.execute(
                "INSERT INTO conversations(id,profile_id,title) VALUES(?1,?1,'旧永久空间')",
                [&persona],
            )
            .unwrap();
            crate::persona::ensure_default(&db, &persona, &device).unwrap();
            let tx = db.transaction().unwrap();
            Store::append_in(&tx,&device,&persona,Entity::Message,&message,crate::event::Operation::Put,json!({"role":"user","content":"升级前合成聊天","persona_id":persona,"conversation_id":persona})).unwrap();
            tx.commit().unwrap();
            if inject_failure {
                db.execute_batch("CREATE INDEX persona_links_target ON personas(name)")
                    .unwrap();
            }
            let hash: String = db
                .query_row(
                    "SELECT checksum FROM schema_migrations WHERE version=4",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                hash,
                sha256_hex(include_str!("../../../db/migrations/0004_lan_sync.sql").as_bytes())
            );
            drop(db);
            if inject_failure {
                assert!(Store::open(&path).is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(
                    db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                        .unwrap(),
                    4
                );
                assert_eq!(
                    db.query_row(
                        "SELECT content FROM messages WHERE id=?1",
                        [&message],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                    "升级前合成聊天"
                );
                assert_eq!(db.query_row("SELECT count(*) FROM pragma_table_info('personas') WHERE name='avatar_resource'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
            } else {
                let store = Store::open(&path).unwrap();
                assert_eq!(store.personas().unwrap()[0].id, persona);
                assert_eq!(store.chat_messages(&persona).unwrap()[0].id, message);
                assert_eq!(store.diagnostics().unwrap().integrity, "ok");
            }
            assert_eq!(
                std::fs::read_dir(&dir)
                    .unwrap()
                    .filter_map(|p| p.ok())
                    .filter(|p| p.file_name().to_string_lossy().contains("pre-v6-"))
                    .count(),
                1
            );
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HostAuthority {
    pub device_id: String,
    pub profile_id: String,
    pub persona_id: String,
}
pub(crate) fn resolve(db: &Connection, id: &str) -> Result<String> {
    Ok(db
        .query_row(
            "SELECT canonical_id FROM persona_links WHERE alias_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_else(|| id.to_owned()))
}
impl Store {
    pub fn local_host_authority(&self) -> Result<HostAuthority> {
        let persona_id: String = self.conn.query_row("SELECT id FROM personas WHERE profile_id=?1 AND is_builtin=1 AND NOT EXISTS(SELECT 1 FROM persona_links WHERE alias_id=personas.id)", [&self.profile_id], |r| r.get(0))?;
        Ok(HostAuthority {
            device_id: self.device_id.clone(),
            profile_id: self.profile_id.clone(),
            persona_id,
        })
    }
    /// Only call after the host's pinned TLS reply and device identity were verified.
    pub fn bind_paired_host(&mut self, host: &HostAuthority) -> Result<()> {
        for id in [&host.device_id, &host.profile_id, &host.persona_id] {
            Uuid::parse_str(id).map_err(|_| Error::Invalid("host UUID"))?;
        }
        if host.device_id == self.device_id
            || !self
                .lan_peers()?
                .iter()
                .any(|p| p.id == host.device_id && !p.revoked)
        {
            return Err(Error::Untrusted);
        }
        let previous: Option<(String, String, String)> = self
            .conn
            .query_row(
                "SELECT host_device_id,profile_id,persona_id FROM lan_host_binding WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if previous.is_some_and(|v| {
            v != (
                host.device_id.clone(),
                host.profile_id.clone(),
                host.persona_id.clone(),
            )
        }) {
            return Err(Error::Conflict);
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO lan_host_binding VALUES(1,?1,?2,?3,?4)",
            params![host.device_id, host.profile_id, host.persona_id, now_ms()],
        )?;
        self.reconcile_paired_persona()
    }
    pub(crate) fn reconcile_paired_persona(&mut self) -> Result<()> {
        let host: Option<(String,String,String)> = self.conn.query_row("SELECT b.host_device_id,b.profile_id,b.persona_id FROM lan_host_binding b JOIN lan_peers p ON p.id=b.host_device_id AND p.revoked=0 WHERE b.id=1", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let Some((device, profile, persona)) = host else {
            return Ok(());
        };
        // Wait for actual authenticated profile/persona/device rows, not QR metadata.
        let ready: bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM personas p JOIN devices d ON d.id=?1 WHERE p.id=?2 AND p.profile_id=?3 AND p.is_builtin=1 AND d.profile_id=p.profile_id)",params![device,persona,profile],|r|r.get(0))?;
        if !ready {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute("INSERT OR IGNORE INTO persona_links(alias_id,canonical_id,host_device_id,source_device_id,reason,created_at_ms) SELECT id,?1,?2,?3,'paired_builtin',?4 FROM personas WHERE profile_id=?5 AND is_builtin=1 AND id<>?1",params![persona,device,self.device_id,now_ms(),self.profile_id])?;
        tx.execute("UPDATE persona_selection SET persona_id=?1 WHERE device_id=?2 AND persona_id IN (SELECT alias_id FROM persona_links WHERE canonical_id=?1)",params![persona,self.device_id])?;
        tx.commit()?;
        Ok(())
    }
}
