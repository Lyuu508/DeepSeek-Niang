//! Durable chat lifecycle. Final messages still use the existing v1 event journal.
use crate::{
    Error, Result,
    event::{Entity, Operation, sha256_hex},
    storage::Store,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub const MAX_MESSAGE_BYTES: usize = 32_000;
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChatSettings {
    #[serde(default)]
    pub dialogue: crate::dialogue::Dialogue,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub history_turns: u32,
    pub timeout_seconds: u32,
    pub include_history: bool,
    pub character_name: String,
    pub user_title: String,
    pub personality: String,
}
impl Default for ChatSettings {
    fn default() -> Self {
        Self{dialogue:Default::default(),model:"deepseek-flash".into(),temperature:0.8,max_tokens:2048,history_turns:20,timeout_seconds:120,include_history:true,character_name:"DeepSeek娘".into(),user_title:"主人".into(),personality:"聪明、温柔、好奇、有幽默感，偶尔调皮。可以自然使用中文互联网流行语和适度吐槽，但不机械撒娇。保持稳定自然的表达，尊重用户的时间和选择。".into()}
    }
}
impl ChatSettings {
    pub fn validate(&self) -> Result<()> {
        self.dialogue.validate()?;
        if !matches!(self.model.as_str(), "deepseek-flash" | "deepseek-v4-pro")
            || !self.temperature.is_finite()
            || !(0.0..=2.0).contains(&self.temperature)
            || !(256..=8192).contains(&self.max_tokens)
            || !(1..=40).contains(&self.history_turns)
            || !(15..=300).contains(&self.timeout_seconds)
        {
            return Err(Error::Invalid("chat settings range"));
        }
        for (s, max) in [
            (&self.character_name, 100),
            (&self.user_title, 100),
            (&self.personality, 12_000),
        ] {
            if s.trim().is_empty() || s.len() > max {
                return Err(Error::Invalid("character text length"));
            }
        }
        Ok(())
    }
    pub fn system_prompt(&self, injection: &ContextInjection) -> String {
        let mut s = format!(
            "你是用户正在使用的 AI 陪伴角色。人物配置：{}。保持人物风格一致，用自然中文交流。没有可核验的活动结果时，不声称正在浏览网络、看视频或在应用关闭时完成真实任务。不要通过威胁、惩罚或愧疚要求用户保持互动。不要编造长期记忆或关系分数。",
            json!({"name":self.character_name,"称呼用户":self.user_title,"personality":self.personality})
        );
        if let Some(v) = &injection.relationship {
            s.push_str(&format!("\n应用提供的关系状态：{v}"));
        }
        if let Some(v) = &injection.mood {
            s.push_str(&format!("\n应用提供的短期情绪：{v}"));
        }
        self.dialogue.append_style(&mut s);
        s
    }
}
/// Future verified relationship/mood adapter. No fabricated default score.
#[derive(Debug, Default, Clone)]
pub struct ContextInjection {
    pub relationship: Option<String>,
    pub mood: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at_ms: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub message_count: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub state: String,
    pub created_at_ms: Option<i64>,
    pub model_id: Option<String>,
    pub request_id: Option<String>,
    pub user_message_id: Option<String>,
    pub revision: i64,
    pub selected: bool,
    pub error_code: Option<String>,
    pub finish_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartChat {
    pub request_id: String,
    pub conversation_id: String,
    pub text: Option<String>,
    pub regenerate_user_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedChat {
    pub request_id: String,
    pub conversation_id: String,
    pub user_message_id: String,
    pub assistant_id: String,
    pub is_new: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}
impl TokenUsage {
    pub fn validate(&self) -> Result<()> {
        if self.prompt_tokens < 0
            || self.completion_tokens < 0
            || self.total_tokens < 0
            || self.total_tokens > 1_000_000_000
            || self.prompt_tokens.checked_add(self.completion_tokens) != Some(self.total_tokens)
        {
            return Err(Error::Invalid("token usage"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub requests: i64,
    pub measured_requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
}

fn uuid(value: &str) -> Result<()> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| Error::Invalid("UUID required"))
}
impl Store {
    pub fn chat_settings(&self) -> Result<ChatSettings> {
        self.persona_settings(&self.active_persona_id()?)
    }
    pub fn save_chat_settings(&mut self, value: &ChatSettings) -> Result<()> {
        self.save_persona_settings(&self.active_persona_id()?, value)
    }
    pub fn create_conversation(&mut self, title: &str) -> Result<Conversation> {
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(Error::Invalid("conversation title"));
        }
        let id = Uuid::new_v4().to_string();
        let now = now_ms();
        self.conn.execute("INSERT INTO conversations(id,profile_id,title,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,?4)",params![id,self.profile_id,title,now])?;
        self.conn.execute("INSERT INTO conversation_personas(conversation_id,persona_id,is_legacy) VALUES(?1,?2,1)",params![id,self.profile_id])?;
        Ok(Conversation {
            id,
            title: title.into(),
            created_at_ms: Some(now),
            updated_at_ms: Some(now),
            message_count: 0,
        })
    }
    pub fn conversations(&self) -> Result<Vec<Conversation>> {
        let mut stmt=self.conn.prepare("SELECT c.id,c.title,c.created_at_ms,c.updated_at_ms,(SELECT count(*) FROM messages m WHERE m.conversation_id=c.id AND m.is_deleted=0) FROM conversations c WHERE (c.profile_id=?1 OR c.profile_id IN (SELECT profile_id FROM lan_profiles)) ORDER BY coalesce(c.updated_at_ms,0) DESC,c.id LIMIT 200")?;
        Ok(stmt
            .query_map([&self.profile_id], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    created_at_ms: r.get(2)?,
                    updated_at_ms: r.get(3)?,
                    message_count: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?)
    }
    pub fn rename_conversation(&mut self, id: &str, title: &str) -> Result<()> {
        uuid(id)?;
        let title = title.trim();
        if title.is_empty() || title.len() > 200 {
            return Err(Error::Invalid("conversation title"));
        }
        if self.conn.execute(
            "UPDATE conversations SET title=?1,revision=revision+1 WHERE id=?2 AND (profile_id=?3 OR profile_id IN (SELECT profile_id FROM lan_profiles))",
            params![title, id, self.profile_id],
        )? != 1
        {
            return Err(Error::Invalid("conversation not found"));
        }
        Ok(())
    }
    pub fn begin_chat(
        &mut self,
        input: &StartChat,
        settings: &ChatSettings,
    ) -> Result<PreparedChat> {
        self.begin_chat_with_image(input, settings, None)
    }
    pub fn begin_chat_with_image(
        &mut self,
        input: &StartChat,
        settings: &ChatSettings,
        image: Option<&crate::chat_images::ChatImage>,
    ) -> Result<PreparedChat> {
        uuid(&input.request_id)?;
        uuid(&input.conversation_id)?;
        settings.validate()?;
        if input.regenerate_user_id.is_some() == input.text.is_some() {
            return Err(Error::Invalid("send or regenerate required"));
        }
        if let Some(text) = &input.text
            && (text.trim().is_empty() || text.len() > MAX_MESSAGE_BYTES)
        {
            return Err(Error::Invalid("message length"));
        }
        if let Some(id) = &input.regenerate_user_id {
            uuid(id)?;
        }
        if let Some(image) = image {
            if input.regenerate_user_id.is_some() {
                return Err(Error::Invalid("CHAT_IMAGE_INVALID"));
            }
            image.validate()?;
        }
        let data_profile = self.conversation_profile(&input.conversation_id)?;
        let input_hash = if let Some(image) = image {
            sha256_hex(&serde_json::to_vec(&(input, image))?)
        } else {
            sha256_hex(&serde_json::to_vec(input)?)
        };
        let latest_user = if input.regenerate_user_id.is_some() {
            self.chat_messages(&input.conversation_id)?
                .into_iter()
                .rev()
                .find(|m| m.role == "user")
                .map(|m| m.id)
        } else {
            None
        };
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing:Option<(String,String,String,String)>=tx.query_row("SELECT input_hash,conversation_id,user_message_id,assistant_message_id FROM chat_requests WHERE id=?1",[&input.request_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        if let Some((hash, conversation, user_id, assistant_id)) = existing {
            if hash != input_hash {
                return Err(Error::Conflict);
            }
            return Ok(PreparedChat {
                request_id: input.request_id.clone(),
                conversation_id: conversation,
                user_message_id: user_id,
                assistant_id,
                is_new: false,
            });
        }
        let title: Option<String> = tx
            .query_row(
                "SELECT title FROM conversations WHERE id=?1 AND (profile_id=?2 OR profile_id IN (SELECT profile_id FROM lan_profiles))",
                params![input.conversation_id, self.profile_id],
                |r| r.get(0),
            )
            .optional()?;
        let title = title.ok_or(Error::Invalid("conversation not found"))?;
        let running: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM chat_requests WHERE state IN ('sending','generating'))",
            [],
            |r| r.get(0),
        )?;
        if running {
            return Err(Error::Invalid("generation already active"));
        }
        let now = now_ms();
        let assistant_id = Uuid::new_v4().to_string();
        let user_id;
        if let Some(target) = &input.regenerate_user_id {
            let last = latest_user;
            if last.as_ref() != Some(target) {
                return Err(Error::Invalid("only latest user turn can regenerate"));
            }
            user_id = target.clone();
        } else {
            user_id = Uuid::new_v4().to_string();
            Self::append_in(
                &tx,
                &self.device_id,
                &data_profile,
                Entity::Message,
                &user_id,
                Operation::Put,
                json!({"role":"user","content":input.text.as_deref().unwrap_or(""),"conversation_id":input.conversation_id,"conversation_title":title,"created_at_ms":now,"request_id":input.request_id}),
            )?;
            if let Some(image) = image {
                crate::chat_images::attach(
                    &tx,
                    &self.profile_id,
                    &self.device_id,
                    &user_id,
                    image,
                )?;
            }
            // First real message gives an untitled conversation a useful, bounded label.
            if title == "新对话" || title == "默认对话" {
                let short: String = input
                    .text
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(18)
                    .collect();
                tx.execute(
                    "UPDATE conversations SET title=?1 WHERE id=?2",
                    params![short, input.conversation_id],
                )?;
            }
        }
        let previous: Option<String> = tx
            .query_row(
                "SELECT selected_generation_id FROM chat_turn_choices WHERE user_message_id=?1",
                [&user_id],
                |r| r.get(0),
            )
            .optional()?;
        tx.execute("INSERT INTO chat_requests(id,conversation_id,user_message_id,assistant_message_id,input_hash,mode,state,model_id,created_at_ms) VALUES(?1,?2,?3,?4,?5,?6,'sending',?7,?8)",params![input.request_id,input.conversation_id,user_id,assistant_id,input_hash,if input.regenerate_user_id.is_some(){"regenerate"}else{"send"},settings.model,now])?;
        tx.execute("INSERT INTO chat_generations(id,request_id,user_message_id,state,model_id,created_at_ms,supersedes_generation_id) VALUES(?1,?2,?3,'sending',?4,?5,?6)",params![assistant_id,input.request_id,user_id,settings.model,now,previous])?;
        tx.execute("INSERT INTO chat_turn_choices(user_message_id,selected_generation_id) VALUES(?1,?2) ON CONFLICT(user_message_id) DO UPDATE SET selected_generation_id=excluded.selected_generation_id",params![user_id,assistant_id])?;
        tx.execute(
            "UPDATE conversations SET updated_at_ms=?1 WHERE id=?2",
            params![now, input.conversation_id],
        )?;
        tx.commit()?;
        Ok(PreparedChat {
            request_id: input.request_id.clone(),
            conversation_id: input.conversation_id.clone(),
            user_message_id: user_id,
            assistant_id,
            is_new: true,
        })
    }
    pub fn checkpoint_chat(&mut self, request: &str, content: &str) -> Result<bool> {
        if content.len() > MAX_MESSAGE_BYTES || serde_json::to_string(content)?.len() > 48_000 {
            return Err(Error::Invalid("reply limit"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed=tx.execute("UPDATE chat_generations SET content=?1,state='generating',revision=revision+1 WHERE request_id=?2 AND state IN ('sending','generating')",params![content,request])?;
        if changed > 0 {
            tx.execute("UPDATE chat_requests SET state='generating' WHERE id=?1 AND state IN ('sending','generating')",[request])?;
        }
        tx.commit()?;
        Ok(changed > 0)
    }
    pub fn finish_chat(
        &mut self,
        request: &str,
        content: &str,
        state: &str,
        error_code: Option<&str>,
        finish_reason: Option<&str>,
        usage: Option<&TokenUsage>,
    ) -> Result<bool> {
        if !matches!(state, "complete" | "interrupted" | "failed")
            || content.len() > MAX_MESSAGE_BYTES
            || serde_json::to_string(content)?.len() > 48_000
            || error_code.is_some_and(|s| s.len() > 100)
            || finish_reason.is_some_and(|s| s.len() > 100)
        {
            return Err(Error::Invalid("chat final state"));
        }
        if state == "complete" && content.trim().is_empty() {
            return Err(Error::Invalid("empty complete response"));
        }
        if let Some(u) = usage {
            u.validate()?;
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,String,String,String,String,i64)>=tx.query_row("SELECT r.state,r.conversation_id,r.user_message_id,r.assistant_message_id,r.model_id,r.created_at_ms FROM chat_requests r WHERE r.id=?1",[request],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
        let Some((old, conversation, user_id, assistant, model, created)) = row else {
            return Err(Error::Invalid("request missing"));
        };
        if !matches!(old.as_str(), "sending" | "generating") {
            return Ok(false);
        }
        let now = now_ms();
        if !content.trim().is_empty() {
            Self::append_in(
                &tx,
                &self.device_id,
                &tx.query_row(
                    "SELECT profile_id FROM conversations WHERE id=?1",
                    [&conversation],
                    |r| r.get::<_, String>(0),
                )?,
                Entity::Message,
                &assistant,
                Operation::Put,
                json!({"role":"assistant","content":content,"conversation_id":conversation,"created_at_ms":created,"model_id":model,"request_id":request,"reply_to_id":user_id,"generation_id":assistant,"status":state}),
            )?;
        }
        tx.execute("UPDATE chat_requests SET state=?1,finished_at_ms=?2,error_code=?3,finish_reason=?4 WHERE id=?5",params![state,now,error_code,finish_reason,request])?;
        tx.execute("UPDATE chat_generations SET content=?1,state=?2,revision=revision+1,finished_at_ms=?3,error_code=?4,finish_reason=?5 WHERE request_id=?6",params![content,state,now,error_code,finish_reason,request])?;
        tx.execute("INSERT INTO lan_outbox(table_name,after_json) SELECT 'chat_turn_choices',json_object('user_message_id',user_message_id,'selected_generation_id',selected_generation_id) FROM chat_turn_choices WHERE user_message_id=?1",[&user_id])?;
        tx.execute("INSERT OR IGNORE INTO api_usage_records(id,profile_id,request_key,provider,model_id,input_tokens,output_tokens,currency,usage_state,created_at_ms,price_snapshot_json,persona_id) VALUES(?1,?2,?3,'deepseek',?4,?5,?6,'USD',?7,?8,'{}',?9)",params![Uuid::new_v4().to_string(),tx.query_row("SELECT profile_id FROM conversations WHERE id=?1",[&conversation],|r|r.get::<_,String>(0))?,request,model,usage.map(|u|u.prompt_tokens),usage.map(|u|u.completion_tokens),if usage.is_some(){"measured"}else{"unknown"},now,tx.query_row("SELECT persona_id FROM conversation_personas WHERE conversation_id=?1",[&conversation],|r|r.get::<_,String>(0))?])?;
        tx.execute(
            "UPDATE conversations SET updated_at_ms=?1 WHERE id=?2",
            params![now, conversation],
        )?;
        tx.commit()?;
        if state == "complete" && self.process_companion_batch().is_err() {
            self.conn.execute("UPDATE companion_worker_status SET error_code='COMPANION_PROCESSING_RETRY',failures=failures+1 WHERE id=1",[])?;
        }
        Ok(true)
    }
    pub(crate) fn recover_chat_requests(&mut self) -> Result<()> {
        let records = {
            let mut stmt=self.conn.prepare("SELECT request_id,content FROM chat_generations WHERE state IN ('sending','generating')")?;
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (id, content) in records {
            self.finish_chat(
                &id,
                &content,
                "interrupted",
                Some("APP_RESTARTED"),
                None,
                None,
            )?;
        }
        Ok(())
    }
    pub fn chat_messages(&self, conversation: &str) -> Result<Vec<ChatMessage>> {
        let mut rows = Vec::new();
        for id in self.thread_ids(conversation)? {
            rows.extend(self.chat_messages_in_conversation(&id)?);
        }
        let mut meta = self
            .conn
            .prepare("SELECT id,created_lamport,origin_device_id FROM messages WHERE (profile_id=?1 OR profile_id IN (SELECT profile_id FROM lan_profiles))")?;
        let clocks: std::collections::BTreeMap<String, (i64, String)> = meta
            .query_map([&self.profile_id], |r| {
                Ok((r.get(0)?, (r.get(1)?, r.get(2)?)))
            })?
            .collect::<std::result::Result<_, _>>()?;
        rows.sort_by_key(|m| {
            let root = m.user_message_id.as_deref().unwrap_or(&m.id);
            let (clock, origin) = clocks
                .get(root)
                .cloned()
                .unwrap_or((i64::MAX, String::new()));
            let (reply_clock, reply_origin) = clocks
                .get(&m.id)
                .cloned()
                .unwrap_or((i64::MAX, String::new()));
            (
                clock,
                origin,
                root.to_owned(),
                m.role != "user",
                reply_clock,
                reply_origin,
                m.id.clone(),
            )
        });
        for row in &mut rows {
            row.conversation_id = conversation.into();
        }
        Ok(rows)
    }
    fn chat_messages_in_conversation(&self, conversation: &str) -> Result<Vec<ChatMessage>> {
        uuid(conversation)?;
        let mut stmt=self.conn.prepare("SELECT m.id,m.conversation_id,m.role,m.content,m.status,m.created_at_ms,m.model_id,m.request_id,m.reply_to_id,coalesce(g.revision,0),CASE WHEN g.id IS NULL THEN 1 ELSE c.selected_generation_id=g.id END,g.error_code,g.finish_reason FROM messages m LEFT JOIN chat_generations g ON g.id=m.id LEFT JOIN chat_turn_choices c ON c.user_message_id=g.user_message_id WHERE m.conversation_id=?1 AND (m.profile_id=?2 OR m.profile_id IN (SELECT profile_id FROM lan_profiles)) AND m.is_deleted=0 ORDER BY m.created_lamport,m.origin_device_id,m.id")?;
        let mut rows = stmt
            .query_map(params![conversation, self.profile_id], |r| {
                Ok(ChatMessage {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    role: r.get(2)?,
                    content: r.get(3)?,
                    state: r.get(4)?,
                    created_at_ms: r.get(5)?,
                    model_id: r.get(6)?,
                    request_id: r.get(7)?,
                    user_message_id: r.get(8)?,
                    revision: r.get(9)?,
                    selected: r.get(10)?,
                    error_code: r.get(11)?,
                    finish_reason: r.get(12)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut drafts=self.conn.prepare("SELECT g.id,r.conversation_id,g.content,g.state,g.created_at_ms,g.model_id,g.request_id,g.user_message_id,g.revision,c.selected_generation_id=g.id,g.error_code,g.finish_reason FROM chat_generations g JOIN chat_requests r ON r.id=g.request_id JOIN conversations cv ON cv.id=r.conversation_id JOIN chat_turn_choices c ON c.user_message_id=g.user_message_id WHERE r.conversation_id=?1 AND (cv.profile_id=?2 OR cv.profile_id IN (SELECT profile_id FROM lan_profiles)) AND NOT EXISTS(SELECT 1 FROM messages m WHERE m.id=g.id)")?;
        rows.extend(
            drafts
                .query_map(params![conversation, self.profile_id], |r| {
                    Ok(ChatMessage {
                        id: r.get(0)?,
                        conversation_id: r.get(1)?,
                        role: "assistant".into(),
                        content: r.get(2)?,
                        state: r.get(3)?,
                        created_at_ms: Some(r.get(4)?),
                        model_id: Some(r.get(5)?),
                        request_id: Some(r.get(6)?),
                        user_message_id: Some(r.get(7)?),
                        revision: r.get(8)?,
                        selected: r.get(9)?,
                        error_code: r.get(10)?,
                        finish_reason: r.get(11)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?,
        );
        // Keep each reply variant next to its user turn; timestamps never resolve sync conflicts.
        let user_order: std::collections::BTreeMap<String, usize> = rows
            .iter()
            .filter(|r| r.role == "user")
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        rows.sort_by_key(|r| {
            let order = if r.role == "user" {
                user_order.get(&r.id)
            } else {
                r.user_message_id.as_ref().and_then(|id| user_order.get(id))
            };
            (
                order.copied().unwrap_or(usize::MAX),
                r.role != "user",
                r.created_at_ms.unwrap_or(0),
                r.id.clone(),
            )
        });
        Ok(rows)
    }
    pub fn generation(&self, id: &str) -> Result<ChatMessage> {
        let conversation:String=self.conn.query_row("SELECT r.conversation_id FROM chat_generations g JOIN chat_requests r ON r.id=g.request_id WHERE g.id=?1",[id],|r|r.get(0))?;
        self.chat_messages(&conversation)?
            .into_iter()
            .find(|m| m.id == id)
            .ok_or(Error::Invalid("generation missing"))
    }
    pub fn chat_context(
        &self,
        conversation: &str,
        settings: &ChatSettings,
        injection: &ContextInjection,
    ) -> Result<Vec<PromptMessage>> {
        settings.validate()?;
        let all = self.chat_messages(conversation)?;
        let users: Vec<_> = all.iter().filter(|m| m.role == "user").collect();
        let take = if settings.include_history {
            settings.history_turns as usize
        } else {
            1
        };
        let first = users.len().saturating_sub(take);
        let included: std::collections::BTreeSet<_> =
            users[first..].iter().map(|m| m.id.clone()).collect();
        let mut history: Vec<_> = all
            .into_iter()
            .filter(|m| {
                (m.role == "user" && included.contains(m.id.as_str()))
                    || (m.role == "assistant"
                        && m.selected
                        && matches!(m.state.as_str(), "complete" | "interrupted")
                        && !m.content.is_empty()
                        && m.user_message_id
                            .as_deref()
                            .is_some_and(|u| included.contains(u)))
            })
            .map(|m| PromptMessage {
                role: m.role,
                content: m.content,
            })
            .collect();
        let mut bytes: usize = history.iter().map(|m| m.content.len()).sum();
        while bytes > 120_000 && history.len() > 1 {
            bytes -= history.remove(0).content.len();
        }
        while history.first().is_some_and(|m| m.role != "user") {
            history.remove(0);
        }
        let mut prompt = vec![PromptMessage {
            role: "system".into(),
            content: settings.system_prompt(injection),
        }];
        prompt.extend(history);
        Ok(prompt)
    }
    pub fn usage_summary(&self) -> Result<UsageSummary> {
        self.usage_for_persona(&self.active_persona_id()?)
    }
}
