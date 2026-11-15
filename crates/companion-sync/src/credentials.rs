//! Explicit, short-lived API-key handoff. Never part of ordinary replication.
use super::*;
use companion_secrets::VaultError;
use serde::{Deserializer, Serializer};
use sha2::Digest;
use subtle::ConstantTimeEq;

/// This is the only wire serializer for an API secret. The domain Secret itself
/// remains non-serializable and redacts Debug. Frame buffers are zeroized too.
pub(super) struct CredentialPacket(pub Secret);
impl Serialize for CredentialPacket {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.expose())
    }
}
impl<'de> Deserialize<'de> for CredentialPacket {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Secret::new(String::deserialize(deserializer)?)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("invalid credential format"))
    }
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid => "INVALID_SYNC_INPUT",
            Self::Address => "LAN_PRIVATE_IP_REQUIRED",
            Self::Limit => "SYNC_FRAME_LIMIT",
            Self::Offline => "PEER_UNREACHABLE",
            Self::Authentication => "PEER_AUTHENTICATION_FAILED",
            Self::Vault => "SECURE_STORAGE_UNAVAILABLE",
            Self::Crypto => "CRYPTOGRAPHY_UNAVAILABLE",
            Self::Discovery => "DISCOVERY_UNAVAILABLE",
            Self::Storage => "STORAGE_ERROR",
            Self::Version => "SYNC_VERSION_MISMATCH",
            Self::Pair => "PAIR_TOKEN_EXPIRED_OR_INVALID",
            Self::Confirmation => "WAITING_FOR_BOTH_CONFIRMATIONS",
            Self::Schema => "SYNC_DATA_SCHEMA_MISMATCH",
            Self::Conflict => "SYNC_DATA_CONFLICT",
            Self::Rejected => "SYNC_DATA_REJECTED",
            Self::Gap => "SYNC_CHECKPOINT_GAP",
            Self::KeyTransfer => "PAIR_KEY_TRANSFER_FAILED",
            Self::Core(
                companion_core::Error::Invalid("unknown sync field")
                | companion_core::Error::Schema,
            ) => "SYNC_DATA_SCHEMA_MISMATCH",
            Self::Core(companion_core::Error::Conflict) => "SYNC_DATA_CONFLICT",
            Self::Core(companion_core::Error::Invalid("CHECKPOINT_GAP")) => "SYNC_CHECKPOINT_GAP",
            Self::Core(companion_core::Error::Untrusted) => "PEER_AUTHENTICATION_FAILED",
            Self::Core(_) => "SYNC_DATA_REJECTED",
        }
    }
    pub(super) fn from_remote(code: &str) -> Self {
        match code {
            "SYNC_VERSION_MISMATCH" => Self::Version,
            "SYNC_DATA_SCHEMA_MISMATCH" => Self::Schema,
            "SYNC_DATA_CONFLICT" => Self::Conflict,
            "SYNC_CHECKPOINT_GAP" => Self::Gap,
            "PEER_AUTHENTICATION_FAILED" => Self::Authentication,
            "PAIR_TOKEN_EXPIRED_OR_INVALID" => Self::Pair,
            "SECURE_STORAGE_UNAVAILABLE" => Self::Vault,
            _ => Self::Rejected,
        }
    }
}
impl SyncService {
    pub fn configure_api_vault(&self, vault: Arc<dyn SecretStore>) -> Result<()> {
        *self.api_vault.lock().map_err(|_| Error::Storage)? = Some(vault);
        Ok(())
    }
    pub(super) fn api_key_snapshot(&self) -> Result<Option<Secret>> {
        let vault = self.api_vault.lock().map_err(|_| Error::Storage)?.clone();
        match vault.map(|v| v.get()) {
            Some(Ok(key)) => Ok(Some(key)),
            None | Some(Err(VaultError::Missing)) => Ok(None),
            _ => Err(Error::Vault),
        }
    }
    pub(super) fn credential_handoff(
        &self,
        session: &str,
        token: &str,
        id: &str,
        client: Option<&str>,
    ) -> Result<Reply> {
        // A bootstrap Pair reply never carries a secret. The second connection
        // must present the trusted phone's TLS certificate, plus its own QR nonce.
        let window = self.window.lock().map_err(|_| Error::Storage)?;
        let store = self.store.lock().map_err(|_| Error::Storage)?;
        let trusted = store
            .lan_peers()?
            .into_iter()
            .find(|p| p.id == id && !p.revoked)
            .ok_or(Error::Authentication)?;
        if client != Some(trusted.certificate.as_str()) {
            return Err(Error::Authentication);
        }
        let w = window.as_ref().ok_or(Error::Pair)?;
        let token = sha2::Sha256::digest(unhex(token)?);
        if !self.windows_host.load(Ordering::Acquire)
            || !w.approved
            || !w.committed
            || w.authority.is_none()
            || w.session != session
            || w.expires <= Instant::now()
            || w.attempts >= 5
            || !w.pending.as_ref().is_some_and(|p| p.id == id)
            || !bool::from(token.as_slice().ct_eq(&w.token_hash))
        {
            return Err(Error::Pair);
        }
        if w.api_key_unavailable {
            return Err(Error::Vault);
        }
        let key = w
            .api_key
            .as_ref()
            .map(|key| {
                Secret::new(key.expose().to_owned())
                    .map(CredentialPacket)
                    .map_err(|_| Error::Vault)
            })
            .transpose()?;
        Ok(Reply::Credentials { key })
    }
    pub(super) fn receive_api_configuration(
        &self,
        invite: &Invitation,
        endpoint: &str,
    ) -> Result<()> {
        let vault = self.api_vault.lock().map_err(|_| Error::Storage)?.clone();
        let state = if let Some(vault) = vault {
            let previous = match vault.get() {
                Ok(key) => Some(key),
                Err(VaultError::Missing) => None,
                Err(_) => {
                    self.key_state(&invite.identity.id, "secure_storage_error");
                    return Err(Error::KeyTransfer);
                }
            };
            if invite.version < 3 {
                "peer_update_required"
            } else {
                let result: Result<Reply> = transport::request(
                    &self.identity,
                    endpoint,
                    &invite.identity.certificate,
                    true,
                    &Request::CredentialHandoff {
                        session: invite.session.clone(),
                        token: invite.token.clone(),
                        id: self.identity.public.id.clone(),
                    },
                );
                match result {
                    Ok(Reply::Credentials { key: Some(packet) }) => {
                        if vault.set(&packet.0).is_err()
                            || !vault.get().ok().is_some_and(|stored| {
                                bool::from(
                                    stored
                                        .expose()
                                        .as_bytes()
                                        .ct_eq(packet.0.expose().as_bytes()),
                                )
                            })
                        {
                            if let Some(previous) = previous {
                                let _ = vault.set(&previous);
                            }
                            self.key_state(&invite.identity.id, "secure_storage_error");
                            return Err(Error::KeyTransfer);
                        }
                        if previous.is_some() {
                            "updated"
                        } else {
                            "received"
                        }
                    }
                    Ok(Reply::Credentials { key: None }) => "source_not_configured",
                    _ => {
                        self.key_state(&invite.identity.id, "transfer_failed");
                        return Err(Error::KeyTransfer);
                    }
                }
            }
        } else {
            "not_available"
        };
        self.key_state(&invite.identity.id, state);
        Ok(())
    }
    fn key_state(&self, id: &str, state: &str) {
        if let Ok(mut status) = self.api_configuration.lock() {
            status.insert(id.into(), state.into());
        }
    }
}
