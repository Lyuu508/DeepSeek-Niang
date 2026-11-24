use crate::{Error, Result, storage::Store};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{MEMBERS, excerpt};

pub const DAY_MS: i64 = 86_400_000;
pub const BUBBLE_SEPARATOR: &str = "\n[气泡]\n";
pub const USER_FRAGMENT_SEPARATOR: &str = "\n[连续消息]\n";

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DialogueIntent {
    Casual,
    FollowUp,
    Emotional,
    Technical,
    Media,
}

impl DialogueIntent {
    pub fn detect(message: &str, visual_evidence: bool) -> Self {
        if visual_evidence {
            Self::Media
        } else if [
            "分析原因",
            "推导",
            "证明",
            "调试",
            "设计方案",
            "修复代码",
            "```",
            "SQL",
            "比较方案",
            "算法",
            "帮我写代码",
        ]
        .iter()
        .any(|c| message.contains(c))
        {
            Self::Technical
        } else if [
            "之前",
            "上次",
            "前几天",
            "昨晚",
            "昨天",
            "我们的约定",
            "还记得",
            "后来",
        ]
        .iter()
        .any(|c| message.contains(c))
        {
            Self::FollowUp
        } else if ["难过", "压力", "委屈", "焦虑", "今天好累", "累死了"]
            .iter()
            .any(|c| message.contains(c))
        {
            Self::Emotional
        } else {
            Self::Casual
        }
    }

    pub fn guidance(self) -> &'static str {
        match self {
            Self::Casual => {
                "适配聊天节奏，问候和玩笑通常一句到三句。不要机械复述、反复称呼或总以问题结尾。"
            }
            Self::FollowUp => {
                "有相符的记忆时自然延续。已完成的事用过去时，计划和事实分开；没有证据就坦白不确定。"
            }
            Self::Emotional => {
                "按当前需要倾听与回应。用自然语言，尊重边界；不强行撒娇、分析或提出任务。"
            }
            Self::Technical => {
                "保持专业和准确，可以完整解释与使用 Markdown、代码、表格。按需澄清条件。"
            }
            Self::Media => {
                "以已授权工具的具体观察为依据。用自己的人格表达答案，并指出重要不确定性。"
            }
        }
    }
}

pub fn plain_reply(content: &str) -> String {
    content.replace(BUBBLE_SEPARATOR, "\n")
}

pub struct ConversationContext {
    pub messages: Vec<crate::chat::PromptMessage>,
    pub intent: DialogueIntent,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    #[default]
    Active,
    Completed,
    Superseded,
    Dismissed,
}

impl MemoryState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Superseded => "superseded",
            Self::Dismissed => "dismissed",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct MemoryContext {
    pub is_event: bool,
    pub shared_story: bool,
    pub state: MemoryState,
    pub occurred_at_ms: i64,
    pub due_at_ms: Option<i64>,
    pub confidence_milli: i64,
    pub recall_allowed: bool,
    pub reminder_allowed: bool,
    pub shared_with_personas: bool,
}

impl Default for MemoryContext {
    fn default() -> Self {
        Self {
            is_event: false,
            shared_story: false,
            state: MemoryState::Active,
            occurred_at_ms: 0,
            due_at_ms: None,
            confidence_milli: 1000,
            recall_allowed: false,
            reminder_allowed: false,
            shared_with_personas: false,
        }
    }
}

pub(crate) fn context_row(r: &Row<'_>, first: usize) -> rusqlite::Result<MemoryContext> {
    let state = match r.get::<_, String>(first + 2)?.as_str() {
        "completed" => MemoryState::Completed,
        "superseded" => MemoryState::Superseded,
        "dismissed" => MemoryState::Dismissed,
        _ => MemoryState::Active,
    };
    Ok(MemoryContext {
        is_event: r.get(first)?,
        shared_story: r.get(first + 1)?,
        state,
        occurred_at_ms: r.get(first + 3)?,
        due_at_ms: r.get(first + 4)?,
        confidence_milli: r.get(first + 5)?,
        recall_allowed: r.get(first + 6)?,
        reminder_allowed: r.get(first + 7)?,
        shared_with_personas: r.get(first + 8)?,
    })
}

