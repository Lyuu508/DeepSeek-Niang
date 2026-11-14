use super::*;
use companion_core::{
    chat::{ChatSettings, StartChat},
    event::Entity,
};
use companion_secrets::{Secret, VaultError};
struct MemoryVault(Mutex<Option<String>>);
impl SecretStore for MemoryVault {
    fn backend_name(&self) -> &'static str {
        "Test memory vault"
    }
    fn get(&self) -> std::result::Result<Secret, VaultError> {
        Secret::new(self.0.lock().unwrap().clone().ok_or(VaultError::Missing)?)
    }
    fn set(&self, s: &Secret) -> std::result::Result<(), VaultError> {
        *self.0.lock().unwrap() = Some(s.expose().into());
        Ok(())
    }
    fn delete(&self) -> std::result::Result<(), VaultError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
struct NoDiscovery;
impl Discovery for NoDiscovery {
    fn start(&self, _: u16, _: &str) -> Result<()> {
        Ok(())
    }
    fn candidates(&self) -> Vec<Candidate> {
        vec![]
    }
    fn stop(&self) {}
}
fn service() -> Arc<SyncService> {
    service_named("fixture")
}
fn service_named(name: &str) -> Arc<SyncService> {
    SyncService::start(
        Arc::new(Mutex::new(Store::open(":memory:").unwrap())),
        Arc::new(MemoryVault(Mutex::new(None))),
        name,
        Arc::new(NoDiscovery),
    )
    .unwrap()
}
fn trust(a: &SyncService, b: &SyncService) {
    a.store
        .lock()
        .unwrap()
        .lan_trust(&peer(&b.identity.public, None))
        .unwrap();
}
fn transfer(a: &SyncService, b: &SyncService) -> Result<()> {
    loop {
        a.capture()?;
        let cursor = b.store.lock().unwrap().lan_cursor(&a.identity.public.id)?;
        let events = a.store.lock().unwrap().lan_export(cursor)?;
        if events.is_empty() {
            return Ok(());
        }
        b.import(&a.identity.public.id, &events)?;
    }
}
#[test]
fn two_independent_profiles_preserve_personas_chats_memories_and_offline_backfill() {
    let a = service();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    let persona = a
        .store
        .lock()
        .unwrap()
        .create_persona("星星", "喜欢天文")
        .unwrap();
    let memory = uuid::Uuid::new_v4().to_string();
    a.store
        .lock()
        .unwrap()
        .put_persona_memory(&persona.id, &memory, "A 的记忆")
        .unwrap();
    transfer(&a, &b).unwrap();
    transfer(&b, &a).unwrap();
    assert_eq!(b.store.lock().unwrap().personas().unwrap().len(), 3);
    b.store.lock().unwrap().select_persona(&persona.id).unwrap();
    assert_eq!(
        b.store
            .lock()
            .unwrap()
            .memories_for_persona(&persona.id)
            .unwrap()[0]
            .text,
        "A 的记忆"
    );
    // B is offline from A: local writes to the imported permanent persona still work.
    let request = StartChat {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: persona.main_conversation_id.clone(),
        text: Some("离线消息".into()),
        regenerate_user_id: None,
    };
    b.store
        .lock()
        .unwrap()
        .begin_chat(&request, &ChatSettings::default())
        .unwrap();
    b.store
        .lock()
        .unwrap()
        .finish_chat(&request.request_id, "", "interrupted", None, None, None)
        .unwrap();
    transfer(&b, &a).unwrap();
    transfer(&b, &a).unwrap();
    assert_eq!(
        a.store
            .lock()
            .unwrap()
            .chat_messages(&persona.main_conversation_id)
            .unwrap()
            .iter()
            .filter(|m| m.role == "user")
            .count(),
        1
    );
    b.store
        .lock()
        .unwrap()
        .delete_persona_memory(&persona.id, &memory)
        .unwrap();
    transfer(&b, &a).unwrap();
    assert!(
        a.store
            .lock()
            .unwrap()
            .memories_for_persona(&persona.id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        a.store.lock().unwrap().diagnostics().unwrap().integrity,
        "ok"
    );
    a.stop();
    b.stop();
}
#[test]
fn signatures_tamper_duplicates_gaps_unknown_origin_and_revocation() {
    let a = service();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    a.capture().unwrap();
    let events = a.store.lock().unwrap().lan_export(0).unwrap();
    assert!(!events.is_empty());
    let mut bad = events.clone();
    bad[0].clock += 1;
    assert!(b.import(&a.identity.public.id, &bad).is_err());
    assert!(b.import(&a.identity.public.id, &events[1..]).is_err());
    b.import(&a.identity.public.id, &events).unwrap();
    assert_eq!(b.import(&a.identity.public.id, &events).unwrap(), 0);
    b.revoke(&a.identity.public.id).unwrap();
    assert!(b.import(&a.identity.public.id, &events).is_err());
    let c = service();
    assert!(c.import(&a.identity.public.id, &events).is_err());
    a.stop();
    b.stop();
    c.stop();
}
#[test]
fn field_conflicts_keep_both_revisions_and_delete_always_wins() {
    let a = service();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    transfer(&a, &b).unwrap();
    transfer(&b, &a).unwrap();
    let persona = a.store.lock().unwrap().profile_id.clone();
    let mut pa = a.store.lock().unwrap().persona_settings(&persona).unwrap();
    let mut pb = pa.clone();
    pa.user_title = "甲".into();
    pb.personality = "乙的性格".into();
    a.store
        .lock()
        .unwrap()
        .save_persona_settings(&persona, &pa)
        .unwrap();
    b.store
        .lock()
        .unwrap()
        .save_persona_settings(&persona, &pb)
        .unwrap();
    transfer(&a, &b).unwrap();
    transfer(&b, &a).unwrap();
    for s in [&a, &b] {
        let p = s.store.lock().unwrap().persona_settings(&persona).unwrap();
        assert_eq!(p.user_title, "甲");
        assert_eq!(p.personality, "乙的性格");
    }
    let id = uuid::Uuid::new_v4().to_string();
    a.store
        .lock()
        .unwrap()
        .put_persona_memory(&persona, &id, "原文")
        .unwrap();
    transfer(&a, &b).unwrap();
    a.store
        .lock()
        .unwrap()
        .delete_persona_memory(&persona, &id)
        .unwrap();
    b.store
        .lock()
        .unwrap()
        .put_persona_memory(&persona, &id, "迟到编辑")
        .unwrap();
    transfer(&a, &b).unwrap();
    transfer(&b, &a).unwrap();
    for s in [&a, &b] {
        assert!(
            s.store
                .lock()
                .unwrap()
                .memories_for_persona(&persona)
                .unwrap()
                .is_empty()
        );
    }
    a.stop();
    b.stop();
}
#[test]
fn pair_requires_both_confirmations_token_expiry_and_private_addresses() {
    let a = service();
    let b = service();
    let code = a.invite().unwrap();
    assert!(code.svg.contains("<svg"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("phase2-qr.json"),
        serde_json::to_vec(&serde_json::json!({"text":code.text,"svg":code.svg})).unwrap(),
    )
    .unwrap();
    let invite: Invitation = serde_json::from_str(&code.text).unwrap();
    assert_eq!(unhex(&invite.token).unwrap().len(), 32);
    let signature = b.identity.sign(
        &pair_bytes(
            &invite.session,
            &invite.token,
            &b.identity.public,
            &a.identity.public.certificate,
            false,
        )
        .unwrap(),
    );
    assert!(matches!(
        a.handle(
            Request::Pair {
                session: invite.session.clone(),
                token: invite.token.clone(),
                identity: b.identity.public.clone(),
                signature,
                tls_signature: b
                    .identity
                    .sign_tls(
                        &pair_bytes(
                            &invite.session,
                            &invite.token,
                            &b.identity.public,
                            &a.identity.public.certificate,
                            false
                        )
                        .unwrap()
                    )
                    .unwrap(),
                confirm: false
            },
            None
        )
        .unwrap(),
        Reply::Waiting
    ));
    assert!(a.store.lock().unwrap().lan_peers().unwrap().is_empty());
    a.confirm(&b.identity.public.id).unwrap();
    assert!(a.store.lock().unwrap().lan_peers().unwrap().is_empty());
    let signature = b.identity.sign(
        &pair_bytes(
            &invite.session,
            &invite.token,
            &b.identity.public,
            &a.identity.public.certificate,
            true,
        )
        .unwrap(),
    );
    assert!(matches!(
        a.handle(
            Request::Pair {
                session: invite.session.clone(),
                token: invite.token.clone(),
                identity: b.identity.public.clone(),
                signature,
                tls_signature: b
                    .identity
                    .sign_tls(
                        &pair_bytes(
                            &invite.session,
                            &invite.token,
                            &b.identity.public,
                            &a.identity.public.certificate,
                            true
                        )
                        .unwrap()
                    )
                    .unwrap(),
                confirm: true
            },
            None
        )
        .unwrap(),
        Reply::Paired { .. }
    ));
    a.invite().unwrap();
    a.window.lock().unwrap().as_mut().unwrap().expires = Instant::now() - Duration::from_secs(1);
    assert!(
        a.handle(
            Request::Pair {
                session: "wrong".into(),
                token: random().unwrap(),
                identity: b.identity.public.clone(),
                signature: String::new(),
                tls_signature: String::new(),
                confirm: true
            },
            None
        )
        .is_err()
    );
    assert!(transport::address("8.8.8.8:443").is_err());
    assert!(transport::address("example.com:443").is_err());
    assert!(transport::address("192.168.1.5:1421").is_ok());
    a.stop();
    b.stop();
}
#[test]
fn accepted_nonblocking_socket_waits_for_real_tls_client() {
    // Windows accept inherits the listener's nonblocking mode. Force that mode
    // on every OS and delay ClientHello so a premature WouldBlock is observable.
    let server = service();
    let client = service();
    trust(&server, &client);
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = listener.local_addr().unwrap();
    let (ready, accepted) = std::sync::mpsc::channel();
    let incoming = server.clone();
    let worker = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();
        ready.send(()).unwrap();
        incoming.incoming(stream)
    });
    let socket = TcpStream::connect(endpoint).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    accepted.recv_timeout(Duration::from_secs(8)).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            unhex(&server.identity.public.certificate).unwrap(),
        ))
        .unwrap();
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_client_auth_cert(
        vec![CertificateDer::from(
            unhex(&client.identity.public.certificate).unwrap(),
        )],
        PrivatePkcs8KeyDer::from(client.identity.key.clone()).into(),
    )
    .unwrap();
    let connection = rustls::ClientConnection::new(
        Arc::new(config),
        ServerName::try_from("deepseek.local").unwrap(),
    )
    .unwrap();
    let mut tls = rustls::StreamOwned::new(connection, socket);
    transport::write(
        &mut tls,
        &Request::Exchange {
            id: client.identity.public.id.clone(),
            cursor: 0,
            events: vec![],
        },
    )
    .unwrap();
    let reply: Reply = transport::read(&mut tls).unwrap();
    assert!(matches!(reply, Reply::Batch { .. }));
    assert_eq!(
        tls.conn.protocol_version(),
        Some(rustls::ProtocolVersion::TLSv1_3)
    );
    worker.join().unwrap().unwrap();
    server.stop();
    client.stop();
}

