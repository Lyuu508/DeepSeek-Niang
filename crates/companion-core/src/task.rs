//! Authority-local lease ledger. Remote replicas must never independently grant leases.
use crate::event::sha256_hex;
use crate::{Error, Result, storage::Store};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub task_id: String,
    pub owner_device_id: String,
    pub fence: i64,
    pub authority_boot_id: String,
    pub until_tick: i64,
}
fn valid_ticks(now: i64, ttl: i64) -> Result<i64> {
    if now < 0 || ttl <= 0 || ttl > 60_000 || now > i64::MAX - 60_000 {
        return Err(Error::Invalid("monotonic lease deadline"));
    }
    Ok(now + ttl)
}
fn verify(
    tx: &Transaction<'_>,
    lease: &Lease,
    authority: &str,
    boot: &str,
    now: i64,
) -> Result<()> {
    if now < 0 || lease.fence <= 0 {
        return Err(Error::StaleLease);
    }
    let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_queue WHERE id=?1 AND authority_device_id=?2 AND owner_device_id=?3 AND fence=?4 AND lease_boot_id=?5 AND state='running' AND lease_until_tick>?6)",params![lease.task_id,authority,lease.owner_device_id,lease.fence,boot,now],|r|r.get(0))?;
    if !valid || lease.authority_boot_id != boot {
        return Err(Error::StaleLease);
    }
    Ok(())
}
fn log(tx: &Transaction<'_>, id: &str, fence: i64, code: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO task_logs(task_id,fence,code) VALUES(?1,?2,?3)",
        params![id, fence, code],
    )?;
    Ok(())
}
impl Store {
    pub fn queue_task(&mut self, kind: &str, key: &str, payload: &Value) -> Result<String> {
        if kind.is_empty()
            || kind.len() > 100
            || key.is_empty()
            || key.len() > 256
            || serde_json::to_vec(payload)?.len() > 65_536
        {
            return Err(Error::Invalid("task input"));
        }
        let persona = self.active_persona_id()?;
        let profile: String = self.conn.query_row(
            "SELECT profile_id FROM personas WHERE id=?1",
            [&persona],
            |r| r.get(0),
        )?;
        let id = Uuid::new_v4().to_string();
        let body = serde_json::to_string(payload)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO task_queue(id,profile_id,kind,dedupe_key,authority_device_id,payload_json,persona_id) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(profile_id,persona_id,dedupe_key) DO NOTHING",params![id,profile,kind,key,self.device_id,body,persona])?;
        let (saved_id, saved_kind, saved_body): (String, String, String) = tx.query_row(
            "SELECT id,kind,payload_json FROM task_queue WHERE profile_id=?1 AND dedupe_key=?2 AND persona_id=?3",
            params![profile, key,persona],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if saved_kind != kind || saved_body != body {
            return Err(Error::Conflict);
        }
        tx.commit()?;
        Ok(saved_id)
    }
    pub fn claim_task(&mut self, id: &str, worker: &str, now_tick: i64, ttl: i64) -> Result<Lease> {
        let until = valid_ticks(now_tick, ttl)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let trusted:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM devices WHERE id=?1 AND profile_id=?2 AND trust_state IN ('local','trusted'))",params![worker,self.profile_id],|r|r.get(0))?;
        if !trusted {
            return Err(Error::Untrusted);
        }
        let changed=tx.execute("UPDATE task_queue SET state='running',owner_device_id=?1,fence=fence+1,lease_boot_id=?2,lease_until_tick=?3,attempt=attempt+1,last_error=NULL WHERE id=?4 AND authority_device_id=?5 AND state='queued' AND attempt<max_attempts",params![worker,self.boot_id,until,id,self.device_id])?;
        if changed != 1 {
            return Err(Error::StaleLease);
        }
        let fence: i64 = tx.query_row("SELECT fence FROM task_queue WHERE id=?1", [id], |r| {
            r.get(0)
        })?;
        log(&tx, id, fence, "CLAIMED")?;
        tx.commit()?;
        Ok(Lease {
            task_id: id.into(),
            owner_device_id: worker.into(),
            fence,
            authority_boot_id: self.boot_id.clone(),
            until_tick: until,
        })
    }
    pub fn heartbeat_task(&mut self, lease: &Lease, now: i64, ttl: i64) -> Result<Lease> {
        let until = valid_ticks(now, ttl)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify(&tx, lease, &self.device_id, &self.boot_id, now)?;
        tx.execute(
            "UPDATE task_queue SET lease_until_tick=?1 WHERE id=?2",
            params![until, lease.task_id],
        )?;
        tx.commit()?;
        Ok(Lease {
            until_tick: until,
            ..lease.clone()
        })
    }
    pub fn checkpoint_task(
        &mut self,
        lease: &Lease,
        now: i64,
        step: &str,
        value: &Value,
    ) -> Result<()> {
        let json = serde_json::to_string(value)?;
        if step.is_empty() || step.len() > 200 || json.len() > 65_536 {
            return Err(Error::Invalid("checkpoint input"));
        }
        let checksum = sha256_hex(json.as_bytes());
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify(&tx, lease, &self.device_id, &self.boot_id, now)?;
        tx.execute("INSERT INTO task_checkpoints(task_id,step_key,fence,checkpoint_json,checksum) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(task_id,step_key) DO UPDATE SET fence=excluded.fence,checkpoint_json=excluded.checkpoint_json,checksum=excluded.checksum",params![lease.task_id,step,lease.fence,json,checksum])?;
        log(&tx, &lease.task_id, lease.fence, "CHECKPOINT")?;
        tx.commit()?;
        Ok(())
    }
    pub fn complete_task(&mut self, lease: &Lease, now: i64) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify(&tx, lease, &self.device_id, &self.boot_id, now)?;
        tx.execute("UPDATE task_queue SET state='completed',lease_until_tick=NULL,lease_boot_id=NULL WHERE id=?1",[&lease.task_id])?;
        log(&tx, &lease.task_id, lease.fence, "COMPLETED")?;
        tx.commit()?;
        Ok(())
    }
    /// Lease expiry cannot prove an external provider did not execute/charge.
    pub fn expire_leases(&mut self, now: i64) -> Result<usize> {
        if now < 0 {
            return Err(Error::Invalid("tick overflow"));
        }
        Ok(self.conn.execute("UPDATE task_queue SET state='uncertain',fence=fence+1,lease_boot_id=NULL,lease_until_tick=NULL,last_error='LEASE_EXPIRED_RECONCILE' WHERE authority_device_id=?1 AND state='running' AND lease_until_tick<=?2",params![self.device_id,now])?)
    }
    pub fn pause_task(&mut self, id: &str) -> Result<()> {
        self.transition(id, "paused", &["running", "queued"])
    }
    pub fn cancel_task(&mut self, id: &str) -> Result<()> {
        self.transition(
            id,
            "cancelled",
            &["running", "queued", "paused", "uncertain", "failed"],
        )
    }
    pub fn resume_task(&mut self, id: &str) -> Result<()> {
        self.transition(id, "queued", &["paused"])
    }
    pub fn retry_failed_task(&mut self, id: &str) -> Result<()> {
        self.transition(id, "queued", &["failed"])
    }
    pub fn fail_task(&mut self, lease: &Lease, now: i64) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify(&tx, lease, &self.device_id, &self.boot_id, now)?;
        tx.execute("UPDATE task_queue SET state='failed',fence=fence+1,lease_boot_id=NULL,lease_until_tick=NULL,last_error='WORKER_FAILED' WHERE id=?1",[&lease.task_id])?;
        log(&tx, &lease.task_id, lease.fence, "FAILED")?;
        tx.commit()?;
        Ok(())
    }
    /// Call only after checking provider request status/checkpoints and idempotency.
    pub fn reconcile_safe_retry(&mut self, id: &str) -> Result<()> {
        self.transition(id, "queued", &["uncertain"])
    }
    fn transition(&mut self, id: &str, target: &str, allowed: &[&str]) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state: Option<String> = tx
            .query_row(
                "SELECT state FROM task_queue WHERE id=?1 AND authority_device_id=?2",
                params![id, self.device_id],
                |r| r.get(0),
            )
            .optional()?;
        if !state.as_deref().is_some_and(|s| allowed.contains(&s)) {
            return Err(Error::StaleLease);
        }
        tx.execute("UPDATE task_queue SET state=?1,fence=fence+1,lease_boot_id=NULL,lease_until_tick=NULL WHERE id=?2",params![target,id])?;
        let fence: i64 = tx.query_row("SELECT fence FROM task_queue WHERE id=?1", [id], |r| {
            r.get(0)
        })?;
        log(&tx, id, fence, target)?;
        tx.commit()?;
        Ok(())
    }
    pub fn task_state(&self, id: &str) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT state FROM task_queue WHERE id=?1", [id], |r| {
                r.get(0)
            })?)
    }
}
