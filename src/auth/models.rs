//! Auth domain types and request/response DTOs.
//!
//! Port of the Go `internal/auth/models.go`. Request DTOs use
//! `#[serde(deny_unknown_fields)]` to mirror Go's `DisallowUnknownFields`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Database representation of a user.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub is_owner: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stored refresh-token record.
#[derive(Debug, Clone, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /auth/register`.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    pub tenant_name: Option<String>,
    #[serde(default)]
    pub country: String,
    pub region: Option<String>,
}

/// Body for `POST /auth/login`.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

/// Body for `POST /auth/refresh`.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

/// Body for `POST /auth/logout`.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct LogoutRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

/// Body for `PATCH /auth/me`.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateMeRequest {
    pub display_name: Option<String>,
    pub current_password: Option<String>,
    pub new_password: Option<String>,
}

/// Returned from register, login, and refresh.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub is_owner: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Returned from `GET /auth/me` and `PATCH /auth/me`.
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub is_owner: bool,
    pub tenant_name: String,
    pub tier: String,
    pub country: String,
    pub region: Option<String>,
    pub feature_flags: HashMap<String, bool>,
    pub created_at: DateTime<Utc>,
}