#[test]
fn real_tls13_mutual_auth_pinning_and_reconnect() {
    let a = service();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    let address = format!("127.0.0.1:{}", a.port);
    b.exchange(&peer(&a.identity.public, Some(address.clone())), &address)
        .unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    b.store
        .lock()
        .unwrap()
        .put(
            Entity::Memory,
            &id,
            serde_json::json!({"text":"TLS 离线补传"}),
        )
        .unwrap();
    b.exchange(&peer(&a.identity.public, Some(address.clone())), &address)
        .unwrap();
    let foreign = service();
    let result: Result<Reply> = transport::request(
        &b.identity,
        &address,
        &foreign.identity.public.certificate,
        true,
        &Request::Exchange {
            id: b.identity.public.id.clone(),
            cursor: 0,
            events: vec![],
        },
    );
    assert!(result.is_err());
    let result: Reply = transport::request(
        &foreign.identity,
        &address,
        &a.identity.public.certificate,
        false,
        &Request::Exchange {
            id: b.identity.public.id.clone(),
            cursor: 0,
            events: vec![],
        },
    )
    .unwrap();
    assert!(matches!(result, Reply::Failure { .. }));
    a.revoke(&b.identity.public.id).unwrap();
    assert!(
        b.exchange(&peer(&a.identity.public, None), &address)
            .is_err()
    );
    a.stop();
    b.stop();
    foreign.stop();
}

