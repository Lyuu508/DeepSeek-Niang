use crate::{ChatFailure, deepseek::DeepSeekClient};
use companion_core::{
    chat::{ChatMessage, ChatSettings, PreparedChat, StartChat, TokenUsage},
    storage::Store,
};
use companion_secrets::{Secret, SecretStore};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
#[path = "proactive.rs"]
mod proactive;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    Started {
        request: PreparedChat,
        user: Box<ChatMessage>,
        message: Box<ChatMessage>,
    },
    Delta {
        request_id: String,
        conversation_id: String,
        message_id: String,
        content: String,
        revision: i64,
    },
    Finished {
        request_id: String,
        conversation_id: String,
        message: Box<ChatMessage>,
        usage: Option<TokenUsage>,
        error: Option<ChatFailure>,
    },
    Failed {
        request_id: String,
        conversation_id: String,
        error: ChatFailure,
    },
}
pub type Observer = Arc<dyn Fn(ChatEvent) -> bool + Send + Sync>;
struct Run {
    conversation: String,
    cancel: CancellationToken,
    done: watch::Receiver<bool>,
}
#[derive(Default)]
struct Active {
    runs: BTreeMap<String, Run>,
    changing_key: bool,
    background: Option<Run>,
}
pub struct ChatService {
    pub store: Arc<Mutex<Store>>,
    pub vault: Arc<dyn SecretStore>,
    provider: DeepSeekClient,
    active: Mutex<Active>,
}
impl ChatService {
    pub fn new(
        store: Arc<Mutex<Store>>,
        vault: Arc<dyn SecretStore>,
        provider: DeepSeekClient,
    ) -> Self {
        Self {
            store,
            vault,
            provider,
            active: Mutex::new(Active::default()),
        }
    }
    pub fn start(
        self: &Arc<Self>,
        input: StartChat,
        observer: Observer,
    ) -> Result<PreparedChat, ChatFailure> {
        self.start_checked(input, None, observer, None)
    }

    pub fn start_with_image(
        self: &Arc<Self>,
        input: StartChat,
        observer: Observer,
        image: companion_core::chat_images::ChatImage,
    ) -> Result<PreparedChat, ChatFailure> {
        self.start_checked(input, None, observer, Some(&image))
    }

    /// The desktop pet submits exactly to the persona shown beside its composer.
    pub fn start_for_active_persona(
        self: &Arc<Self>,
        input: StartChat,
        persona_id: &str,
        observer: Observer,
    ) -> Result<PreparedChat, ChatFailure> {
        self.start_checked(input, Some(persona_id), observer, None)
    }

