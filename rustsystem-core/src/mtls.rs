use std::io::BufReader;
use std::sync::Arc;

use rustls::server::WebPkiClientVerifier;
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use rustls_pemfile::{certs, private_key};

use std::time::Duration;

use crate::config::ConfigError;

/// Every service-to-service call gives up after this long, so a hung peer can never hold a
/// meeting's lock indefinitely.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

fn err(what: &str) -> impl Fn(rustls::Error) -> ConfigError + '_ {
    move |e| ConfigError(format!("mTLS: {what}: {e}"))
}

fn load_certs(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, ConfigError> {
    let mut reader = BufReader::new(pem);
    let certs = certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ConfigError(format!("mTLS: unreadable certificate PEM: {e}")))?;
    if certs.is_empty() {
        return Err(ConfigError("mTLS: no certificate found in PEM".into()));
    }
    Ok(certs)
}

fn load_private_key(pem: &[u8]) -> Result<PrivateKeyDer<'static>, ConfigError> {
    let mut reader = BufReader::new(pem);
    private_key(&mut reader)
        .map_err(|e| ConfigError(format!("mTLS: unreadable private key PEM: {e}")))?
        .ok_or_else(|| ConfigError("mTLS: no private key found in PEM".into()))
}

fn load_ca_store(pem: &[u8]) -> Result<RootCertStore, ConfigError> {
    let mut store = RootCertStore::empty();
    for cert in load_certs(pem)? {
        store.add(cert).map_err(err("CA certificate"))?;
    }
    Ok(store)
}

pub fn build_mtls_server_config(
    server_cert_pem: &[u8],
    server_key_pem: &[u8],
    ca_cert_pem: &[u8],
) -> Result<rustls::ServerConfig, ConfigError> {
    let server_certs = load_certs(server_cert_pem)?;
    let server_key = load_private_key(server_key_pem)?;

    let roots = Arc::new(load_ca_store(ca_cert_pem)?);

    // Require and verify client certificates against our CA.
    let client_verifier = WebPkiClientVerifier::builder(roots)
        .build()
        .map_err(|e| ConfigError(format!("mTLS: client verifier: {e}")))?;

    let mut cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(server_certs, server_key)
        .map_err(err("server certificate"))?;

    // Optional but recommended (HTTP/2 + HTTP/1.1)
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(cfg)
}

pub fn build_mtls_client(
    ca_cert_pem: &[u8],
    client_cert_pem: &[u8],
    client_key_pem: &[u8],
) -> Result<reqwest::Client, ConfigError> {
    let ca = reqwest::Certificate::from_pem(ca_cert_pem)
        .map_err(|e| ConfigError(format!("mTLS: CA certificate: {e}")))?;

    // Combine client cert + key into one PEM buffer
    let mut identity_pem = Vec::new();
    identity_pem.extend_from_slice(client_cert_pem);
    identity_pem.extend_from_slice(client_key_pem);

    let identity = reqwest::Identity::from_pem(&identity_pem)
        .map_err(|e| ConfigError(format!("mTLS: client identity: {e}")))?;

    reqwest::Client::builder()
        .add_root_certificate(ca)
        .identity(identity)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| ConfigError(format!("mTLS: client: {e}")))
}