#[test]
fn paginated_incremental_batches_ack_only_committed_data_and_survive_reopen() {
    let a = service();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    for i in 0..100 {
        a.store
            .lock()
            .unwrap()
            .put(
                Entity::Memory,
                &uuid::Uuid::new_v4().to_string(),
                serde_json::json!({"text":format!("分页记忆 {i}")}),
            )
            .unwrap();
    }
    a.capture().unwrap();
    let page = a.store.lock().unwrap().lan_export(0).unwrap();
    assert_eq!(page.len(), 64);
    let old = b
        .store
        .lock()
        .unwrap()
        .lan_cursor(&a.identity.public.id)
        .unwrap();
    let mut malformed = page.clone();
    malformed.last_mut().unwrap().signature = "00".repeat(64);
    assert!(b.import(&a.identity.public.id, &malformed).is_err());
    assert_eq!(
        b.store
            .lock()
            .unwrap()
            .lan_cursor(&a.identity.public.id)
            .unwrap(),
        old
    );
    transfer(&a, &b).unwrap();
    assert_eq!(
        b.store
            .lock()
            .unwrap()
            .memories_for_persona(&a.store.lock().unwrap().profile_id)
            .unwrap()
            .len(),
        100
    );
    a.stop();
    b.stop();
}
#[test]
fn concurrent_same_field_retains_losing_candidate_and_converges() {
    let a = service();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    transfer(&a, &b).unwrap();
    transfer(&b, &a).unwrap();
    let persona = a.store.lock().unwrap().profile_id.clone();
    for (s, name) in [(&a, "甲"), (&b, "乙")] {
        let mut prefs = s.store.lock().unwrap().persona_settings(&persona).unwrap();
        prefs.user_title = name.into();
        s.store
            .lock()
            .unwrap()
            .save_persona_settings(&persona, &prefs)
            .unwrap();
        s.capture().unwrap();
    }
    transfer(&a, &b).unwrap();
    transfer(&b, &a).unwrap();
    assert_eq!(
        a.store
            .lock()
            .unwrap()
            .persona_settings(&persona)
            .unwrap()
            .user_title,
        b.store
            .lock()
            .unwrap()
            .persona_settings(&persona)
            .unwrap()
            .user_title
    );
    for s in [&a, &b] {
        assert!(!s.store.lock().unwrap().lan_conflicts().unwrap().is_empty());
    }
    a.stop();
    b.stop();
}
#[test]
fn signing_identity_restores_from_same_secure_slot_and_secrets_do_not_enter_outbox() {
    let vault = MemoryVault(Mutex::new(None));
    let id = uuid::Uuid::new_v4().to_string();
    let a = Identity::load(&id, "A", &vault).unwrap();
    let b = Identity::load(&id, "A", &vault).unwrap();
    assert_eq!(a.public, b.public);
    verify(&a.public.signing_key, b"restore", &b.sign(b"restore")).unwrap();
    let s = service();
    s.capture().unwrap();
    let wire = serde_json::to_string(&s.store.lock().unwrap().lan_export(0).unwrap()).unwrap();
    assert!(!wire.contains("secret_ref"));
    assert!(!wire.contains("identity_public_key"));
    assert!(!wire.contains("credential_ref"));
    assert!(!wire.contains("lease_boot_id"));
    s.stop();
}

