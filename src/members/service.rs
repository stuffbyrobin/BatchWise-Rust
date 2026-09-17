//! Member management rules: who may change whom, and the last-Owner guard.

use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    CreateInvitationRequest, CreatedInvitation, InvitationList, Member, MemberList,
    PatchMemberRequest,
};
use super::repository as repo;
use crate::audit;
use crate::auth::refresh::generate_refresh_token;
use crate::platform::authz::Role;
use crate::platform::errors::{is_unique_violation, ApiError};
use crate::state::AppState;

/// How long an invitation link stays valid.
pub const INVITATION_TTL_DAYS: i64 = 7;

/// Every member of the tenant.
pub async fn list(state: &AppState, tenant_id: Uuid) -> Result<MemberList, ApiError> {
    Ok(MemberList {
        items: repo::select_members(&state.pool, tenant_id).await?,
    })
}

/// Changes a member's role and/or active flag.
///
/// Managers may only change Brewer, Sales and Viewer members, and only to one
/// of those roles. The tenant's last active Owner cannot be demoted or
/// deactivated. Deactivation revokes the member's refresh tokens, and every
/// change applies from the member's next request.
pub async fn patch(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    actor_role: Role,
    id: Uuid,
    req: PatchMemberRequest,
) -> Result<Member, ApiError> {
    if req.role.is_none() && req.is_active.is_none() {
        return Err(ApiError::validation("body", "at least one field required"));
    }
    let new_role = req.role.as_deref().map(parse_role).transpose()?;

    let mut tx = state.pool.begin().await?;
    // Lock the Owners first, so two Owners cannot demote each other at once.
    repo::lock_owners(&mut *tx, tenant_id).await?;
    let current = repo::select_member_for_update(&mut *tx, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("member"))?;
    let current_role = parse_role(&current.role)?;
    let role = new_role.unwrap_or(current_role);
    let active = req.is_active.unwrap_or(current.is_active);

    if actor_role != Role::Owner && !(is_staff(current_role) && is_staff(role)) {
        return Err(ApiError::forbidden(
            "A Manager can only manage Brewer, Sales and Viewer members.",
        ));
    }
    let loses_owner =
        current_role == Role::Owner && current.is_active && (role != Role::Owner || !active);
    if loses_owner && repo::count_other_active_owners(&mut *tx, tenant_id, id).await? == 0 {
        return Err(last_owner());
    }

    let updated = repo::update_member(&mut *tx, tenant_id, id, role.as_str(), active).await?;
    if current.is_active && !active {
        repo::delete_refresh_tokens(&mut *tx, id).await?;
        // Tokens stay revoked if the member is reactivated later.
        crate::auth::repository::revoke_all_access_tokens(&mut *tx, id).await?;
    }
    if role != current_role {
        audit::service::write(
            &mut *tx,
            audit::models::WriteRequest {
                tenant_id,
                event_type: audit::models::EVENT_MEMBER_ROLE_CHANGED,
                entity_type: "user",
                entity_id: Some(id),
                actor_user_id: actor_id,
                event_data: json!({
                    "email": current.email,
                    "from_role": current_role.as_str(),
                    "to_role": role.as_str(),
                }),
            },
        )
        .await?;
    }
    if active != current.is_active {
        audit::service::write(
            &mut *tx,
            audit::models::WriteRequest {
                tenant_id,
                event_type: if active {
                    audit::models::EVENT_MEMBER_REACTIVATED
                } else {
                    audit::models::EVENT_MEMBER_DEACTIVATED
                },
                entity_type: "user",
                entity_id: Some(id),
                actor_user_id: actor_id,
                event_data: json!({"email": current.email, "role": role.as_str()}),
            },
        )
        .await?;
    }
    tx.commit().await?;

    state.roles.invalidate(id);
    Ok(updated)
}

/// Whether `user_id`, with `role`, is the tenant's only active Owner.
pub async fn is_last_active_owner(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    role: Role,
) -> Result<bool, ApiError> {
    Ok(
        role == Role::Owner
            && repo::count_other_active_owners(pool, tenant_id, user_id).await? == 0,
    )
}

/// Refusal for any change that would leave a tenant without an active Owner.
pub fn last_owner() -> ApiError {
    ApiError::business_rule(
        "last_owner",
        "A tenant must keep an active Owner. Make another member an Owner first.",
        Default::default(),
    )
}

/// Every open invitation of the tenant (expired ones included, so they can be
/// revoked or replaced).
pub async fn list_invitations(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<InvitationList, ApiError> {
    Ok(InvitationList {
        items: repo::select_open_invitations(&state.pool, tenant_id).await?,
    })
}

/// Invites someone to the tenant with a role and returns the one-time token,
/// which is stored only as a hash. Managers may only invite Brewer, Sales and
/// Viewer members. Emails that already have an account are refused, since an
/// account belongs to one tenant; so is a second open invitation for the same
/// email (an expired one is replaced).
pub async fn invite(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    actor_role: Role,
    req: CreateInvitationRequest,
) -> Result<CreatedInvitation, ApiError> {
    let role = parse_role(&req.role)?;
    if actor_role != Role::Owner && !is_staff(role) {
        return Err(ApiError::forbidden(
            "A Manager can only invite Brewer, Sales and Viewer members.",
        ));
    }
    let email = req.email.trim().to_lowercase();

    let mut tx = state.pool.begin().await?;
    if repo::email_registered(&mut *tx, &email).await? {
        return Err(ApiError::conflict("email", "already has an account"));
    }
    repo::revoke_expired_invitation(&mut *tx, tenant_id, &email).await?;
    let (token, token_hash) = generate_refresh_token();
    let expires_at = Utc::now() + Duration::days(INVITATION_TTL_DAYS);
    let invitation = match repo::insert_invitation(
        &mut *tx,
        tenant_id,
        &email,
        role.as_str(),
        &token_hash,
        actor_id,
        expires_at,
    )
    .await
    {
        Ok(invitation) => invitation,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "email",
                "already has an open invitation",
            ));
        }
        Err(e) => return Err(e.into()),
    };
    audit::service::write(
        &mut *tx,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_MEMBER_INVITED,
            entity_type: "invitation",
            entity_id: Some(invitation.id),
            actor_user_id: actor_id,
            event_data: json!({"email": email, "role": role.as_str()}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(CreatedInvitation { invitation, token })
}

/// Revokes an open invitation. Managers may only revoke Brewer, Sales and
/// Viewer invitations.
pub async fn revoke_invitation(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    actor_role: Role,
    id: Uuid,
) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    let invitation = repo::select_open_invitation_for_update(&mut *tx, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("invitation"))?;
    let role = parse_role(&invitation.role)?;
    if actor_role != Role::Owner && !is_staff(role) {
        return Err(ApiError::forbidden(
            "A Manager can only manage Brewer, Sales and Viewer members.",
        ));
    }
    repo::revoke_invitation(&mut *tx, tenant_id, id).await?;
    audit::service::write(
        &mut *tx,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_MEMBER_INVITATION_REVOKED,
            entity_type: "invitation",
            entity_id: Some(id),
            actor_user_id: actor_id,
            event_data: json!({"email": invitation.email, "role": role.as_str()}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Brewer, Sales and Viewer: the members a Manager may manage.
fn is_staff(role: Role) -> bool {
    matches!(role, Role::Brewer | Role::Sales | Role::Viewer)
}

fn parse_role(role: &str) -> Result<Role, ApiError> {
    role.parse()
        .map_err(|_| ApiError::validation("role", "invalid"))
}