    fn start_checked(
        self: &Arc<Self>,
        input: StartChat,
        required_persona: Option<&str>,
        observer: Observer,
        image: Option<&companion_core::chat_images::ChatImage>,
    ) -> Result<PreparedChat, ChatFailure> {
        let mut active = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
        if active.changing_key || !active.runs.is_empty() {
            return Err(ChatFailure::new("BUSY"));
        }
        if let Some(background) = &active.background {
            background.cancel.cancel();
        }
        let (prepared, settings, prompt, message, user) = {
            let mut store = self
                .store
                .lock()
                .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
            if let Some(persona) = required_persona
                && (store.active_persona_id()? != persona
                    || store.permanent_persona(&input.conversation_id)?.id != persona)
            {
                return Err(ChatFailure::new("PET_PERSONA_CHANGED"));
            }
            let settings = store.settings_for_conversation(&input.conversation_id)?;
            let prepared = store.begin_chat_with_image(&input, &settings, image)?;
            let prompt = match store.companion_context(&prepared.conversation_id, &settings) {
                Ok(prompt) => prompt,
                Err(error) => {
                    store.finish_chat(
                        &prepared.request_id,
                        "",
                        "failed",
                        Some("CONTEXT_BUDGET_EXCEEDED"),
                        None,
                        None,
                    )?;
                    return Err(error.into());
                }
            };
            let message = store.generation(&prepared.assistant_id)?;
            let user = store
                .chat_messages(&prepared.conversation_id)?
                .into_iter()
                .find(|m| m.id == prepared.user_message_id)
                .ok_or_else(|| ChatFailure::new("STORAGE_ERROR"))?;
            (prepared, settings, prompt, message, user)
        };
        if !prepared.is_new {
            observer(ChatEvent::Finished {
                request_id: prepared.request_id.clone(),
                conversation_id: prepared.conversation_id.clone(),
                message: Box::new(message),
                usage: None,
                error: None,
            });
            return Ok(prepared);
        }
        let cancel = CancellationToken::new();
        let (done_tx, done_rx) = watch::channel(false);
        active.runs.insert(
            prepared.request_id.clone(),
            Run {
                conversation: prepared.conversation_id.clone(),
                cancel: cancel.clone(),
                done: done_rx,
            },
        );
        drop(active);
        if !observer(ChatEvent::Started {
            request: prepared.clone(),
            user: Box::new(user),
            message: Box::new(message),
        }) {
            cancel.cancel();
        }
        let service = self.clone();
        let task = prepared.clone();
        tokio::spawn(async move {
            service
                .run(task.clone(), settings, prompt, cancel, observer)
                .await;
            // Remove before notifying waiters, so credential changes cannot race a new request.
            if let Ok(mut state) = service.active.lock() {
                state.runs.remove(&task.request_id);
            }
            let _ = done_tx.send(true);
        });
        Ok(prepared)
    }
    async fn run(
        &self,
        task: PreparedChat,
        settings: ChatSettings,
        prompt: Vec<companion_core::chat::PromptMessage>,
        cancel: CancellationToken,
        observer: Observer,
    ) {
        let mut revision = 0;
        let mut storage_failed = false;
        let result = match self.vault.get() {
            Ok(key) => {
                self.provider
                    .stream(&key, &settings, &prompt, &cancel, |content| {
                        let saved = self
                            .store
                            .lock()
                            .ok()
                            .and_then(|mut s| s.checkpoint_chat(&task.request_id, content).ok())
                            .unwrap_or(false);
                        if !saved {
                            storage_failed = true;
                            return false;
                        }
                        revision += 1;
                        observer(ChatEvent::Delta {
                            request_id: task.request_id.clone(),
                            conversation_id: task.conversation_id.clone(),
                            message_id: task.assistant_id.clone(),
                            content: content.into(),
                            revision,
                        })
                    })
                    .await
            }
            Err(error) => crate::deepseek::StreamResult {
                content: String::new(),
                usage: None,
                finish_reason: None,
                error: Some(error.into()),
            },
        };
        let mut error = result.error;
        if storage_failed {
            error = Some(ChatFailure::new("STORAGE_ERROR"));
        }
        if cancel.is_cancelled() && !storage_failed {
            error = Some(ChatFailure::new("CANCELLED"));
        }
        let state = match error.as_ref().map(|e| e.code.as_str()) {
            None => "complete",
            Some("CANCELLED" | "OUTPUT_LIMIT" | "TIMEOUT" | "STREAM_INCOMPLETE") => "interrupted",
            _ => "failed",
        };
        let final_message = (|| -> Result<ChatMessage, ChatFailure> {
            let mut store = self
                .store
                .lock()
                .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
            store.finish_chat(
                &task.request_id,
                &result.content,
                state,
                error.as_ref().map(|e| e.code.as_str()),
                result.finish_reason.as_deref(),
                result.usage.as_ref(),
            )?;
            Ok(store.generation(&task.assistant_id)?)
        })();
        match final_message {
            Ok(message) => {
                observer(ChatEvent::Finished {
                    request_id: task.request_id,
                    conversation_id: task.conversation_id,
                    message: Box::new(message),
                    usage: result.usage,
                    error,
                });
            }
            Err(error) => {
                observer(ChatEvent::Failed {
                    request_id: task.request_id,
                    conversation_id: task.conversation_id,
                    error,
                });
            }
        }
    }
    pub fn cancel_life_generation(&self) {
        if let Ok(active) = self.active.lock()
            && let Some(run) = &active.background
        {
            run.cancel.cancel();
        }
    }
    /// Stop provider work after a local or synchronized lifecycle transition.
    pub fn cancel_inactive_personas(&self) {
        let Ok(active) = self.active.lock() else {
            return;
        };
        if active.runs.is_empty() && active.background.is_none() {
            return;
        }
        let Ok(store) = self.store.lock() else { return };
        for run in active.runs.values() {
            if store.permanent_persona(&run.conversation).is_err() {
                run.cancel.cancel();
            }
        }
        if let Some(run) = &active.background
            && store.persona(&run.conversation).is_err()
        {
            run.cancel.cancel();
        }
    }
    pub fn stop(&self, request_id: &str) -> Result<bool, ChatFailure> {
        let state = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
        if let Some(run) = state.runs.get(request_id) {
            run.cancel.cancel();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn active_request(&self) -> Option<String> {
        self.active_generation().map(|(request, _)| request)
    }
    pub fn active_generation(&self) -> Option<(String, String)> {
        self.active.lock().ok().and_then(|s| {
            s.runs
                .iter()
                .next()
                .map(|(id, run)| (id.clone(), run.conversation.clone()))
        })
    }
    /// Return only after the interrupted draft is durable and a new request may start.
    /// Also works after the WebView which owned the event channel was recreated.
    pub async fn stop_and_wait(&self, request_id: &str) -> Result<bool, ChatFailure> {
        let waiter = {
            let state = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
            state.runs.get(request_id).map(|run| {
                run.cancel.cancel();
                run.done.clone()
            })
        };
        let Some(mut waiter) = waiter else {
            return Ok(false);
        };
        if !*waiter.borrow() {
            tokio::time::timeout(std::time::Duration::from_secs(3), waiter.changed())
                .await
                .map_err(|_| ChatFailure::new("BUSY"))?
                .map_err(|_| ChatFailure::new("BUSY"))?;
        }
        Ok(true)
    }
    pub async fn replace_key(&self, key: Option<Secret>) -> Result<(), ChatFailure> {
        let waiters = {
            let mut state = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
            if state.changing_key {
                return Err(ChatFailure::new("BUSY"));
            }
            state.changing_key = true;
            state
                .runs
                .values()
                .chain(state.background.iter())
                .map(|r| {
                    r.cancel.cancel();
                    r.done.clone()
                })
                .collect::<Vec<_>>()
        };
        let result = async {
            for mut waiter in waiters {
                if !*waiter.borrow() {
                    tokio::time::timeout(std::time::Duration::from_secs(3), waiter.changed())
                        .await
                        .map_err(|_| ChatFailure::new("BUSY"))?
                        .map_err(|_| ChatFailure::new("BUSY"))?;
                }
            }
            if let Some(key) = key {
                self.vault.set(&key)?;
                // Verify actual native readback without exposing the key to JS or a log.
                if self.vault.get()?.expose() != key.expose() {
                    return Err(ChatFailure::new("SECURE_STORAGE_CORRUPT"));
                }
            } else {
                self.vault.delete()?;
            }
            Ok(())
        }
        .await;
        if let Ok(mut state) = self.active.lock() {
            state.changing_key = false;
        }
        result
    }
}
