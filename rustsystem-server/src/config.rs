//! The server's runtime configuration, read from the environment at startup.
//!
//! | Variable | Example | Meaning |
//! |---|---|---|
//! | `SERVER_PUBLIC_URL` | `https://rosta.fsektionen.se` | Where browsers reach the server; used in invite links. `https://` marks cookies `Secure`. |
//! | `TRUSTAUTH_PUBLIC_URL` | `https://rosta.trustauth.fsektionen.se` | Where browsers reach trustauth; the frontend learns it from `GET /api/config`. |
//! | `TRUSTAUTH_INTERNAL_URL` | `https://rustsystem-trustauth:2444` | Trustauth's internal (mTLS) API. |
//! | `SERVER_PUBLIC_ADDR` | `0.0.0.0:1443` | Listener (plain HTTP, behind the TLS proxy). |
//! | `MTLS_CA_CERT`, `MTLS_CERT`, `MTLS_KEY` | `mtls/ca/ca.crt`, … | PEM files for calling trustauth. |
//! | `MEETINGS_DIR` | `meetings` | Where encrypted tally files and per-meeting logs go. |
//! | `FRONTEND_DIR` | `frontend/dist` | The built frontend. |
//! | `RUSTSYSTEM_TRUSTED_PROXIES` | `172.18.0.1` | See `rustsystem_core::limits`. |
//! | `RUSTSYSTEM_DISABLE_RATE_LIMIT` | `1` | Tests only. |

use std::{net::SocketAddr, path::PathBuf};

use rustsystem_core::{
    config::{ConfigError, optional, read_file, url},
    limits::ClientIp,
    mtls::build_mtls_client,
};

use crate::{Settings, trustauth::Trustauth};

pub struct Config {
    pub settings: Settings,
    pub trustauth: Trustauth,
    pub addr: SocketAddr,
    pub frontend_dir: PathBuf,
    pub client_ips: ClientIp,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let public_url = url("SERVER_PUBLIC_URL")?;
        let http = build_mtls_client(&read_file("MTLS_CA_CERT")?, &read_file("MTLS_CERT")?, &read_file("MTLS_KEY")?)?;
        let addr_text = optional("SERVER_PUBLIC_ADDR", "0.0.0.0:1443");
        let addr = addr_text
            .parse()
            .map_err(|_| ConfigError(format!("SERVER_PUBLIC_ADDR: '{addr_text}' is not an address like 0.0.0.0:1443")))?;

        Ok(Self {
            settings: Settings {
                secure_cookies: public_url.starts_with("https://"),
                public_url,
                trustauth_public_url: url("TRUSTAUTH_PUBLIC_URL")?,
                meetings_dir: optional("MEETINGS_DIR", "meetings").into(),
            },
            trustauth: Trustauth::new(http, url("TRUSTAUTH_INTERNAL_URL")?),
            addr,
            frontend_dir: optional("FRONTEND_DIR", "frontend/dist").into(),
            client_ips: ClientIp::from_env()?,
        })
    }
}
