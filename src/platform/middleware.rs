//! HTTP middleware: JWT authentication and in-memory rate limiting.
//!
//! Port of the Go `internal/platform/middleware` (auth.go, ratelimit) adapted
//! to axum. `FeatureGate`/`TierGate` arrive with the modules that need them.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
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

/// Tier feature gate. Must run *after* [`require_auth`] (so the tenant id is in
/// the request context). Returns 403 with `{required_feature, current_tier}`
/// when the tenant's `feature_flags` does not enable `key`. Port of the Go
/// `FeatureGate` middleware.
pub async fn check_feature(
    state: &AppState,
    key: &str,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    use std::collections::HashMap;

    let ctx = req
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_default();
    let tenant_id = ctx.tenant_id()?;

    let row: Option<(String, sqlx::types::Json<HashMap<String, bool>>)> =
        sqlx::query_as("SELECT tier, feature_flags FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(&state.pool)
            .await?;

    let (tier, flags) = row.ok_or_else(|| ApiError::unauthorized("missing tenant context"))?;
    if flags.0.get(key).copied().unwrap_or(false) {
        Ok(next.run(req).await)
    } else {
        Err(ApiError::feature_disabled(key, &tier))
    }
}
/// (per-IP for auth routes). In-memory only; Redis is a future enhancement.
/// Keys are evicted lazily during a sweep triggered every `sweep_every` operations
/// to prevent unbounded growth when an attacker rotates source IPs.
#[derive(Debug)]
pub struct RateLimiter {
    limit: usize,
    window: Duration,
    sweep_every: usize,
    ops: AtomicUsize,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// New limiter allowing `limit` requests per 60-second window.
    pub fn per_minute(limit: u32) -> Self {
        Self {
            limit: limit.max(1) as usize,
            window: Duration::from_secs(60),
            sweep_every: 1024,
            ops: AtomicUsize::new(0),
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// New limiter with a custom window for testing.
    #[cfg(test)]
    pub fn with_window(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            window,
            sweep_every: 1024,
            ops: AtomicUsize::new(0),
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Records a hit for `key`. Returns `Err(retry_after_seconds)` when the
    /// limit is exceeded, `Ok(())` otherwise.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter mutex");
        // Every call counts toward the sweep, including rejected ones, so a
        // flood of blocked requests cannot starve eviction.
        let op_count = self.ops.fetch_add(1, Ordering::Relaxed) + 1;
        if op_count.is_multiple_of(self.sweep_every) {
            hits.retain(|_, v| {
                v.retain(|&t| now.duration_since(t) < self.window);
                !v.is_empty()
            });
        }
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

    /// Returns `Some(retry_after_seconds)` when the bucket for `key` is already
    /// at the limit, without recording a hit. Used for the per-account failed-login
    /// check, where only failures count as hits.
    pub fn is_limited(&self, key: &str) -> Option<u64> {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter mutex");
        let bucket = hits.entry(key.to_string()).or_default();
        bucket.retain(|&t| now.duration_since(t) < self.window);
        if bucket.len() >= self.limit {
            let oldest = bucket.first().copied().unwrap_or(now);
            let retry = self.window.saturating_sub(now.duration_since(oldest));
            Some(retry.as_secs().max(1))
        } else {
            None
        }
    }

    /// Returns the number of tracked keys, for testing sweep behavior.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.hits.lock().expect("rate limiter mutex").len()
    }
}

/// Best-effort client IP for rate-limit keying.
/// When `trust_proxy_headers` is true and the request has an `x-forwarded-for` header,
/// returns the LAST comma-separated entry (trimmed) as the client IP — the entry
/// appended by the trusted reverse proxy. Otherwise falls back to the ConnectInfo
/// socket address or "unknown".
pub fn client_ip(req: &Request, trust_proxy_headers: bool) -> String {
    if trust_proxy_headers {
        if let Some(values) = req.headers().get("x-forwarded-for") {
            if let Ok(header) = values.to_str() {
                if let Some(last) = header.rsplit(',').next() {
                    return last.trim().to_string();
                }
            }
        }
    }
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }
    "unknown".to_string()
}

/// State for [`rate_limit`]: a shared limiter plus whether to key on `X-Forwarded-For`.
#[derive(Clone)]
pub struct RateLimit {
    pub limiter: Arc<RateLimiter>,
    pub trust_proxy_headers: bool,
}

impl RateLimit {
    /// A fresh per-minute limiter.
    pub fn per_minute(limit: u32, trust_proxy_headers: bool) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::per_minute(limit)),
            trust_proxy_headers,
        }
    }
}

/// Per-client-IP rate limit. Install with
/// `axum::middleware::from_fn_with_state(RateLimit::per_minute(n, trust), rate_limit)`.
/// Over the limit, returns 429 `rate_limited` with `Retry-After`.
pub async fn rate_limit(
    State(rl): State<RateLimit>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    match rl.limiter.check(&client_ip(&req, rl.trust_proxy_headers)) {
        Ok(()) => Ok(next.run(req).await),
        Err(retry) => Err(ApiError::rate_limited(retry)),
    }
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

    #[test]
    fn is_limited_does_not_record() {
        let rl = RateLimiter::per_minute(1);
        assert_eq!(rl.is_limited("key"), None);
        assert!(rl.check("key").is_ok());
        assert!(rl.is_limited("key").is_some());
    }

    #[test]
    fn sweep_evicts_idle_keys() {
        let rl = RateLimiter::with_window(1, Duration::from_millis(10));
        assert!(rl.check("a").is_ok());
        assert!(rl.check("b").is_ok());
        assert_eq!(rl.len(), 2);
        std::thread::sleep(Duration::from_millis(20));
        // Rejected checks still count toward the sweep; 1024 ops trigger it.
        for _ in 0..1024 {
            let _ = rl.check("c");
        }
        assert_eq!(rl.len(), 1);
    }
}
