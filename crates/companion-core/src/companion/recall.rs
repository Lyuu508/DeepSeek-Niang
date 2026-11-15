//! Local, bounded retrieval from the permanent timeline. No model/embedding calls.
use super::{MEMBERS, memory::normalized};
use crate::{Result, storage::Store};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct ChatExcerpt {
    pub message_id: String,
    pub role: String,
    pub text: String,
    pub recorded_at_ms: Option<i64>,
}
fn sensitive(text: &str) -> bool {
    let normalized = normalized(text);
    [
        "apikey",
        "secretkey",
        "密码",
        "私钥",
        "密钥",
        "银行卡",
        "不要记住",
        "别记住",
    ]
    .iter()
    .any(|word| normalized.contains(word))
        || text.to_lowercase().contains("sk-")
}
fn terms(query: &str) -> Vec<String> {
    let text = normalized(query).chars().take(512).collect::<Vec<_>>();
    let mut result = text
        .windows(3)
        .map(|word| word.iter().collect::<String>())
        .filter(|word| {
            ![
                "你还记",
                "还记得",
                "记不记",
                "不记得",
                "之前我",
                "上次我",
                "跟你说",
                "是什么",
                "的时候",
                "了什么",
                "那件事",
            ]
            .contains(&word.as_str())
        })
        .collect::<Vec<_>>();
    result.extend(
        query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| word.is_ascii() && word.len() >= 3)
            .map(str::to_lowercase),
    );
    if result.is_empty() && text.len() >= 2 {
        result.push(text.iter().collect());
    }
    let mut seen = BTreeSet::new();
    result.retain(|term| seen.insert(term.clone()));
    result.truncate(48);
    result
}
fn snippet(text: &str, terms: &[String], max: usize) -> String {
    let lower = text.to_lowercase();
    let byte = terms
        .iter()
        .filter_map(|term| lower.find(term))
        .min()
        .unwrap_or(0);
    // Find the original UTF-8 char boundary; no lossy byte slicing of Chinese.
    let at = text
        .char_indices()
        .take_while(|(offset, _)| *offset < byte)
        .count();
    let start = at.saturating_sub(max / 3);
    let mut result = if start > 0 {
        "…".to_owned()
    } else {
        String::new()
    };
    result.extend(text.chars().skip(start).take(max));
    if text.chars().count() > start + max {
        result.push('…');
    }
    result
}
impl Store {
    pub fn chat_recall_stats(&self, persona: &str) -> Result<Value> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let count:i64=self.conn.query_row(&format!("SELECT count(*) FROM messages m JOIN conversation_personas c ON c.conversation_id=m.conversation_id WHERE c.persona_id IN ({MEMBERS}) AND m.is_deleted=0 AND m.status='complete' AND m.role IN ('user','assistant')"),[persona],|row|row.get(0))?;
        Ok(
            json!({"stored_messages":count,"strategy":"local_full_text_excerpts","additional_model_requests":0}),
        )
    }
    pub fn retrieve_chat_recall(
        &self,
        persona: &str,
        query: &str,
        already_present: &[&str],
        budget: usize,
        limit: usize,
    ) -> Result<Vec<ChatExcerpt>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        if limit == 0 || budget < 150 {
            return Ok(Vec::new());
        }
        let terms = terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let scoped = format!(
            "c.persona_id IN ({MEMBERS}) AND m.is_deleted=0 AND m.status='complete' AND m.role IN ('user','assistant') AND (m.role='user' OR (m.generation_id IS NULL AND g.id IS NULL) OR (ch.selected_generation_id=g.id AND g.state='complete')) AND NOT EXISTS(SELECT 1 FROM companion_memory_meta d WHERE d.source_message_id=m.id OR d.source_message_id=coalesce(g.user_message_id,m.reply_to_id))"
        );
        let fields = "m.id,m.role,m.content,m.created_at_ms";
        let joins = "JOIN conversation_personas c ON c.conversation_id=m.conversation_id LEFT JOIN chat_generations g ON g.id=m.id LEFT JOIN chat_turn_choices ch ON ch.user_message_id=g.user_message_id";
        let long = terms
            .iter()
            .filter(|term| term.chars().count() >= 3)
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        if !long.is_empty() {
            let mut statement=self.conn.prepare(&format!("SELECT {fields} FROM chat_recall_fts JOIN messages m ON m.rowid=chat_recall_fts.rowid {joins} WHERE chat_recall_fts MATCH ?2 AND {scoped} ORDER BY bm25(chat_recall_fts),m.created_lamport DESC LIMIT 96"))?;
            rows = statement
                .query_map(params![persona, long.join(" OR ")], |row| {
                    Ok(ChatExcerpt {
                        message_id: row.get(0)?,
                        role: row.get(1)?,
                        text: row.get(2)?,
                        recorded_at_ms: row.get(3)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }
        // A two-character topic cannot be found by a trigram index. This local
        // fallback keeps historical recall useful for names such as “黄山”.
        if rows.is_empty() && query.chars().count() <= 12 {
            let small = normalized(query);
            if small.chars().count() >= 2 {
                let mut statement=self.conn.prepare(&format!("SELECT {fields} FROM messages m {joins} WHERE {scoped} AND instr(lower(m.content),?2)>0 ORDER BY m.created_lamport DESC LIMIT 40"))?;
                rows = statement
                    .query_map(params![persona, small], |row| {
                        Ok(ChatExcerpt {
                            message_id: row.get(0)?,
                            role: row.get(1)?,
                            text: row.get(2)?,
                            recorded_at_ms: row.get(3)?,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
            }
        }
        let private: Vec<String> = {
            let mut statement=self.conn.prepare(&format!("SELECT text FROM memories WHERE persona_id IN ({MEMBERS}) AND is_deleted=0 AND sensitivity<>'api_allowed' LIMIT 1000"))?;
            statement
                .query_map([persona], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
                .iter()
                .map(|text| normalized(text))
                .filter(|text| text.chars().count() >= 3)
                .collect()
        };
        let excluded = already_present
            .iter()
            .map(|text| normalized(text))
            .collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        let mut used = 2;
        for mut row in rows {
            let normal = normalized(&row.text);
            if sensitive(&row.text)
                || excluded.contains(&normal)
                || !seen.insert(normal.clone())
                || private.iter().any(|fact| normal.contains(fact))
            {
                continue;
            }
            row.text = snippet(&row.text, &terms, 160);
            let length = serde_json::to_vec(&row)?.len() + 1;
            if used + length > budget {
                continue;
            }
            used += length;
            result.push(row);
            if result.len() >= limit.min(6) {
                break;
            }
        }
        Ok(result)
    }
}
