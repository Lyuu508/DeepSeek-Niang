//! All network tests use a real loopback HTTP server and explicitly synthetic credentials/data.
use crate::{
    deepseek::{DeepSeekClient, OFFICIAL_ENDPOINT, request_body},
    service::{ChatEvent, ChatService, Observer},
    sse::SseDecoder,
};
use companion_core::{
    chat::{ChatMessage, ChatSettings, PromptMessage, StartChat},
    storage::Store,
};
use companion_secrets::{Secret, SecretStore, VaultError};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const FIXTURE_KEY: &str = "fixture-only-no-real-api-secret-123456";
#[derive(Clone)]
struct Plan {
    status: u16,
    extra: String,
    body: Vec<u8>,
    stall: bool,
}
#[derive(Clone)]
struct Request {
    headers: String,
    body: Value,
    raw: Vec<u8>,
}
fn sse(content: &str, done: bool) -> Vec<u8> {
    let mut bytes=format!(": keepalive\r\n\r\ndata: {}\r\n\r\n",json!({"choices":[{"index":0,"delta":{"role":"assistant","content":content},"finish_reason":null}],"usage":null})).into_bytes();
    if done {
        bytes.extend_from_slice(format!("data: {}\n\ndata: [DONE]\n\n",json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":9,"completion_tokens":3,"total_tokens":12}})).as_bytes());
    }
    bytes
}
fn success(text: &str) -> Plan {
    Plan {
        status: 200,
        extra: "Content-Type: text/event-stream\r\n".into(),
        body: sse(text, true),
        stall: false,
    }
}
async fn read_request(socket: &mut TcpStream) -> Request {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let head = loop {
        let count = socket.read(&mut buffer).await.unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            break i + 4;
        }
        assert!(bytes.len() < 262_144);
    };
    let headers = String::from_utf8(bytes[..head].to_vec()).unwrap();
    let length: usize = headers
        .lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse().unwrap())
        })
        .unwrap_or_default();
    while bytes.len() < head + length {
        let n = socket.read(&mut buffer).await.unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
    }
    let raw = bytes[head..head + length].to_vec();
    let body = serde_json::from_slice(&raw).unwrap_or(Value::Null);
    Request { headers, body, raw }
}
async fn server(plans: Vec<Plan>) -> (String, Arc<Mutex<Vec<Request>>>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/chat/completions", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let output = requests.clone();
    let task = tokio::spawn(async move {
        for plan in plans {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_request(&mut socket).await;
            output.lock().unwrap().push(request);
            let head = format!(
                "HTTP/1.1 {} Fixture\r\nConnection: close\r\n{}\r\n",
                plan.status, plan.extra
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            for part in plan.body.chunks(3) {
                if socket.write_all(part).await.is_err() {
                    break;
                }
            }
            if plan.stall {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    });
    (endpoint, requests, task)
}
fn turns() -> Vec<PromptMessage> {
    vec![
        PromptMessage {
            role: "system".into(),
            content: "明确的测试人设".into(),
        },
        PromptMessage {
            role: "user".into(),
            content: "中文测试消息".into(),
        },
    ]
}
fn secret() -> Secret {
    Secret::new(FIXTURE_KEY.into()).unwrap()
}

fn json_response(value: Value) -> Plan {
    Plan {
        status: 200,
        extra: "Content-Type: application/json\r\n".to_owned(),
        body: value.to_string().into_bytes(),
        stall: false,
    }
}

#[tokio::test]
async fn video_cloud_sends_visual_input_only_to_qwen_and_retries_the_deepseek_answer_only() {
    use crate::video_cloud::{self, QwenClient, VideoInput, VideoRecord};
    use base64::{Engine, engine::general_purpose::STANDARD};
    let planning = json!({"focus":"蓝色物体的动作", "start_ms":100,"end_ms":1800});
    let observations = json!({"observations":[{"at_ms":650,"scene":"蓝色方块向右移动","objects":["方块"],"action":"向右移动","text":""}],"limitations":"仅有画面"});
    let success_evidence = json!({"output":{"choices":[{"finish_reason":"stop","message":{"content":[{"text":observations.to_string()}]}}]}});
    let upload_policy = json!({"data":{
        "upload_host":"https://fixture.oss-cn-hangzhou.aliyuncs.com", "upload_dir":"fixture/dir", "policy":"fake-policy",
        "oss_access_key_id":"fake-oss-id","signature":"fake-oss-signature","x_oss_object_acl":"private","x_oss_forbid_overwrite":"true"
    }});
    let (qwen_endpoint, qwen_requests, qwen_task) = server(vec![
        json_response(upload_policy),
        json_response(success_evidence),
    ])
    .await;
    let (upload_endpoint, upload_requests, upload_task) =
        server(vec![json_response(json!({}))]).await;
    let fail = Plan {
        status: 503,
        extra: String::new(),
        body: Vec::new(),
        stall: false,
    };
    let (deepseek_endpoint, deepseek_requests, deepseek_task) = server(vec![
        success(&planning.to_string()),
        fail,
        success("在 0.6 秒附近，蓝色方块向右移动。"),
    ])
    .await;
    let deepseek = DeepSeekClient::fixture(deepseek_endpoint, Duration::from_secs(5));
    let qwen = QwenClient::fixture(
        qwen_endpoint
            .trim_end_matches("/chat/completions")
            .to_owned(),
        &upload_endpoint,
    );
    let qwen_key = Secret::new("fixture-qwen-only-synthetic-not-a-real-key".into()).unwrap();
    let settings = ChatSettings::default();
    let question = "视频中的方块做了什么？";
    let plan = video_cloud::plan(&deepseek, &secret(), &settings, question, 2000)
        .await
        .unwrap();
    let mut record = VideoRecord {
        id: "fixture-record".into(),
        question: question.into(),
        duration_ms: 2000,
        plan,
        evidence: None,
        answer: None,
    };
    assert!(
        video_cloud::answer(&deepseek, &secret(), &settings, &record)
            .await
            .is_err()
    );
    let mp4 = STANDARD.encode(b"\0\0\0\x18ftypisomsynthetic-clip");
    record.evidence = Some(
        qwen.recognize(&qwen_key, &record, VideoInput::Clip { mp4 })
            .await
            .unwrap(),
    );
    assert_eq!(record.evidence.as_ref().unwrap().observations[0].at_ms, 650);
    assert!(
        video_cloud::answer(&deepseek, &secret(), &settings, &record)
            .await
            .is_err()
    );
    assert!(
        video_cloud::answer(&deepseek, &secret(), &settings, &record)
            .await
            .unwrap()
            .contains("蓝色方块")
    );
    upload_task.await.unwrap();
    qwen_task.await.unwrap();
    deepseek_task.await.unwrap();
    let qwen_calls = qwen_requests.lock().unwrap();
    assert_eq!(qwen_calls.len(), 2);
    assert!(
        qwen_calls[0]
            .headers
            .starts_with("GET /uploads?action=getPolicy&model=")
    );
    assert!(
        qwen_calls[1]
            .headers
            .to_lowercase()
            .contains("x-dashscope-ossresourceresolve: enable")
    );
    for call in qwen_calls.iter() {
        assert!(call.headers.contains(qwen_key.expose()));
        assert!(!call.headers.contains(FIXTURE_KEY));
        assert!(!call.body.to_string().contains(question));
    }
    let upload_calls = upload_requests.lock().unwrap();
    assert!(
        !upload_calls[0]
            .headers
            .to_lowercase()
            .contains("authorization:")
    );
    assert!(
        upload_calls[0]
            .raw
            .windows(14)
            .any(|bytes| bytes == b"synthetic-clip")
    );
    let calls = deepseek_requests.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].body.to_string().contains(question));
    assert_eq!(calls[1].body, calls[2].body);
    assert!(calls[2].body.to_string().contains("蓝色方块向右移动"));
    assert!(!calls[2].body.to_string().contains("oss://"));
    assert!(!calls[2].headers.contains(qwen_key.expose()));
}

#[test]
fn sse_parser_handles_every_utf8_split_and_multiline_events() {
    let raw = "data: 中文🌊\r\ndata: 第二行\r\n\r\n: ping\n\ndata: [DONE]\n\n".as_bytes();
    for split in 0..=raw.len() {
        let mut decoder = SseDecoder::default();
        let mut events = decoder.push(&raw[..split]).unwrap();
        events.extend(decoder.push(&raw[split..]).unwrap());
        assert_eq!(events, vec!["中文🌊\n第二行", "[DONE]"]);
    }
    let mut decoder = SseDecoder::default();
    let mut events = Vec::new();
    for byte in raw {
        events.extend(decoder.push(&[*byte]).unwrap());
    }
    assert_eq!(events.len(), 2);
}
#[test]
fn sse_rejects_oversized_or_invalid_utf8_frames() {
    let mut d = SseDecoder::default();
    assert!(d.push(&vec![b'x'; 1_048_577]).is_err());
    let mut d = SseDecoder::default();
    assert!(d.push(b"data: \xff\n\n").is_err());
}
#[test]
fn official_request_is_documented_and_has_no_key_in_json() {
    let body = request_body(&ChatSettings::default(), &turns()).unwrap();
    assert_eq!(
        OFFICIAL_ENDPOINT,
        "https://api.deepseek.com/chat/completions"
    );
    assert_eq!(body["model"], "deepseek-flash");
    assert_eq!(body["stream"], true);
    assert_eq!(body["thinking"]["type"], "disabled");
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert!(!body.to_string().contains(FIXTURE_KEY));
}
#[tokio::test]
async fn real_loopback_http_stream_assembles_chinese_and_measured_usage() {
    let (endpoint, requests, task) = server(vec![success("你好🌊主人")]).await;
    let client = DeepSeekClient::fixture(endpoint, Duration::from_secs(2));
    let mut updates = Vec::new();
    let result = client
        .stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &CancellationToken::new(),
            |s| {
                updates.push(s.to_owned());
                true
            },
        )
        .await;
    assert!(result.error.is_none());
    assert_eq!(result.content, "你好🌊主人");
    assert_eq!(result.usage.unwrap().total_tokens, 12);
    assert_eq!(updates.last().unwrap(), "你好🌊主人");
    task.await.unwrap();
    let rows = requests.lock().unwrap();
    assert!(
        rows[0]
            .headers
            .starts_with("POST /chat/completions HTTP/1.1")
    );
    assert!(
        rows[0]
            .headers
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {FIXTURE_KEY}"))
    );
    assert_eq!(rows[0].body["messages"][1]["content"], "中文测试消息");
}
#[tokio::test]
async fn untrusted_http_error_body_cannot_leak_key_or_get_retried() {
    let (endpoint, requests, task) = server(vec![Plan {
        status: 401,
        extra: "Content-Type: application/json\r\n".into(),
        body: format!("invalid key {FIXTURE_KEY}").into_bytes(),
        stall: false,
    }])
    .await;
    let client = DeepSeekClient::fixture(endpoint, Duration::from_secs(1));
    let result = client
        .stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &CancellationToken::new(),
            |_| true,
        )
        .await;
    let error = result.error.unwrap();
    assert_eq!(error.code, "HTTP_401");
    assert!(!format!("{error:?} {error}").contains(FIXTURE_KEY));
    task.await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn explicitly_rejected_429_retries_once_without_duplicate_output() {
    let (endpoint, requests, task) = server(vec![
        Plan {
            status: 429,
            extra: "Retry-After: 0\r\n".into(),
            body: Vec::new(),
            stall: false,
        },
        success("限流后成功"),
    ])
    .await;
    let result = DeepSeekClient::fixture(endpoint, Duration::from_secs(1))
        .stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &CancellationToken::new(),
            |_| true,
        )
        .await;
    assert!(result.error.is_none());
    assert_eq!(result.content, "限流后成功");
    task.await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn cancelled_stream_keeps_partial_content_and_never_waits_for_eof() {
    let mut plan = success("半句");
    plan.body = sse("半句", false);
    plan.stall = true;
    let (endpoint, _, task) = server(vec![plan]).await;
    let client = DeepSeekClient::fixture(endpoint, Duration::from_secs(5));
    let token = CancellationToken::new();
    let cancel = token.clone();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client.stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &token,
            move |_| {
                cancel.cancel();
                true
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(result.content, "半句");
    assert_eq!(result.error.unwrap().code, "CANCELLED");
    task.abort();
}
#[tokio::test]
async fn idle_timeout_and_truncated_stream_are_not_success() {
    let mut plan = success("超时前半句");
    plan.body = sse("超时前半句", false);
    plan.stall = true;
    let (endpoint, _, task) = server(vec![plan]).await;
    let result = DeepSeekClient::fixture(endpoint, Duration::from_millis(80))
        .stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &CancellationToken::new(),
            |_| true,
        )
        .await;
    assert_eq!(result.error.unwrap().code, "TIMEOUT");
    assert_eq!(result.content, "超时前半句");
    task.abort();
    let (endpoint, _, task) = server(vec![Plan {
        status: 200,
        extra: "Content-Type: text/event-stream\r\n".into(),
        body: sse("不完整", false),
        stall: false,
    }])
    .await;
    let result = DeepSeekClient::fixture(endpoint, Duration::from_secs(1))
        .stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &CancellationToken::new(),
            |_| true,
        )
        .await;
    assert_eq!(result.error.unwrap().code, "STREAM_INCOMPLETE");
    task.await.unwrap();
}
#[tokio::test]
async fn redirects_are_rejected_without_forwarding_credentials() {
    let (endpoint, requests, task) = server(vec![Plan {
        status: 302,
        extra: "Location: http://127.0.0.1:1/steal\r\n".into(),
        body: Vec::new(),
        stall: false,
    }])
    .await;
    let result = DeepSeekClient::fixture(endpoint, Duration::from_secs(1))
        .stream(
            &secret(),
            &ChatSettings::default(),
            &turns(),
            &CancellationToken::new(),
            |_| true,
        )
        .await;
    assert_eq!(result.error.unwrap().code, "STREAM_INVALID");
    task.await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), 1);
}

