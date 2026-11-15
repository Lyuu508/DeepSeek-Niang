use super::{CompanionPreferences, excerpt, stable_id};
use crate::{
    Error, Result,
    event::{Entity, Operation},
    storage::Store,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};

pub fn activity_label(kind: &str) -> &'static str {
    match kind {
        "rest" => "休息",
        "study" => "学习",
        "video" => "看视频",
        "music" => "听音乐",
        "organize" => "整理记忆",
        "recall" => "回忆往事",
        "share" => "分享发现",
        _ => "等待主人",
    }
}
fn next_kind(kind: &str) -> &'static str {
    match kind {
        "rest" => "study",
        "study" => "organize",
        "organize" => "music",
        "music" => "recall",
        "recall" => "video",
        "video" => "wait",
        "wait" => "share",
        _ => "rest",
    }
}
pub fn quiet(now: i64, prefs: &CompanionPreferences) -> bool {
    let minute = (now / 60000 + prefs.timezone_offset_minutes).rem_euclid(1440);
    let (start, end) = (prefs.quiet_start_minute, prefs.quiet_end_minute);
    if start == end {
        false
    } else if start < end {
        minute >= start && minute < end
    } else {
        minute >= start || minute < end
    }
}
pub fn proactive_limits(prefs: &CompanionPreferences, stage: &str) -> (i64, i64) {
    let minutes = match stage {
        "初识" => prefs.proactive_interval_minutes * 2,
        "熟悉" => prefs.proactive_interval_minutes * 3 / 2,
        _ => prefs.proactive_interval_minutes,
    };
    (
        minutes,
        if stage == "初识" {
            prefs.proactive_daily_limit.min(1)
        } else {
            prefs.proactive_daily_limit
        },
    )
}
fn clock(db: &Connection, device: &str) -> Result<i64> {
    db.execute(
        "UPDATE local_clock SET lamport=lamport+1 WHERE device_id=?1",
        [device],
    )?;
    Ok(db.query_row(
        "SELECT lamport FROM local_clock WHERE device_id=?1",
        [device],
        |r| r.get(0),
    )?)
}
fn event(
    db: &Connection,
    persona: &str,
    activity: &str,
    host: &str,
    kind: &str,
    description: &str,
    now: i64,
) -> Result<String> {
    let id = stable_id("life", &format!("{activity}/{kind}"));
    db.execute(
        "INSERT OR IGNORE INTO companion_life_events VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, persona, activity, host, kind, description, now],
    )?;
    Ok(id)
}
fn candidate(
    db: &Connection,
    persona: &str,
    activity: &str,
    host: &str,
    kind: &str,
    now: i64,
) -> Result<()> {
    let source = event(
        db,
        persona,
        activity,
        host,
        "completed",
        &format!("结束一段虚拟{}", activity_label(kind)),
        now,
    )?;
    db.execute("INSERT OR IGNORE INTO companion_proactive(id,persona_id,source_life_event_id,host_device_id,content,created_at_ms,state) VALUES(?1,?2,?3,?4,?5,?6,'candidate')",params![stable_id("proactive",&source),persona,source,host,format!("一段虚拟{}的安排已经结束。等你有空，我们可以接着聊聊最近的小事。",activity_label(kind)),now])?;
    Ok(())
}
fn deliver(
    tx: &Transaction<'_>,
    persona: &str,
    device: &str,
    prefs: &CompanionPreferences,
    now: i64,
    stage: &str,
) -> Result<usize> {
    let (interval, daily_limit) = proactive_limits(prefs, stage);
    if !prefs.proactive_enabled || quiet(now, prefs) {
        return Ok(0);
    }
    let last:Option<i64>=tx.query_row("SELECT max(delivered_at_ms) FROM companion_proactive WHERE persona_id=?1 AND state='delivered'",[persona],|r|r.get(0))?;
    if last.is_some_and(|last| now.saturating_sub(last) < interval * 60000) {
        return Ok(0);
    }
    let start = (now + prefs.timezone_offset_minutes * 60000).div_euclid(86400000) * 86400000
        - prefs.timezone_offset_minutes * 60000;
    let count:i64=tx.query_row("SELECT count(*) FROM companion_proactive WHERE persona_id=?1 AND delivered_at_ms>=?2 AND delivered_at_ms<?3",params![persona,start,start+86400000],|r|r.get(0))?;
    if count >= daily_limit {
        return Ok(0);
    }
    let item:Option<(String,String)>=tx.query_row("SELECT id,content FROM companion_proactive WHERE persona_id=?1 AND state='candidate' ORDER BY created_at_ms,id LIMIT 1",[persona],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let Some((id, mut content)) = item else {
        return Ok(0);
    };
    let memory:Option<String>=tx.query_row("SELECT m.text FROM memories m JOIN companion_memory_meta d ON d.memory_id=m.id WHERE m.persona_id=?1 AND m.is_deleted=0 AND m.sensitivity='api_allowed' AND d.pinned=1 ORDER BY m.importance_milli DESC,m.id LIMIT 1",[persona],|r|r.get(0)).optional()?;
    if let Some(memory) = memory {
        content = format!(
            "想起我们记下的一件事：「{}」。你有空时，想听听你的近况。",
            excerpt(&memory, 80)
        );
    }
    let (profile, conversation): (String, String) = tx.query_row(
        "SELECT profile_id,main_conversation_id FROM personas WHERE id=?1",
        [persona],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let message = stable_id("proactive-chat", &id);
    Store::append_in(
        tx,
        device,
        &profile,
        Entity::Message,
        &message,
        Operation::Put,
        json!({"persona_id":persona,"conversation_id":conversation,"role":"assistant","content":content,"created_at_ms":now}),
    )?;
    tx.execute("UPDATE companion_proactive SET state='delivered',content=?1,delivered_at_ms=?2,message_id=?3 WHERE id=?4",params![content,now,message,id])?;
    Ok(1)
}
impl Store {
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
    /// Called only by the Windows native runtime while the application is alive.
    /// It never catches up fictional external work after a process or heartbeat gap.
    pub fn advance_companion_life(&mut self, now: i64, windows_host: bool) -> Result<usize> {
        if !windows_host {
            return Ok(0);
        }
        if !(0..crate::event::MAX_CLOCK - 86400000).contains(&now) {
            return Err(Error::Invalid("life time"));
        }
        let mut deliveries = 0;
        for persona in self.personas()? {
            let prefs = self.companion_preferences(&persona.id)?;
            let stage = self.bond_snapshot(&persona.id)?.stage;
            let previous:Option<(String,String,i64,String)>=self.conn.query_row("SELECT device_id,boot_id,heartbeat_at_ms,next_activity FROM companion_hosts WHERE persona_id=?1",[&persona.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
            if previous.as_ref().is_some_and(|v| v.0 != self.device_id) {
                continue;
            }
            if previous
                .as_ref()
                .is_some_and(|v| v.1 == self.boot_id && now.saturating_sub(v.2) < 30000)
            {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute(
                "INSERT OR IGNORE INTO companion_hosts VALUES(?1,?2,?3,?4,'rest')",
                params![persona.id, self.device_id, self.boot_id, now],
            )?;
            if !prefs.proactive_enabled {
                tx.execute("UPDATE companion_proactive SET state='dismissed' WHERE persona_id=?1 AND host_device_id=?2 AND state='candidate'",params![persona.id,self.device_id])?;
            }
            let running:Option<(String,String,i64,i64)>=tx.query_row("SELECT id,kind,started_at_ms,scheduled_end_at_ms FROM companion_activities WHERE persona_id=?1 AND state='running'",[&persona.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
            let gap = previous
                .as_ref()
                .is_some_and(|v| v.1 != self.boot_id || now.saturating_sub(v.2) > 90000);
            let mut ongoing = running.is_some();
            let mut next = previous
                .as_ref()
                .map(|v| v.3.clone())
                .unwrap_or_else(|| "rest".into());
            if let Some((id, kind, start, end)) = running {
                if gap || !prefs.life_enabled {
                    let paused = previous
                        .as_ref()
                        .map(|v| v.2)
                        .unwrap_or(now)
                        .max(start)
                        .min(now);
                    tx.execute("UPDATE companion_activities SET state='paused',ended_at_ms=?1,updated_at_ms=?2 WHERE id=?3",params![paused,now,id])?;
                    event(
                        &tx,
                        &persona.id,
                        &id,
                        &self.device_id,
                        "paused",
                        "应用离线或日常已关闭；保留暂停记录，未补算观看或学习",
                        now,
                    )?;
                    ongoing = false;
                } else if now >= end {
                    tx.execute("UPDATE companion_activities SET state='completed',ended_at_ms=?1,updated_at_ms=?1 WHERE id=?2",params![now,id])?;
                    event(
                        &tx,
                        &persona.id,
                        &id,
                        &self.device_id,
                        "completed",
                        &format!("结束一段虚拟{}", activity_label(&kind)),
                        now,
                    )?;
                    if prefs.proactive_enabled {
                        candidate(&tx, &persona.id, &id, &self.device_id, &kind, now)?;
                    }
                    next = next_kind(&kind).into();
                    ongoing = false;
                } else {
                    tx.execute(
                        "UPDATE companion_activities SET updated_at_ms=?1 WHERE id=?2",
                        params![now, id],
                    )?;
                }
            }
            if prefs.life_enabled && !ongoing {
                let kind = if prefs.preferred_activity == "auto" {
                    next.clone()
                } else {
                    prefs.preferred_activity.clone()
                };
                let id = uuid::Uuid::new_v4().to_string();
                let duration = match kind.as_str() {
                    "share" => 5,
                    "organize" => 8,
                    "wait" => 10,
                    "rest" => 15,
                    _ => 20,
                } * 60000;
                tx.execute("INSERT INTO companion_activities(id,persona_id,host_device_id,kind,state,started_at_ms,scheduled_end_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,'running',?5,?6,?5)",params![id,persona.id,self.device_id,kind,now,now+duration])?;
                event(
                    &tx,
                    &persona.id,
                    &id,
                    &self.device_id,
                    "started",
                    &format!(
                        "开始虚拟{}；不代表真实联网、媒体播放或已获取新知识",
                        activity_label(&kind)
                    ),
                    now,
                )?;
                let mood = match kind.as_str() {
                    "study" | "organize" => "curious",
                    "rest" => "tired",
                    "recall" => "missing",
                    "share" => "expectant",
                    _ => "calm",
                };
                let conversational_mood:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM companion_mood_events WHERE persona_id IN (SELECT alias_id FROM persona_links WHERE canonical_id=?1 UNION SELECT ?1) AND source_message_id IS NOT NULL AND started_at_ms<=?2 AND expires_at_ms>?2)",params![persona.id,now],|r|r.get(0))?;
                if !conversational_mood {
                    tx.execute("INSERT INTO companion_mood_events(id,persona_id,origin_device_id,activity_id,mood,intensity_milli,started_at_ms,expires_at_ms,lamport) VALUES(?1,?2,?3,?4,?5,300,?6,?7,?8)",params![stable_id("life-mood",&id),persona.id,self.device_id,id,mood,now,now+duration,clock(&tx,&self.device_id)?])?;
                }
                next = next_kind(&kind).into();
            }
            tx.execute("INSERT INTO companion_hosts VALUES(?1,?2,?3,?4,?5) ON CONFLICT(persona_id) DO UPDATE SET boot_id=excluded.boot_id,heartbeat_at_ms=excluded.heartbeat_at_ms,next_activity=excluded.next_activity",params![persona.id,self.device_id,self.boot_id,now,next])?;
            deliveries += deliver(&tx, &persona.id, &self.device_id, &prefs, now, &stage)?;
            tx.commit()?;
        }
        Ok(deliveries)
    }
    pub fn life_snapshot(&self, persona: &str, now: i64) -> Result<Value> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let prefs = self.companion_preferences(persona)?;
        let limits = proactive_limits(&prefs, &self.bond_snapshot(persona)?.stage);
        let host:Option<(String,i64,String)>=self.conn.query_row("SELECT device_id,heartbeat_at_ms,next_activity FROM companion_hosts WHERE persona_id=?1",[persona],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let current:Option<Value>=self.conn.query_row("SELECT id,kind,state,started_at_ms,scheduled_end_at_ms,updated_at_ms FROM companion_activities WHERE persona_id=?1 ORDER BY started_at_ms DESC,id DESC LIMIT 1",[persona],|r|{let kind=r.get::<_,String>(1)?;Ok(json!({"id":r.get::<_,String>(0)?,"kind":kind,"label":format!("虚拟{}",activity_label(&kind)),"state":r.get::<_,String>(2)?,"started_at_ms":r.get::<_,i64>(3)?,"scheduled_end_at_ms":r.get::<_,i64>(4)?,"updated_at_ms":r.get::<_,i64>(5)?,"execution_kind":"virtual"}))}).optional()?;
        let start = (now + prefs.timezone_offset_minutes * 60000).div_euclid(86400000) * 86400000
            - prefs.timezone_offset_minutes * 60000;
        let mut query=self.conn.prepare("SELECT id,description,occurred_at_ms,kind FROM companion_life_events WHERE persona_id=?1 AND occurred_at_ms>=?2 AND occurred_at_ms<?3 ORDER BY occurred_at_ms DESC,id DESC LIMIT 100")?;
        let events=query.query_map(params![persona,start,start+86400000],|r|Ok(json!({"id":r.get::<_,String>(0)?,"description":r.get::<_,String>(1)?,"occurred_at_ms":r.get::<_,i64>(2)?,"kind":r.get::<_,String>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut query=self.conn.prepare("SELECT id,content,created_at_ms,delivered_at_ms,state,message_id FROM companion_proactive WHERE persona_id=?1 ORDER BY created_at_ms DESC,id DESC LIMIT 20")?;
        let proactive=query.query_map([persona],|r|Ok(json!({"id":r.get::<_,String>(0)?,"content":r.get::<_,String>(1)?,"created_at_ms":r.get::<_,i64>(2)?,"delivered_at_ms":r.get::<_,Option<i64>>(3)?,"state":r.get::<_,String>(4)?,"message_id":r.get::<_,Option<String>>(5)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(
            json!({"effective_proactive_interval_minutes":limits.0,"effective_proactive_daily_limit":limits.1,"host_device_id":host.as_ref().map(|v|&v.0),"heartbeat_at_ms":host.as_ref().map(|v|v.1),"host_online":host.as_ref().is_some_and(|v|now>=v.1&&now-v.1<=90000),"current":current,"next_activity":host.as_ref().map(|v|activity_label(&v.2)),"today_events":events,"proactive_messages":proactive,"day_start_ms":start,"execution_kind":"virtual","external_media_executed":false}),
        )
    }
}
