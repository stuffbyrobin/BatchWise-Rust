//! Data access for the water module.
//!
//! Port of the Go `internal/water/repository.go`. Free async functions over a
//! `&PgPool`. All queries are parameterised. Water profiles are read as the
//! union of the system tenant and the caller's tenant (matching the Go
//! `tenant_id = $2 OR tenant_id = $3` reads); writes target only the caller's
//! tenant. Water adjustments are strictly tenant-scoped. `NUMERIC` columns are
//! selected as `float8` so they decode into `f64`; nullable result columns
//! decode into `Option<f64>`; the additions arrays are `JSONB`.

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    Adjustment, AdjustmentFilter, AdjustmentRow, CreateWaterAdjustmentRequest,
    CreateWaterProfileRequest, Page, Profile, ProfileFilter, Result as WaterResult,
    UpdateWaterAdjustmentRequest, UpdateWaterProfileRequest, SYSTEM_TENANT_ID,
};
use crate::platform::errors::ApiError;

// ---- pagination helper ----

/// Clamps page (>=1) and page_size (1..=100, default 20). Mirrors the Go
/// service-layer clamping.
fn clamp_page(page: i32, page_size: i32) -> (i32, i32) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 {
        20
    } else if page_size > 100 {
        100
    } else {
        page_size
    };
    (page, page_size)
}

/// Maps a `sqlx` error to an [`ApiError`], surfacing unique-violations as 409
/// with the supplied message.
fn map_insert_err(e: sqlx::Error, conflict_field: &str, conflict_msg: &str) -> ApiError {
    if e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
    {
        ApiError::conflict(conflict_field, conflict_msg)
    } else {
        e.into()
    }
}

// ---- Water profiles ----

/// Column projection for `water_profiles`, with `NUMERIC`s cast to `float8` and
/// the computed `is_system` flag (true when the row belongs to the system tenant).
const PROFILE_COLS: &str = "id, tenant_id, name, description, \
    calcium_ppm::float8 AS calcium_ppm, magnesium_ppm::float8 AS magnesium_ppm, \
    sodium_ppm::float8 AS sodium_ppm, sulfate_ppm::float8 AS sulfate_ppm, \
    chloride_ppm::float8 AS chloride_ppm, bicarbonate_ppm::float8 AS bicarbonate_ppm, \
    notes, (tenant_id = '00000000-0000-0000-0000-000000000000') AS is_system, \
    created_at, updated_at";

/// Resolves the profile sort string to a trusted `ORDER BY` fragment, matching
/// the Go `resolveProfileSort` (default `name ASC`).
fn resolve_profile_sort(sort: &str) -> &'static str {
    match sort {
        "-name" => "name DESC",
        "created_at" => "created_at ASC",
        "-created_at" => "created_at DESC",
        _ => "name ASC",
    }
}

/// Inserts a water profile owned by the caller and returns the persisted row.
pub async fn insert_water_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    req: &CreateWaterProfileRequest,
) -> std::result::Result<Profile, ApiError> {
    let sql = format!(
        "INSERT INTO water_profiles (tenant_id, name, description, calcium_ppm, magnesium_ppm, \
         sodium_ppm, sulfate_ppm, chloride_ppm, bicarbonate_ppm, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING {PROFILE_COLS}"
    );
    sqlx::query_as::<_, Profile>(&sql)
        .bind(tenant_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.calcium_ppm)
        .bind(req.magnesium_ppm)
        .bind(req.sodium_ppm)
        .bind(req.sulfate_ppm)
        .bind(req.chloride_ppm)
        .bind(req.bicarbonate_ppm)
        .bind(&req.notes)
        .fetch_one(pool)
        .await
        .map_err(|e| map_insert_err(e, "name", "a water profile with this name already exists"))
}

/// Fetches a water profile by id from the union of system and caller tenants.
pub async fn select_water_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> std::result::Result<Option<Profile>, sqlx::Error> {
    // CROSS-TENANT QUERY: reads include the shared system-tenant water profiles.
    let sql = format!(
        "SELECT {PROFILE_COLS} FROM water_profiles \
         WHERE id = $1 AND (tenant_id = $2 OR tenant_id = $3)"
    );
    sqlx::query_as::<_, Profile>(&sql)
        .bind(id)
        .bind(tenant_id)
        .bind(SYSTEM_TENANT_ID)
        .fetch_optional(pool)
        .await
}

