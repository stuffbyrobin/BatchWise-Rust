//! Per-user membership cache for authorisation.
//!
//! Every authenticated request needs the caller's role, whether the account is
//! still active, and whether its access token has been revoked (the user's token
//! version and any single tokens revoked at logout). All of it is cached for a
//! short TTL and invalidated whenever it changes, so a change applies on the
//! next request on this instance and within the TTL on any other.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use super::authz::Role;
use super::errors::ApiError;
use crate::auth::jwt::Claims;

/// How long a user's membership is trusted before being re-read.
pub const ROLE_CACHE_TTL: Duration = Duration::from_secs(30);

/// A user's tenant, role, whether the account is active, and what revokes
/// their access tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    pub tenant_id: Uuid,
    pub role: Role,
    pub active: bool,
    /// Access tokens issued at any other version are revoked.
    pub token_version: i32,
    /// Unexpired single tokens revoked at logout.
    pub revoked_jtis: Arc<[Uuid]>,
}

impl Membership {
    /// Whether an access token with these claims may still be used: the account
    /// is active in the token's tenant and the token has not been revoked.
    pub fn accepts(&self, claims: &Claims) -> bool {
        self.active
            && self.tenant_id == claims.tenant_id
            && self.token_version == claims.version
            && !self.revoked_jtis.contains(&claims.jti)
    }
}

#[derive(Clone)]
struct Entry {
    fetched: Instant,
    membership: Membership,
}

/// Cached [`Membership`] per user.
#[derive(Default)]
pub struct RoleCache {
    entries: Mutex<HashMap<Uuid, Entry>>,
}

impl RoleCache {
    /// The user's membership, or `None` if the user no longer exists.
    pub async fn get(&self, pool: &PgPool, user_id: Uuid) -> Result<Option<Membership>, ApiError> {
        if let Some(m) = self.fresh(user_id) {
            return Ok(Some(m));
        }
        let row: Option<(Uuid, String, bool, i32, Vec<Uuid>)> = sqlx::query_as(
            "SELECT u.tenant_id, u.role, u.is_active, u.token_version, \
             ARRAY(SELECT r.jti FROM revoked_access_tokens r \
                   WHERE r.user_id = u.id AND r.expires_at > now()) \
             FROM users u WHERE u.id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        let Some((tenant_id, role, active, token_version, revoked_jtis)) = row else {
            return Ok(None);
        };
        let role = role
            .parse::<Role>()
            .map_err(|_| ApiError::internal(format!("unknown role {role:?}")))?;
        let membership = Membership {
            tenant_id,
            role,
            active,
            token_version,
            revoked_jtis: revoked_jtis.into(),
        };
        let mut entries = self.entries.lock().expect("role cache mutex");
        // Bounded by the number of users, but drop stale entries once it grows.
        if entries.len() > 16_384 {
            entries.retain(|_, e| e.fetched.elapsed() < ROLE_CACHE_TTL);
        }
        entries.insert(
            user_id,
            Entry {
                fetched: Instant::now(),
                membership: membership.clone(),
            },
        );
        Ok(Some(membership))
    }

    /// Drops a user's cached membership (call after changing their role, active
    /// flag or token version, or revoking one of their tokens).
    pub fn invalidate(&self, user_id: Uuid) {
        self.entries
            .lock()
            .expect("role cache mutex")
            .remove(&user_id);
    }

    fn fresh(&self, user_id: Uuid) -> Option<Membership> {
        let entries = self.entries.lock().expect("role cache mutex");
        entries
            .get(&user_id)
            .filter(|e| e.fetched.elapsed() < ROLE_CACHE_TTL)
            .map(|e| e.membership.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn membership(tenant_id: Uuid) -> Membership {
        Membership {
            tenant_id,
            role: Role::Brewer,
            active: true,
            token_version: 2,
            revoked_jtis: Arc::from([]),
        }
    }

    #[test]
    fn accepts_only_live_tokens_of_the_current_version() {
        let tenant_id = Uuid::new_v4();
        let claims = Claims {
            subject: Uuid::new_v4(),
            tenant_id,
            jti: Uuid::new_v4(),
            version: 2,
            expires_at: Utc::now(),
        };
        let m = membership(tenant_id);
        assert!(m.accepts(&claims));
        assert!(!m.accepts(&Claims {
            version: 1,
            ..claims
        }));
        assert!(!m.accepts(&Claims {
            tenant_id: Uuid::new_v4(),
            ..claims
        }));
        assert!(!Membership {
            active: false,
            ..m.clone()
        }
        .accepts(&claims));
        assert!(!Membership {
            revoked_jtis: Arc::from([claims.jti]),
            ..m
        }
        .accepts(&claims));
    }

    #[test]
    fn invalidate_removes_entry() {
        let cache = RoleCache::default();
        let id = Uuid::new_v4();
        cache.entries.lock().unwrap().insert(
            id,
            Entry {
                fetched: Instant::now(),
                membership: membership(Uuid::new_v4()),
            },
        );
        assert!(cache.fresh(id).is_some());
        cache.invalidate(id);
        assert!(cache.fresh(id).is_none());
    }
}
