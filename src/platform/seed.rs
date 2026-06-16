//! Idempotent reference-data seeding.
//!
//! Equivalent of the Go project's `make seed`: applies the SQL files under
//! `migrations/seed/` (default BJCP styles, yeasts, equipment, fermentables,
//! water profiles, allergen lots) into the system tenant. Every statement uses
//! `ON CONFLICT DO NOTHING` on deterministic UUIDs, so running it repeatedly is
//! safe. Seed data is never applied automatically — only via the `--seed` CLI
//! flag or the test harness.

use sqlx::PgPool;

/// Seed SQL embedded at compile time, applied in lexical order: BJCP styles,
/// yeasts, equipment, fermentables, default water profiles, and the system-tenant
/// allergen reference lots (each with a unique lot number).
const SEED_FILES: &[&str] = &[
    include_str!("../../migrations/seed/001_styles.sql"),
    include_str!("../../migrations/seed/002_yeasts.sql"),
    include_str!("../../migrations/seed/003_equipment.sql"),
    include_str!("../../migrations/seed/004_fermentables.sql"),
    include_str!("../../migrations/seed/005_water_profiles.sql"),
    include_str!("../../migrations/seed/006_allergen_lots.sql"),
];

/// Applies all seed files. Idempotent.
pub async fn run(pool: &PgPool) -> Result<(), sqlx::Error> {
    for sql in SEED_FILES {
        sqlx::raw_sql(sql).execute(pool).await?;
    }
    Ok(())
}
