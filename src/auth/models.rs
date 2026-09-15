//! Auth domain types and request/response DTOs.
//!
//! Port of the Go `internal/auth/models.go`. Request DTOs use
//! `#[serde(deny_unknown_fields)]` to mirror Go's `DisallowUnknownFields`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

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
    /// The 12-char minimum and character classes are enforced by
    /// `password::check_password_policy`; this only bounds the input size.
    #[validate(length(min = 1, max = 128))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    #[validate(length(min = 1, max = 100))]
    pub tenant_name: Option<String>,
    /// ISO 3166-1 alpha-2; empty means "GB" (the column is `CHAR(2)`).
    #[serde(default)]
    #[validate(custom(function = "validate_country"))]
    pub country: String,
    #[validate(length(max = 100))]
    pub region: Option<String>,
}

/// Accepts an empty string (defaulted later) or exactly two ASCII letters.
fn validate_country(v: &str) -> Result<(), ValidationError> {
    if v.is_empty() || (v.len() == 2 && v.chars().all(|c| c.is_ascii_alphabetic())) {
        Ok(())
    } else {
        Err(ValidationError::new("country_code"))
    }
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
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,
    #[validate(length(min = 1, max = 128))]
    pub current_password: Option<String>,
    #[validate(length(min = 1, max = 128))]
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
