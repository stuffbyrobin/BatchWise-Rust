//! Dev helper: grant a tenant every gated feature (and bump it to the
//! `enterprise` tier) so a test user can exercise the whole app.
//!
//! Usage:  cargo run --example grant_all_features -- <user-email>
//!
//! Connects to the same database the app uses (DATABASE_URL from .env) and
//! sets the owning tenant's `feature_flags` to all-true. Not part of the app.

use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;

// Every key the `check_feature` middleware gates on, plus the base features a
// fresh "home" tenant gets — i.e. the full set, so nothing is blocked.
const ALL_FEATURES: &[&str] = &[
    "inventory",
    "recipes",
    "batches",
    "calendar",
    "yeastkinetics",
    "library",
    "water",
    "yeast_banking",
    "fermentation",
    "tracking",
    "reporting",
    "sales",
    "duty",
    "allergens",
    "labels",
    "label_design",
    "packaging",
    "traceability",
    "equipment_maintenance",
    "procurement",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let email = std::env::args()
        .nth(1)
        .expect("usage: cargo run --example grant_all_features -- <user-email>");
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;

    let flags: HashMap<&str, bool> = ALL_FEATURES.iter().map(|k| (*k, true)).collect();
    let flags_json = serde_json::to_value(&flags)?;

    let res = sqlx::query(
        "UPDATE tenants SET tier = 'enterprise', feature_flags = $1, updated_at = now() \
         WHERE id = (SELECT tenant_id FROM users WHERE email = $2)",
    )
    .bind(&flags_json)
    .bind(&email)
    .execute(&pool)
    .await?;

    println!(
        "Updated {} tenant(s) for {email}: tier=enterprise, {} features enabled.",
        res.rows_affected(),
        ALL_FEATURES.len()
    );
    Ok(())
}
