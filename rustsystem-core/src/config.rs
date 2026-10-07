//! Reading configuration from environment variables at startup.
//!
//! Configuration is read once, at runtime, so the same binary runs in development, tests and
//! production. A missing or malformed variable stops the service with a message naming it.

use std::fmt;

#[derive(Debug)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "configuration error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

impl From<String> for ConfigError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

pub fn required(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError(format!("{name} must be set")))
}

pub fn optional(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

/// A URL without its trailing slash, so `format!("{url}/path")` is always right.
pub fn url(name: &str) -> Result<String, ConfigError> {
    let value = required(name)?;
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return Err(ConfigError(format!("{name} must start with http:// or https://")));
    }
    Ok(value.trim_end_matches('/').to_owned())
}

pub fn read_file(name: &str) -> Result<Vec<u8>, ConfigError> {
    let path = required(name)?;
    std::fs::read(&path).map_err(|e| ConfigError(format!("{name}: cannot read '{path}': {e}")))
}