#[test]
fn actual_disconnect_restart_same_identity_and_offline_backfill() {
    let store = Arc::new(Mutex::new(Store::open(":memory:").unwrap()));
    let vault: Arc<dyn SecretStore> = Arc::new(MemoryVault(Mutex::new(None)));
    let a = SyncService::start(store.clone(), vault.clone(), "A", Arc::new(NoDiscovery)).unwrap();
    let b = service();
    trust(&a, &b);
    trust(&b, &a);
    let original = a.identity.public.clone();
    let endpoint = format!("127.0.0.1:{}", a.port);
    b.exchange(&peer(&original, None), &endpoint).unwrap();
    a.stop();
    std::thread::sleep(Duration::from_millis(250));
    assert!(b.exchange(&peer(&original, None), &endpoint).is_err());
    let persona = b.store.lock().unwrap().profile_id.clone();
    b.store
        .lock()
        .unwrap()
        .put_persona_memory(
            &persona,
            &uuid::Uuid::new_v4().to_string(),
            "断线期间的记忆",
        )
        .unwrap();
    let restarted = SyncService::start(store, vault, "A", Arc::new(NoDiscovery)).unwrap();
    assert_eq!(restarted.identity.public, original);
    let endpoint = format!("127.0.0.1:{}", restarted.port);
    b.exchange(&peer(&original, None), &endpoint).unwrap();
    assert_eq!(
        restarted
            .store
            .lock()
            .unwrap()
            .memories_for_persona(&persona)
            .unwrap()[0]
            .text,
        "断线期间的记忆"
    );
    restarted.stop();
    b.stop();
}

