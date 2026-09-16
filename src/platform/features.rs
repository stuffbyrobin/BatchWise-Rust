//! Per-tenant feature-flag cache for the [`check_feature`](super::middleware::check_feature) gate.
//!
//! Every gated request used to read the tenant's `tier` and `feature_flags`. A
//! cached "enabled" answer is now trusted for a short TTL. A cached "disabled"
//! answer is always re-read before refusing, so enabling a feature takes effect
//! on the next request and only revocation can lag, by at most the TTL.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use uuid::Uuid;

use super::errors::ApiError;

/// How long a tenant's flags are trusted before being re-read.
pub const FEATURE_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone)]
struct Entry {
    fetched: Instant,
    tier: String,
    flags: HashMap<String, bool>,
}

/// Cached `tier` + `feature_flags` per tenant.
#[derive(Default)]
pub struct FeatureCache {
    entries: Mutex<HashMap<Uuid, Entry>>,
}

impl FeatureCache {
    /// Returns whether `key` is enabled for the tenant, and the tenant's tier.
    /// Errors with 401 when the tenant no longer exists.
    pub async fn check(
        &self,
        pool: &PgPool,
        tenant_id: Uuid,
        key: &str,
    ) -> Result<(bool, String), ApiError> {
        if let Some(e) = self.fresh(tenant_id) {
            if e.flags.get(key).copied().unwrap_or(false) {
                return Ok((true, e.tier));
            }
        }
        let e = self.load(pool, tenant_id).await?;
        Ok((e.flags.get(key).copied().unwrap_or(false), e.tier))
    }

    /// Drops a tenant's cached flags (call after changing its tier or flags).
    pub fn invalidate(&self, tenant_id: Uuid) {
        self.entries
            .lock()
            .expect("feature cache mutex")
            .remove(&tenant_id);
    }

    fn fresh(&self, tenant_id: Uuid) -> Option<Entry> {
        let entries = self.entries.lock().expect("feature cache mutex");
        entries
            .get(&tenant_id)
            .filter(|e| e.fetched.elapsed() < FEATURE_CACHE_TTL)
            .cloned()
    }

    async fn load(&self, pool: &PgPool, tenant_id: Uuid) -> Result<Entry, ApiError> {
        let row: Option<(String, sqlx::types::Json<HashMap<String, bool>>)> =
            sqlx::query_as("SELECT tier, feature_flags FROM tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_optional(pool)
                .await?;
        let (tier, flags) = row.ok_or_else(|| ApiError::unauthorized("missing tenant context"))?;
        let entry = Entry {
            fetched: Instant::now(),
            tier,
            flags: flags.0,
        };
        let mut entries = self.entries.lock().expect("feature cache mutex");
        // Bounded by the number of tenants, but drop stale entries once it grows.
        if entries.len() > 4096 {
            entries.retain(|_, e| e.fetched.elapsed() < FEATURE_CACHE_TTL);
        }
        entries.insert(tenant_id, entry.clone());
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidate_removes_entry() {
        let cache = FeatureCache::default();
        let id = Uuid::new_v4();
        cache.entries.lock().unwrap().insert(
            id,
            Entry {
                fetched: Instant::now(),
                tier: "home".into(),
                flags: HashMap::from([("recipes".to_string(), true)]),
            },
        );
        assert!(cache.fresh(id).is_some());
        cache.invalidate(id);
        assert!(cache.fresh(id).is_none());
    }
}
