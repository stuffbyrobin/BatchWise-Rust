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

use axum::extract::{ConnectInfo, MatchedPath, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use super::authz;
use super::context::RequestContext;
use super::errors::ApiError;
use super::redis_limits::RedisRateLimits;
use crate::state::AppState;

/// Validates the `Authorization: Bearer <jwt>` header, checks the account is
/// still active in the token's tenant and the token is not revoked, authorises the caller's role for the
/// matched route ([`authz`]), and merges the user id, tenant id and role into
/// the request's [`RequestContext`] (preserving the request id).
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

    // Deactivation, role changes and revocation apply before the token expires.
    let membership = state
        .roles
        .get(&state.pool, claims.subject)
        .await?
        .filter(|m| m.accepts(&claims))
        .ok_or_else(|| {
            ApiError::unauthorized("token revoked, or account inactive or no longer exists")
        })?;
    // The route template, e.g. `/api/v1/batches/{id}`.
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_default();
    authz::authorize(membership.role, req.method(), &path)?;

    let mut ctx = req
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_default();
    ctx.user_id = Some(claims.subject);
    ctx.tenant_id = Some(claims.tenant_id);
    ctx.actor_id = Some(claims.subject);
    ctx.role = Some(membership.role);
    req.extensions_mut().insert(ctx);

    Ok(next.run(req).await)
}

/// Tier feature gate. Must run *after* [`require_auth`] (so the tenant id is in
/// the request context). Returns 403 with `{required_feature, current_tier}`
/// when the tenant's `feature_flags` does not enable `key`. Port of the Go
/// `FeatureGate` middleware.
/// Flags come from [`AppState::features`](crate::state::AppState::features), so
/// most requests skip the tenants query.
pub async fn check_feature(
    state: &AppState,
    key: &str,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let ctx = req
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_default();
    let tenant_id = ctx.tenant_id()?;

    let (enabled, tier) = state.features.check(&state.pool, tenant_id, key).await?;
    if enabled {
        Ok(next.run(req).await)
    } else {
        Err(ApiError::feature_disabled(key, &tier))
    }
}
/// Sliding-window rate limiter keyed by the caller: the client IP for route
/// limits, the e-mail address for the failed-login limit. The window is kept in
/// memory and, when a [`RedisRateLimits`] is configured, in Redis as well; the
/// shared answer then decides, so every instance shares one limit.
/// Keys are evicted lazily during a sweep triggered every `sweep_every` operations
/// to prevent unbounded growth when an attacker rotates source IPs.
#[derive(Debug)]
pub struct RateLimiter {
    name: &'static str,
    limit: usize,
    window: Duration,
    sweep_every: usize,
    ops: AtomicUsize,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
    shared: Option<Arc<RedisRateLimits>>,
}

impl RateLimiter {
    /// New limiter allowing `limit` requests per 60-second window, shared
    /// through Redis when `shared` is set. `name` separates the limiters
    /// sharing one Redis.
    pub fn per_minute(
        name: &'static str,
        limit: u32,
        shared: Option<Arc<RedisRateLimits>>,
    ) -> Self {
        Self {
            name,
            limit: limit.max(1) as usize,
            window: Duration::from_secs(60),
            sweep_every: 1024,
            ops: AtomicUsize::new(0),
            hits: Mutex::new(HashMap::new()),
            shared,
        }
    }

    /// New in-memory limiter with a custom window for testing.
    #[cfg(test)]
    pub fn with_window(limit: usize, window: Duration) -> Self {
        Self {
            name: "test",
            limit,
            window,
            sweep_every: 1024,
            ops: AtomicUsize::new(0),
            hits: Mutex::new(HashMap::new()),
            shared: None,
        }
    }

    /// Records a hit for `key`. Returns `Err(retry_after_seconds)` when the
    /// limit is exceeded, `Ok(())` otherwise.
    pub async fn check(&self, key: &str) -> Result<(), u64> {
        self.hit(key, true).await
    }

    /// Returns `Some(retry_after_seconds)` when the bucket for `key` is already
    /// at the limit, without recording a hit. Used for the per-account failed-login
    /// check, where only failures count as hits.
    pub async fn is_limited(&self, key: &str) -> Option<u64> {
        self.hit(key, false).await.err()
    }

    async fn hit(&self, key: &str, record: bool) -> Result<(), u64> {
        // The in-memory window keeps counting even while Redis answers, so an
        // outage falls back to per-instance limiting with a warm window.
        let local = self.local(key, record);
        let Some(shared) = &self.shared else {
            return local;
        };
        match shared
            .hit(self.name, key, self.window, self.limit, record)
            .await
        {
            Ok(None) => Ok(()),
            Ok(Some(retry)) => Err(retry),
            Err(e) => {
                shared.warn(self.name, &e);
                local
            }
        }
    }

    /// The in-memory half of [`Self::hit`].
    fn local(&self, key: &str, record: bool) -> Result<(), u64> {
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
        if record {
            bucket.push(now);
        }
        Ok(())
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
    /// A fresh per-minute limiter, shared through Redis when `shared` is set.
    pub fn per_minute(
        name: &'static str,
        limit: u32,
        trust_proxy_headers: bool,
        shared: Option<Arc<RedisRateLimits>>,
    ) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::per_minute(name, limit, shared)),
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
    match rl
        .limiter
        .check(&client_ip(&req, rl.trust_proxy_headers))
        .await
    {
        Ok(()) => Ok(next.run(req).await),
        Err(retry) => Err(ApiError::rate_limited(retry)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_up_to_limit_then_blocks() {
        let rl = RateLimiter::per_minute("test", 3, None);
        assert!(rl.check("ip").await.is_ok());
        assert!(rl.check("ip").await.is_ok());
        assert!(rl.check("ip").await.is_ok());
        assert!(rl.check("ip").await.is_err());
    }

    #[tokio::test]
    async fn separate_keys_are_independent() {
        let rl = RateLimiter::per_minute("test", 1, None);
        assert!(rl.check("a").await.is_ok());
        assert!(rl.check("b").await.is_ok());
        assert!(rl.check("a").await.is_err());
    }

    #[tokio::test]
    async fn is_limited_does_not_record() {
        let rl = RateLimiter::per_minute("test", 1, None);
        assert_eq!(rl.is_limited("key").await, None);
        assert!(rl.check("key").await.is_ok());
        assert!(rl.is_limited("key").await.is_some());
    }

    #[tokio::test]
    async fn sweep_evicts_idle_keys() {
        let rl = RateLimiter::with_window(1, Duration::from_millis(10));
        assert!(rl.check("a").await.is_ok());
        assert!(rl.check("b").await.is_ok());
        assert_eq!(rl.len(), 2);
        tokio::time::sleep(Duration::from_millis(20)).await;
        // Rejected checks still count toward the sweep; 1024 ops trigger it.
        for _ in 0..1024 {
            let _ = rl.check("c").await;
        }
        assert_eq!(rl.len(), 1);
    }
}