#[test]
fn copied_public_certificate_cannot_prove_its_tls_private_key() {
    let a = service();
    let b = service();
    let proof = b"DeepSeek-Niang/LAN/pair/v2/bound-identity";
    let signature = b.identity.sign_tls(proof).unwrap();
    assert!(identity::verify_tls(&a.identity.public.certificate, proof, &signature).is_err());
    identity::verify_tls(&b.identity.public.certificate, proof, &signature).unwrap();
    a.stop();
    b.stop();
}

#[test]
fn real_bootstrap_tls_pairing_both_confirmations_then_mutual_sync() {
    let a = service();
    let b = service();
    let code = a.invite().unwrap();
    let address = format!("127.0.0.1:{}", a.port);
    b.request_pair(&code.text, &address).unwrap();
    assert_eq!(a.status().unwrap().pending.len(), 1);
    assert_eq!(b.status().unwrap().pending.len(), 1);
    assert!(a.store.lock().unwrap().lan_peers().unwrap().is_empty());
    a.confirm(&b.identity.public.id).unwrap();
    b.confirm(&a.identity.public.id).unwrap();
    b.tick().unwrap();
    assert_eq!(a.store.lock().unwrap().lan_peers().unwrap().len(), 1);
    assert_eq!(b.store.lock().unwrap().lan_peers().unwrap().len(), 1);
    assert_eq!(a.store.lock().unwrap().personas().unwrap().len(), 2);
    assert_eq!(b.store.lock().unwrap().personas().unwrap().len(), 2);
    a.stop();
    b.stop();
}