/// Lists water profiles in the union of the system and caller tenants.
pub async fn select_water_profiles(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ProfileFilter,
) -> std::result::Result<Page<Profile>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = resolve_profile_sort(&filter.sort);

    // CROSS-TENANT QUERY: reads include the shared system-tenant water profiles.
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM water_profiles WHERE tenant_id = $1 OR tenant_id = $2",
    )
    .bind(tenant_id)
    .bind(SYSTEM_TENANT_ID)
    .fetch_one(pool)
    .await?;

    let offset = (page - 1) * page_size;
    // CROSS-TENANT QUERY: reads include the shared system-tenant water profiles.
    let sql = format!(
        "SELECT {PROFILE_COLS} FROM water_profiles WHERE tenant_id = $1 OR tenant_id = $2 \
         ORDER BY {order_by} LIMIT $3 OFFSET $4"
    );
    let items: Vec<Profile> = sqlx::query_as::<_, Profile>(&sql)
        .bind(tenant_id)
        .bind(SYSTEM_TENANT_ID)
        .bind(i64::from(page_size))
        .bind(i64::from(offset))
        .fetch_all(pool)
        .await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Persists a full caller-owned profile update and returns the fresh row.
/// `None` when the profile is not owned by the caller.
pub async fn update_water_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: &UpdateWaterProfileRequest,
) -> std::result::Result<Option<Profile>, sqlx::Error> {
    let sql = format!(
        "UPDATE water_profiles SET name=$1, description=$2, calcium_ppm=$3, magnesium_ppm=$4, \
         sodium_ppm=$5, sulfate_ppm=$6, chloride_ppm=$7, bicarbonate_ppm=$8, notes=$9, \
         updated_at=now() WHERE id=$10 AND tenant_id=$11 RETURNING {PROFILE_COLS}"
    );
    sqlx::query_as::<_, Profile>(&sql)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.calcium_ppm)
        .bind(req.magnesium_ppm)
        .bind(req.sodium_ppm)
        .bind(req.sulfate_ppm)
        .bind(req.chloride_ppm)
        .bind(req.bicarbonate_ppm)
        .bind(&req.notes)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a caller-owned water profile. Returns the number of rows affected.
pub async fn delete_water_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> std::result::Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM water_profiles WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// ---- Water adjustments ----

/// Column projection for `water_adjustments`, with `NUMERIC`s cast to `float8`
/// (nullable result columns decode into `Option<f64>`).
const ADJ_COLS: &str = "id, tenant_id, name, source_profile_id, target_profile_id, \
    batch_id, recipe_id, volume_liters::float8 AS volume_liters, \
    mineral_additions, acid_additions, grain_additions, \
    result_calcium_ppm::float8 AS result_calcium_ppm, \
    result_magnesium_ppm::float8 AS result_magnesium_ppm, \
    result_sodium_ppm::float8 AS result_sodium_ppm, \
    result_sulfate_ppm::float8 AS result_sulfate_ppm, \
    result_chloride_ppm::float8 AS result_chloride_ppm, \
    result_bicarbonate_ppm::float8 AS result_bicarbonate_ppm, \
    result_alkalinity::float8 AS result_alkalinity, \
    result_residual_alk::float8 AS result_residual_alk, \
    result_sulfate_to_chloride::float8 AS result_sulfate_to_chloride, \
    result_mash_ph::float8 AS result_mash_ph, \
    notes, created_at, updated_at";

/// Resolves the adjustment sort string to a trusted `ORDER BY` fragment,
/// matching the Go `resolveAdjSort` (default `created_at DESC`).
fn resolve_adj_sort(sort: &str) -> &'static str {
    match sort {
        "name" => "name ASC",
        "-name" => "name DESC",
        "created_at" => "created_at ASC",
        _ => "created_at DESC",
    }
}

/// The ten nullable result-column binds, in column order. All `None` when no
/// cached result is supplied (mirrors the Go `resultFields`).
fn result_binds(result: Option<&WaterResult>) -> [Option<f64>; 10] {
    match result {
        Some(r) => [
            Some(r.calcium_ppm),
            Some(r.magnesium_ppm),
            Some(r.sodium_ppm),
            Some(r.sulfate_ppm),
            Some(r.chloride_ppm),
            Some(r.bicarbonate_ppm),
            Some(r.alkalinity),
            Some(r.residual_alk),
            Some(r.sulfate_to_chloride),
            Some(r.mash_ph),
        ],
        None => [None; 10],
    }
}

