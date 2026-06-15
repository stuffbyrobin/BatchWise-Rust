//! Background refresh-token cleanup.
//!
//! Port of the Go `internal/auth/cleanup.go`. Every hour, deletes expired and
//! stale-used refresh tokens. The task runs for the lifetime of the process.

use std::time::Duration;

use sqlx::PgPool;

use super::repository;

/// Spawns the hourly cleanup loop.
pub fn start_cleanup_loop(pool: PgPool) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        // Skip the immediate first tick so cleanup runs an hour after startup.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match repository::cleanup_expired_refresh_tokens(&pool).await {
                Ok(n) => tracing::info!(deleted = n, "refresh token cleanup"),
                Err(e) => tracing::error!(error = %e, "refresh token cleanup failed"),
            }
        }
    });
}