#[test]
fn quick_scan_real_tls_pairs_once_and_syncs_without_extra_confirmation() {
    let pc = service_named("Windows 电脑");
    let phone = service_named("Android 手机");
    let persona = pc
        .store
        .lock()
        .unwrap()
        .create_persona("扫码空间", "合成测试")
        .unwrap();
    let address = format!("127.0.0.1:{}", pc.port);
    let code = pc.make_invite(vec![address], true).unwrap();
    assert!(pc.status().unwrap().pairing_code_active);
    phone.pair_scanned(&code.text).unwrap();
    phone.tick().unwrap();
    assert!(!pc.status().unwrap().pairing_code_active);
    assert!(
        phone
            .store
            .lock()
            .unwrap()
            .personas()
            .unwrap()
            .iter()
            .any(|p| p.id == persona.id)
    );
    let request = StartChat {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: persona.main_conversation_id.clone(),
        text: Some("扫码后的消息".into()),
        regenerate_user_id: None,
    };
    phone
        .store
        .lock()
        .unwrap()
        .begin_chat(&request, &ChatSettings::default())
        .unwrap();
    phone
        .store
        .lock()
        .unwrap()
        .finish_chat(&request.request_id, "", "interrupted", None, None, None)
        .unwrap();
    phone.tick().unwrap();
    assert!(
        pc.store
            .lock()
            .unwrap()
            .chat_messages(&persona.main_conversation_id)
            .unwrap()
            .iter()
            .any(|m| m.content == "扫码后的消息")
    );
    // A lost TLS reply may be retried by the same proved identity, not another phone.
    phone.pair_scanned(&code.text).unwrap();
    let other = service();
    assert!(other.pair_scanned(&code.text).is_err());
    pc.revoke(&phone.identity.public.id).unwrap();
    assert!(phone.pair_scanned(&code.text).is_err());
    let qr = pc
        .make_invite(vec![format!("192.168.1.20:{}", pc.port)], true)
        .unwrap();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/phase2-quick-qr.json");
    std::fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({"text":qr.text,"svg":qr.svg})).unwrap(),
    )
    .unwrap();
    pc.stop();
    phone.stop();
    other.stop();
}

#[test]
fn qr_quick_flag_cannot_replace_desktop_consent_in_legacy_pairing() {
    let pc = service();
    let phone = service();
    let legacy = pc.invite().unwrap();
    let mut qr: serde_json::Value = serde_json::from_str(&legacy.text).unwrap();
    assert!(qr.get("quick").is_none() && qr.get("addresses").is_none());
    qr["quick"] = serde_json::json!(true);
    qr["addresses"] = serde_json::json!([format!("127.0.0.1:{}", pc.port)]);
    assert!(matches!(
        phone.pair_scanned(&qr.to_string()),
        Err(Error::Confirmation)
    ));
    assert!(pc.store.lock().unwrap().lan_peers().unwrap().is_empty());
    assert!(phone.store.lock().unwrap().lan_peers().unwrap().is_empty());
    pc.stop();
    phone.stop();
}

#[test]
fn scan_validates_all_endpoints_before_any_network_request() {
    let pc = service();
    let phone = service();
    let code = pc
        .make_invite(vec![format!("127.0.0.1:{}", pc.port)], true)
        .unwrap();
    let original: serde_json::Value = serde_json::from_str(&code.text).unwrap();
    for invalid in [
        "8.8.8.8:443",
        "example.com:443",
        "0.0.0.0:80",
        "192.168.1.2:0",
    ] {
        let mut qr = original.clone();
        qr["addresses"] = serde_json::json!([format!("127.0.0.1:{}", pc.port), invalid]);
        assert!(matches!(
            phone.pair_scanned(&qr.to_string()),
            Err(Error::Address)
        ));
        assert!(pc.status().unwrap().pending.is_empty());
    }
    pc.stop();
    phone.stop();
}

#[test]
fn quick_qr_expiry_cancellation_and_attempt_limit_are_enforced() {
    let pc = service();
    let phone = service();
    let endpoints = vec![format!("127.0.0.1:{}", pc.port)];
    let code = pc.make_invite(endpoints.clone(), true).unwrap();
    pc.window.lock().unwrap().as_mut().unwrap().expires = Instant::now() - Duration::from_secs(1);
    assert!(phone.pair_scanned(&code.text).is_err());
    let code = pc.make_invite(endpoints.clone(), true).unwrap();
    pc.cancel_pair();
    assert!(phone.pair_scanned(&code.text).is_err());
    let code = pc.make_invite(endpoints, true).unwrap();
    let mut invalid: serde_json::Value = serde_json::from_str(&code.text).unwrap();
    invalid["token"] = serde_json::json!("00".repeat(32));
    for _ in 0..5 {
        assert!(phone.pair_scanned(&invalid.to_string()).is_err());
    }
    assert!(phone.pair_scanned(&code.text).is_err());
    pc.stop();
    phone.stop();
}
