//! Auth business logic.
//!
//! Port of the Go `internal/auth/service.go`: registration (atomic tenant and
//! user creation), login (global email lookup), refresh-token rotation, logout,
//! and profile read/update/delete.

use chrono::{Duration, Utc};
use uuid::Uuid;

use super::models::{
    AuthResponse, LoginRequest, MeResponse, RegisterRequest, UpdateMeRequest, User,
};
use super::password::{
    check_password_policy, dummy_hash, hash_password_async, verify_password_async,
};
use super::refresh::{generate_refresh_token, hash_refresh_token};
use super::repository as repo;
use crate::platform::errors::is_unique_violation;
use crate::platform::errors::ApiError;
use crate::state::AppState;
use crate::tenant::{presets, repository as tenant_repo};

/// Maximum failed login attempts allowed per minute per account.
/// This is a per-account throttle, independent of the per-IP limiter,
/// so distributed credential stuffing against one account is slowed.
pub const MAX_FAILED_LOGINS_PER_MINUTE: u32 = 10;

/// Well-known tenant for bootstrap admin users.
/// Used as the default tenant when `tenant_name` is omitted during registration
/// and `BOOTSTRAP_REGISTRATION_ENABLED` is on.
fn system_tenant_id() -> Uuid {
    Uuid::nil()
}

/// Registers a new user, creating a tenant when `tenant_name` is supplied.
///
/// The 409 responses for a taken email or tenant name are a deliberate usability
/// trade-off (they reveal existence) mitigated by the per-IP register rate limit.
/// When `tenant_name` is omitted and `BOOTSTRAP_REGISTRATION_ENABLED` is on,
/// the user lands in the shared system tenant (Uuid::nil) and can edit the
/// reference library every tenant reads; this is intended only for initial
/// operator setup.
pub async fn register(state: &AppState, req: RegisterRequest) -> Result<AuthResponse, ApiError> {
    check_password_policy(&req.password)?;

    if req.tenant_name.is_none() && !state.config.bootstrap_registration_enabled {
        return Err(ApiError::validation("tenant_name", "is required"));
    }

    let country = if req.country.is_empty() {
        "GB".to_string()
    } else {
        req.country.clone()
    };
    let hash = hash_password_async(req.password.clone()).await?;
    let email = req.email.to_lowercase();

    // Name conflict check mirrors the Go service (runs on the pool).
    if let Some(name) = &req.tenant_name {
        if tenant_repo::get_by_name(&state.pool, name).await?.is_some() {
            return Err(ApiError::conflict("tenant_name", "already taken"));
        }
    }

    let mut tx = state.pool.begin().await?;

    let (tenant_id, is_owner) = if let Some(name) = &req.tenant_name {
        let flags = presets::preset_for_tier("home");
        let id = tenant_repo::insert(
            &mut *tx,
            name,
            "home",
            &country,
            req.region.as_deref(),
            &flags,
        )
        .await?;
        (id, true)
    } else {
        (system_tenant_id(), false)
    };

    let user = match repo::create_user(
        &mut *tx,
        tenant_id,
        &email,
        &hash,
        &req.display_name,
        is_owner,
        true,
    )
    .await
    {
        Ok(u) => u,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict("email", "already registered"));
        }
        Err(e) => return Err(e.into()),
    };

    tx.commit().await?;

    issue_token_pair(state, &user).await
}

/// Authenticates by email (global lookup) and password.
pub async fn login(state: &AppState, req: LoginRequest) -> Result<AuthResponse, ApiError> {
    let invalid = || ApiError::unauthorized("Invalid email or password.");
    let email = req.email.to_lowercase();

    if let Some(retry) = state.login_failures.is_limited(&email) {
        return Err(ApiError::rate_limited(retry));
    }

    let user = repo::get_user_by_email_global(&state.pool, &email).await?;

    match user {
        Some(user) => {
            if !verify_password_async(req.password.clone(), user.password_hash.clone()).await {
                let _ = state.login_failures.check(&email);
                return Err(invalid());
            }
            if !user.is_active {
                return Err(ApiError::forbidden("Account is inactive."));
            }
            issue_token_pair(state, &user).await
        }
        None => {
            // Keep unknown and known emails on the same timing path.
            let _ = verify_password_async(req.password.clone(), dummy_hash().to_string()).await;
            Err(invalid())
        }
    }
}

