//! The `sqlx` Postgres pool and migration runner.
//!
//! Port of the Go `internal/platform/database` package. Migrations are embedded
//! at compile time from the `migrations/` directory and applied on startup
//! unless disabled.

use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

/// Port used by Supabase's transaction-mode connection pooler (Supavisor).
/// Connections through this port do not support the server-side prepared
/// statements that `sqlx` caches by default, so the cache must be disabled.
const SUPABASE_TXN_POOLER_PORT: u16 = 6543;

/// Embedded migrator built from the `migrations/` directory.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Errors from connecting or migrating.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("failed to connect to database: {0}")]
    Connect(#[source] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    Migrate(#[source] sqlx::migrate::MigrateError),
}

/// Opens a connection pool to `database_url`.
///
/// Works against any standard Postgres (including a Supabase project's direct
/// connection or session pooler). When the URL points at Supabase's
/// transaction-mode pooler (port 6543), statement caching is disabled
/// automatically so prepared statements don't fail under that pooler.
pub async fn connect(database_url: &str) -> Result<PgPool, DatabaseError> {
    let mut opts = PgConnectOptions::from_str(database_url).map_err(DatabaseError::Connect)?;

    if opts.get_port() == SUPABASE_TXN_POOLER_PORT {
        opts = opts.statement_cache_capacity(0);
    }

    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await
        .map_err(DatabaseError::Connect)
}

/// Applies all pending schema migrations.
pub async fn migrate(pool: &PgPool) -> Result<(), DatabaseError> {
    MIGRATOR.run(pool).await.map_err(DatabaseError::Migrate)
}
