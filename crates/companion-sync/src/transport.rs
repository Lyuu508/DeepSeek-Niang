use crate::{
    Error, Result,
    identity::{Identity, unhex},
};
use rustls::{
    ClientConfig, RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::Arc,
    time::Duration,
};
pub const MAX_FRAME: usize = 4 * 1024 * 1024;
pub fn private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() && !ip.is_unspecified() && !ip.is_broadcast(),
        IpAddr::V6(_) => false,
    }
}
pub fn address(value: &str) -> Result<SocketAddr> {
    let addr: SocketAddr = value.parse().map_err(|_| Error::Address)?;
    if addr.port() == 0 || !(private(addr.ip()) || cfg!(test) && addr.ip().is_loopback()) {
        return Err(Error::Address);
    }
    Ok(addr)
}
fn cert(identity: &Identity) -> Result<Vec<CertificateDer<'static>>> {
    Ok(vec![CertificateDer::from(unhex(
        &identity.public.certificate,
    )?)])
}
fn key(identity: &Identity) -> PrivateKeyDer<'static> {
    PrivatePkcs8KeyDer::from(identity.key.clone()).into()
}
fn roots(certs: &[String]) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    for cert in certs {
        roots
            .add(CertificateDer::from(unhex(cert)?))
            .map_err(|_| Error::Authentication)?;
    }
    Ok(roots)
}
pub fn server_config(identity: &Identity, certs: &[String]) -> Result<Arc<ServerConfig>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| Error::Crypto)?;
    let config = if certs.is_empty() {
        builder.with_no_client_auth()
    } else {
        let verify = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots(certs)?),
            provider,
        )
        .allow_unauthenticated()
        .build()
        .map_err(|_| Error::Crypto)?;
        builder.with_client_cert_verifier(verify)
    }
    .with_single_cert(cert(identity)?, key(identity))
    .map_err(|_| Error::Crypto)?;
    Ok(Arc::new(config))
}
pub fn request<T: Serialize, R: DeserializeOwned>(
    identity: &Identity,
    endpoint: &str,
    certificate: &str,
    paired: bool,
    message: &T,
) -> Result<R> {
    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| Error::Crypto)?
            .with_root_certificates(roots(&[certificate.into()])?);
    let config = if paired {
        builder
            .with_client_auth_cert(cert(identity)?, key(identity))
            .map_err(|_| Error::Crypto)?
    } else {
        builder.with_no_client_auth()
    };
    let connection = rustls::ClientConnection::new(
        Arc::new(config),
        ServerName::try_from("deepseek.local").map_err(|_| Error::Crypto)?,
    )
    .map_err(|_| Error::Crypto)?;
    let stream = TcpStream::connect_timeout(&address(endpoint)?, Duration::from_secs(4))
        .map_err(|_| Error::Offline)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|_| Error::Offline)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .map_err(|_| Error::Offline)?;
    let mut tls = rustls::StreamOwned::new(connection, stream);
    write(&mut tls, message)?;
    read(&mut tls)
}
pub fn write<T: Serialize>(stream: &mut impl Write, value: &T) -> Result<()> {
    let data = serde_json::to_vec(value).map_err(|_| Error::Invalid)?;
    if data.len() > MAX_FRAME {
        return Err(Error::Limit);
    }
    stream
        .write_all(&(data.len() as u32).to_be_bytes())
        .and_then(|_| stream.write_all(&data))
        .and_then(|_| stream.flush())
        .map_err(|_| Error::Offline)
}
pub fn read<T: DeserializeOwned>(stream: &mut impl Read) -> Result<T> {
    let mut len = [0; 4];
    stream.read_exact(&mut len).map_err(|_| Error::Offline)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(Error::Limit);
    }
    let mut bytes = vec![0; len];
    stream.read_exact(&mut bytes).map_err(|_| Error::Offline)?;
    serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)
}