/// Rotates a refresh token, returning a fresh token pair.
pub async fn refresh(state: &AppState, refresh_token: &str) -> Result<AuthResponse, ApiError> {
    let invalid = || ApiError::unauthorized("Invalid or expired refresh token.");
    let rt = repo::get_refresh_token_by_hash(&state.pool, &hash_refresh_token(refresh_token))
        .await?
        .ok_or_else(invalid)?;

    if rt.used_at.is_some() {
        // Presenting an already-rotated token is the primary signal of token theft
        // (RFC 6819 \xA75.2.2.3), so the whole family for that user is revoked.
        repo::delete_refresh_tokens_for_user(&state.pool, rt.user_id).await?;
        return Err(ApiError::unauthorized("Refresh token already used."));
    }
    if Utc::now() > rt.expires_at {
        return Err(ApiError::unauthorized("Refresh token expired."));
    }

    let user = repo::get_user_by_id(&state.pool, rt.user_id)
        .await?
        .ok_or_else(invalid)?;
    if !user.is_active {
        return Err(ApiError::forbidden("Account is inactive."));
    }

    if !repo::mark_refresh_token_used(&state.pool, rt.id).await? {
        // Lost a concurrent rotation race: treat exactly like a replay.
        repo::delete_refresh_tokens_for_user(&state.pool, rt.user_id).await?;
        return Err(ApiError::unauthorized("Refresh token already used."));
    }
    issue_token_pair(state, &user).await
}

/// Invalidates a refresh token. Idempotent: unknown tokens succeed silently.
pub async fn logout(state: &AppState, refresh_token: &str) -> Result<(), ApiError> {
    if let Some(rt) =
        repo::get_refresh_token_by_hash(&state.pool, &hash_refresh_token(refresh_token)).await?
    {
        let _ = repo::mark_refresh_token_used(&state.pool, rt.id).await?;
    }
    Ok(())
}

/// Returns the current user's profile joined with tenant info.
pub async fn me(state: &AppState, user_id: Uuid) -> Result<MeResponse, ApiError> {
    let user = repo::get_user_by_id(&state.pool, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;
    let tenant = tenant_repo::get_by_id(&state.pool, user.tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))?;

    Ok(MeResponse {
        user_id: user.id,
        tenant_id: user.tenant_id,
        email: user.email,
        display_name: user.display_name,
        is_owner: user.is_owner,
        role: user.role,
        tenant_name: tenant.tenant_name,
        tier: tenant.tier,
        country: tenant.country,
        region: tenant.region,
        feature_flags: tenant.feature_flags,
        created_at: user.created_at,
    })
}

/// Updates display name and/or password. Changing the password requires the
/// current one and invalidates all refresh tokens.
pub async fn update_me(
    state: &AppState,
    user_id: Uuid,
    req: UpdateMeRequest,
) -> Result<MeResponse, ApiError> {
    let user = repo::get_user_by_id(&state.pool, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;

    let mut new_hash = user.password_hash.clone();
    let changing_password = req.new_password.is_some();
    if let Some(new_password) = &req.new_password {
        let current = req.current_password.as_deref().ok_or_else(|| {
            ApiError::validation("current_password", "required when changing password")
        })?;
        if !verify_password_async(current.to_string(), user.password_hash.clone()).await {
            return Err(ApiError::validation("current_password", "incorrect"));
        }
        check_password_policy(new_password)?;
        new_hash = hash_password_async(new_password.clone()).await?;
    }

    let new_display = req
        .display_name
        .unwrap_or_else(|| user.display_name.clone());
    repo::update_user(
        &state.pool,
        user_id,
        &new_display,
        &new_hash,
        user.is_active,
    )
    .await?;

    if changing_password {
        repo::delete_refresh_tokens_for_user(&state.pool, user_id).await?;
    }
    me(state, user_id).await
}

/// Soft-deletes the user and revokes all refresh tokens.
pub async fn delete_me(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    repo::deactivate_user(&state.pool, user_id).await?;
    repo::delete_refresh_tokens_for_user(&state.pool, user_id).await?;
    // Refuse the still-valid access token from the next request.
    state.roles.invalidate(user_id);
    Ok(())
}

async fn issue_token_pair(state: &AppState, user: &User) -> Result<AuthResponse, ApiError> {
    let (token, hash) = generate_refresh_token();
    let expires_at = Utc::now() + Duration::days(state.config.refresh_token_expiry_days);
    repo::insert_refresh_token(&state.pool, user.id, &hash, expires_at).await?;

    let (access_token, expires_in) = state
        .jwt
        .issue(user.id, user.tenant_id)
        .map_err(|e| ApiError::internal(format!("issue jwt: {e}")))?;

    Ok(AuthResponse {
        user_id: user.id,
        tenant_id: user.tenant_id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        is_owner: user.is_owner,
        role: user.role.clone(),
        access_token,
        refresh_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
    })
}
