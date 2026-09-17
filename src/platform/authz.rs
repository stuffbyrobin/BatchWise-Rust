//! Role-based access control.
//!
//! Every authenticated request is authorised in
//! [`require_auth`](super::middleware::require_auth) from its route template:
//! the first path segment after `/api/v1` names an [`Area`], and the method
//! gives the [`Access`] needed (reads for `GET`, writes otherwise). Each
//! [`Role`] has a fixed access level per area, from the table agreed in
//! `docs/remediation-plan.md` (Phase 15). Routes outside the map are refused.
//!
//! Rules that depend on the record rather than the route (spoiling a completed
//! batch, approving a label record) are checked in their services with
//! [`Role::is_manager`]. Record locks (Phase 14) apply to every role.

use std::fmt;
use std::str::FromStr;

use axum::http::Method;

use super::errors::ApiError;

/// A tenant member's role, stored in `users.role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Manager,
    Brewer,
    Sales,
    Viewer,
}

impl Role {
    pub const ALL: [Role; 5] = [
        Role::Owner,
        Role::Manager,
        Role::Brewer,
        Role::Sales,
        Role::Viewer,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Manager => "manager",
            Role::Brewer => "brewer",
            Role::Sales => "sales",
            Role::Viewer => "viewer",
        }
    }

    /// Owner or Manager: compliance sign-off and corrections to finished records.
    pub fn is_manager(self) -> bool {
        matches!(self, Role::Owner | Role::Manager)
    }
}

impl FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Role::ALL.into_iter().find(|r| r.as_str() == s).ok_or(())
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A group of routes that share one row of the permission table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    /// The caller's own account (`/auth/me`, logout).
    Account,
    /// Tenant settings. Everyone reads them (the editors need, for example, the
    /// IBU method); only the Owner changes them.
    Tenant,
    /// The dashboard summary.
    Dashboard,
    /// Recipes, library, water, yeast bank, inventory, suppliers, purchase
    /// orders, batches, fermentation, fermenters, equipment, calendar, packaging
    /// runs, label designs and records, and traceability.
    Production,
    /// Distribution movements and returnable containers.
    Distribution,
    /// Customers and sales orders.
    Sales,
    /// Cost rates, batch costs and cost reports.
    Costs,
    /// Duty returns and duty events.
    Duty,
    /// The compliance audit log.
    Audit,
    /// Tenant members. Owners and Managers manage them (Managers only Brewer,
    /// Sales and Viewer members, checked in the members service); nobody else
    /// sees the list.
    Members,
}

/// How much a role may do in an area. Ordered: `Write` includes `Read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    None,
    Read,
    Write,
}

/// The most `role` may do in `area`.
pub fn granted(role: Role, area: Area) -> Access {
    use Access::{None, Read, Write};
    match area {
        Area::Account => Write,
        Area::Dashboard => Read,
        Area::Tenant => match role {
            Role::Owner => Write,
            _ => Read,
        },
        Area::Production => match role {
            Role::Owner | Role::Manager | Role::Brewer => Write,
            Role::Sales | Role::Viewer => Read,
        },
        Area::Distribution => match role {
            Role::Viewer => Read,
            _ => Write,
        },
        Area::Sales => match role {
            Role::Owner | Role::Manager | Role::Sales => Write,
            Role::Brewer | Role::Viewer => Read,
        },
        Area::Costs => match role {
            Role::Owner | Role::Manager => Write,
            Role::Brewer | Role::Viewer => Read,
            Role::Sales => None,
        },
        Area::Duty => match role {
            Role::Owner | Role::Manager => Write,
            Role::Viewer => Read,
            Role::Brewer | Role::Sales => None,
        },
        Area::Audit => match role {
            Role::Owner | Role::Manager | Role::Viewer => Read,
            Role::Brewer | Role::Sales => None,
        },
        Area::Members => match role {
            Role::Owner | Role::Manager => Write,
            Role::Brewer | Role::Sales | Role::Viewer => None,
        },
    }
}

