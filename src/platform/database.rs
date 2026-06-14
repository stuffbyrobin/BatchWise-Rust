//! The `sqlx` Postgres pool and migration runner.
//!
//! Port of the Go `internal/platform/database` package. Migrations are embedded
//! at compile time from the `migrations/` directory and applied on startup
//! unless disabled.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

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
pub async fn connect(database_url: &str) -> Result<PgPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .map_err(DatabaseError::Connect)
}

/// Applies all pending schema migrations.
pub async fn migrate(pool: &PgPool) -> Result<(), DatabaseError> {
    MIGRATOR.run(pool).await.map_err(DatabaseError::Migrate)
}
