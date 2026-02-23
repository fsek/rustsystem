use std::{fs::File, io::BufReader, sync::Arc};

use rustls::server::WebPkiClientVerifier;
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use rustls_pemfile::{certs, private_key};

fn load_certs(path: &str) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(File::open(path)?);
    let certs = certs(&mut reader).collect::<Result<Vec<_>, _>>()?;
    Ok(certs)
}

fn load_private_key(path: &str) -> anyhow::Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(File::open(path)?);
    let key = private_key(&mut reader)?
        .ok_or_else(|| anyhow::anyhow!("No private key found in {}", path))?;
    Ok(key)
}

fn load_ca_store(ca_path: &str) -> anyhow::Result<RootCertStore> {
    let mut store = RootCertStore::empty();
    for cert in load_certs(ca_path)? {
        store.add(cert)?;
    }
    Ok(store)
}

pub fn build_mtls_server_config(
    server_cert_pem: &str,
    server_key_pem: &str,
    ca_cert_pem: &str,
) -> anyhow::Result<rustls::ServerConfig> {
    let server_certs = load_certs(server_cert_pem)?;
    let server_key = load_private_key(server_key_pem)?;

    let roots = Arc::new(load_ca_store(ca_cert_pem)?);

    // Require and verify client certificates against our CA.
    let client_verifier = WebPkiClientVerifier::builder(roots).build()?; // :contentReference[oaicite:1]{index=1}

    let mut cfg = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(server_certs, server_key)?;

    // Optional but recommended (HTTP/2 + HTTP/1.1)
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(cfg)
}

use std::fs;

pub fn build_mtls_client(endpoint: &str) -> anyhow::Result<reqwest::Client> {
    // Trust your internal CA
    let ca_pem = fs::read("mtls/ca/ca.crt")?;
    let ca = reqwest::Certificate::from_pem(&ca_pem)?;

    // Combine client cert + key into one PEM buffer
    let mut identity_pem = Vec::new();
    identity_pem.extend(fs::read(format!("mtls/{endpoint}/{endpoint}.crt"))?);
    identity_pem.extend(fs::read(format!("mtls/{endpoint}/{endpoint}.key"))?);

    let identity = reqwest::Identity::from_pem(&identity_pem)?;

    let client = reqwest::Client::builder()
        .add_root_certificate(ca)
        .identity(identity)
        .use_rustls_tls()
        .build()?;

    Ok(client)
}
