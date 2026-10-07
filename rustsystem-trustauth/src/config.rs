//! Trustauth's runtime configuration, read from the environment at startup.
//!
//! | Variable | Example | Meaning |
//! |---|---|---|
//! | `TRUSTAUTH_PUBLIC_URL` | `https://rosta.trustauth.fsektionen.se` | Where browsers reach trustauth. `https://` marks cookies `Secure`. |
//! | `SERVER_PUBLIC_URLS` | `https://rosta.fsektionen.se` | Comma-separated origins the voter page is served from (CORS). |
//! | `TRUSTAUTH_PUBLIC_ADDR` | `0.0.0.0:2443` | Public listener (plain HTTP, behind the TLS proxy). |
//! | `TRUSTAUTH_INTERNAL_ADDR` | `0.0.0.0:2444` | Internal mTLS listener for the server. |
//! | `MTLS_CA_CERT`, `MTLS_CERT`, `MTLS_KEY` | `mtls/ca/ca.crt`, … | PEM files for the internal listener. |
//! | `RUSTSYSTEM_TRUSTED_PROXIES` | `172.18.0.1` | See `rustsystem_core::limits`. |
//! | `RUSTSYSTEM_DISABLE_RATE_LIMIT` | `1` | Tests only. |

use std::net::SocketAddr;

use axum::http::HeaderValue;
use rustsystem_core::{
    config::{ConfigError, optional, read_file, required, url},
    limits::ClientIp,
};

pub struct Config {
    pub secure_cookies: bool,
    pub allowed_origins: Vec<HeaderValue>,
    pub public_addr: SocketAddr,
    pub internal_addr: SocketAddr,
    pub ca_cert: Vec<u8>,
    pub cert: Vec<u8>,
    pub key: Vec<u8>,
    pub client_ips: ClientIp,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let public_url = url("TRUSTAUTH_PUBLIC_URL")?;
        let allowed_origins = required("SERVER_PUBLIC_URLS")?
            .split(',')
            .map(|o| o.trim().trim_end_matches('/'))
            .filter(|o| !o.is_empty())
            .map(|o| {
                HeaderValue::from_str(o)
                    .map_err(|_| ConfigError(format!("SERVER_PUBLIC_URLS: '{o}' is not a valid origin")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if allowed_origins.is_empty() {
            return Err(ConfigError("SERVER_PUBLIC_URLS must name at least one origin".into()));
        }

        Ok(Self {
            secure_cookies: public_url.starts_with("https://"),
            allowed_origins,
            public_addr: addr("TRUSTAUTH_PUBLIC_ADDR", "0.0.0.0:2443")?,
            internal_addr: addr("TRUSTAUTH_INTERNAL_ADDR", "0.0.0.0:2444")?,
            ca_cert: read_file("MTLS_CA_CERT")?,
            cert: read_file("MTLS_CERT")?,
            key: read_file("MTLS_KEY")?,
            client_ips: ClientIp::from_env()?,
        })
    }
}

fn addr(name: &str, default: &str) -> Result<SocketAddr, ConfigError> {
    let value = optional(name, default);
    value
        .parse()
        .map_err(|_| ConfigError(format!("{name}: '{value}' is not an address like 0.0.0.0:2443")))
}
