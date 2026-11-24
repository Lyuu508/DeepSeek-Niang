use crate::{ChatFailure, sse::SseDecoder};
use companion_core::chat::{ChatSettings, MAX_MESSAGE_BYTES, PromptMessage, TokenUsage};
use companion_secrets::Secret;
use futures_util::StreamExt;
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderValue},
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub const OFFICIAL_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
pub struct DeepSeekClient {
    client: Client,
    endpoint: String,
}
pub struct StreamResult {
    pub content: String,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
    pub error: Option<ChatFailure>,
}
impl StreamResult {
    fn empty() -> Self {
        Self {
            content: String::new(),
            usage: None,
            finish_reason: None,
            error: None,
        }
    }
}
pub fn request_body(
    settings: &ChatSettings,
    messages: &[PromptMessage],
) -> Result<Value, ChatFailure> {
    settings.validate().map_err(ChatFailure::from)?;
    Ok(
        json!({"model":settings.model,"messages":messages,"stream":true,"stream_options":{"include_usage":true},"thinking":{"type":"disabled"},"temperature":settings.temperature,"max_tokens":settings.max_tokens}),
    )
}
impl DeepSeekClient {
    pub fn official() -> Result<Self, ChatFailure> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("DeepSeekCompanion/0.1-local")
            .build()
            .map_err(|_| ChatFailure::new("NETWORK_ERROR"))?;
        Ok(Self {
            client,
            endpoint: OFFICIAL_ENDPOINT.into(),
        })
    }
    #[cfg(test)]
    pub(crate) fn fixture(endpoint: String, timeout: Duration) -> Self {
        Self {
            client: Client::builder()
                .no_proxy()
                .read_timeout(timeout)
                .connect_timeout(timeout)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            endpoint,
        }
    }
    pub async fn stream(
        &self,
        key: &Secret,
        settings: &ChatSettings,
        messages: &[PromptMessage],
        cancel: &CancellationToken,
        progress: impl FnMut(&str) -> bool + Send,
    ) -> StreamResult {
        self.stream_request(
            key,
            settings,
            request_body(settings, messages),
            cancel,
            progress,
        )
        .await
    }

    pub async fn stream_conversation(
        &self,
        key: &Secret,
        settings: &ChatSettings,
        context: &companion_core::companion::intelligence::ConversationContext,
        cancel: &CancellationToken,
        progress: impl FnMut(&str) -> bool + Send,
    ) -> StreamResult {
        use companion_core::companion::intelligence::DialogueIntent;
        let body = request_body(settings, &context.messages).map(|mut body| {
            if context.intent == DialogueIntent::Technical && settings.max_tokens >= 1024 {
                body["thinking"] = json!({"type":"enabled"});
                body["reasoning_effort"] = json!("high");
                body.as_object_mut()
                    .expect("request body object")
                    .remove("temperature");
            }
            body
        });
        self.stream_request(key, settings, body, cancel, progress)
            .await
    }

    async fn stream_request(
        &self,
        key: &Secret,
        settings: &ChatSettings,
        body: Result<Value, ChatFailure>,
        cancel: &CancellationToken,
        mut progress: impl FnMut(&str) -> bool + Send,
    ) -> StreamResult {
        let mut output = StreamResult::empty();
        let result = match body {
            Ok(body) => {
                self.run(key, settings, body, cancel, &mut output, &mut progress)
                    .await
            }
            Err(error) => Err(error),
        };
        if let Err(e) = result {
            output.error = Some(e);
        }
        output
    }
    async fn run(
        &self,
        key: &Secret,
        settings: &ChatSettings,
        body: Value,
        cancel: &CancellationToken,
        output: &mut StreamResult,
        progress: &mut (impl FnMut(&str) -> bool + Send),
    ) -> Result<(), ChatFailure> {
        let mut authorization = HeaderValue::from_str(&format!("Bearer {}", key.expose()))
            .map_err(|_| ChatFailure::new("KEY_FORMAT_INVALID"))?;
        authorization.set_sensitive(true);
        let mut attempt = 0;
        let response = loop {
            let send = self
                .client
                .post(&self.endpoint)
                .header(AUTHORIZATION, authorization.clone())
                .header("Accept", "text/event-stream")
                .timeout(Duration::from_secs(settings.timeout_seconds as u64))
                .json(&body)
                .send();
            let response = tokio::select! {biased;_=cancel.cancelled()=>return Err(ChatFailure::new("CANCELLED")),r=send=>r.map_err(network_error)?};
            if response.status().as_u16() == 429 && attempt == 0 {
                attempt += 1;
                let seconds = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|h| h.parse::<u64>().ok())
                    .unwrap_or(2);
                // Retry only an explicitly rejected 429, at most once. Never replay an uncertain POST.
                if seconds > 10 {
                    return Err(ChatFailure::new("HTTP_429"));
                }
                tokio::select! {biased;_=cancel.cancelled()=>return Err(ChatFailure::new("CANCELLED")),_=tokio::time::sleep(Duration::from_secs(seconds))=>{}}
                continue;
            }
            break response;
        };
        if !response.status().is_success() {
            // Discard untrusted error bodies; they may echo Authorization or user content.
            return Err(ChatFailure::new(match response.status().as_u16() {
                400 => "HTTP_400",
                401 => "HTTP_401",
                402 => "HTTP_402",
                403 => "HTTP_403",
                429 => "HTTP_429",
                500..=599 => "HTTP_SERVER",
                _ => "STREAM_INVALID",
            }));
        }
        let media_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !media_type
            .to_ascii_lowercase()
            .starts_with("text/event-stream")
        {
            return Err(ChatFailure::new("STREAM_INVALID"));
        }
        let mut decoder = SseDecoder::default();
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {biased;_=cancel.cancelled()=>return Err(ChatFailure::new("CANCELLED")),next=stream.next()=>next};
            let Some(chunk) = next else {
                return Err(ChatFailure::new("STREAM_INCOMPLETE"));
            };
            for data in decoder.push(&chunk.map_err(network_error)?)? {
                if data.trim() == "[DONE]" {
                    if output.finish_reason.is_none() || output.content.trim().is_empty() {
                        return Err(ChatFailure::new("STREAM_INVALID"));
                    }
                    return Ok(());
                }
                let value: Value =
                    serde_json::from_str(&data).map_err(|_| ChatFailure::new("STREAM_INVALID"))?;
                if value.get("error").is_some() {
                    return Err(ChatFailure::new("STREAM_INVALID"));
                }
                if let Some(usage) = value.get("usage").filter(|v| !v.is_null()) {
                    let parsed: TokenUsage = serde_json::from_value(usage.clone())
                        .map_err(|_| ChatFailure::new("STREAM_INVALID"))?;
                    parsed
                        .validate()
                        .map_err(|_| ChatFailure::new("STREAM_INVALID"))?;
                    output.usage = Some(parsed);
                }
                let choices = value
                    .get("choices")
                    .and_then(Value::as_array)
                    .ok_or_else(|| ChatFailure::new("STREAM_INVALID"))?;
                for choice in choices {
                    if choice.get("index").and_then(Value::as_u64).unwrap_or(0) != 0 {
                        continue;
                    }
                    if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                        if !matches!(reason, "stop" | "length") {
                            return Err(ChatFailure::new("STREAM_INVALID"));
                        }
                        output.finish_reason = Some(reason.into());
                    }
                    if let Some(delta) = choice
                        .get("delta")
                        .and_then(|v| v.get("content"))
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                    {
                        if output.content.len() + delta.len() > MAX_MESSAGE_BYTES {
                            return Err(ChatFailure::new("OUTPUT_LIMIT"));
                        }
                        let candidate = format!("{}{delta}", output.content);
                        if serde_json::to_string(&candidate)
                            .map_err(|_| ChatFailure::new("STREAM_INVALID"))?
                            .len()
                            > 48_000
                        {
                            return Err(ChatFailure::new("OUTPUT_LIMIT"));
                        }
                        output.content.push_str(delta);
                        if !progress(&output.content) {
                            return Err(ChatFailure::new("CANCELLED"));
                        }
                    }
                }
            }
        }
    }
}
fn network_error(error: reqwest::Error) -> ChatFailure {
    ChatFailure::new(if error.is_timeout() {
        "TIMEOUT"
    } else {
        "NETWORK_ERROR"
    })
}
