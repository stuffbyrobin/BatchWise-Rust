//! Data access for tenant members (rows of `users`).

use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::models::Member;

const MEMBER_COLS: &str = "id, email::text AS email, display_name, role, is_active, created_at";

/// Every user of the tenant, oldest first.
pub async fn select_members(pool: &PgPool, tenant_id: Uuid) -> Result<Vec<Member>, sqlx::Error> {
    let sql =
        format!("SELECT {MEMBER_COLS} FROM users WHERE tenant_id = $1 ORDER BY created_at, id");
    sqlx::query_as::<_, Member>(&sql)
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

/// Locks the tenant's Owner rows, so concurrent changes cannot remove the last
/// active Owner.
pub async fn lock_owners<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT id FROM users WHERE tenant_id = $1 AND role = 'owner' FOR UPDATE")
        .bind(tenant_id)
        .fetch_all(exec)
        .await
        .map(|_| ())
}

/// Fetches and locks a member of the tenant.
pub async fn select_member_for_update<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Member>, sqlx::Error> {
    let sql =
        format!("SELECT {MEMBER_COLS} FROM users WHERE id = $1 AND tenant_id = $2 FOR UPDATE");
    sqlx::query_as::<_, Member>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(exec)
        .await
}

/// The number of active Owners of the tenant other than `except`.
pub async fn count_other_active_owners<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    except: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM users \
         WHERE tenant_id = $1 AND role = 'owner' AND is_active AND id <> $2",
    )
    .bind(tenant_id)
    .bind(except)
    .fetch_one(exec)
    .await
}

/// Sets a member's role and active flag.
pub async fn update_member<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    role: &str,
    is_active: bool,
) -> Result<Member, sqlx::Error> {
    let sql = format!(
        "UPDATE users SET role = $3, is_active = $4, updated_at = now() \
         WHERE id = $1 AND tenant_id = $2 RETURNING {MEMBER_COLS}"
    );
    sqlx::query_as::<_, Member>(&sql)
        .bind(id)
        .bind(tenant_id)
        .bind(role)
        .bind(is_active)
        .fetch_one(exec)
        .await
}

/// Revokes all of a member's refresh tokens.
pub async fn delete_refresh_tokens<'e, E: PgExecutor<'e>>(
    exec: E,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(exec)
        .await
        .map(|_| ())
}
