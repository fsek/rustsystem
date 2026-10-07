//! Code shared by `rustsystem-server` and `rustsystem-trustauth`.
//!
//! - [`error`], [`extract`] — the one error type every endpoint returns, and extractors that use it.
//! - [`internal`] — the server → trustauth API.
//! - [`blind`] — RSA blind signatures, the voting protocol's only cryptography.
//! - [`secret`], [`session`] — random tokens, how they are stored, and session cookies.
//! - [`limits`] — body-size and rate limits.
//! - [`config`], [`mtls`], [`logging`] — startup plumbing.
//!
//! The protocol these implement is specified in `docs/PROTOCOL.md`.

pub mod blind;
pub mod config;
pub mod error;
pub mod extract;
pub mod internal;
pub mod limits;
pub mod logging;
pub mod mtls;
pub mod secret;
pub mod session;

pub use error::{ApiError, ApiResult, ErrorCode};
