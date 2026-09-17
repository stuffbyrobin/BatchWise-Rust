//! Shared application state, cloned into every handler.
//!
//! This is the Rust analogue of the dependency wiring that `cmd/server/main.go`
//! performs: a single place holding the pool, config, and JWT keys. `Arc`s keep
//! cloning cheap.

use std::sync::Arc;

use sqlx::PgPool;

use crate::auth::jwt::Jwt;
use crate::platform::config::Config;
use crate::platform::features::FeatureCache;
use crate::platform::middleware::RateLimiter;
use crate::platform::redis_limits::RedisRateLimits;
use crate::platform::roles::RoleCache;

/// Cloneable, shared-by-`Arc` application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<Jwt>,
    /// Failed-login counter keyed by lower-cased email, so credential stuffing
    /// against one account is throttled even when it comes from many IPs.
    pub login_failures: Arc<RateLimiter>,
    /// Cached tenant feature flags for the tier gate.
    pub features: Arc<FeatureCache>,
    /// Cached user roles and active flags for route authorisation.
    pub roles: Arc<RoleCache>,
    /// Rate-limit windows shared by every instance, when `REDIS_URL` is set.
    pub rate_limits: Option<Arc<RedisRateLimits>>,
}

impl AppState {
    /// Builds the state from a pool and loaded config.
    pub fn new(pool: PgPool, config: Config) -> Self {
        let jwt = Jwt::new(
            &config.jwt_secret,
            &config.jwt_issuer,
            &config.jwt_audience,
            config.jwt_expiry_minutes,
        );
        // The URL was parsed once already when the config loaded, so a failure
        // here means Redis was configured wrongly: say so and limit per instance.
        let rate_limits = config.redis_url.as_deref().and_then(|url| {
            match RedisRateLimits::new(url, &config.redis_key_prefix) {
                Ok(limits) => Some(Arc::new(limits)),
                Err(e) => {
                    tracing::error!(error = %e, "REDIS_URL unusable; rate limits stay per instance");
                    None
                }
            }
        });
        Self {
            pool,
            config: Arc::new(config),
            jwt: Arc::new(jwt),
            login_failures: Arc::new(RateLimiter::per_minute(
                "login-failures",
                crate::auth::service::MAX_FAILED_LOGINS_PER_MINUTE,
                rate_limits.clone(),
            )),
            features: Arc::new(FeatureCache::default()),
            roles: Arc::new(RoleCache::default()),
            rate_limits,
        }
    }
}
