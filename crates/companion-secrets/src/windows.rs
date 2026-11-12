//! Windows Generic Credential API; the OS protects the blob for the current account.
use super::{Secret, SecretStore, VaultError};
use std::{ptr, slice};
use windows_sys::Win32::{
    Foundation::{ERROR_NOT_FOUND, GetLastError},
    Security::Credentials::*,
};

pub struct WindowsVault {
    target: Vec<u16>,
    user: Vec<u16>,
}
impl WindowsVault {
    pub fn new(profile: &str) -> Result<Self, VaultError> {
        uuid::Uuid::parse_str(profile).map_err(|_| VaultError::Unavailable)?;
        Ok(Self {
            target: format!("io.local.deepseekcompanion/deepseek/{profile}")
                .encode_utf16()
                .chain(Some(0))
                .collect(),
            user: "DeepSeek娘".encode_utf16().chain(Some(0)).collect(),
        })
    }
}
struct OwnedCredential(*mut CREDENTIALW);
impl Drop for OwnedCredential {
    fn drop(&mut self) {
        unsafe {
            CredFree(self.0.cast());
        }
    }
}
impl SecretStore for WindowsVault {
    fn backend_name(&self) -> &'static str {
        "Windows Credential Manager"
    }
    fn set(&self, key: &Secret) -> Result<(), VaultError> {
        // CREDENTIALW consists of C integers and nullable pointers; zero initialization is valid.
        let mut cred: CREDENTIALW = unsafe { std::mem::zeroed() };
        cred.Type = CRED_TYPE_GENERIC;
        cred.TargetName = self.target.as_ptr().cast_mut();
        cred.CredentialBlobSize = key.expose().len() as u32;
        cred.CredentialBlob = key.expose().as_ptr().cast_mut();
        cred.Persist = CRED_PERSIST_LOCAL_MACHINE;
        cred.UserName = self.user.as_ptr().cast_mut();
        // Windows reads the buffers during this call, without retaining these pointers.
        if unsafe { CredWriteW(&cred, 0) } == 0 {
            return Err(VaultError::Unavailable);
        }
        Ok(())
    }
    fn get(&self) -> Result<Secret, VaultError> {
        let mut value = ptr::null_mut();
        if unsafe { CredReadW(self.target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut value) } == 0 {
            return Err(if unsafe { GetLastError() } == ERROR_NOT_FOUND {
                VaultError::Missing
            } else {
                VaultError::Unavailable
            });
        }
        if value.is_null() {
            return Err(VaultError::Corrupt);
        }
        let owned = OwnedCredential(value);
        let cred = unsafe { &*owned.0 };
        if cred.CredentialBlobSize == 0
            || cred.CredentialBlobSize > 2048
            || cred.CredentialBlob.is_null()
        {
            return Err(VaultError::Corrupt);
        }
        let bytes =
            unsafe { slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize) };
        let text = std::str::from_utf8(bytes).map_err(|_| VaultError::Corrupt)?;
        let result = Secret::new(text.to_owned()).map_err(|_| VaultError::Corrupt);
        // Scrub the OS-returned allocation before CredFree; never include it in error strings.
        for index in 0..cred.CredentialBlobSize as usize {
            unsafe {
                std::ptr::write_volatile(cred.CredentialBlob.add(index), 0);
            }
        }
        result
    }
    fn delete(&self) -> Result<(), VaultError> {
        if unsafe { CredDeleteW(self.target.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && unsafe { GetLastError() } != ERROR_NOT_FOUND
        {
            return Err(VaultError::Unavailable);
        }
        Ok(())
    }
}
