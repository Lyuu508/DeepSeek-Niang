use super::{MEMBERS, excerpt, stable_id};
use crate::{Result, storage::Store};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct BondHistory {
    pub id: String,
    pub source_message_id: String,
    pub kind: String,
    pub explanation: String,
    pub occurred_at_ms: i64,
    pub delta: [i32; 5],
    pub applied: bool,
    pub decision: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct BondSnapshot {
    pub familiarity: i32,
    pub trust: i32,
    pub affection: i32,
    pub intimacy: i32,
    pub comfort: i32,
    pub stage: String,
    pub rule_version: u32,
    pub applied_events: usize,
    pub distinct_days: usize,
    pub history: Vec<BondHistory>,
}
#[derive(Debug, Clone, Serialize)]
pub struct MoodSnapshot {
    pub kind: String,
    pub label: String,
    pub intensity_milli: i64,
    pub source: String,
    pub started_at_ms: Option<i64>,
    pub expires_at_ms: Option<i64>,
}
pub fn style(stage: &str) -> &'static str {
    match stage {
        "熟悉" => "沿用主人的称呼，自然提及已核实的偏好，适度关心，不假装了解未知经历",
        "信任" => "更坦诚温暖地回应，共同约定可作为关心依据，给建议前尊重主人的选择",
        "亲密" => "在双方愿意时可轻声撒娇、表达想念和更细致的关心，仍保留意见与边界",
        "深度陪伴" => "以稳定熟悉的陪伴语气回忆有来源的共同经历，减少机械客套，尊重独立生活",
        _ => "初次相识的自然温柔语气，使用已配置称呼，克制亲昵，不假装已有共同经历",
    }
}
pub fn mood_label(kind: &str) -> &'static str {
    match kind {
        "happy" => "开心",
        "shy" => "害羞",
        "expectant" => "期待",
        "curious" => "好奇",
        "hurt" => "委屈",
        "sad" => "失落",
        "angry" => "生气",
        "tired" => "疲惫",
        "excited" => "兴奋",
        "missing" => "想念",
        _ => "平静",
    }
}
pub fn mood_style(kind: &str) -> &'static str {
    match kind {
        "happy" | "excited" => "表达轻快一些，可以适量表情符号，避免过度热情",
        "shy" => "语气稍含蓄、简短，亲昵取决于关系阶段，不强行暧昧",
        "expectant" | "curious" => "适当追问一件相关的小事，保持好奇而不连续盘问",
        "hurt" | "sad" => "用温和直接的话解释感受，不以委屈要求主人安慰或延长聊天",
        "angry" => "平静陈述边界与分歧，不辱骂、不威胁，允许自然平复",
        "tired" => "回复可以简短舒缓，说明是虚拟状态，不冒充身体病症",
        "missing" => "可以轻轻表达想起共同经历，不责怪主人离开、不制造内疚",
        _ => "自然平和，回复长度按话题决定，不随机变换情绪",
    }
}
fn evidence(text: &str) -> Option<(&'static str, &'static str)> {
    if text.contains(char::from(96)) || text.contains("假装") || text.contains("举个例子") {
        return None;
    }
    if text.contains("约定没能做到") || text.contains("没有兑现我们的约定") {
        Some((
            "broken_promise",
            "主人自述未兑现约定；小幅调整信任，不作失联惩罚",
        ))
    } else if text.contains("你这样让我很难过") || text.contains("我对你很失望") {
        Some(("disappointment", "主人明确表达不愉快，保留分歧与边界"))
    } else if text.contains("我兑现了我们的约定") || text.contains("答应你的事我做到了")
    {
        Some(("promise_kept", "主人自述兑现约定，未将其冒充外部核验"))
    } else if text.contains("谢谢你一直陪") || text.contains("你也要好好休息") {
        Some(("care", "主人明确表达持续关心"))
    } else if text.contains("还记得我们一起") || text.contains("记得我们上次") {
        Some((
            "recalled_experience",
            "主人主动回忆共同交流；具体事实由记忆来源核对",
        ))
    } else if text.contains("我们和好吧") || text.contains("刚才误会你了") {
        Some(("reconciliation", "主人明确提出和解"))
    } else if text.starts_with("今天我") && text.chars().count() >= 15 {
        Some((
            "shared_day",
            "主人分享具体日常，自述经历而非已验证的外部事实",
        ))
    } else {
        None
    }
}
pub(crate) fn evidence_kind(text: &str) -> Option<&'static str> {
    evidence(text).map(|v| v.0)
}
pub(crate) fn inferred_mood(text: &str) -> Option<&'static str> {
    if text.contains("你这样让我很难过") {
        Some("hurt")
    } else if text.contains("我对你很失望") {
        Some("sad")
    } else if text.contains("不要再越过我的边界") {
        Some("angry")
    } else if text.contains("你也要好好休息") {
        Some("tired")
    } else if text.contains("谢谢你一直陪") {
        Some("happy")
    } else if text.contains("好喜欢你") {
        Some("shy")
    } else if text.contains("下次我们一起") {
        Some("expectant")
    } else if text.contains("一起研究一下") {
        Some("curious")
    } else if text.contains("太棒了我们做到了") {
        Some("excited")
    } else if text.contains("好久没聊，想你了") {
        Some("missing")
    } else if text.contains("我们和好吧") {
        Some("calm")
    } else {
        None
    }
}
pub(crate) fn record(
    db: &Connection,
    persona: &str,
    device: &str,
    id: &str,
    text: &str,
    at: i64,
    clock: i64,
) -> Result<()> {
    if let Some((kind, explanation)) = evidence(text) {
        db.execute("INSERT OR IGNORE INTO companion_bond_events(id,persona_id,origin_device_id,source_message_id,kind,topic,explanation,occurred_at_ms,lamport) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![stable_id("bond",&format!("{id}/{kind}")),persona,device,id,kind,excerpt(text,80),explanation,at,clock])?;
    }
    if let Some(mood) = inferred_mood(text) {
        db.execute("INSERT OR IGNORE INTO companion_mood_events(id,persona_id,origin_device_id,source_message_id,mood,intensity_milli,started_at_ms,expires_at_ms,lamport) VALUES(?1,?2,?3,?4,?5,600,?6,?7,?8)",
            params![stable_id("mood",id),persona,device,id,mood,at,at.saturating_add(30*60000),clock])?;
    }
    Ok(())
}
fn stage(values: [i32; 5], current: &str, days: usize, events: usize) -> String {
    let [f, t, a, i, c] = values;
    if current == "深度陪伴" && t >= 55000 && i >= 50000 {
        return current.into();
    }
    if f >= 65000 && t >= 65000 && i >= 60000 && days >= 60 && events >= 60 {
        return "深度陪伴".into();
    }
    if current == "亲密" && t >= 30000 && i >= 24000 && c >= 26000 {
        return current.into();
    }
    if f >= 30000 && t >= 40000 && a >= 45000 && i >= 30000 && days >= 21 {
        return "亲密".into();
    }
    if current == "信任" && t >= 20000 && c >= 27000 {
        return current.into();
    }
    if t >= 24000 && c >= 35000 && days >= 7 && events >= 10 {
        return "信任".into();
    }
    if current == "熟悉" && f >= 10000 {
        return current.into();
    }
    if f >= 12000 && days >= 3 && events >= 4 {
        return "熟悉".into();
    }
    "初识".into()
}
impl Store {
    pub fn bond_snapshot(&self, persona: &str) -> Result<BondSnapshot> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let raw: String = self.conn.query_row(
            "SELECT relationship_json FROM persona_states WHERE persona_id=?1",
            [persona],
            |r| r.get(0),
        )?;
        let legacy: serde_json::Value = serde_json::from_str(&raw)?;
        let mut values = [5000, 10000, 10000, 0, 30000];
        for (i, key) in [
            "familiarity_milli",
            "trust_milli",
            "affection_milli",
            "intimacy_milli",
            "comfort_milli",
        ]
        .iter()
        .enumerate()
        {
            if let Some(n) = legacy[key].as_i64() {
                values[i] = n.clamp(0, 100000) as i32;
            }
        }
        if let Some(d) = legacy["dimensions"].as_array() {
            for (target, source) in [(2, 0), (1, 1), (3, 2), (4, 3)] {
                if let Some(n) = d.get(source).and_then(|v| v.as_i64()) {
                    values[target] = n.clamp(0, 100000) as i32;
                }
            }
        }
        let mut current = legacy["stage"]
            .as_str()
            .filter(|s| matches!(*s, "初识" | "熟悉" | "信任" | "亲密" | "深度陪伴"))
            .unwrap_or("初识")
            .to_owned();
        let mut query=self.conn.prepare(&format!("SELECT id,source_message_id,kind,explanation,occurred_at_ms FROM companion_bond_events WHERE persona_id IN ({MEMBERS}) ORDER BY lamport,origin_device_id,id LIMIT 100000"))?;
        let rows = query
            .query_map([persona], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut history = Vec::new();
        let mut cooldown = BTreeMap::<String, i64>::new();
        let mut daily = BTreeMap::<i64, i32>::new();
        let mut repeats = BTreeMap::<(i64, String), i32>::new();
        let mut days = BTreeSet::new();
        let mut applied = 0;
        let mut seen = BTreeSet::new();
        for (id, source, kind, explanation, at) in rows {
            let day = at / 86400000;
            let base = match kind.as_str() {
                "care" => [150, 200, 300, 50, 350],
                "shared_day" => [250, 100, 100, 50, 200],
                "recalled_experience" => [250, 350, 250, 100, 200],
                "promise_kept" => [100, 500, 250, 100, 250],
                "reconciliation" => [100, 300, 100, 50, 400],
                "disappointment" => [0, -250, -150, -50, -500],
                "broken_promise" => [0, -650, -200, -100, -300],
                _ => [0; 5],
            };
            let mut delta = [0; 5];
            let mut decision = "已应用缓慢变化".to_owned();
            let duplicate = !seen.insert((source.clone(), kind.clone()));
            if duplicate {
                decision = "同一证据已计入".into();
            } else if cooldown
                .get(&kind)
                .is_some_and(|last| at.saturating_sub(*last) < 30 * 60000)
            {
                decision = "同类事件冷却中".into();
            } else {
                let used = daily.entry(day).or_default();
                let count = repeats.entry((day, kind.clone())).or_default();
                let factor = 1 + (*count).min(4);
                let remaining = (1500 - *used).max(0);
                for (d, b) in delta.iter_mut().zip(base) {
                    *d = (b / factor).clamp(-remaining, remaining);
                }
                let cost = delta.iter().map(|d| d.abs()).max().unwrap_or(0);
                if cost == 0 {
                    decision = "已达到当日变化上限".into();
                } else {
                    let before = values;
                    for (v, d) in values.iter_mut().zip(delta) {
                        *v = (*v + d).clamp(0, 100000);
                    }
                    delta = std::array::from_fn(|i| values[i] - before[i]);
                    *used += cost;
                    *count += 1;
                    cooldown.insert(kind.clone(), at);
                    days.insert(day);
                    applied += 1;
                    current = stage(values, &current, days.len(), applied);
                }
            }
            history.push(BondHistory {
                id,
                source_message_id: source,
                kind,
                explanation,
                occurred_at_ms: at,
                delta,
                applied: delta.iter().any(|d| *d != 0),
                decision,
            });
        }
        history.reverse();
        history.truncate(100);
        Ok(BondSnapshot {
            familiarity: values[0],
            trust: values[1],
            affection: values[2],
            intimacy: values[3],
            comfort: values[4],
            stage: current,
            rule_version: 2,
            applied_events: applied,
            distinct_days: days.len(),
            history,
        })
    }
    pub fn current_mood(&self, persona: &str, now: i64) -> Result<MoodSnapshot> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let row:Option<(String,i64,i64,i64,bool)>=self.conn.query_row(&format!("SELECT mood,intensity_milli,started_at_ms,expires_at_ms,source_message_id IS NOT NULL FROM companion_mood_events WHERE persona_id IN ({MEMBERS}) ORDER BY lamport DESC,origin_device_id DESC,id DESC LIMIT 1"),[persona],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        if let Some((kind, strength, start, end, chat)) = row
            && now >= start
            && now < end
        {
            let relation = self.bond_snapshot(persona)?;
            let factor = if matches!(kind.as_str(), "angry" | "hurt" | "sad") {
                100 - (relation.trust / 10000).min(10)
            } else {
                80 + (relation.affection / 5000).min(20)
            };
            let intensity = strength * (end - now) / (end - start).max(1) * i64::from(factor) / 100;
            return Ok(MoodSnapshot {
                label: mood_label(&kind).into(),
                kind,
                intensity_milli: intensity,
                source: if chat {
                    "最近交流的规则推断"
                } else {
                    "有记录的虚拟生活事件"
                }
                .into(),
                started_at_ms: Some(start),
                expires_at_ms: Some(end),
            });
        }
        Ok(MoodSnapshot {
            kind: "calm".into(),
            label: "平静".into(),
            intensity_milli: 200,
            source: "初始状态或情绪自然平复".into(),
            started_at_ms: None,
            expires_at_ms: None,
        })
    }
}
