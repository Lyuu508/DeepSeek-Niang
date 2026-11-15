//! LAN-only, consent-gated pairing and TLS 1.3 incremental replication.
pub mod discovery;
pub mod identity;
pub mod transport;
use companion_core::{
    pairing::HostAuthority,
    replication::{Change, Peer},
    storage::Store,
};
use companion_secrets::SecretStore;
use discovery::{Candidate, Discovery};
use identity::{Identity, PublicIdentity, fingerprint, hex, random, unhex, verify};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("INVALID_SYNC_INPUT")]
    Invalid,
    #[error("LAN_PRIVATE_IP_REQUIRED")]
    Address,
    #[error("SYNC_FRAME_LIMIT")]
    Limit,
    #[error("PEER_UNREACHABLE")]
    Offline,
    #[error("PEER_AUTHENTICATION_FAILED")]
    Authentication,
    #[error("SECURE_STORAGE_UNAVAILABLE")]
    Vault,
    #[error("CRYPTOGRAPHY_UNAVAILABLE")]
    Crypto,
    #[error("DISCOVERY_UNAVAILABLE")]
    Discovery,
    #[error("STORAGE_ERROR")]
    Storage,
    #[error("SYNC_VERSION_MISMATCH")]
    Version,
    #[error("PAIR_TOKEN_EXPIRED_OR_INVALID")]
    Pair,
    #[error("WAITING_FOR_BOTH_CONFIRMATIONS")]
    Confirmation,
    #[error("{0}")]
    Core(#[from] companion_core::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invitation {
    pub version: u32,
    pub session: String,
    pub identity: PublicIdentity,
    pub token: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quick: bool,
}
#[derive(Serialize)]
pub struct PairCode {
    pub text: String,
    pub svg: String,
    pub expires_in_seconds: u32,
}
struct PairWindow {
    authority: Option<HostAuthority>,
    session: String,
    token_hash: Vec<u8>,
    expires: Instant,
    attempts: u8,
    pending: Option<PublicIdentity>,
    approved: bool,
    committed: bool,
}
struct Outgoing {
    expires: Instant,
    invite: Invitation,
    address: String,
    approved: bool,
}
#[derive(Clone, Serialize)]
pub struct Pending {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
    pub direction: String,
    pub approved: bool,
}
#[derive(Clone, Serialize, Default)]
pub struct Progress {
    pub state: String,
    pub error: Option<String>,
    pub transferred: usize,
    pub last_attempt_ms: Option<i64>,
}
#[derive(Serialize)]
pub struct PeerStatus {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
    pub revoked: bool,
    pub address: Option<String>,
    pub last_success_ms: Option<i64>,
    pub pending: i64,
    pub progress: Progress,
}
#[derive(Serialize)]
pub struct Status {
    pub fingerprint: String,
    pub enabled: bool,
    pub device_id: String,
    pub name: String,
    pub port: u16,
    pub candidates: Vec<Candidate>,
    pub peers: Vec<PeerStatus>,
    pub pending: Vec<Pending>,
    pub conflicts: Vec<serde_json::Value>,
    pub discovery_error: Option<String>,
    pub pairing_code_active: bool,
    pub local_addresses: Vec<String>,
    pub pairing_session: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Request {
    Pair {
        session: String,
        token: String,
        identity: PublicIdentity,
        signature: String,
        tls_signature: String,
        confirm: bool,
    },
    Exchange {
        #[serde(default = "legacy_schema")]
        schema: u32,
        id: String,
        cursor: i64,
        events: Vec<Change>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Reply {
    Waiting,
    Paired {
        identity: PublicIdentity,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        authority: Option<HostAuthority>,
    },
    Batch {
        ack: i64,
        events: Vec<Change>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        authority: Option<HostAuthority>,
    },
    Failure {
        code: String,
    },
}
fn legacy_schema() -> u32 {
    4
}
fn pair_bytes(
    session: &str,
    token: &str,
    identity: &PublicIdentity,
    server: &str,
    confirm: bool,
) -> Result<Vec<u8>> {
    serde_json::to_vec(&(
        "DeepSeek-Niang/LAN/pair/v2",
        session,
        token,
        identity,
        server,
        confirm,
    ))
    .map_err(|_| Error::Invalid)
}
fn peer(identity: &PublicIdentity, address: Option<String>) -> Peer {
    Peer {
        id: identity.id.clone(),
        name: identity.name.clone(),
        certificate: identity.certificate.clone(),
        signing_key: identity.signing_key.clone(),
        revoked: false,
        address,
        last_success_ms: None,
        acknowledged_seq: 0,
    }
}
fn local_endpoints(port: u16) -> Vec<String> {
    let mut addresses = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| interface.is_oper_up())
        .map(|interface| interface.ip())
        .filter(|ip| transport::private(*ip))
        .map(|ip| format!("{ip}:{port}"))
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    addresses.truncate(8);
    addresses
}
pub struct SyncService {
    windows_host: AtomicBool,
    android_phone: AtomicBool,
    store: Arc<Mutex<Store>>,
    identity: Identity,
    discovery: Arc<dyn Discovery>,
    enabled: AtomicBool,
    port: u16,
    window: Mutex<Option<PairWindow>>,
    outgoing: Mutex<Option<Outgoing>>,
    progress: Mutex<BTreeMap<String, Progress>>,
    discovery_error: Mutex<Option<String>>,
    wake: AtomicBool,
}
impl SyncService {
    pub fn configure_platform(&self, windows_host: bool, android_phone: bool) {
        self.windows_host.store(windows_host, Ordering::Release);
        self.android_phone.store(android_phone, Ordering::Release);
    }
    pub fn start(
        store: Arc<Mutex<Store>>,
        vault: Arc<dyn SecretStore>,
        name: &str,
        discovery: Arc<dyn Discovery>,
    ) -> Result<Arc<Self>> {
        let id = store.lock().map_err(|_| Error::Storage)?.device_id.clone();
        if !store
            .lock()
            .map_err(|_| Error::Storage)?
            .lan_peers()?
            .is_empty()
            && !vault.has().map_err(|_| Error::Vault)?
        {
            return Err(Error::Vault);
        }
        let identity = Identity::load(&id, name, vault.as_ref())?;
        let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|_| Error::Offline)?;
        listener.set_nonblocking(true).map_err(|_| Error::Offline)?;
        let port = listener.local_addr().map_err(|_| Error::Offline)?.port();
        let service = Arc::new(Self {
            windows_host: AtomicBool::new(false),
            android_phone: AtomicBool::new(false),
            store,
            identity,
            discovery,
            enabled: AtomicBool::new(true),
            port,
            window: Mutex::new(None),
            outgoing: Mutex::new(None),
            progress: Mutex::new(BTreeMap::new()),
            discovery_error: Mutex::new(None),
            wake: AtomicBool::new(true),
        });
        let instance = uuid::Uuid::new_v4().to_string();
        if let Err(e) = service.discovery.start(port, &instance) {
            *service.discovery_error.lock().map_err(|_| Error::Storage)? = Some(e.to_string());
        }
        let weak = Arc::downgrade(&service);
        let active = Arc::new(AtomicUsize::new(0));
        std::thread::spawn(move || {
            loop {
                let Some(service) = weak.upgrade() else {
                    break;
                };
                if !service.enabled.load(Ordering::Acquire) {
                    break;
                }
                match listener.accept() {
                    Ok((stream, address)) => {
                        if !(transport::private(address.ip())
                            || cfg!(test) && address.ip().is_loopback())
                            || active.load(Ordering::Acquire) >= 8
                        {
                            continue;
                        }
                        let active = active.clone();
                        active.fetch_add(1, Ordering::AcqRel);
                        std::thread::spawn(move || {
                            let _ = service.incoming(stream);
                            active.fetch_sub(1, Ordering::AcqRel);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(100))
                    }
                    Err(_) => break,
                }
            }
        });
        let weak = Arc::downgrade(&service);
        std::thread::spawn(move || {
            let mut delay = 1u64;
            let mut next = Instant::now();
            loop {
                let Some(service) = weak.upgrade() else {
                    break;
                };
                if !service.enabled.load(Ordering::Acquire) {
                    break;
                }
                if service.wake.swap(false, Ordering::AcqRel) || Instant::now() >= next {
                    let outcome = service.tick();
                    delay = if outcome.is_ok() {
                        2
                    } else {
                        (delay * 2).min(30)
                    };
                    let jitter = ring::rand::SystemRandom::new();
                    let mut b = [0];
                    let _ = ring::rand::SecureRandom::fill(&jitter, &mut b);
                    next =
                        Instant::now() + Duration::from_millis(delay * 1000 + u64::from(b[0]) * 3);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        });
        Ok(service)
    }
    pub fn stop(&self) {
        self.enabled.store(false, Ordering::Release);
        self.discovery.stop();
        if let Ok(mut w) = self.window.lock() {
            *w = None;
        }
        if let Ok(mut o) = self.outgoing.lock() {
            *o = None;
        }
    }
    pub fn wake(&self) {
        self.wake.store(true, Ordering::Release);
    }
    pub fn status(&self) -> Result<Status> {
        let window = self.window.lock().map_err(|_| Error::Storage)?;
        let outgoing = self.outgoing.lock().map_err(|_| Error::Storage)?;
        let store = self.store.lock().map_err(|_| Error::Storage)?;
        let progresses = self.progress.lock().map_err(|_| Error::Storage)?;
        let peers = store
            .lan_peers()?
            .into_iter()
            .map(|p| {
                Ok(PeerStatus {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    fingerprint: fingerprint(&p.certificate)?,
                    revoked: p.revoked,
                    address: p.address.clone(),
                    last_success_ms: p.last_success_ms,
                    pending: store.lan_pending(&p)?,
                    progress: progresses.get(&p.id).cloned().unwrap_or(Progress {
                        state: "waiting_reconnect".into(),
                        ..Default::default()
                    }),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut pending = Vec::new();
        if let Some(w) = window
            .as_ref()
            .filter(|w| Instant::now() < w.expires && !w.committed)
            && let Some(p) = &w.pending
        {
            pending.push(Pending {
                id: p.id.clone(),
                name: p.name.clone(),
                fingerprint: fingerprint(&p.certificate)?,
                direction: "incoming".into(),
                approved: w.approved,
            });
        }
        if let Some(o) = outgoing.as_ref() {
            let p = &o.invite.identity;
            pending.push(Pending {
                id: p.id.clone(),
                name: p.name.clone(),
                fingerprint: fingerprint(&p.certificate)?,
                direction: "outgoing".into(),
                approved: o.approved,
            });
        }
        Ok(Status {
            fingerprint: fingerprint(&self.identity.public.certificate)?,
            enabled: self.enabled.load(Ordering::Acquire),
            device_id: self.identity.public.id.clone(),
            name: self.identity.public.name.clone(),
            port: self.port,
            candidates: self.discovery.candidates(),
            peers,
            pending,
            conflicts: store.lan_conflicts()?,
            discovery_error: self.discovery.error().or(self
                .discovery_error
                .lock()
                .map_err(|_| Error::Storage)?
                .clone()),
            pairing_code_active: window
                .as_ref()
                .is_some_and(|w| w.expires > Instant::now() && !w.committed),
            local_addresses: local_endpoints(self.port),
            pairing_session: window.as_ref().map(|w| w.session.clone()),
        })
    }
    pub fn invite(&self) -> Result<PairCode> {
        self.make_invite(Vec::new(), false)
    }
    /// The desktop user explicitly authorizes one scanner for two minutes by
    /// opening this QR. The phone consents by scanning it. Both private-key
    /// proofs, token validation and TLS pinning remain mandatory.
    pub fn invite_quick(&self) -> Result<PairCode> {
        let addresses = local_endpoints(self.port);
        self.make_invite(addresses, true)
    }
    fn make_invite(&self, mut addresses: Vec<String>, quick: bool) -> Result<PairCode> {
        addresses.sort();
        addresses.dedup();
        addresses.truncate(8);
        if quick && addresses.is_empty() {
            return Err(Error::Address);
        }
        let invite = Invitation {
            version: 2,
            session: uuid::Uuid::new_v4().to_string(),
            identity: self.identity.public.clone(),
            token: random()?,
            addresses,
            quick,
        };
        let text = serde_json::to_string(&invite).map_err(|_| Error::Invalid)?;
        let code = qrcode::QrCode::new(text.as_bytes()).map_err(|_| Error::Invalid)?;
        let width = code.width() + 8;
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" shape-rendering=\"crispEdges\" viewBox=\"0 0 {width} {width}\"><path fill=\"white\" d=\"M0 0h{width}v{width}H0z\"/><path fill=\"black\" d=\""
        );
        for y in 0..code.width() {
            for x in 0..code.width() {
                if code[(x, y)] == qrcode::Color::Dark {
                    svg.push_str(&format!("M{} {}h1v1h-1z", x + 4, y + 4));
                }
            }
        }
        svg.push_str("\"/></svg>");
        *self.window.lock().map_err(|_| Error::Storage)? = Some(PairWindow {
            authority: if quick {
                Some(
                    self.store
                        .lock()
                        .map_err(|_| Error::Storage)?
                        .local_host_authority()?,
                )
            } else {
                None
            },
            session: invite.session.clone(),
            token_hash: {
                use sha2::Digest;
                sha2::Sha256::digest(unhex(&invite.token)?).to_vec()
            },
            expires: Instant::now() + Duration::from_secs(120),
            attempts: 0,
            pending: None,
            approved: quick,
            committed: false,
        });
        Ok(PairCode {
            text,
            svg,
            expires_in_seconds: 120,
        })
    }
    pub fn cancel_pair(&self) {
        if let Ok(mut w) = self.window.lock() {
            *w = None;
        }
        if let Ok(mut o) = self.outgoing.lock() {
            *o = None;
        }
    }
    pub fn request_pair(&self, text: &str, endpoint: &str) -> Result<()> {
        if text.len() > 8192 {
            return Err(Error::Limit);
        }
        transport::address(endpoint)?;
        let invite: Invitation = serde_json::from_str(text).map_err(|_| Error::Invalid)?;
        invite.identity.validate()?;
        if invite.version != 2
            || invite.identity.id == self.identity.public.id
            || unhex(&invite.token)?.len() != 32
        {
            return Err(Error::Invalid);
        }
        let proof = pair_bytes(
            &invite.session,
            &invite.token,
            &self.identity.public,
            &invite.identity.certificate,
            false,
        )?;
        let signature = self.identity.sign(&proof);
        let tls_signature = self.identity.sign_tls(&proof)?;
        let reply: Reply = transport::request(
            &self.identity,
            endpoint,
            &invite.identity.certificate,
            false,
            &Request::Pair {
                session: invite.session.clone(),
                token: invite.token.clone(),
                identity: self.identity.public.clone(),
                signature,
                tls_signature,
                confirm: false,
            },
        )?;
        if !matches!(reply, Reply::Waiting) {
            return Err(Error::Pair);
        }
        *self.outgoing.lock().map_err(|_| Error::Storage)? = Some(Outgoing {
            expires: Instant::now() + Duration::from_secs(120),
            invite,
            address: endpoint.into(),
            approved: false,
        });
        Ok(())
    }
    /// Address candidates come only from the locally scanned invitation, never
    /// from a subnet scan. No data is trusted before a pinned TLS handshake.
    pub fn pair_scanned(&self, text: &str) -> Result<()> {
        if text.len() > 8192 {
            return Err(Error::Limit);
        }
        let invite: Invitation = serde_json::from_str(text).map_err(|_| Error::Invalid)?;
        invite.identity.validate()?;
        if invite.version != 2
            || !invite.quick
            || invite.identity.id == self.identity.public.id
            || unhex(&invite.token)?.len() != 32
            || invite.addresses.is_empty()
            || invite.addresses.len() > 8
        {
            return Err(Error::Invalid);
        }
        // Validate the complete list first: a malicious QR cannot mix private
        // endpoints with a public address or DNS destination.
        for endpoint in &invite.addresses {
            transport::address(endpoint)?;
        }
        let proof = pair_bytes(
            &invite.session,
            &invite.token,
            &self.identity.public,
            &invite.identity.certificate,
            true,
        )?;
        let request = Request::Pair {
            session: invite.session.clone(),
            token: invite.token.clone(),
            identity: self.identity.public.clone(),
            signature: self.identity.sign(&proof),
            tls_signature: self.identity.sign_tls(&proof)?,
            confirm: true,
        };
        let mut last = Error::Offline;
        for endpoint in &invite.addresses {
            let reply: Result<Reply> = transport::request(
                &self.identity,
                endpoint,
                &invite.identity.certificate,
                false,
                &request,
            );
            match reply {
                Ok(Reply::Paired {
                    identity,
                    authority,
                }) => {
                    if identity != invite.identity {
                        return Err(Error::Authentication);
                    }
                    let authority = authority.ok_or(Error::Invalid)?;
                    if authority.device_id != identity.id {
                        return Err(Error::Authentication);
                    }
                    let mut store = self.store.lock().map_err(|_| Error::Storage)?;
                    store.lan_trust(&peer(&identity, Some(endpoint.clone())))?;
                    store.bind_paired_host(&authority)?;
                    drop(store);
                    self.set_progress(&identity.id, "syncing", None, 0);
                    self.wake();
                    return Ok(());
                }
                Ok(Reply::Waiting) => return Err(Error::Confirmation),
                Ok(_) => return Err(Error::Pair),
                Err(error) => last = error,
            }
        }
        Err(last)
    }
    pub fn confirm(&self, id: &str) -> Result<()> {
        if let Some(w) = self.window.lock().map_err(|_| Error::Storage)?.as_mut()
            && w.expires > Instant::now()
            && w.pending.as_ref().is_some_and(|p| p.id == id)
        {
            w.approved = true;
        }
        if let Some(o) = self.outgoing.lock().map_err(|_| Error::Storage)?.as_mut()
            && o.invite.identity.id == id
        {
            o.approved = true;
        }
        self.wake();
        Ok(())
    }
    pub fn revoke(&self, id: &str) -> Result<()> {
        self.store
            .lock()
            .map_err(|_| Error::Storage)?
            .lan_revoke(id)?;
        self.set_progress(id, "revoked", None, 0);
        Ok(())
    }
    pub fn reconnect(&self, id: &str, endpoint: &str) -> Result<()> {
        transport::address(endpoint)?;
        self.store
            .lock()
            .map_err(|_| Error::Storage)?
            .lan_address(id, endpoint)?;
        self.wake();
        Ok(())
    }
    fn set_progress(&self, id: &str, state: &str, error: Option<String>, transferred: usize) {
        if let Ok(mut p) = self.progress.lock() {
            p.insert(
                id.into(),
                Progress {
                    state: state.into(),
                    error,
                    transferred,
                    last_attempt_ms: Some(companion_core::chat::now_ms()),
                },
            );
        }
    }
    fn capture(&self) -> Result<()> {
        for _ in 0..64 {
            let mut store = self.store.lock().map_err(|_| Error::Storage)?;
            store.lan_capture(|bytes| Ok(self.identity.sign(bytes)))?;
            if store.lan_outbox_count()? == 0 {
                return Ok(());
            }
        }
        Err(Error::Limit)
    }
    fn import(&self, id: &str, events: &[Change]) -> Result<usize> {
        if !self.enabled.load(Ordering::Acquire) {
            return Err(Error::Authentication);
        }
        self.capture()?;
        let mut store = self.store.lock().map_err(|_| Error::Storage)?;
        let p = store
            .lan_peers()?
            .into_iter()
            .find(|p| p.id == id && !p.revoked)
            .ok_or(Error::Authentication)?;
        for e in events {
            if e.origin != id {
                return Err(Error::Authentication);
            }
            verify(&p.signing_key, &e.signing_bytes()?, &e.signature)?;
        }
        Ok(store.lan_import(id, events)?)
    }
    fn incoming(&self, stream: TcpStream) -> Result<()> {
        // Windows accepted sockets inherit the nonblocking listener's mode.
        // This worker uses blocking rustls I/O with bounded read/write timeouts;
        // leaving that mode inherited makes a delayed ClientHello fail Offline.
        stream.set_nonblocking(false).map_err(|_| Error::Offline)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(8)))
            .map_err(|_| Error::Offline)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(8)))
            .map_err(|_| Error::Offline)?;
        let certs = self
            .store
            .lock()
            .map_err(|_| Error::Storage)?
            .lan_peers()?
            .into_iter()
            .filter(|p| !p.revoked)
            .map(|p| p.certificate)
            .collect::<Vec<_>>();
        let connection =
            rustls::ServerConnection::new(transport::server_config(&self.identity, &certs)?)
                .map_err(|_| Error::Crypto)?;
        let mut tls = rustls::StreamOwned::new(connection, stream);
        let request: Request = transport::read(&mut tls)?;
        let client = tls
            .conn
            .peer_certificates()
            .and_then(|c| c.first())
            .map(|c| hex(c.as_ref()));
        let reply = match self.handle(request, client) {
            Ok(reply) => reply,
            Err(error) => Reply::Failure {
                code: error.to_string(),
            },
        };
        transport::write(&mut tls, &reply)
    }
    fn handle(&self, request: Request, client: Option<String>) -> Result<Reply> {
        if !self.enabled.load(Ordering::Acquire) {
            return Err(Error::Authentication);
        }
        match request {
            Request::Pair {
                session,
                token,
                identity,
                signature,
                tls_signature,
                confirm,
            } => {
                identity.validate()?;
                let mut lock = self.window.lock().map_err(|_| Error::Storage)?;
                let w = lock.as_mut().ok_or(Error::Pair)?;
                if Instant::now() >= w.expires || w.attempts >= 5 {
                    return Err(Error::Pair);
                }
                // Constant-time token comparison, and proof bound to the pinned server certificate.
                let matched = unhex(&token).and_then(|t| {
                    use sha2::Digest;
                    let expected = &w.token_hash;
                    let t = sha2::Sha256::digest(&t);
                    {
                        use subtle::ConstantTimeEq;
                        if bool::from(t.ct_eq(expected)) {
                            Ok(())
                        } else {
                            Err(Error::Pair)
                        }
                    }
                });
                let proof = pair_bytes(
                    &session,
                    &token,
                    &identity,
                    &self.identity.public.certificate,
                    confirm,
                )?;
                if session != w.session
                    || matched.is_err()
                    || identity::verify_tls(&identity.certificate, &proof, &tls_signature).is_err()
                    || verify(
                        &identity.signing_key,
                        &pair_bytes(
                            &session,
                            &token,
                            &identity,
                            &self.identity.public.certificate,
                            confirm,
                        )?,
                        &signature,
                    )
                    .is_err()
                {
                    w.attempts += 1;
                    return Err(Error::Pair);
                }
                if w.pending.as_ref().is_some_and(|p| p != &identity) {
                    return Err(Error::Pair);
                }
                w.pending = Some(identity.clone());
                if !confirm || !w.approved {
                    return Ok(Reply::Waiting);
                }
                if !w.committed {
                    self.store
                        .lock()
                        .map_err(|_| Error::Storage)?
                        .lan_trust(&peer(&identity, None))?;
                    w.committed = true;
                } else if !self
                    .store
                    .lock()
                    .map_err(|_| Error::Storage)?
                    .lan_peers()?
                    .iter()
                    .any(|p| p.id == identity.id && !p.revoked)
                {
                    // A consumed code can retry a lost reply for the same peer,
                    // but it must never undo an explicit subsequent revocation.
                    return Err(Error::Authentication);
                }
                Ok(Reply::Paired {
                    identity: self.identity.public.clone(),
                    authority: w.authority.clone(),
                })
            }
            Request::Exchange {
                id,
                cursor,
                events,
                schema,
            } => {
                if schema != companion_core::storage::SCHEMA_VERSION {
                    return Err(Error::Version);
                }
                let p = self
                    .store
                    .lock()
                    .map_err(|_| Error::Storage)?
                    .lan_peers()?
                    .into_iter()
                    .find(|p| p.id == id && !p.revoked)
                    .ok_or(Error::Authentication)?;
                if client.as_deref() != Some(p.certificate.as_str()) || cursor < 0 {
                    return Err(Error::Authentication);
                }
                self.capture()?;
                self.set_progress(&id, "authenticating", None, 0);
                let count = self.import(&id, &events)?;
                let mut store = self.store.lock().map_err(|_| Error::Storage)?;
                // Revocation is checked again while holding the same lock used for ACK/export.
                if !store.lan_peers()?.iter().any(|p| p.id == id && !p.revoked) {
                    return Err(Error::Authentication);
                }
                if !self.enabled.load(Ordering::Acquire) {
                    return Err(Error::Authentication);
                }
                let latest = store.lan_cursor(&self.identity.public.id)?;
                if cursor > latest {
                    return Err(Error::Invalid);
                }
                let outgoing = store.lan_export(cursor)?;
                let ack = store.lan_cursor(&id)?;
                if outgoing.is_empty() && events.is_empty() {
                    store.lan_success(&id, cursor)?;
                    self.set_progress(&id, "succeeded", None, count);
                } else {
                    self.set_progress(&id, "syncing", None, count + outgoing.len());
                }
                Ok(Reply::Batch {
                    ack,
                    events: outgoing,
                    authority: if self.windows_host.load(Ordering::Acquire) {
                        Some(store.local_host_authority()?)
                    } else {
                        None
                    },
                })
            }
        }
    }
    fn tick(&self) -> Result<()> {
        {
            let mut w = self.window.lock().map_err(|_| Error::Storage)?;
            if w.as_ref().is_some_and(|w| w.expires <= Instant::now()) {
                *w = None;
            }
        }
        {
            let mut o = self.outgoing.lock().map_err(|_| Error::Storage)?;
            if o.as_ref().is_some_and(|o| o.expires <= Instant::now()) {
                *o = None;
            }
        }
        self.capture()?;
        // No store locks are held during network I/O or user confirmation.
        let outgoing = {
            let o = self.outgoing.lock().map_err(|_| Error::Storage)?;
            o.as_ref()
                .filter(|o| o.approved)
                .map(|o| (o.invite.clone(), o.address.clone()))
        };
        if let Some((invite, address)) = outgoing {
            let proof = pair_bytes(
                &invite.session,
                &invite.token,
                &self.identity.public,
                &invite.identity.certificate,
                true,
            )?;
            let signature = self.identity.sign(&proof);
            let tls_signature = self.identity.sign_tls(&proof)?;
            let result: Result<Reply> = transport::request(
                &self.identity,
                &address,
                &invite.identity.certificate,
                false,
                &Request::Pair {
                    session: invite.session,
                    token: invite.token,
                    identity: self.identity.public.clone(),
                    signature,
                    tls_signature,
                    confirm: true,
                },
            );
            if let Ok(Reply::Paired { identity, .. }) = result {
                if identity != invite.identity {
                    return Err(Error::Authentication);
                }
                self.store
                    .lock()
                    .map_err(|_| Error::Storage)?
                    .lan_trust(&peer(&identity, Some(address)))?;
                *self.outgoing.lock().map_err(|_| Error::Storage)? = None;
            }
        }
        let peers = self.store.lock().map_err(|_| Error::Storage)?.lan_peers()?;
        let candidates = self.discovery.candidates();
        let mut failure = false;
        for p in peers.into_iter().filter(|p| !p.revoked) {
            let mut addresses = p
                .address
                .iter()
                .cloned()
                .chain(candidates.iter().map(|c| c.address.clone()))
                .collect::<Vec<_>>();
            addresses.sort();
            addresses.dedup();
            let mut result = Err(Error::Offline);
            for address in addresses {
                if transport::address(&address).is_err() {
                    continue;
                }
                result = self.exchange(&p, &address);
                if result.is_ok() {
                    break;
                }
            }
            if let Err(error) = result {
                failure = true;
                self.set_progress(&p.id, "waiting_reconnect", Some(error.to_string()), 0);
            }
        }
        if failure { Err(Error::Offline) } else { Ok(()) }
    }
    fn exchange(&self, p: &Peer, address: &str) -> Result<()> {
        self.set_progress(&p.id, "connecting", None, 0);
        let mut sent = 0;
        for _ in 0..128 {
            if !self.enabled.load(Ordering::Acquire) {
                return Err(Error::Authentication);
            }
            self.capture()?;
            let (cursor, events) = {
                let s = self.store.lock().map_err(|_| Error::Storage)?;
                let p = s
                    .lan_peers()?
                    .into_iter()
                    .find(|v| v.id == p.id && !v.revoked)
                    .ok_or(Error::Authentication)?;
                (s.lan_cursor(&p.id)?, s.lan_export(p.acknowledged_seq)?)
            };
            let empty = events.is_empty();
            let length = events.len();
            let reply: Reply = transport::request(
                &self.identity,
                address,
                &p.certificate,
                true,
                &Request::Exchange {
                    schema: companion_core::storage::SCHEMA_VERSION,
                    id: self.identity.public.id.clone(),
                    cursor,
                    events,
                },
            )?;
            match reply {
                Reply::Batch {
                    ack,
                    events,
                    authority,
                } => {
                    if self.android_phone.load(Ordering::Acquire)
                        && let Some(authority) = authority
                    {
                        if authority.device_id != p.id {
                            return Err(Error::Authentication);
                        }
                        self.store
                            .lock()
                            .map_err(|_| Error::Storage)?
                            .bind_paired_host(&authority)?;
                    }
                    let received = events.len();
                    self.import(&p.id, &events)?;
                    let mut s = self.store.lock().map_err(|_| Error::Storage)?;
                    if ack > s.lan_cursor(&self.identity.public.id)? || ack < p.acknowledged_seq {
                        return Err(Error::Invalid);
                    }
                    s.lan_address(&p.id, address)?;
                    // ACK only updates retention metadata. Success requires both directions empty and committed.
                    s.lan_ack(&p.id, ack)?;
                    sent += length + received;
                    if empty && received == 0 {
                        s.lan_success(&p.id, ack)?;
                        self.set_progress(&p.id, "succeeded", None, sent);
                        return Ok(());
                    }
                    self.set_progress(&p.id, "syncing", None, sent);
                }
                Reply::Failure { code } if code == "SYNC_VERSION_MISMATCH" => {
                    return Err(Error::Version);
                }
                Reply::Failure { .. } => return Err(Error::Authentication),
                _ => return Err(Error::Invalid),
            }
        }
        Err(Error::Limit)
    }
}
#[cfg(test)]
mod tests;