pub(crate) fn save_context(
    db: &Connection,
    memory_id: &str,
    persona: &str,
    context: &MemoryContext,
) -> Result<()> {
    if !(0..=1000).contains(&context.confidence_milli)
        || !(0..253_402_300_800_000).contains(&context.occurred_at_ms)
        || context
            .due_at_ms
            .is_some_and(|v| !(0..253_402_300_800_000).contains(&v))
        || !context.is_event && (context.due_at_ms.is_some() || context.reminder_allowed)
        || context.reminder_allowed && !context.recall_allowed
    {
        return Err(Error::Invalid("memory event context"));
    }
    db.execute(
        "INSERT INTO companion_memory_context VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(memory_id) DO UPDATE SET is_event=excluded.is_event,shared_story=excluded.shared_story,
         state=excluded.state,occurred_at_ms=excluded.occurred_at_ms,due_at_ms=excluded.due_at_ms,
         confidence_milli=excluded.confidence_milli,recall_allowed=excluded.recall_allowed,
         reminder_allowed=excluded.reminder_allowed,shared_with_personas=excluded.shared_with_personas",
        params![memory_id, persona, context.is_event, context.shared_story, context.state.as_str(),
            context.occurred_at_ms, context.due_at_ms, context.confidence_milli, context.recall_allowed,
            context.reminder_allowed, context.shared_with_personas],
    )?;
    Ok(())
}

pub(crate) fn safe_source(text: &str) -> bool {
    let lower = text.to_lowercase();
    ![
        "密码",
        "私钥",
        "api key",
        "api_key",
        "银行卡",
        "不要记住",
        "别记住",
        "保密",
        "pretend",
        "假设",
        "如果我",
        "举个例子",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

pub(crate) fn clear_statement(text: &str) -> bool {
    safe_source(text)
        && !['?', '？', '“', '”', '「', '」', '`']
            .iter()
            .any(|mark| text.contains(*mark))
}

pub(crate) fn observed_topic(text: &str) -> Option<&'static str> {
    for (terms, name) in [
        (&["睡", "熬夜", "作息", "早起"][..], "routine/sleep"),
        (&["设计", "改图", "画稿"][..], "plan/design"),
        (&["软件", "代码", "程序", "技术问题"][..], "plan/software"),
        (&["考试", "测试题", "备考"][..], "plan/exam"),
        (&["旅游", "旅行", "出游"][..], "plan/travel"),
        (&["面试", "求职"][..], "plan/interview"),
        (&["项目", "需求", "工作", "任务"][..], "plan/work"),
    ] {
        if terms.iter().any(|term| text.contains(term)) {
            return Some(name);
        }
    }
    None
}

pub(crate) fn query_aliases(topic: Option<&str>) -> &'static [&'static str] {
    match topic {
        Some("routine/sleep") => &["作息", "熬夜", "睡觉", "早点睡", "睡了"],
        Some("plan/design") => &["设计", "改图", "画稿"],
        Some("plan/software") => &["软件", "技术问题", "代码"],
        Some("plan/exam") => &["考试", "备考"],
        Some("plan/travel") => &["旅游", "旅行"],
        Some("plan/interview") => &["面试", "求职"],
        Some("plan/work") => &["项目", "需求", "任务"],
        _ => &[],
    }
}

pub(crate) fn explicit_correction(text: &str) -> bool {
    clear_statement(text)
        && [
            "已经改成",
            "现在改成",
            "现在已经",
            "更正一下",
            "现在不再",
            "现在我不喜欢",
            "现在我喜欢",
        ]
        .iter()
        .any(|value| text.contains(value))
}

pub(crate) fn completed(text: &str) -> bool {
    clear_statement(text)
        && !["还没", "没有", "没做完", "没解决", "如果", "准备"]
            .iter()
            .any(|v| text.contains(v))
        && [
            "已经完成",
            "已经做完",
            "做完了",
            "解决了",
            "考完了",
            "旅行结束",
        ]
        .iter()
        .any(|v| text.contains(v))
}

pub(crate) fn event_context(text: &str, at: i64, offset: i64) -> Option<MemoryContext> {
    if !clear_statement(text)
        || observed_topic(text).is_none()
        || !["我", "我们", "提醒我", "我们的约定"]
            .iter()
            .any(|v| text.starts_with(v))
        || ![
            "明天",
            "今晚",
            "今天",
            "后天",
            "下周",
            "正在",
            "计划",
            "准备",
            "打算",
            "约定",
            "尽量",
            "没做完",
            "还没",
            "提醒我",
            "以后",
        ]
        .iter()
        .any(|v| text.contains(v))
        || explicit_correction(text)
    {
        return None;
    }
    let midnight = (at + offset * 60_000).div_euclid(DAY_MS) * DAY_MS - offset * 60_000;
    let due = if text.contains("后天") {
        Some(midnight + 3 * DAY_MS - 1)
    } else if text.contains("明天") {
        Some(midnight + 2 * DAY_MS - 1)
    } else if text.contains("今晚") || text.contains("今天") {
        Some(midnight + DAY_MS - 1)
    } else {
        None
    };
    let reminder = text.contains("提醒我") || text.contains("到时候记得问我");
    Some(MemoryContext {
        is_event: true,
        occurred_at_ms: at,
        due_at_ms: due.filter(|v| *v >= at),
        confidence_milli: 900,
        recall_allowed: reminder || text.contains("我们的约定"),
        reminder_allowed: reminder,
        shared_story: text.contains("我们的约定"),
        ..Default::default()
    })
}

