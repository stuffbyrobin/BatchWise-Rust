//! Per-user membership cache for authorisation.
//!
//! Every authenticated request needs the caller's role and whether the account
//! is still active. Both are cached for a short TTL and invalidated whenever a
//! user's role or active flag changes, so a change applies on the next request
//! on this instance and within the TTL on any other.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use super::authz::Role;
use super::errors::ApiError;

/// How long a user's membership is trusted before being re-read.
pub const ROLE_CACHE_TTL: Duration = Duration::from_secs(30);

/// A user's tenant, role and whether the account is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Membership {
    pub tenant_id: Uuid,
    pub role: Role,
    pub active: bool,
}

#[derive(Clone, Copy)]
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
        let row: Option<(Uuid, String, bool)> =
            sqlx::query_as("SELECT tenant_id, role, is_active FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
        let Some((tenant_id, role, active)) = row else {
            return Ok(None);
        };
        let role = role
            .parse::<Role>()
            .map_err(|_| ApiError::internal(format!("unknown role {role:?}")))?;
        let membership = Membership {
            tenant_id,
            role,
            active,
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
                membership,
            },
        );
        Ok(Some(membership))
    }

    /// Drops a user's cached membership (call after changing their role or
    /// active flag).
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
            .map(|e| e.membership)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidate_removes_entry() {
        let cache = RoleCache::default();
        let id = Uuid::new_v4();
        cache.entries.lock().unwrap().insert(
            id,
            Entry {
                fetched: Instant::now(),
                membership: Membership {
                    tenant_id: Uuid::new_v4(),
                    role: Role::Brewer,
                    active: true,
                },
            },
        );
        assert!(cache.fresh(id).is_some());
        cache.invalidate(id);
        assert!(cache.fresh(id).is_none());
    }
}
