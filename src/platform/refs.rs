//! Tenant checks for ids a client supplies in a request body.
//!
//! A foreign key only proves the referenced row exists, not that the caller's
//! tenant owns it, so without a check one tenant could attach its records to
//! another tenant's batch, recipe or fermenter. Services call [`ensure_ref`]
//! (or [`ensure_opt_ref`]) before writing. A row owned by another tenant is
//! rejected with exactly the same 400 as an id that does not exist (the same
//! error the foreign-key violation mapping produces), so the response cannot
//! be used to probe for other tenants' ids.

use sqlx::PgExecutor;
use uuid::Uuid;

use super::errors::ApiError;
use crate::library::models::SYSTEM_TENANT_ID;

/// A table a request body may reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ref {
    Batch,
    Fermenter,
    Recipe,
    WaterProfile,
    /// Library style: the caller's own or a shared system-tenant row.
    Style,
    /// Library equipment profile: the caller's own or a shared system-tenant row.
    EquipmentProfile,
    /// Library mash profile: the caller's own or a shared system-tenant row.
    MashProfile,
    /// Library yeast: the caller's own or a shared system-tenant row.
    LibraryYeast,
}

impl Ref {
    /// The referenced table. Static strings only, so it is safe to format into SQL.
    fn table(self) -> &'static str {
        match self {
            Ref::Batch => "batches",
            Ref::Fermenter => "fermenters",
            Ref::Recipe => "recipes",
            Ref::WaterProfile => "water_profiles",
            Ref::Style => "styles",
            Ref::EquipmentProfile => "equipment_profiles",
            Ref::MashProfile => "mash_profiles",
            Ref::LibraryYeast => "yeasts",
        }
    }

    /// Shared library tables also accept rows owned by the system tenant.
    fn shared(self) -> bool {
        matches!(
            self,
            Ref::Style | Ref::EquipmentProfile | Ref::MashProfile | Ref::LibraryYeast
        )
    }
}

/// Returns `Ok(())` when `id` is a row of `r` visible to `tenant_id`, otherwise a
/// 400 validation error on `field` ("references a resource that does not exist").
pub async fn ensure_ref<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    r: Ref,
    id: Uuid,
    field: &str,
) -> Result<(), ApiError> {
    let also = if r.shared() {
        SYSTEM_TENANT_ID
    } else {
        tenant_id
    };
    let sql = format!(
        "SELECT EXISTS (SELECT 1 FROM {} WHERE id = $1 AND tenant_id IN ($2, $3))",
        r.table()
    );
    let visible: bool = sqlx::query_scalar(&sql)
        .bind(id)
        .bind(tenant_id)
        .bind(also)
        .fetch_one(exec)
        .await?;
    if visible {
        Ok(())
    } else {
        Err(ApiError::validation(
            field,
            "references a resource that does not exist",
        ))
    }
}

/// [`ensure_ref`] for an optional id; `None` always passes.
pub async fn ensure_opt_ref<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    r: Ref,
    id: Option<Uuid>,
    field: &str,
) -> Result<(), ApiError> {
    match id {
        Some(id) => ensure_ref(exec, tenant_id, r, id, field).await,
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_library_tables_are_shared() {
        assert!(Ref::Style.shared());
        assert!(Ref::EquipmentProfile.shared());
        assert!(Ref::MashProfile.shared());
        assert!(Ref::LibraryYeast.shared());
        for r in [Ref::Batch, Ref::Fermenter, Ref::Recipe, Ref::WaterProfile] {
            assert!(!r.shared(), "{r:?}");
        }
    }

    #[test]
    fn tables_are_plain_identifiers() {
        for r in [
            Ref::Batch,
            Ref::Fermenter,
            Ref::Recipe,
            Ref::WaterProfile,
            Ref::Style,
            Ref::EquipmentProfile,
            Ref::MashProfile,
            Ref::LibraryYeast,
        ] {
            assert!(r
                .table()
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_'));
        }
    }
}