struct MockVault(Mutex<Option<String>>);
impl SecretStore for MockVault {
    fn backend_name(&self) -> &'static str {
        "explicit mock vault"
    }
    fn get(&self) -> Result<Secret, VaultError> {
        Secret::new(self.0.lock().unwrap().clone().ok_or(VaultError::Missing)?)
    }
    fn set(&self, key: &Secret) -> Result<(), VaultError> {
        *self.0.lock().unwrap() = Some(key.expose().into());
        Ok(())
    }
    fn delete(&self) -> Result<(), VaultError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
fn input(conversation: &str, text: &str) -> StartChat {
    StartChat {
        request_id: Uuid::new_v4().to_string(),
        conversation_id: conversation.into(),
        text: Some(text.into()),
        regenerate_user_id: None,
    }
}
fn service(endpoint: String) -> (Arc<ChatService>, String) {
    let mut store = Store::open(":memory:").unwrap();
    let conversation = store.create_conversation("服务测试").unwrap().id;
    (
        Arc::new(ChatService::new(
            Arc::new(Mutex::new(store)),
            Arc::new(MockVault(Mutex::new(Some(FIXTURE_KEY.into())))),
            DeepSeekClient::fixture(endpoint, Duration::from_secs(2)),
        )),
        conversation,
    )
}
fn observer() -> (Observer, mpsc::UnboundedReceiver<ChatEvent>) {
    let (tx, rx) = mpsc::unbounded_channel();
    (Arc::new(move |event| tx.send(event).is_ok()), rx)
}
async fn final_event(rx: &mut mpsc::UnboundedReceiver<ChatEvent>) -> ChatMessage {
    tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(event) = rx.recv().await {
            match event {
                ChatEvent::Finished { message, .. } => return *message,
                ChatEvent::Failed { error, .. } => {
                    panic!("unexpected fixture persistence failure: {error}")
                }
                _ => {}
            }
        }
        panic!("fixture channel closed")
    })
    .await
    .unwrap()
}
async fn idle(service: &ChatService) {
    for _ in 0..100 {
        if service.active_request().is_none() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("service did not clear active request")
}
#[tokio::test]
async fn service_persists_stream_and_reuses_user_on_regenerate_with_clean_context() {
    let (endpoint, requests, task) = server(vec![
        success("旧版本回复"),
        success("新版本回复"),
        success("第二轮回答"),
    ])
    .await;
    let (service, c) = service(endpoint);
    let (observe, mut rx) = observer();
    let first = service.start(input(&c, "第一轮"), observe.clone()).unwrap();
    assert_eq!(final_event(&mut rx).await.state, "complete");
    idle(&service).await;
    let second = service
        .start(
            StartChat {
                request_id: Uuid::new_v4().to_string(),
                conversation_id: c.clone(),
                text: None,
                regenerate_user_id: Some(first.user_message_id.clone()),
            },
            observe.clone(),
        )
        .unwrap();
    final_event(&mut rx).await;
    idle(&service).await;
    service.start(input(&c, "继续聊"), observe).unwrap();
    final_event(&mut rx).await;
    idle(&service).await;
    {
        let store = service.store.lock().unwrap();
        let rows = store.chat_messages(&c).unwrap();
        assert_eq!(rows.iter().filter(|m| m.role == "user").count(), 2);
        assert_ne!(first.assistant_id, second.assistant_id);
        assert_eq!(store.usage_summary().unwrap().measured_requests, 3);
    }
    task.await.unwrap();
    let captured = requests.lock().unwrap();
    let context = captured[2].body["messages"].to_string();
    assert!(context.contains("新版本回复"));
    assert!(!context.contains("旧版本回复"));
    assert!(!context.contains(FIXTURE_KEY));
}
#[tokio::test]
async fn service_stop_is_durable_and_cannot_duplicate_user_message() {
    let mut plan = success("等待中");
    plan.body = sse("等待中", false);
    plan.stall = true;
    let (endpoint, _, task) = server(vec![plan]).await;
    let (service, c) = service(endpoint);
    let (observe, mut rx) = observer();
    let run = service
        .start(input(&c, "停止测试"), observe.clone())
        .unwrap();
    assert_eq!(
        service
            .start(input(&c, "重复发送"), observe)
            .unwrap_err()
            .code,
        "BUSY"
    );
    while let Some(event) = rx.recv().await {
        if matches!(event, ChatEvent::Delta { .. }) {
            break;
        }
    }
    assert_eq!(
        service.active_generation(),
        Some((run.request_id.clone(), c.clone()))
    );
    assert!(service.stop_and_wait(&run.request_id).await.unwrap());
    assert!(service.active_generation().is_none());
    assert!(!service.stop_and_wait(&run.request_id).await.unwrap());
    let final_message = final_event(&mut rx).await;
    assert_eq!(final_message.state, "interrupted");
    assert_eq!(final_message.content, "等待中");
    idle(&service).await;
    assert_eq!(
        service
            .store
            .lock()
            .unwrap()
            .chat_messages(&c)
            .unwrap()
            .iter()
            .filter(|m| m.role == "user")
            .count(),
        1
    );
    task.abort();
}
#[tokio::test]
async fn credential_delete_cancels_active_generation_and_removes_mock_key() {
    let mut plan = success("进行中");
    plan.body = sse("进行中", false);
    plan.stall = true;
    let (endpoint, _, task) = server(vec![plan]).await;
    let (service, c) = service(endpoint);
    let (observe, mut rx) = observer();
    service.start(input(&c, "删密钥测试"), observe).unwrap();
    while let Some(event) = rx.recv().await {
        if matches!(event, ChatEvent::Delta { .. }) {
            break;
        }
    }
    service.replace_key(None).await.unwrap();
    assert!(!service.vault.has().unwrap());
    assert_eq!(final_event(&mut rx).await.state, "interrupted");
    task.abort();
}
#[tokio::test]
async fn missing_key_has_explicit_failure_and_still_keeps_sent_user_text() {
    let (service, c) = service("http://127.0.0.1:1/chat/completions".into());
    service.replace_key(None).await.unwrap();
    let (observe, mut rx) = observer();
    service.start(input(&c, "保留用户消息"), observe).unwrap();
    let response = final_event(&mut rx).await;
    assert_eq!(response.error_code.as_deref(), Some("KEY_NOT_CONFIGURED"));
    assert_eq!(response.state, "failed");
    assert!(
        service
            .store
            .lock()
            .unwrap()
            .message_contents()
            .unwrap()
            .contains(&"保留用户消息".into())
    );
}

#[tokio::test]
async fn companion_context_reaches_the_actual_request_without_extra_analysis_calls() {
    let (endpoint, requests, task) = server(vec![success("只有这一条聊天模型请求")]).await;
    let (service, c) = service(endpoint);
    let persona = service.store.lock().unwrap().profile_id.clone();
    service
        .store
        .lock()
        .unwrap()
        .edit_companion_memory(
            &persona,
            &companion_core::companion::memory::MemoryEdit {
                id: None,
                expected_version: None,
                expected_revision: None,
                text: "我喜欢去海边看星星".into(),
                kind: "semantic".into(),
                importance_milli: 900,
                pinned: true,
                api_allowed: true,
            },
        )
        .unwrap();
    let (observe, mut events) = observer();
    service
        .start(input(&c, "星星适合什么时候看？谢谢你一直陪着我"), observe)
        .unwrap();
    assert_eq!(final_event(&mut events).await.state, "complete");
    idle(&service).await;
    task.await.unwrap();
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 1);
    let system = captured[0].body["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("初次相识"));
    assert!(system.contains("我喜欢去海边看星星"));
    assert!(system.contains("仅虚拟安排"));
    assert!(!system.contains(FIXTURE_KEY));
    let store = service.store.lock().unwrap();
    assert_eq!(store.bond_snapshot(&persona).unwrap().applied_events, 1);
    assert_eq!(store.usage_summary().unwrap().measured_requests, 1);
}

#[tokio::test]
async fn context_budget_rejection_keeps_input_and_closes_the_generation_without_network() {
    let (endpoint, requests, task) = server(vec![]).await;
    let (service, c) = service(endpoint);
    let persona = service.store.lock().unwrap().profile_id.clone();
    let settings = ChatSettings {
        personality: "甲".repeat(3000),
        ..Default::default()
    };
    service
        .store
        .lock()
        .unwrap()
        .save_persona_settings(&persona, &settings)
        .unwrap();
    let prefs = companion_core::companion::CompanionPreferences {
        context_budget_bytes: 8000,
        ..Default::default()
    };
    service
        .store
        .lock()
        .unwrap()
        .save_companion_preferences(&persona, &prefs)
        .unwrap();
    let (observe, _) = observer();
    let error = service
        .start(input(&c, "本次输入必须保留"), observe)
        .unwrap_err();
    assert_eq!(error.code, "CONTEXT_BUDGET_EXCEEDED");
    assert!(service.active_request().is_none());
    task.await.unwrap();
    assert!(requests.lock().unwrap().is_empty());
    assert!(
        service
            .store
            .lock()
            .unwrap()
            .chat_messages(&c)
            .unwrap()
            .iter()
            .any(|m| m.role == "user" && m.content == "本次输入必须保留")
    );
}

fn seed_life(service: &ChatService) -> String {
    use companion_core::companion::{CompanionPreferences, living::LifePlanInput};
    let mut s = service.store.lock().unwrap();
    s.set_device_platform("windows").unwrap();
    let p = s.active_persona_id().unwrap();
    s.save_companion_preferences(
        &p,
        &CompanionPreferences {
            proactive_enabled: true,
            proactive_model_enabled: true,
            proactive_token_budget: 20000,
            quiet_start_minute: 0,
            quiet_end_minute: 0,
            ..Default::default()
        },
    )
    .unwrap();
    s.create_life_plan(
        &p,
        &LifePlanInput {
            name: "有来源的虚拟计划".into(),
            kind: "create".into(),
            steps: 1,
            duration_ms: 30000,
        },
        1000000,
    )
    .unwrap();
    for n in 0..3 {
        s.advance_life_at(1000000 + n * 30000, 1000000 + n * 30000, true)
            .unwrap();
    }
    p
}
#[tokio::test]
async fn proactive_model_uses_real_loopback_http_and_records_measured_life_tokens_once() {
    let (endpoint, requests, task) = server(vec![success("合成分享：虚拟创作安排结束了。")]).await;
    let (service, _) = service(endpoint);
    let p = seed_life(&service);
    assert_eq!(service.life_tick(1090000, 1090000, true).await.unwrap(), 1);
    assert_eq!(service.life_tick(1120000, 1120000, true).await.unwrap(), 0);
    let request = requests.lock().unwrap();
    assert_eq!(request.len(), 1);
    assert_eq!(request[0].body["max_tokens"], 256);
    assert!(request[0].body.to_string().contains("virtual"));
    assert!(!request[0].body.to_string().contains(FIXTURE_KEY));
    drop(request);
    assert_eq!(
        service
            .store
            .lock()
            .unwrap()
            .life_usage_window(&p, 0, 2000000)
            .unwrap()["by_activity"][0]["prompt_tokens"],
        9
    );
    task.abort();
}
#[tokio::test]
async fn absent_key_and_disabled_proactive_never_open_provider_connection() {
    let (endpoint, requests, task) = server(vec![]).await;
    let (service, _) = service(endpoint);
    let p = seed_life(&service);
    service.vault.delete().unwrap();
    assert_eq!(service.life_tick(1090000, 1090000, true).await.unwrap(), 0);
    assert!(requests.lock().unwrap().is_empty());
    service.vault.set(&secret()).unwrap();
    {
        let mut s = service.store.lock().unwrap();
        let mut prefs = s.companion_preferences(&p).unwrap();
        prefs.proactive_enabled = false;
        s.save_companion_preferences(&p, &prefs).unwrap();
    }
    assert_eq!(service.life_tick(1120000, 1120000, true).await.unwrap(), 0);
    assert!(requests.lock().unwrap().is_empty());
    task.abort();
}

#[tokio::test]
async fn pet_chat_rejects_a_stale_persona_and_writes_to_the_shared_permanent_thread() {
    let (endpoint, requests, server_task) = server(vec![success("桌宠的实际测试回复")]).await;
    let (service, _) = service(endpoint);
    let (first, second) = {
        let mut store = service.store.lock().unwrap();
        let first = store.personas().unwrap().remove(0);
        let second = store.create_persona("另一位", "保持平静").unwrap();
        store.select_persona(&second.id).unwrap();
        (first, second)
    };
    let (observe, mut events) = observer();
    let stale = service
        .start_for_active_persona(
            input(&first.main_conversation_id, "这条草稿不应被误送"),
            &first.id,
            observe.clone(),
        )
        .unwrap_err();
    assert_eq!(stale.code, "PET_PERSONA_CHANGED");
    assert!(requests.lock().unwrap().is_empty());
    let prepared = service
        .start_for_active_persona(
            input(&second.main_conversation_id, "桌宠快聊实验"),
            &second.id,
            observe,
        )
        .unwrap();
    let finished = final_event(&mut events).await;
    assert_eq!(finished.id, prepared.assistant_id);
    server_task.await.unwrap();
    let store = service.store.lock().unwrap();
    assert!(
        store
            .chat_messages(&first.main_conversation_id)
            .unwrap()
            .is_empty()
    );
    let shared = store.chat_messages(&second.main_conversation_id).unwrap();
    assert_eq!(shared.iter().filter(|m| m.role == "user").count(), 1);
    assert!(
        shared
            .iter()
            .any(|m| m.id == finished.id && m.state == "complete")
    );
}
