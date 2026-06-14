//! Data access for auth (users + refresh tokens).
//!
//! Port of the Go `internal/auth/repository.go`. Functions are generic over the
//! sqlx executor so callers can run them on the pool or inside a transaction
//! (the register flow inserts a tenant and a user atomically).
//!
//! `email` is a `citext` column; it is bound as `$n::citext` and selected as
//! `email::text` so sqlx sees a plain `text` type.

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::models::{RefreshToken, User};

const USER_COLS: &str = "id, tenant_id, email::text AS email, password_hash, display_name, \
                         is_owner, is_active, created_at, updated_at";

/// Inserts a new user, returning the created row.
#[allow(clippy::too_many_arguments)]
pub async fn create_user<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    email: &str,
    password_hash: &str,
    display_name: &str,
    is_owner: bool,
    is_active: bool,
) -> Result<User, sqlx::Error> {
    let sql = format!(
        "INSERT INTO users (tenant_id, email, password_hash, display_name, is_owner, is_active) \
         VALUES ($1, $2::citext, $3, $4, $5, $6) RETURNING {USER_COLS}"
    );
    sqlx::query_as::<_, User>(&sql)
        .bind(tenant_id)
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .bind(is_owner)
        .bind(is_active)
        .fetch_one(exec)
        .await
}

/// CROSS-TENANT QUERY: login looks up by email across all tenants because the
/// client does not know which tenant the user belongs to.
pub async fn get_user_by_email_global(
    pool: &PgPool,
    email: &str,
) -> Result<Option<User>, sqlx::Error> {
    let sql = format!("SELECT {USER_COLS} FROM users WHERE email = $1::citext");
    sqlx::query_as::<_, User>(&sql)
        .bind(email)
        .fetch_optional(pool)
        .await
}

/// Fetches a user by id.
pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let sql = format!("SELECT {USER_COLS} FROM users WHERE id = $1");
    sqlx::query_as::<_, User>(&sql)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Updates a user's display name, password hash, and active flag.
pub async fn update_user(
    pool: &PgPool,
    user_id: Uuid,
    display_name: &str,
    password_hash: &str,
    is_active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET display_name=$1, password_hash=$2, is_active=$3, updated_at=now() WHERE id=$4",
    )
    .bind(display_name)
    .bind(password_hash)
    .bind(is_active)
    .bind(user_id)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Sets `is_active = false` (soft delete).
pub async fn deactivate_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET is_active = false, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Inserts a refresh-token record.
pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Fetches a refresh token by its hash.
pub async fn get_refresh_token_by_hash(
    pool: &PgPool,
    hash: &str,
) -> Result<Option<RefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, RefreshToken>(
        "SELECT id, user_id, token_hash, expires_at, used_at, created_at \
         FROM refresh_tokens WHERE token_hash = $1",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await
}

/// Marks a refresh token as used (rotation / logout).
pub async fn mark_refresh_token_used(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE refresh_tokens SET used_at=now() WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Deletes all refresh tokens for a user (password change / account deletion).
pub async fn delete_refresh_tokens_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id=$1")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Deletes expired and stale-used tokens; returns the number removed.
pub async fn cleanup_expired_refresh_tokens(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM refresh_tokens WHERE expires_at < now() OR used_at < now() - interval '1 day'",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