impl Store {
    pub fn remember_device_clock(&self, offset_minutes: i64, zone_name: &str) -> Result<()> {
        if !(-840..=840).contains(&offset_minutes)
            || !(1..=80).contains(&zone_name.len())
            || !zone_name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "/_-+".contains(ch))
        {
            return Err(Error::Invalid("device timezone"));
        }
        self.conn.execute("INSERT INTO companion_device_clock VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET offset_minutes=excluded.offset_minutes,zone_name=excluded.zone_name WHERE offset_minutes<>excluded.offset_minutes OR zone_name<>excluded.zone_name", params![offset_minutes, zone_name])?;
        Ok(())
    }

    pub(crate) fn clock_zone(&self, fallback_offset: i64) -> Result<(i64, String)> {
        Ok(self
            .conn
            .query_row(
                "SELECT offset_minutes,zone_name FROM companion_device_clock WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((fallback_offset, "configured-offset".into())))
    }

    pub(crate) fn time_evidence(&self, persona: &str, now: i64) -> Result<Value> {
        let (offset, zone) =
            self.clock_zone(self.companion_preferences(persona)?.timezone_offset_minutes)?;
        let shifted = now.saturating_add(offset * 60_000);
        let (today, clock, yesterday, tomorrow): (String, String, String, String) = self.conn.query_row(
            "SELECT date(?1/1000,'unixepoch'),time(?1/1000,'unixepoch'),date(?1/1000,'unixepoch','-1 day'),date(?1/1000,'unixepoch','+1 day')",
            [shifted], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let previous: Option<i64> = self.conn.query_row(&format!("SELECT created_at_ms FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE c.persona_id IN ({MEMBERS}) AND m.role='user' AND m.is_deleted=0 AND m.created_at_ms<=?2 ORDER BY m.created_at_ms DESC,m.created_lamport DESC LIMIT 1 OFFSET 1"), params![persona, now], |r| r.get(0)).optional()?.flatten();
        Ok(
            json!({"today":today,"local_time":clock,"zone":zone,"utc_offset_minutes":offset,
            "yesterday":yesterday,"tomorrow":tomorrow,"now_ms":now,
            "minutes_since_previous_user_message":previous.map(|v| now.saturating_sub(v)/60000)}),
        )
    }

    pub(crate) fn record_memory_use(&self, id: &str, now: i64) -> Result<()> {
        self.conn.execute("INSERT INTO companion_memory_usage VALUES(?1,?2) ON CONFLICT(memory_id) DO UPDATE SET last_used_at_ms=max(last_used_at_ms,excluded.last_used_at_ms)", params![id, now])?;
        Ok(())
    }

    pub fn set_persona_interest(&self, persona: &str, id: &str, weight: Option<i64>) -> Result<()> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        if weight.is_some_and(|v| !(0..=1000).contains(&v)) {
            return Err(Error::Invalid("interest weight"));
        }
        let owner: Option<String> = self
            .conn
            .query_row(
                "SELECT persona_id FROM character_interests WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        let owner = owner.ok_or(Error::Untrusted)?;
        if crate::pairing::resolve(&self.conn, &owner)?
            != crate::pairing::resolve(&self.conn, persona)?
        {
            return Err(Error::Untrusted);
        }
        if let Some(weight) = weight {
            self.conn.execute("INSERT INTO companion_interest_overrides VALUES(?1,?2,?3) ON CONFLICT(interest_id) DO UPDATE SET weight_milli=excluded.weight_milli", params![id,owner,weight])?;
        } else {
            self.conn.execute(
                "DELETE FROM companion_interest_overrides WHERE interest_id=?1",
                [id],
            )?;
        }
        Ok(())
    }

    pub(crate) fn persona_interest_evidence(&self, persona: &str) -> Result<Vec<Value>> {
        let mut rows = self.conn.prepare(&format!("SELECT i.topic,coalesce(o.weight_milli,i.weight_milli) AS weight FROM character_interests i LEFT JOIN companion_interest_overrides o ON o.interest_id=i.id WHERE i.persona_id IN ({MEMBERS}) AND weight>0 ORDER BY weight DESC,i.topic LIMIT 4"))?;
        Ok(rows
            .query_map([persona], |r| {
                let topic: String = r.get(0)?;
                let label = topic
                    .split_once(" · ")
                    .map_or(topic.as_str(), |(label, _)| label);
                Ok(json!({"topic":excerpt(label,30),"weight_milli":r.get::<_,i64>(1)?}))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
