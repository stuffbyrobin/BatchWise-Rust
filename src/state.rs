//! Shared application state, cloned into every handler.
//!
//! This is the Rust analogue of the dependency wiring that `cmd/server/main.go`
//! performs: a single place holding the pool, config, and JWT keys. `Arc`s keep
//! cloning cheap.

use std::sync::Arc;

use sqlx::PgPool;

use crate::auth::jwt::Jwt;
use crate::platform::config::Config;

/// Cloneable, shared-by-`Arc` application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub jwt: Arc<Jwt>,
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
        Self {
            pool,
            config: Arc::new(config),
            jwt: Arc::new(jwt),
        }
    }
}
