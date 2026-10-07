//! Request limits shared by both services: body size and per-IP rate limiting.
//!
//! Rate limits are keyed on the client's IP address. Both services normally sit behind a
//! TLS-terminating reverse proxy, where every request's TCP peer is the proxy itself, so the
//! client's address has to come from `X-Forwarded-For`. That header is trivially forged, so it is
//! **only** honoured when the request comes from an address listed in
//! `RUSTSYSTEM_TRUSTED_PROXIES`; for anyone else the TCP peer address is used, and setting the
//! header buys nothing.
//!
//! Limits are deliberately generous: a whole meeting room may share one NAT address, and when a
//! round opens every voter's page fetches state, status, a signature and submits a ballot within
//! seconds. They exist to stop floods, not to meter use.

use std::{net::IpAddr, net::SocketAddr, sync::Arc, time::Duration};

use axum::{extract::ConnectInfo, http::Request, response::IntoResponse};
use tower_governor::{
    GovernorError, GovernorLayer, governor::GovernorConfigBuilder, key_extractor::KeyExtractor,
};
use tracing::warn;

use crate::error::{ApiError, ErrorCode};

/// Largest request body either service accepts. The biggest legitimate body is a vote round
/// with many long candidate names, well under this.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Turns rate limiting off at runtime. Needed for the live end-to-end suite, which creates many
/// meetings in seconds. Never set in production.
pub const DISABLE_RATE_LIMIT_ENV: &str = "RUSTSYSTEM_DISABLE_RATE_LIMIT";

/// Comma-separated IP addresses of reverse proxies whose `X-Forwarded-For` is trusted.
pub const TRUSTED_PROXIES_ENV: &str = "RUSTSYSTEM_TRUSTED_PROXIES";

#[derive(Clone, Copy, Debug)]
pub struct Quota {
    /// Requests allowed in a burst.
    pub burst: u32,
    /// One more request is allowed every this often.
    pub replenish_every: Duration,
}

/// Every public API route: bursts of 1000, then 200 requests per second — enough for a few
/// hundred voters behind one address all voting at once.
pub const GENERAL: Quota = Quota {
    burst: 1000,
    replenish_every: Duration::from_millis(5),
};

/// Creating meetings: bursts of 5, then one a minute.
pub const CREATE_MEETING: Quota = Quota {
    burst: 5,
    replenish_every: Duration::from_secs(60),
};

/// Finds the client IP for rate limiting. See the module docs.
#[derive(Clone, Debug, Default)]
pub struct ClientIp {
    trusted_proxies: Arc<Vec<IpAddr>>,
}

impl ClientIp {
    pub fn new(trusted_proxies: Vec<IpAddr>) -> Self {
        Self {
            trusted_proxies: Arc::new(trusted_proxies),
        }
    }

    /// Reads [`TRUSTED_PROXIES_ENV`]. Unparseable entries are an error, not silently skipped.
    pub fn from_env() -> Result<Self, String> {
        let Ok(list) = std::env::var(TRUSTED_PROXIES_ENV) else {
            return Ok(Self::default());
        };
        let proxies = list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse()
                    .map_err(|_| format!("{TRUSTED_PROXIES_ENV}: '{s}' is not an IP address"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(proxies))
    }

    pub fn client_ip<T>(&self, req: &Request<T>) -> Option<IpAddr> {
        let peer = req.extensions().get::<ConnectInfo<SocketAddr>>()?.0.ip();
        if !self.trusted_proxies.contains(&peer) {
            return Some(peer);
        }
        // The proxy appends the address it saw to the end of the header, so the last entry is
        // the only one the proxy vouches for; earlier entries come from the client.
        let forwarded = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|ip| ip.trim().parse().ok());
        Some(forwarded.unwrap_or(peer))
    }
}

impl KeyExtractor for ClientIp {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        self.client_ip(req).ok_or(GovernorError::UnableToExtractKey)
    }
}

pub type RateLimitLayer = GovernorLayer<
    ClientIp,
    governor::middleware::NoOpMiddleware<governor::clock::QuantaInstant>,
    axum::body::Body,
>;

pub fn rate_limit_disabled() -> bool {
    std::env::var_os(DISABLE_RATE_LIMIT_ENV).is_some()
}

/// A rate-limit layer for `quota`, or `None` when [`DISABLE_RATE_LIMIT_ENV`] is set.
///
/// Must be called inside a Tokio runtime: it spawns a task that forgets idle clients so the
/// limiter's memory stays bounded.
pub fn rate_limit(quota: Quota, ips: ClientIp) -> Option<RateLimitLayer> {
    if rate_limit_disabled() {
        warn!("{DISABLE_RATE_LIMIT_ENV} is set: rate limiting is OFF. Never do this in production.");
        return None;
    }
    Some(rate_limit_forced(quota, ips))
}

/// Like [`rate_limit`] but ignores [`DISABLE_RATE_LIMIT_ENV`], so tests of the limiter itself
/// can't be turned into no-ops by the environment.
pub fn rate_limit_forced(quota: Quota, ips: ClientIp) -> RateLimitLayer {
    let config = GovernorConfigBuilder::default()
        .period(quota.replenish_every)
        .burst_size(quota.burst)
        .key_extractor(ips)
        .finish()
        .expect("quota has a non-zero period and burst");

    let limiter = config.limiter().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            limiter.retain_recent();
        }
    });

    GovernorLayer::new(config).error_handler(|err| match err {
        GovernorError::TooManyRequests { .. } => ApiError::new(ErrorCode::RateLimited).into_response(),
        other => ApiError::internal(format!("rate limiter: {other}")).into_response(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn request(peer: &str, xff: Option<&str>) -> Request<Body> {
        let mut req = Request::new(Body::empty());
        req.extensions_mut()
            .insert(ConnectInfo::<SocketAddr>(format!("{peer}:5000").parse().unwrap()));
        if let Some(v) = xff {
            req.headers_mut().insert("x-forwarded-for", v.parse().unwrap());
        }
        req
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn untrusted_peer_cannot_forge_forwarded_for() {
        let ips = ClientIp::new(vec![ip("10.0.0.1")]);
        let req = request("203.0.113.9", Some("1.2.3.4"));
        assert_eq!(ips.client_ip(&req), Some(ip("203.0.113.9")));
    }

    #[test]
    fn trusted_proxy_uses_last_forwarded_entry() {
        let ips = ClientIp::new(vec![ip("10.0.0.1")]);
        // A client forged "1.2.3.4"; the proxy appended the real address.
        let req = request("10.0.0.1", Some("1.2.3.4, 198.51.100.7"));
        assert_eq!(ips.client_ip(&req), Some(ip("198.51.100.7")));
    }

    #[test]
    fn trusted_proxy_without_header_falls_back_to_peer() {
        let ips = ClientIp::new(vec![ip("10.0.0.1")]);
        assert_eq!(ips.client_ip(&request("10.0.0.1", None)), Some(ip("10.0.0.1")));
        assert_eq!(ips.client_ip(&request("10.0.0.1", Some("garbage"))), Some(ip("10.0.0.1")));
    }

    #[test]
    fn missing_connect_info_is_none() {
        let ips = ClientIp::default();
        assert_eq!(ips.client_ip(&Request::new(Body::empty())), None);
    }
}
