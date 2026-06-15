//! HTTP middleware: JWT authentication and in-memory rate limiting.
//!
//! Port of the Go `internal/platform/middleware` (auth.go, ratelimit) adapted
//! to axum. `FeatureGate`/`TierGate` arrive with the modules that need them.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use super::context::RequestContext;
use super::errors::ApiError;
use crate::state::AppState;

/// Validates the `Authorization: Bearer <jwt>` header and merges the user and
/// tenant ids into the request's [`RequestContext`] (preserving the request id).
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("missing or malformed Authorization header"))?;

    let claims = state
        .jwt
        .verify(token)
        .map_err(|_| ApiError::unauthorized("invalid or expired token"))?;

    let mut ctx = req
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_default();
    ctx.user_id = Some(claims.subject);
    ctx.tenant_id = Some(claims.tenant_id);
    ctx.actor_id = Some(claims.subject);
    req.extensions_mut().insert(ctx);

    Ok(next.run(req).await)
}

/// A fixed-capacity sliding-window rate limiter keyed by an arbitrary string
/// (per-IP for auth routes). In-memory only; Redis is a future enhancement.
#[derive(Debug)]
pub struct RateLimiter {
    limit: usize,
    window: Duration,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// New limiter allowing `limit` requests per 60-second window.
    pub fn per_minute(limit: u32) -> Self {
        Self {
            limit: limit.max(1) as usize,
            window: Duration::from_secs(60),
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records a hit for `key`. Returns `Err(retry_after_seconds)` when the
    /// limit is exceeded, `Ok(())` otherwise.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter mutex");
        let bucket = hits.entry(key.to_string()).or_default();
        bucket.retain(|&t| now.duration_since(t) < self.window);
        if bucket.len() >= self.limit {
            let oldest = bucket.first().copied().unwrap_or(now);
            let retry = self.window.saturating_sub(now.duration_since(oldest));
            return Err(retry.as_secs().max(1));
        }
        bucket.push(now);
        Ok(())
    }
}

/// Best-effort client IP for rate-limit keying.
pub fn client_ip(req: &Request) -> String {
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_then_blocks() {
        let rl = RateLimiter::per_minute(3);
        assert!(rl.check("ip").is_ok());
        assert!(rl.check("ip").is_ok());
        assert!(rl.check("ip").is_ok());
        assert!(rl.check("ip").is_err());
    }

    #[test]
    fn separate_keys_are_independent() {
        let rl = RateLimiter::per_minute(1);
        assert!(rl.check("a").is_ok());
        assert!(rl.check("b").is_ok());
        assert!(rl.check("a").is_err());
    }
}
