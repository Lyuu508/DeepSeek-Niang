//! Native-only credentials. Secret has neither Serialize nor plaintext Debug.
use std::fmt;
use zeroize::Zeroizing;

pub struct Secret(Zeroizing<String>);
impl Secret {
    pub fn new(value: String) -> Result<Self, VaultError> {
        let value = Zeroizing::new(value);
        if value.len() < 8 || value.len() > 2048 || !value.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(VaultError::InvalidKey);
        }
        Ok(Self(value))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VaultError {
    #[error("KEY_NOT_CONFIGURED")]
    Missing,
    #[error("KEY_FORMAT_INVALID")]
    InvalidKey,
    #[error("SECURE_STORAGE_UNSUPPORTED")]
    Unsupported,
    #[error("SECURE_STORAGE_UNAVAILABLE")]
    Unavailable,
    #[error("SECURE_STORAGE_CORRUPT")]
    Corrupt,
}
pub trait SecretStore: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn get(&self) -> Result<Secret, VaultError>;
    fn set(&self, key: &Secret) -> Result<(), VaultError>;
    fn delete(&self) -> Result<(), VaultError>;
    fn has(&self) -> Result<bool, VaultError> {
        match self.get() {
            Ok(_) => Ok(true),
            Err(VaultError::Missing) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
pub struct UnsupportedVault;
impl SecretStore for UnsupportedVault {
    fn backend_name(&self) -> &'static str {
        "unsupported"
    }
    fn get(&self) -> Result<Secret, VaultError> {
        Err(VaultError::Unsupported)
    }
    fn set(&self, _: &Secret) -> Result<(), VaultError> {
        Err(VaultError::Unsupported)
    }
    fn delete(&self) -> Result<(), VaultError> {
        Err(VaultError::Unsupported)
    }
}
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::WindowsVault;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_and_errors_never_contain_secret() {
        let s = Secret::new("fixture-key-no-real-credentials".into()).unwrap();
        assert_eq!(format!("{s:?}"), "Secret([REDACTED])");
        assert!(!format!("{}", VaultError::Unavailable).contains(s.expose()));
    }
    #[test]
    fn rejects_header_injection_and_whitespace() {
        for key in [
            "short",
            "fixture\r\nAuthorization:evil",
            "fixture key has spaces",
        ] {
            assert!(Secret::new(key.into()).is_err());
        }
    }
    #[test]
    fn unsupported_platform_fails_closed() {
        let s = UnsupportedVault;
        assert_eq!(s.has(), Err(VaultError::Unsupported));
        assert!(
            s.set(&Secret::new("fixture-only-key".into()).unwrap())
                .is_err()
        );
    }
    #[cfg(windows)]
    #[test]
    fn windows_real_credential_manager_roundtrip() {
        let v = WindowsVault::new(&uuid::Uuid::new_v4().to_string()).unwrap();
        let key = Secret::new("fixture-only-windows-credential-roundtrip".into()).unwrap();
        v.set(&key).unwrap();
        assert_eq!(v.get().unwrap().expose(), key.expose());
        v.delete().unwrap();
        assert!(!v.has().unwrap());
    }
}