/// Inserts a water adjustment owned by the caller and returns the persisted row.
pub async fn insert_water_adjustment(
    pool: &PgPool,
    tenant_id: Uuid,
    req: &CreateWaterAdjustmentRequest,
    result: Option<&WaterResult>,
) -> std::result::Result<Adjustment, ApiError> {
    let r = result_binds(result);
    let sql = format!(
        "INSERT INTO water_adjustments \
         (tenant_id, name, source_profile_id, target_profile_id, batch_id, recipe_id, \
          volume_liters, mineral_additions, acid_additions, grain_additions, \
          result_calcium_ppm, result_magnesium_ppm, result_sodium_ppm, result_sulfate_ppm, \
          result_chloride_ppm, result_bicarbonate_ppm, result_alkalinity, result_residual_alk, \
          result_sulfate_to_chloride, result_mash_ph, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10::jsonb, $11, $12, $13, \
          $14, $15, $16, $17, $18, $19, $20, $21) RETURNING {ADJ_COLS}"
    );
    let row = sqlx::query_as::<_, AdjustmentRow>(&sql)
        .bind(tenant_id)
        .bind(&req.name)
        .bind(req.source_profile_id)
        .bind(req.target_profile_id)
        .bind(req.batch_id)
        .bind(req.recipe_id)
        .bind(req.volume_liters)
        .bind(sqlx::types::Json(&req.mineral_additions))
        .bind(sqlx::types::Json(&req.acid_additions))
        .bind(sqlx::types::Json(&req.grain_additions))
        .bind(r[0])
        .bind(r[1])
        .bind(r[2])
        .bind(r[3])
        .bind(r[4])
        .bind(r[5])
        .bind(r[6])
        .bind(r[7])
        .bind(r[8])
        .bind(r[9])
        .bind(&req.notes)
        .fetch_one(pool)
        .await?;
    Ok(row.into_adjustment())
}

/// Fetches a caller-owned water adjustment by id.
pub async fn select_water_adjustment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> std::result::Result<Option<Adjustment>, sqlx::Error> {
    let sql = format!("SELECT {ADJ_COLS} FROM water_adjustments WHERE id = $1 AND tenant_id = $2");
    let row = sqlx::query_as::<_, AdjustmentRow>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(AdjustmentRow::into_adjustment))
}

/// Lists caller-owned water adjustments, optionally filtered by batch/recipe.
pub async fn select_water_adjustments(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &AdjustmentFilter,
) -> std::result::Result<Page<Adjustment>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = resolve_adj_sort(&filter.sort);

    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM water_adjustments WHERE tenant_id = ");
    count.push_bind(tenant_id);
    if let Some(batch_id) = filter.batch_id {
        count.push(" AND batch_id = ").push_bind(batch_id);
    }
    if let Some(recipe_id) = filter.recipe_id {
        count.push(" AND recipe_id = ").push_bind(recipe_id);
    }
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let offset = (page - 1) * page_size;
    let mut list: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT {ADJ_COLS} FROM water_adjustments WHERE tenant_id = "
    ));
    list.push_bind(tenant_id);
    if let Some(batch_id) = filter.batch_id {
        list.push(" AND batch_id = ").push_bind(batch_id);
    }
    if let Some(recipe_id) = filter.recipe_id {
        list.push(" AND recipe_id = ").push_bind(recipe_id);
    }
    list.push(format!(" ORDER BY {order_by} LIMIT "))
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let rows: Vec<AdjustmentRow> = list.build_query_as().fetch_all(pool).await?;

    let items = rows
        .into_iter()
        .map(AdjustmentRow::into_adjustment)
        .collect();
    Ok(Page::new(items, total, page, page_size))
}

/// Persists a full caller-owned adjustment update and returns the fresh row.
/// `None` when the adjustment is not owned by the caller.
pub async fn update_water_adjustment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    req: &UpdateWaterAdjustmentRequest,
    result: Option<&WaterResult>,
) -> std::result::Result<Option<Adjustment>, sqlx::Error> {
    let r = result_binds(result);
    let sql = format!(
        "UPDATE water_adjustments SET name=$1, source_profile_id=$2, target_profile_id=$3, \
         batch_id=$4, recipe_id=$5, volume_liters=$6, mineral_additions=$7::jsonb, \
         acid_additions=$8::jsonb, grain_additions=$9::jsonb, result_calcium_ppm=$10, \
         result_magnesium_ppm=$11, result_sodium_ppm=$12, result_sulfate_ppm=$13, \
         result_chloride_ppm=$14, result_bicarbonate_ppm=$15, result_alkalinity=$16, \
         result_residual_alk=$17, result_sulfate_to_chloride=$18, result_mash_ph=$19, \
         notes=$20, updated_at=now() WHERE id=$21 AND tenant_id=$22 RETURNING {ADJ_COLS}"
    );
    let row = sqlx::query_as::<_, AdjustmentRow>(&sql)
        .bind(&req.name)
        .bind(req.source_profile_id)
        .bind(req.target_profile_id)
        .bind(req.batch_id)
        .bind(req.recipe_id)
        .bind(req.volume_liters)
        .bind(sqlx::types::Json(&req.mineral_additions))
        .bind(sqlx::types::Json(&req.acid_additions))
        .bind(sqlx::types::Json(&req.grain_additions))
        .bind(r[0])
        .bind(r[1])
        .bind(r[2])
        .bind(r[3])
        .bind(r[4])
        .bind(r[5])
        .bind(r[6])
        .bind(r[7])
        .bind(r[8])
        .bind(r[9])
        .bind(&req.notes)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(AdjustmentRow::into_adjustment))
}

/// Deletes a caller-owned water adjustment. Returns the number of rows affected.
pub async fn delete_water_adjustment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> std::result::Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM water_adjustments WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
