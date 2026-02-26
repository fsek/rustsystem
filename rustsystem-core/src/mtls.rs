use std::io::BufReader;
use std::sync::Arc;

use rustls::server::WebPkiClientVerifier;
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use rustls_pemfile::{certs, private_key};

use crate::{APIError, APIErrorCode};

fn load_certs(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, APIError> {
    let mut reader = BufReader::new(pem);
    certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))
}

fn load_private_key(pem: &[u8]) -> Result<PrivateKeyDer<'static>, APIError> {
    let mut reader = BufReader::new(pem);
    private_key(&mut reader)
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?
        .ok_or_else(|| APIError::new(APIErrorCode::InitError, "No private key found in PEM data", 500))
}

fn load_ca_store(pem: &[u8]) -> Result<RootCertStore, APIError> {
    let mut store = RootCertStore::empty();
    for cert in load_certs(pem)? {
        store
            .add(cert)
            .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;
    }
    Ok(store)
}

pub fn build_mtls_server_config(
    server_cert_pem: &[u8],
    server_key_pem: &[u8],
    ca_cert_pem: &[u8],
) -> Result<rustls::ServerConfig, APIError> {
    let server_certs = load_certs(server_cert_pem)?;
    let server_key = load_private_key(server_key_pem)?;

    let roots = Arc::new(load_ca_store(ca_cert_pem)?);

    // Require and verify client certificates against our CA.
    let client_verifier = WebPkiClientVerifier::builder(roots)
        .build()
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;

    let mut cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(server_certs, server_key)
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;

    // Optional but recommended (HTTP/2 + HTTP/1.1)
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(cfg)
}

pub fn build_mtls_client(
    ca_cert_pem: &[u8],
    client_cert_pem: &[u8],
    client_key_pem: &[u8],
) -> Result<reqwest::Client, APIError> {
    let ca = reqwest::Certificate::from_pem(ca_cert_pem)
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;

    // Combine client cert + key into one PEM buffer
    let mut identity_pem = Vec::new();
    identity_pem.extend_from_slice(client_cert_pem);
    identity_pem.extend_from_slice(client_key_pem);

    let identity = reqwest::Identity::from_pem(&identity_pem)
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;

    reqwest::Client::builder()
        .add_root_certificate(ca)
        .identity(identity)
        .use_rustls_tls()
        .build()
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))
}