/// The area and access a route needs, from its method and matched path
/// template (for example `/api/v1/batches/{id}`). `None` for routes outside the
/// map, which are refused.
pub fn requirement(method: &Method, path: &str) -> Option<(Area, Access)> {
    let rest = path.strip_prefix("/api/v1/")?;
    let area = match rest.split('/').next().unwrap_or("") {
        "auth" => Area::Account,
        "tenants" => Area::Tenant,
        "dashboard" => Area::Dashboard,
        "library" | "inventory" | "stock-movements" | "recipes" | "batches" | "calendar-events"
        | "fermenters" | "yeast-kinetics" | "water-profiles" | "water-adjustments"
        | "yeast-bank" | "equipment" | "maintenance-due" | "suppliers" | "purchase-orders"
        | "packaging-runs" | "label-records" | "label-designs" | "brand-profiles"
        | "brand-assets" | "traceability" => Area::Production,
        "distribution-movements" | "container-assets" | "container-logs" | "qr-codes" => {
            Area::Distribution
        }
        "customers" | "orders" => Area::Sales,
        "cost-rates" | "batch-costs" | "cost-reports" => Area::Costs,
        "duty-returns" | "duty-events" => Area::Duty,
        "compliance-audit" => Area::Audit,
        "members" => Area::Members,
        _ => return None,
    };
    // A calculation that changes nothing is a read.
    let read = *method == Method::GET
        || *method == Method::HEAD
        || path == "/api/v1/water-adjustments/calculate";
    Some((area, if read { Access::Read } else { Access::Write }))
}

/// Refuses the request unless `role` may call the route.
pub fn authorize(role: Role, method: &Method, path: &str) -> Result<(), ApiError> {
    let (area, needed) = requirement(method, path)
        .ok_or_else(|| ApiError::forbidden("This route has no permission rule."))?;
    if granted(role, area) >= needed {
        Ok(())
    } else {
        Err(ApiError::forbidden(&format!(
            "The {role} role does not allow this action."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: Access = Access::Write;
    const R: Access = Access::Read;
    const N: Access = Access::None;

    /// The table in docs/remediation-plan.md (Phase 15), one row per area.
    /// Columns: Owner, Manager, Brewer, Sales, Viewer.
    #[test]
    fn grants_match_the_agreed_table() {
        let table: [(Area, [Access; 5]); 10] = [
            (Area::Account, [W, W, W, W, W]),
            (Area::Tenant, [W, R, R, R, R]),
            (Area::Dashboard, [R, R, R, R, R]),
            (Area::Production, [W, W, W, R, R]),
            (Area::Distribution, [W, W, W, W, R]),
            (Area::Sales, [W, W, R, W, R]),
            (Area::Costs, [W, W, R, N, R]),
            (Area::Duty, [W, W, N, N, R]),
            (Area::Audit, [R, R, N, N, R]),
            (Area::Members, [W, W, N, N, N]),
        ];
        for (area, row) in table {
            for (role, expected) in Role::ALL.into_iter().zip(row) {
                assert_eq!(granted(role, area), expected, "{role} in {area:?}");
            }
        }
    }

    #[test]
    fn routes_map_to_an_area_and_access() {
        let cases = [
            (
                Method::GET,
                "/api/v1/batches/{id}",
                Some((Area::Production, R)),
            ),
            (
                Method::POST,
                "/api/v1/batches/{id}/transition",
                Some((Area::Production, W)),
            ),
            (
                Method::POST,
                "/api/v1/water-adjustments/calculate",
                Some((Area::Production, R)),
            ),
            (
                Method::PATCH,
                "/api/v1/duty-returns/{id}",
                Some((Area::Duty, W)),
            ),
            (
                Method::GET,
                "/api/v1/compliance-audit",
                Some((Area::Audit, R)),
            ),
            (
                Method::POST,
                "/api/v1/distribution-movements/{id}/void",
                Some((Area::Distribution, W)),
            ),
            (
                Method::PATCH,
                "/api/v1/members/{id}",
                Some((Area::Members, W)),
            ),
            (Method::GET, "/api/v1/not-a-route", None),
            (Method::GET, "/healthz", None),
        ];
        for (method, path, expected) in cases {
            assert_eq!(requirement(&method, path), expected, "{method} {path}");
        }
    }

    #[test]
    fn authorize_refuses_missing_access_and_unmapped_routes() {
        assert!(authorize(Role::Brewer, &Method::POST, "/api/v1/recipes").is_ok());
        assert!(authorize(Role::Viewer, &Method::POST, "/api/v1/recipes").is_err());
        assert!(authorize(Role::Brewer, &Method::POST, "/api/v1/duty-returns/compile").is_err());
        assert!(authorize(Role::Owner, &Method::GET, "/api/v1/not-a-route").is_err());
    }

    #[test]
    fn roles_round_trip_through_their_names() {
        for role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>(), Ok(role));
        }
        assert!("admin".parse::<Role>().is_err());
    }
}
