use crate::{Error, Result};
use companion_secrets::{Secret, SecretStore};
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn unhex(value: &str) -> Result<Vec<u8>> {
    if value.len() > 16 * 1024 || !value.len().is_multiple_of(2) {
        return Err(Error::Invalid);
    }
    (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(value.get(i..i + 2).ok_or(Error::Invalid)?, 16)
                .map_err(|_| Error::Invalid)
        })
        .collect()
}
pub fn random() -> Result<String> {
    let mut bytes = [0; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Crypto)?;
    Ok(hex(&bytes))
}
pub fn fingerprint(cert: &str) -> Result<String> {
    Ok(hex(&Sha256::digest(unhex(cert)?)))
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PublicIdentity {
    pub id: String,
    pub name: String,
    pub certificate: String,
    pub signing_key: String,
}
impl PublicIdentity {
    pub fn validate(&self) -> Result<()> {
        uuid::Uuid::parse_str(&self.id).map_err(|_| Error::Invalid)?;
        if self.name.trim().is_empty()
            || self.name.len() > 100
            || unhex(&self.signing_key)?.len() != 32
            || unhex(&self.certificate)?.len() > 2048
        {
            return Err(Error::Invalid);
        }
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(unhex(
                &self.certificate,
            )?))
            .map_err(|_| Error::Authentication)?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
struct PrivateIdentity {
    tls: String,
    cert: String,
    seed: String,
}
pub struct Identity {
    pub public: PublicIdentity,
    pub(crate) key: Vec<u8>,
    signer: Ed25519KeyPair,
}
impl Identity {
    pub fn load(id: &str, name: &str, vault: &dyn SecretStore) -> Result<Self> {
        let private = match vault.get() {
            Ok(secret) => serde_json::from_str::<PrivateIdentity>(secret.expose())
                .map_err(|_| Error::Vault)?,
            Err(companion_secrets::VaultError::Missing) => {
                let certified = rcgen::generate_simple_self_signed(vec!["deepseek.local".into()])
                    .map_err(|_| Error::Crypto)?;
                let value = PrivateIdentity {
                    tls: hex(&certified.signing_key.serialize_der()),
                    cert: hex(certified.cert.der()),
                    seed: random()?,
                };
                vault
                    .set(
                        &Secret::new(serde_json::to_string(&value).map_err(|_| Error::Vault)?)
                            .map_err(|_| Error::Vault)?,
                    )
                    .map_err(|_| Error::Vault)?;
                value
            }
            Err(_) => return Err(Error::Vault),
        };
        let signer = Ed25519KeyPair::from_seed_unchecked(&unhex(&private.seed)?)
            .map_err(|_| Error::Crypto)?;
        let public = PublicIdentity {
            id: id.into(),
            name: name.into(),
            certificate: private.cert,
            signing_key: hex(signer.public_key().as_ref()),
        };
        public.validate()?;
        Ok(Self {
            public,
            key: unhex(&private.tls)?,
            signer,
        })
    }
    pub fn sign_tls(&self, bytes: &[u8]) -> Result<String> {
        use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair};
        let rng = SystemRandom::new();
        let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &self.key, &rng)
            .map_err(|_| Error::Crypto)?;
        Ok(hex(key
            .sign(&rng, bytes)
            .map_err(|_| Error::Crypto)?
            .as_ref()))
    }
    pub fn sign(&self, bytes: &[u8]) -> String {
        hex(self.signer.sign(bytes).as_ref())
    }
}
pub fn verify(public: &str, bytes: &[u8], signature: &str) -> Result<()> {
    UnparsedPublicKey::new(&ED25519, unhex(public)?)
        .verify(bytes, &unhex(signature)?)
        .map_err(|_| Error::Authentication)
}

/// First-pair proof of the TLS private key, bound to the same identity/nonce as Ed25519.
pub fn verify_tls(certificate: &str, bytes: &[u8], signature: &str) -> Result<()> {
    let der = unhex(certificate)?;
    let (remaining, cert) =
        x509_parser::parse_x509_certificate(&der).map_err(|_| Error::Authentication)?;
    if !remaining.is_empty() {
        return Err(Error::Authentication);
    }
    UnparsedPublicKey::new(
        &ring::signature::ECDSA_P256_SHA256_ASN1,
        cert.public_key().subject_public_key.data.as_ref(),
    )
    .verify(bytes, &unhex(signature)?)
    .map_err(|_| Error::Authentication)
}
