use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DialogueMode {
    #[default]
    Standard,
    Free,
    Custom,
}
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Natural,
    Gentle,
    Calm,
    Playful,
}
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplyLength {
    Short,
    #[default]
    Balanced,
    Detailed,
}
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Roleplay {
    #[default]
    Daily,
    InCharacter,
    CoWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Dialogue {
    pub mode: DialogueMode,
    pub tone: Tone,
    pub reply_length: ReplyLength,
    pub humor: u8,
    pub creativity: u8,
    pub expression: u8,
    pub roleplay: Roleplay,
    pub extra_style: String,
}

impl Default for Dialogue {
    fn default() -> Self {
        Self {
            mode: DialogueMode::Standard,
            tone: Tone::Natural,
            reply_length: ReplyLength::Balanced,
            humor: 2,
            creativity: 2,
            expression: 2,
            roleplay: Roleplay::Daily,
            extra_style: String::new(),
        }
    }
}

impl Dialogue {
    pub fn validate(&self) -> Result<()> {
        if self.humor > 4
            || self.creativity > 4
            || self.expression > 4
            || self.extra_style.chars().count() > 600
            || self.extra_style.len() > 2400
            || self
                .extra_style
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
        {
            return Err(Error::Invalid("dialogue style range"));
        }
        Ok(())
    }

    pub(crate) fn append_style(&self, prompt: &mut String) {
        match self.mode {
            DialogueMode::Standard => {}
            DialogueMode::Free => prompt.push_str(
                "\n对话风格偏好：跟随用户当前话题，自然承接上文。减少固定开场、总结和机械追问。适量使用角色幽默与情绪，可以在明确虚构的场景里即兴共创，保持已提供的人格、关系和事实依据。"
            ),
            DialogueMode::Custom => {
                let tone = match self.tone { Tone::Natural=>"自然",Tone::Gentle=>"温柔",Tone::Calm=>"冷静",Tone::Playful=>"俏皮" };
                let length = match self.reply_length { ReplyLength::Short=>"简短",ReplyLength::Balanced=>"适中",ReplyLength::Detailed=>"详细" };
                let roleplay = match self.roleplay {
                    Roleplay::Daily=>"日常对话",Roleplay::InCharacter=>"沿用角色口吻",Roleplay::CoWrite=>"明确虚构时一起创作故事",
                };
                prompt.push_str(&format!(
                    "\n以下为用户的表达偏好，只用于调整回复风格。基础角色约束、真实来源和用户选择仍适用。{}",
                    json!({"语气":tone,"回复长度":length,"幽默":self.humor,"创意":self.creativity,
                        "情绪表达":self.expression,"表达方式":roleplay,"附加风格":self.extra_style})
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chat::{ChatSettings, StartChat},
        storage::Store,
    };
    #[test]
    fn dialogue_changes_only_the_selected_persona_system_style_and_keeps_raw_user_input() {
        let mut s = Store::open(":memory:").unwrap();
        let first = s.active_persona_id().unwrap();
        let other = s.create_persona("共创", "好奇温柔").unwrap();
        let base = s.persona_settings(&other.id).unwrap();
        let mut prefs = base.clone();
        prefs.dialogue = Dialogue {
            mode: DialogueMode::Custom,
            tone: Tone::Gentle,
            roleplay: Roleplay::CoWrite,
            extra_style: "自然收尾，少用格式化总结。".into(),
            ..Dialogue::default()
        };
        s.save_persona_settings(&other.id, &prefs).unwrap();
        assert_eq!(
            s.persona_settings(&first).unwrap().dialogue.mode,
            DialogueMode::Standard
        );
        let text = "  合成用户原话\n保留空格  ";
        s.begin_chat(
            &StartChat {
                request_id: uuid::Uuid::new_v4().to_string(),
                conversation_id: other.main_conversation_id.clone(),
                text: Some(text.into()),
                regenerate_user_id: None,
            },
            &prefs,
        )
        .unwrap();
        let prompt = s
            .companion_context(
                &other.main_conversation_id,
                &s.persona_settings(&other.id).unwrap(),
            )
            .unwrap();
        assert!(prompt[0].content.contains("自然收尾"));
        assert_eq!(prompt.last().unwrap().content, text);
        let mut old = serde_json::to_value(base).unwrap();
        old.as_object_mut().unwrap().remove("dialogue");
        assert_eq!(
            serde_json::from_value::<ChatSettings>(old)
                .unwrap()
                .dialogue
                .mode,
            DialogueMode::Standard
        );
    }
}
