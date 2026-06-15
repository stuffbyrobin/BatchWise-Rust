//! Idempotent reference-data seeding.
//!
//! Equivalent of the Go project's `make seed`: applies the SQL files under
//! `migrations/seed/` (default BJCP styles, yeasts, equipment, fermentables,
//! water profiles, allergen lots) into the system tenant. Every statement uses
//! `ON CONFLICT DO NOTHING` on deterministic UUIDs, so running it repeatedly is
//! safe. Seed data is never applied automatically — only via the `--seed` CLI
//! flag or the test harness.

use sqlx::PgPool;

/// Seed SQL embedded at compile time, applied in lexical order.
///
/// Scoped to the Phase 2 reference data (styles, yeasts, equipment,
/// fermentables). Water profiles (`005`) and allergen lots (`006`) are seeded
/// by their respective later phases.
const SEED_FILES: &[&str] = &[
    include_str!("../../migrations/seed/001_styles.sql"),
    include_str!("../../migrations/seed/002_yeasts.sql"),
    include_str!("../../migrations/seed/003_equipment.sql"),
    include_str!("../../migrations/seed/004_fermentables.sql"),
];

/// Applies all seed files. Idempotent.
pub async fn run(pool: &PgPool) -> Result<(), sqlx::Error> {
    for sql in SEED_FILES {
        sqlx::raw_sql(sql).execute(pool).await?;
    }
    Ok(())
}
