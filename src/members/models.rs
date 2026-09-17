//! Member DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::platform::authz::Role;

/// A user of the tenant, as shown to Owners and Managers.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Member {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Every member of a tenant, oldest first.
#[derive(Debug, Serialize)]
pub struct MemberList {
    pub items: Vec<Member>,
}

/// Payload to change a member's role and/or active flag.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchMemberRequest {
    #[validate(custom(function = "validate_role"))]
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

/// An open invitation (not accepted or revoked), as shown to Owners and Managers.
/// It may have expired; `expires_at` says when.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Invitation {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub invited_by: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Every open invitation of a tenant, newest first.
#[derive(Debug, Serialize)]
pub struct InvitationList {
    pub items: Vec<Invitation>,
}

/// Returned once when an invitation is created. Only a hash of the token is
/// stored, so it cannot be shown again.
#[derive(Debug, Serialize)]
pub struct CreatedInvitation {
    #[serde(flatten)]
    pub invitation: Invitation,
    pub token: String,
}

/// Payload to invite someone to the tenant.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateInvitationRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

fn validate_role(role: &str) -> Result<(), ValidationError> {
    role.parse::<Role>()
        .map(|_| ())
        .map_err(|_| ValidationError::new("role"))
}
