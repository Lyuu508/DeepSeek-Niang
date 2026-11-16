//! One virtual life ledger, driven by the authority-local task runner. No external work is simulated.
use super::{CompanionPreferences, stable_id};
use crate::{
    Error, Result,
    event::{Entity, Operation, sha256_hex},
    storage::Store,
    task::{Lease, log, verify},
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};

type ActivityRow = (
    String,
    String,
    Option<String>,
    i64,
    i64,
    i64,
    Option<String>,
);

pub fn activity_label(kind: &str) -> &'static str {
    match kind {
        "wake" => "起床",
        "rest" => "休息",
        "study" => "学习",
        "read" => "阅读已保存的话题",
        "organize" => "整理记忆",
        "recall" => "回顾聊天",
        "interest" => "思考兴趣",
        "create" => "创作构思",
        "journal" => "记录生活",
        "share" => "准备分享",
        "video" => "视频话题构想（未播放）",
        "music" => "音乐话题构想（未播放）",
        _ => "等待主人",
    }
}
fn next_kind(kind: &str) -> &'static str {
    match kind {
        "wake" => "read",
        "read" => "study",
        "study" => "organize",
        "organize" => "recall",
        "recall" => "interest",
        "interest" => "create",
        "create" => "journal",
        "journal" => "share",
        "share" => "rest",
        _ => "wake",
    }
}
pub fn quiet(now: i64, p: &CompanionPreferences) -> bool {
    let m = (now / 60000 + p.timezone_offset_minutes).rem_euclid(1440);
    let (a, b) = (p.quiet_start_minute, p.quiet_end_minute);
    a != b
        && if a < b {
            m >= a && m < b
        } else {
            m >= a || m < b
        }
}
pub fn proactive_limits(p: &CompanionPreferences, stage: &str) -> (i64, i64) {
    let base = match p.proactive_mode.as_str() {
        "quiet" => p.proactive_interval_minutes.max(720),
        "active" => p.proactive_interval_minutes.min(120),
        _ => p.proactive_interval_minutes,
    };
    let interval = match stage {
        "初识" => base * 2,
        "熟悉" => base * 3 / 2,
        _ => base,
    };
    (
        interval,
        if stage == "初识" || p.proactive_mode == "quiet" {
            p.proactive_daily_limit.min(1)
        } else {
            p.proactive_daily_limit
        },
    )
}
#[allow(clippy::too_many_arguments)] // One immutable event includes its explicit source and checkpoint sequence.
pub(crate) fn event(
    db: &Connection,
    p: &str,
    a: &str,
    host: &str,
    kind: &str,
    description: &str,
    now: i64,
    seq: i64,
) -> Result<String> {
    let id = if seq == 0 {
        stable_id("life", &format!("{a}/{kind}"))
    } else {
        stable_id("life", &format!("{a}/{kind}/{seq}"))
    };
    db.execute("INSERT OR IGNORE INTO companion_life_events(id,persona_id,activity_id,host_device_id,kind,description,occurred_at_ms,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,p,a,host,kind,description,now,seq])?;
    Ok(id)
}
pub(crate) fn publish(
    tx: &Transaction<'_>,
    p: &str,
    device: &str,
    id: &str,
    content: &str,
    now: i64,
) -> Result<bool> {
    let state: String = tx.query_row(
        "SELECT state FROM companion_proactive WHERE id=?1 AND persona_id=?2",
        params![id, p],
        |r| r.get(0),
    )?;
    if state != "candidate" {
        return Ok(false);
    }
    let (profile, conversation): (String, String) = tx.query_row(
        "SELECT profile_id,main_conversation_id FROM personas WHERE id=?1",
        [p],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let message = stable_id("proactive-chat", id);
    Store::append_in(
        tx,
        device,
        &profile,
        Entity::Message,
        &message,
        Operation::Put,
        json!({"persona_id":p,"conversation_id":conversation,"role":"assistant","content":content,"created_at_ms":now}),
    )?;
    tx.execute("UPDATE companion_proactive SET state='delivered',content=?1,delivered_at_ms=?2,message_id=?3 WHERE id=?4",params![content,now,message,id])?;
    Ok(true)
}
pub(crate) fn policy(
    db: &Connection,
    p: &str,
    prefs: &CompanionPreferences,
    now: i64,
    stage: &str,
) -> Result<bool> {
    if !prefs.proactive_enabled || prefs.proactive_mode == "off" || quiet(now, prefs) {
        return Ok(false);
    }
    let (interval, limit) = proactive_limits(prefs, stage);
    let focused:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM companion_presence WHERE id=1 AND focused=1 AND updated_at_ms<=?1 AND updated_at_ms>?2)",params![now,now-90000],|r|r.get(0))?;
    let interval = if focused { interval } else { interval * 2 };
    let distressed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM companion_mood_events WHERE persona_id=?1 AND mood IN ('hurt','angry','sad') AND intensity_milli>=700 AND started_at_ms<=?2 AND expires_at_ms>?2)",params![p,now],|r|r.get(0))?;
    if distressed {
        return Ok(false);
    }
    let last: Option<i64> = db.query_row(
        "SELECT max(delivered_at_ms) FROM companion_proactive WHERE persona_id=?1",
        [p],
        |r| r.get(0),
    )?;
    if last.is_some_and(|v| now.saturating_sub(v) < interval * 60000) {
        return Ok(false);
    }
    // A rolling window prevents midnight / timezone edits from resetting a quota.
    let count: i64 = db.query_row(
        "SELECT count(*) FROM companion_proactive WHERE persona_id=?1 AND delivered_at_ms>?2",
        params![p, now - 86400000],
        |r| r.get(0),
    )?;
    let busy:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM chat_requests r JOIN conversation_personas c ON c.conversation_id=r.conversation_id WHERE c.persona_id=?1 AND r.state IN ('sending','generating'))",[p],|r|r.get(0))?;
    let recent:Option<i64>=db.query_row("SELECT max(m.created_at_ms) FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE c.persona_id=?1 AND m.role='user'",[p],|r|r.get(0))?;
    Ok(count < limit && !busy && !recent.is_some_and(|v| now >= v && now - v < 120000))
}
impl Store {
    pub fn record_life_presence(&mut self, now: i64, focused: bool) -> Result<()> {
        self.conn.execute("INSERT INTO companion_presence VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET focused=excluded.focused,updated_at_ms=excluded.updated_at_ms",params![focused,now])?;
        Ok(())
    }
    pub fn set_device_platform(&mut self, platform: &str) -> Result<()> {
        if !matches!(platform, "windows" | "android" | "linux" | "unknown") {
            return Err(Error::Invalid("platform"));
        }
        self.conn.execute(
            "UPDATE devices SET platform=?1 WHERE id=?2",
            params![platform, self.device_id],
        )?;
        Ok(())
    }
    pub(crate) fn owns_life(&self, p: &str) -> Result<bool> {
        let host: Option<String> = self
            .conn
            .query_row(
                "SELECT device_id FROM companion_hosts WHERE persona_id=?1",
                [p],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(host) = host {
            return Ok(host == self.device_id);
        }
        // Only the persona's originating profile can establish its sole authority. A replica cannot self-elect offline.
        let profile: String =
            self.conn
                .query_row("SELECT profile_id FROM personas WHERE id=?1", [p], |r| {
                    r.get(0)
                })?;
        Ok(profile == self.profile_id)
    }
    pub fn advance_companion_life(&mut self, now: i64, windows_host: bool) -> Result<usize> {
        self.advance_life_at(now, now, windows_host)
    }
    pub fn advance_life_at(&mut self, now: i64, tick: i64, windows_host: bool) -> Result<usize> {
        if !windows_host {
            return Ok(0);
        }
        if !(0..crate::event::MAX_CLOCK - 86400000).contains(&now) || tick < 0 {
            return Err(Error::Invalid("life time"));
        }
        let mut delivered = 0;
        for persona in self.personas()? {
            let p = &persona.id;
            if !self.owns_life(p)? {
                continue;
            }
            let prefs = self.companion_preferences(p)?;
            let stage = self.bond_snapshot(p)?.stage;
            let previous:Option<(String,i64,String)>=self.conn.query_row("SELECT boot_id,heartbeat_at_ms,next_activity FROM companion_hosts WHERE persona_id=?1",[p],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
            if previous
                .as_ref()
                .is_some_and(|v| v.0 == self.boot_id && now >= v.1 && now - v.1 < 30000)
            {
                continue;
            }
            let gap = previous
                .as_ref()
                .is_some_and(|v| v.0 != self.boot_id || now < v.1 || now - v.1 > 90000);
            self.conn.execute(
                "INSERT OR IGNORE INTO companion_hosts VALUES(?1,?2,?3,?4,'rest')",
                params![p, self.device_id, self.boot_id, now],
            )?;
            let mut next = previous
                .as_ref()
                .map(|v| v.2.clone())
                .unwrap_or_else(|| "rest".into());
            let current:Option<ActivityRow>=self.conn.query_row("SELECT id,kind,task_id,elapsed_ms,duration_ms,checkpoint_seq,plan_id FROM companion_activities WHERE persona_id=?1 AND state IN ('running','queued','uncertain','paused','failed') ORDER BY started_at_ms DESC,id DESC LIMIT 1",[p],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
            let mut ongoing = current.is_some();
            if let Some((id, kind, task_id, elapsed, duration, seq, plan)) = current {
                if let Some(task) = task_id {
                    let mut state = self.task_state(&task)?;
                    if matches!(state.as_str(), "failed" | "cancelled") {
                        self.conn.execute("UPDATE companion_activities SET state=?1,updated_at_ms=max(updated_at_ms,?2) WHERE id=?3",params![state,now,id])?;
                    }
                    if gap && state == "running" {
                        self.pause_task(&task)?;
                        state = "paused".into();
                        self.conn.execute("UPDATE task_queue SET last_error='OFFLINE_VIRTUAL_CHECKPOINT' WHERE id=?1",[&task])?;
                    }
                    if !prefs.life_enabled && state == "running" {
                        self.pause_task(&task)?;
                        state = "paused".into();
                    }
                    if gap || !prefs.life_enabled {
                        let tx = self.conn.transaction()?;
                        tx.execute("UPDATE companion_activities SET state='paused',updated_at_ms=max(updated_at_ms,?1),checkpoint_seq=checkpoint_seq+1 WHERE id=?2",params![now,id])?;
                        event(
                            &tx,
                            p,
                            &id,
                            &self.device_id,
                            "paused",
                            "应用离线或日常已关闭；保留暂停记录，未补算观看或学习",
                            now,
                            seq + 1,
                        )?;
                        tx.commit()?;
                    } else if state == "uncertain" {
                        // Local-only simulation has no external side effects; committed outcome/checkpoint is definitive.
                        let done:bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM companion_life_events WHERE activity_id=?1 AND kind='completed')",[&id],|r|r.get(0))?;
                        if !done {
                            self.reconcile_safe_retry(&task)?;
                            self.conn
                                .execute("UPDATE task_queue SET attempt=0 WHERE id=?1", [&task])?;
                            state = "queued".into();
                        }
                    } else if state == "paused" && prefs.life_enabled {
                        let reason: Option<String> = self.conn.query_row(
                            "SELECT last_error FROM task_queue WHERE id=?1",
                            [&task],
                            |r| r.get(0),
                        )?;
                        if reason.as_deref() == Some("OFFLINE_VIRTUAL_CHECKPOINT") {
                            self.resume_task(&task)?;
                            self.conn
                                .execute("UPDATE task_queue SET attempt=0 WHERE id=?1", [&task])?;
                            state = "queued".into();
                        }
                    }
                    if !gap && prefs.life_enabled && matches!(state.as_str(), "queued" | "running")
                    {
                        let lease = if state == "queued" {
                            self.claim_task(&task, &self.device_id.clone(), tick, 60000)?
                        } else {
                            let lease:Lease=self.conn.query_row("SELECT owner_device_id,fence,lease_boot_id,lease_until_tick,authority_epoch FROM task_queue WHERE id=?1",[&task],|r|Ok(Lease{task_id:task.clone(),owner_device_id:r.get(0)?,fence:r.get(1)?,authority_boot_id:r.get(2)?,until_tick:r.get(3)?,authority_epoch:r.get(4)?}))?;
                            if lease.until_tick <= tick {
                                self.expire_leases(tick)?;
                                self.conn.execute(
                                    "UPDATE companion_activities SET state='uncertain' WHERE id=?1",
                                    [&id],
                                )?;
                                continue;
                            }
                            self.heartbeat_task(&lease, tick, 60000)?
                        };
                        let delta = if state == "running" {
                            previous
                                .as_ref()
                                .map(|v| now.saturating_sub(v.1).clamp(0, 90000))
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        let elapsed = (elapsed + delta).min(duration);
                        let completed = elapsed >= duration;
                        let sequence = seq + 1;
                        let checkpoint = json!({"activity_id":id,"elapsed_ms":elapsed,"execution_kind":"virtual","sequence":sequence,"result":if completed{local_result(&self.conn,p,&kind)?}else{Value::Null}});
                        let body = serde_json::to_string(&checkpoint)?;
                        let tx = self
                            .conn
                            .transaction_with_behavior(TransactionBehavior::Immediate)?;
                        verify(
                            &tx,
                            &lease,
                            &self.device_id,
                            &self.boot_id,
                            self.authority_epoch,
                            tick,
                        )?;
                        tx.execute("INSERT INTO task_checkpoints(task_id,step_key,fence,checkpoint_json,checksum) VALUES(?1,'life',?2,?3,?4) ON CONFLICT(task_id,step_key) DO UPDATE SET fence=excluded.fence,checkpoint_json=excluded.checkpoint_json,checksum=excluded.checksum",params![task,lease.fence,body,sha256_hex(body.as_bytes())])?;
                        tx.execute("UPDATE companion_activities SET state=?1,elapsed_ms=?2,progress_milli=?3,updated_at_ms=max(updated_at_ms,?4),scheduled_end_at_ms=max(started_at_ms+1,?5),ended_at_ms=?6,checkpoint_seq=?7 WHERE id=?8",params![if completed{"completed"}else{"running"},elapsed,elapsed*1000/duration,now,now+duration-elapsed,if completed{Some(now)}else{None},sequence,id])?;
                        if completed {
                            tx.execute("UPDATE task_queue SET state='completed',lease_until_tick=NULL,lease_boot_id=NULL WHERE id=?1",[&task])?;
                            log(&tx, &task, lease.fence, "COMPLETED")?;
                            let source = event(
                                &tx,
                                p,
                                &id,
                                &self.device_id,
                                "completed",
                                &format!(
                                    "结束一段虚拟{}（本地模拟，无联网内容）",
                                    activity_label(&kind)
                                ),
                                now,
                                0,
                            )?;
                            if let Some(plan) = plan {
                                tx.execute("UPDATE life_plans SET next_step=next_step+1,revision=revision+1,next_due_at_ms=?1,state=CASE WHEN next_step+1>=json_extract(plan_json,'$.steps') THEN 'completed' ELSE state END WHERE id=?2",params![now+86400000,plan])?;
                            }
                            super::living::interest_from_completion(&tx, p, &id, &source, now)?;
                            super::living::remember_completed_plan(
                                &tx,
                                p,
                                &id,
                                &source,
                                &self.device_id,
                                now,
                                prefs.automatic_memory,
                            )?;
                            if prefs.proactive_enabled
                                && prefs.proactive_mode != "off"
                                && !matches!(kind.as_str(), "rest" | "wake" | "wait")
                            {
                                let content = format!(
                                    "一段虚拟{}的安排已经结束。等你有空，我们可以接着聊聊最近的小事。",
                                    activity_label(&kind)
                                );
                                tx.execute("INSERT OR IGNORE INTO companion_proactive(id,persona_id,source_life_event_id,host_device_id,content,created_at_ms,state) VALUES(?1,?2,?3,?4,?5,?6,'candidate')",params![stable_id("proactive",&source),p,source,self.device_id,content,now])?;
                            }
                            next = next_kind(&kind).into();
                            ongoing = false;
                        } else if state == "queued" {
                            event(
                                &tx,
                                p,
                                &id,
                                &self.device_id,
                                "resumed",
                                "从已保存检查点继续；未计入离线时间",
                                now,
                                sequence,
                            )?;
                        }
                        tx.commit()?;
                    }
                } else {
                    // Preserve legacy activities and their UUIDs; adopt rather than replace them.
                    let task = self.queue_persona_task(
                        p,
                        "companion_life",
                        &format!("activity/{id}"),
                        &json!({"activity_id":id}),
                    )?;
                    self.conn.execute(
                        "UPDATE companion_activities SET task_id=?1,state='paused' WHERE id=?2",
                        params![task, id],
                    )?;
                    self.pause_task(&task)?;
                    self.conn.execute(
                        "UPDATE task_queue SET last_error='OFFLINE_VIRTUAL_CHECKPOINT' WHERE id=?1",
                        [task],
                    )?;
                }
            }
            if prefs.life_enabled && !ongoing {
                self.start_life_activity(p, &prefs, &next, now)?;
            }
            self.conn.execute("UPDATE companion_hosts SET boot_id=?1,heartbeat_at_ms=max(heartbeat_at_ms,?2),next_activity=?3 WHERE persona_id=?4",params![self.boot_id,now,next,p])?;
            if !prefs.proactive_enabled || prefs.proactive_mode == "off" {
                self.conn.execute("UPDATE companion_proactive SET state='dismissed' WHERE persona_id=?1 AND state='candidate'",[p])?;
            }
            if !prefs.proactive_model_enabled && policy(&self.conn, p, &prefs, now, &stage)? {
                let item:Option<(String,String)>=self.conn.query_row("SELECT id,content FROM companion_proactive WHERE persona_id=?1 AND host_device_id=?2 AND state='candidate' ORDER BY created_at_ms,id LIMIT 1",params![p,self.device_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
                if let Some((id, content)) = item {
                    let name = self.persona_settings(p)?.user_title;
                    let style = super::bond::style(&stage);
                    let content = format!("{name}，{content}（{}）", super::excerpt(style, 35));
                    let tx = self.conn.transaction()?;
                    delivered += publish(&tx, p, &self.device_id, &id, &content, now)? as usize;
                    tx.commit()?;
                }
            }
        }
        Ok(delivered)
    }
    fn start_life_activity(
        &mut self,
        p: &str,
        prefs: &CompanionPreferences,
        next: &str,
        now: i64,
    ) -> Result<()> {
        if prefs.preferred_activity == "auto" {
            let any: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM life_plans WHERE persona_id=?1 AND host_device_id=?2)",
                params![p, self.device_id],
                |r| r.get(0),
            )?;
            if !any {
                let settings = self.persona_settings(p)?;
                let (kind, topic) = if settings.personality.contains("画")
                    || settings.personality.contains("创作")
                {
                    ("create", "创作构思")
                } else if settings.personality.contains("天文") {
                    ("read", "天文话题构想")
                } else {
                    ("study", "保存的话题")
                };
                let profile: String = self.conn.query_row(
                    "SELECT profile_id FROM personas WHERE id=?1",
                    [p],
                    |r| r.get(0),
                )?;
                self.conn.execute("INSERT OR IGNORE INTO life_plans(id,profile_id,persona_id,name,plan_json,host_device_id,created_at_ms,next_due_at_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?7)",params![stable_id("initial-life-plan",p),profile,p,format!("{}的三日{}计划",settings.character_name,topic),serde_json::to_string(&json!({"kind":kind,"steps":3,"duration_ms":1200000,"execution_kind":"virtual","source":"persona_preferences_local_rules"}))?,self.device_id,now])?;
            }
        }
        let plan:Option<(String,String,i64)>=self.conn.query_row("SELECT id,plan_json,next_step FROM life_plans WHERE persona_id=?1 AND host_device_id=?2 AND state='active' AND next_due_at_ms<=?3 ORDER BY created_at_ms,id LIMIT 1",params![p,self.device_id,now],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let (id, kind, duration, plan_id) = if let Some((plan, body, step)) = plan {
            let spec: Value = serde_json::from_str(&body)?;
            (
                stable_id("plan-step", &format!("{plan}/{step}")),
                spec["kind"].as_str().unwrap_or("study").to_owned(),
                spec["duration_ms"].as_i64().unwrap_or(1200000),
                Some(plan),
            )
        } else {
            (
                uuid::Uuid::new_v4().to_string(),
                if prefs.preferred_activity == "auto" {
                    if quiet(now, prefs) {
                        "rest".into()
                    } else {
                        next.into()
                    }
                } else {
                    prefs.preferred_activity.clone()
                },
                if next == "rest" { 900000 } else { 1200000 },
                None,
            )
        };
        let task = self.queue_persona_task(
            p,
            "companion_life",
            &format!("activity/{id}"),
            &json!({"activity_id":id}),
        )?;
        let tx = self.conn.transaction()?;
        tx.execute("INSERT OR IGNORE INTO companion_activities(id,persona_id,host_device_id,kind,state,started_at_ms,scheduled_end_at_ms,updated_at_ms,task_id,plan_id,duration_ms) VALUES(?1,?2,?3,?4,'queued',?5,?6,?5,?7,?8,?9)",params![id,p,self.device_id,kind,now,now+duration,task,plan_id,duration])?;
        event(
            &tx,
            p,
            &id,
            &self.device_id,
            "started",
            &format!(
                "开始虚拟{}；不代表真实联网、媒体播放或已获取新知识",
                activity_label(&kind)
            ),
            now,
            0,
        )?;
        let mood = match kind.as_str() {
            "study" | "organize" | "interest" => "curious",
            "rest" => "tired",
            "recall" => "missing",
            "share" => "expectant",
            _ => "calm",
        };
        let conversational:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM companion_mood_events WHERE persona_id=?1 AND source_message_id IS NOT NULL AND expires_at_ms>?2)",params![p,now],|r|r.get(0))?;
        if !conversational {
            tx.execute(
                "UPDATE local_clock SET lamport=lamport+1 WHERE device_id=?1",
                [&self.device_id],
            )?;
            tx.execute("INSERT OR IGNORE INTO companion_mood_events(id,persona_id,origin_device_id,activity_id,mood,intensity_milli,started_at_ms,expires_at_ms,lamport) VALUES(?1,?2,?3,?4,?5,300,?6,?7,(SELECT lamport FROM local_clock WHERE device_id=?3))",params![stable_id("life-mood",&id),p,self.device_id,id,mood,now,now+duration])?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn local_result(db: &Connection, p: &str, kind: &str) -> Result<Value> {
    let mut q=db.prepare("SELECT id FROM memories WHERE persona_id=?1 AND is_deleted=0 ORDER BY importance_milli DESC,id LIMIT 6")?;
    let memory_ids = q
        .query_map([p], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let chats:i64=db.query_row("SELECT count(*) FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE c.persona_id=?1 AND m.is_deleted=0",[p],|r|r.get(0))?;
    Ok(
        json!({"generator":"local_rules","execution_kind":"virtual","kind":kind,"memory_ids":memory_ids,"recorded_chat_count":chats,"verified_external_knowledge":false,"note":if kind=="create"{"完成虚拟创作构思时段；未生成或声称存在完整作品"}else{"仅回顾应用已有资料；未取得新网络内容"}}),
    )
}
