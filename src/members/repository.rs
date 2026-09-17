//! Data access for tenant members (rows of `users`).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor, PgPool};
use uuid::Uuid;

use super::models::{Invitation, Member};

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

// ---- invitations ----

const INVITATION_COLS: &str = "id, email::text AS email, role, invited_by, expires_at, created_at";

/// Open (not accepted or revoked) invitations of the tenant, newest first.
pub async fn select_open_invitations(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<Invitation>, sqlx::Error> {
    let sql = format!(
        "SELECT {INVITATION_COLS} FROM invitations \
         WHERE tenant_id = $1 AND accepted_at IS NULL AND revoked_at IS NULL \
         ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, Invitation>(&sql)
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

/// Whether any user, in any tenant, already has this email.
pub async fn email_registered<'e, E: PgExecutor<'e>>(
    exec: E,
    email: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1::citext)")
        .bind(email)
        .fetch_one(exec)
        .await
}

/// Revokes the tenant's expired open invitation for `email`, so a new one can
/// take its place.
pub async fn revoke_expired_invitation<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    email: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE invitations SET revoked_at = now() \
         WHERE tenant_id = $1 AND email = $2::citext \
           AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at <= now()",
    )
    .bind(tenant_id)
    .bind(email)
    .execute(exec)
    .await
    .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_invitation<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    email: &str,
    role: &str,
    token_hash: &str,
    invited_by: Option<Uuid>,
    expires_at: DateTime<Utc>,
) -> Result<Invitation, sqlx::Error> {
    let sql = format!(
        "INSERT INTO invitations (tenant_id, email, role, token_hash, invited_by, expires_at) \
         VALUES ($1, $2::citext, $3, $4, $5, $6) RETURNING {INVITATION_COLS}"
    );
    sqlx::query_as::<_, Invitation>(&sql)
        .bind(tenant_id)
        .bind(email)
        .bind(role)
        .bind(token_hash)
        .bind(invited_by)
        .bind(expires_at)
        .fetch_one(exec)
        .await
}

/// Fetches and locks an open invitation of the tenant.
pub async fn select_open_invitation_for_update<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Invitation>, sqlx::Error> {
    let sql = format!(
        "SELECT {INVITATION_COLS} FROM invitations \
         WHERE id = $1 AND tenant_id = $2 AND accepted_at IS NULL AND revoked_at IS NULL \
         FOR UPDATE"
    );
    sqlx::query_as::<_, Invitation>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(exec)
        .await
}

pub async fn revoke_invitation<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invitations SET revoked_at = now() WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(exec)
        .await
        .map(|_| ())
}

/// An invitation found by its token, with its tenant's name.
#[derive(Debug, FromRow)]
pub struct InvitationByToken {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub email: String,
    pub role: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Finds an invitation by token hash; `lock` locks it for acceptance.
pub async fn select_invitation_by_token_hash<'e, E: PgExecutor<'e>>(
    exec: E,
    token_hash: &str,
    lock: bool,
) -> Result<Option<InvitationByToken>, sqlx::Error> {
    let sql = format!(
        "SELECT i.id, i.tenant_id, t.tenant_name, i.email::text AS email, i.role, i.expires_at, \
                i.accepted_at, i.revoked_at \
         FROM invitations i JOIN tenants t ON t.id = i.tenant_id \
         WHERE i.token_hash = $1{}",
        if lock { " FOR UPDATE OF i" } else { "" }
    );
    sqlx::query_as::<_, InvitationByToken>(&sql)
        .bind(token_hash)
        .fetch_optional(exec)
        .await
}

pub async fn mark_invitation_accepted<'e, E: PgExecutor<'e>>(
    exec: E,
    id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invitations SET accepted_at = now(), accepted_user_id = $2 WHERE id = $1")
        .bind(id)
        .bind(user_id)
        .execute(exec)
        .await
        .map(|_| ())
}
