//! Data access for the library module.
//!
//! Port of the Go `internal/library/repository.go`. Free async functions over a
//! `&PgPool` (or `&mut PgConnection` inside transactions). All queries are
//! parameterised. Reads return the union of the system tenant and the caller's
//! tenant; writes target only the caller's tenant. `NUMERIC` columns are
//! selected as `float8` so they decode into `f64`.

use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use super::models::{
    EquipmentFilter, EquipmentProfile, Fermentable, FermentableFilter, MashFilter, MashProfile,
    MashProfileRow, MashStep, MashStepRequest, Page, Style, StyleFilter, Yeast, YeastFilter,
    SYSTEM_TENANT_ID,
};
use crate::platform::errors::ApiError;

// ---- pagination / sort helpers ----

/// Clamps page (>=1) and page_size (1..=100, default 20).
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

/// Validates a comma-separated sort string against an allow-list and builds an
/// `ORDER BY` fragment. An unknown field yields a validation error (matching the
/// Go `ErrInvalidSort` → 400 mapping). The returned fragment is built only from
/// the trusted allow-list values, never from user input.
fn parse_sort(sort: &str, allowed: &[(&str, &str)], default_col: &str) -> Result<String, ApiError> {
    let sort = sort.trim();
    if sort.is_empty() {
        return Ok(format!("{default_col} ASC"));
    }
    let mut parts = Vec::new();
    for field in sort.split(',') {
        let field = field.trim();
        let (name, dir) = match field.strip_prefix('-') {
            Some(rest) => (rest, "DESC"),
            None => (field, "ASC"),
        };
        let col = allowed
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, c)| *c)
            .ok_or_else(|| ApiError::validation("sort", &format!("invalid sort field: {name}")))?;
        parts.push(format!("{col} {dir}"));
    }
    Ok(parts.join(", "))
}

const STYLE_ALLOWED_SORT: &[(&str, &str)] = &[
    ("name", "name"),
    ("category", "category"),
    ("og_min", "og_min"),
    ("og_max", "og_max"),
    ("created_at", "created_at"),
];

const EQUIP_ALLOWED_SORT: &[(&str, &str)] = &[
    ("name", "name"),
    ("batch_size_liters", "batch_size_liters"),
    ("created_at", "created_at"),
];

const MASH_ALLOWED_SORT: &[(&str, &str)] = &[("name", "name"), ("created_at", "created_at")];

const YEAST_ALLOWED_SORT: &[(&str, &str)] = &[
    ("name", "name"),
    ("type", "type"),
    ("product_code", "product_code"),
    ("created_at", "created_at"),
];

const FERMENTABLE_ALLOWED_SORT: &[(&str, &str)] = &[
    ("name", "name"),
    ("supplier", "supplier"),
    ("type", "type"),
    ("created_at", "created_at"),
];

// ---- Styles ----

const STYLE_COLS: &str = "id, tenant_id, name, category, \
    og_min::float8 AS og_min, og_max::float8 AS og_max, \
    fg_min::float8 AS fg_min, fg_max::float8 AS fg_max, \
    abv_min::float8 AS abv_min, abv_max::float8 AS abv_max, \
    ibu_min::float8 AS ibu_min, ibu_max::float8 AS ibu_max, \
    color_ebc_min::float8 AS color_ebc_min, color_ebc_max::float8 AS color_ebc_max, \
    description, created_at, updated_at";

/// Inserts a style and returns the persisted row.
#[allow(clippy::too_many_arguments)]
pub async fn insert_style(pool: &PgPool, s: &Style) -> Result<Style, sqlx::Error> {
    let sql = format!(
        "INSERT INTO styles (tenant_id, name, category, og_min, og_max, fg_min, fg_max, \
         abv_min, abv_max, ibu_min, ibu_max, color_ebc_min, color_ebc_max, description, \
         created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, now(), now()) \
         RETURNING {STYLE_COLS}"
    );
    sqlx::query_as::<_, Style>(&sql)
        .bind(s.tenant_id)
        .bind(&s.name)
        .bind(&s.category)
        .bind(s.og_min)
        .bind(s.og_max)
        .bind(s.fg_min)
        .bind(s.fg_max)
        .bind(s.abv_min)
        .bind(s.abv_max)
        .bind(s.ibu_min)
        .bind(s.ibu_max)
        .bind(s.color_ebc_min)
        .bind(s.color_ebc_max)
        .bind(&s.description)
        .fetch_one(pool)
        .await
}

/// Lists styles in the union of the system and caller tenants.
pub async fn select_styles(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &StyleFilter,
) -> Result<Page<Style>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = parse_sort(&filter.sort, STYLE_ALLOWED_SORT, "name")?;

    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM styles WHERE tenant_id IN (");
    count
        .push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    if let Some(category) = &filter.category {
        count.push(" AND category = ").push_bind(category.clone());
    }
    if let Some(name) = &filter.name {
        count
            .push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let offset = (page - 1) * page_size;
    let mut list: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT {STYLE_COLS} FROM styles WHERE tenant_id IN ("
    ));
    list.push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    if let Some(category) = &filter.category {
        list.push(" AND category = ").push_bind(category.clone());
    }
    if let Some(name) = &filter.name {
        list.push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    list.push(format!(" ORDER BY {order_by} LIMIT "))
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let items: Vec<Style> = list.build_query_as().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a style by id from the union of system and caller tenants.
pub async fn select_style_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Style>, sqlx::Error> {
    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let sql = format!("SELECT {STYLE_COLS} FROM styles WHERE id = $1 AND tenant_id IN ($2, $3)");
    sqlx::query_as::<_, Style>(&sql)
        .bind(id)
        .bind(SYSTEM_TENANT_ID)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Fetches a caller-owned style by id (for update/locking; never matches system rows).
pub async fn select_owned_style(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Style>, sqlx::Error> {
    let sql = format!("SELECT {STYLE_COLS} FROM styles WHERE id = $1 AND tenant_id = $2");
    sqlx::query_as::<_, Style>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Persists a full style update and returns the fresh row.
pub async fn update_style(pool: &PgPool, s: &Style) -> Result<Option<Style>, sqlx::Error> {
    let sql = format!(
        "UPDATE styles SET name=$1, category=$2, og_min=$3, og_max=$4, fg_min=$5, fg_max=$6, \
         abv_min=$7, abv_max=$8, ibu_min=$9, ibu_max=$10, color_ebc_min=$11, color_ebc_max=$12, \
         description=$13, updated_at=now() WHERE id=$14 AND tenant_id=$15 RETURNING {STYLE_COLS}"
    );
    sqlx::query_as::<_, Style>(&sql)
        .bind(&s.name)
        .bind(&s.category)
        .bind(s.og_min)
        .bind(s.og_max)
        .bind(s.fg_min)
        .bind(s.fg_max)
        .bind(s.abv_min)
        .bind(s.abv_max)
        .bind(s.ibu_min)
        .bind(s.ibu_max)
        .bind(s.color_ebc_min)
        .bind(s.color_ebc_max)
        .bind(&s.description)
        .bind(s.id)
        .bind(s.tenant_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a caller-owned style. Returns the number of rows affected.
pub async fn delete_style(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM styles WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// ---- Equipment profiles ----

const EQUIP_COLS: &str = "id, tenant_id, name, \
    batch_size_liters::float8 AS batch_size_liters, \
    batch_volume_target_liters::float8 AS batch_volume_target_liters, \
    element_power_watts::float8 AS element_power_watts, \
    boil_size_liters::float8 AS boil_size_liters, \
    pre_boil_volume_liters::float8 AS pre_boil_volume_liters, \
    boil_time_minutes, \
    boil_off_rate_liters_per_hour::float8 AS boil_off_rate_liters_per_hour, \
    boil_temp_c::float8 AS boil_temp_c, \
    trub_loss_liters::float8 AS trub_loss_liters, \
    mash_tun_deadspace_liters::float8 AS mash_tun_deadspace_liters, \
    mash_tun_loss_liters::float8 AS mash_tun_loss_liters, \
    hlt_deadspace_liters::float8 AS hlt_deadspace_liters, \
    fermenter_loss_liters::float8 AS fermenter_loss_liters, \
    top_up_liters::float8 AS top_up_liters, \
    mash_time_minutes, \
    brewhouse_efficiency_pct::float8 AS brewhouse_efficiency_pct, \
    mash_efficiency_pct::float8 AS mash_efficiency_pct, \
    hop_utilisation_pct::float8 AS hop_utilisation_pct, \
    aroma_hop_utilisation_pct::float8 AS aroma_hop_utilisation_pct, \
    hop_stand_temp_c::float8 AS hop_stand_temp_c, \
    altitude_m::float8 AS altitude_m, \
    cooling_shrinkage_pct::float8 AS cooling_shrinkage_pct, \
    grain_absorption_l_per_kg::float8 AS grain_absorption_l_per_kg, \
    water_to_grain_ratio::float8 AS water_to_grain_ratio, \
    sparge_water_reminder_liters::float8 AS sparge_water_reminder_liters, \
    notes, created_at, updated_at";

const EQUIP_INSERT_COLS: &str = "tenant_id, name, batch_size_liters, batch_volume_target_liters, \
    element_power_watts, boil_size_liters, pre_boil_volume_liters, boil_time_minutes, \
    boil_off_rate_liters_per_hour, boil_temp_c, trub_loss_liters, mash_tun_deadspace_liters, \
    mash_tun_loss_liters, hlt_deadspace_liters, fermenter_loss_liters, top_up_liters, \
    mash_time_minutes, brewhouse_efficiency_pct, mash_efficiency_pct, hop_utilisation_pct, \
    aroma_hop_utilisation_pct, hop_stand_temp_c, altitude_m, cooling_shrinkage_pct, \
    grain_absorption_l_per_kg, water_to_grain_ratio, sparge_water_reminder_liters, notes";

/// Binds the 28 updatable equipment fields onto a query in column order.
fn bind_equipment<'q>(
    q: sqlx::query::QueryAs<'q, Postgres, EquipmentProfile, sqlx::postgres::PgArguments>,
    ep: &'q EquipmentProfile,
) -> sqlx::query::QueryAs<'q, Postgres, EquipmentProfile, sqlx::postgres::PgArguments> {
    q.bind(&ep.name)
        .bind(ep.batch_size_liters)
        .bind(ep.batch_volume_target_liters)
        .bind(ep.element_power_watts)
        .bind(ep.boil_size_liters)
        .bind(ep.pre_boil_volume_liters)
        .bind(ep.boil_time_minutes)
        .bind(ep.boil_off_rate_liters_per_hour)
        .bind(ep.boil_temp_c)
        .bind(ep.trub_loss_liters)
        .bind(ep.mash_tun_deadspace_liters)
        .bind(ep.mash_tun_loss_liters)
        .bind(ep.hlt_deadspace_liters)
        .bind(ep.fermenter_loss_liters)
        .bind(ep.top_up_liters)
        .bind(ep.mash_time_minutes)
        .bind(ep.brewhouse_efficiency_pct)
        .bind(ep.mash_efficiency_pct)
        .bind(ep.hop_utilisation_pct)
        .bind(ep.aroma_hop_utilisation_pct)
        .bind(ep.hop_stand_temp_c)
        .bind(ep.altitude_m)
        .bind(ep.cooling_shrinkage_pct)
        .bind(ep.grain_absorption_l_per_kg)
        .bind(ep.water_to_grain_ratio)
        .bind(ep.sparge_water_reminder_liters)
        .bind(&ep.notes)
}

/// Inserts an equipment profile and returns the persisted row.
pub async fn insert_equipment(
    pool: &PgPool,
    ep: &EquipmentProfile,
) -> Result<EquipmentProfile, sqlx::Error> {
    let sql = format!(
        "INSERT INTO equipment_profiles ({EQUIP_INSERT_COLS}, created_at, updated_at) VALUES \
         ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, \
         $20, $21, $22, $23, $24, $25, $26, $27, $28, now(), now()) RETURNING {EQUIP_COLS}"
    );
    let q = sqlx::query_as::<_, EquipmentProfile>(&sql).bind(ep.tenant_id);
    // tenant_id is $1; bind_equipment binds name ($2) .. notes ($28).
    bind_equipment(q, ep).fetch_one(pool).await
}

/// Lists equipment profiles in the union of the system and caller tenants.
pub async fn select_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &EquipmentFilter,
) -> Result<Page<EquipmentProfile>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = parse_sort(&filter.sort, EQUIP_ALLOWED_SORT, "name")?;

    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM equipment_profiles WHERE tenant_id IN (");
    count
        .push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    if let Some(name) = &filter.name {
        count
            .push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let offset = (page - 1) * page_size;
    let mut list: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT {EQUIP_COLS} FROM equipment_profiles WHERE tenant_id IN ("
    ));
    list.push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    if let Some(name) = &filter.name {
        list.push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    list.push(format!(" ORDER BY {order_by} LIMIT "))
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let items: Vec<EquipmentProfile> = list.build_query_as().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Fetches an equipment profile by id from the union of system and caller tenants.
pub async fn select_equipment_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<EquipmentProfile>, sqlx::Error> {
    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let sql = format!(
        "SELECT {EQUIP_COLS} FROM equipment_profiles WHERE id = $1 AND tenant_id IN ($2, $3)"
    );
    sqlx::query_as::<_, EquipmentProfile>(&sql)
        .bind(id)
        .bind(SYSTEM_TENANT_ID)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Fetches a caller-owned equipment profile by id (never matches system rows).
pub async fn select_owned_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<EquipmentProfile>, sqlx::Error> {
    let sql =
        format!("SELECT {EQUIP_COLS} FROM equipment_profiles WHERE id = $1 AND tenant_id = $2");
    sqlx::query_as::<_, EquipmentProfile>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Persists a full equipment-profile update and returns the fresh row.
pub async fn update_equipment(
    pool: &PgPool,
    ep: &EquipmentProfile,
) -> Result<Option<EquipmentProfile>, sqlx::Error> {
    let sql = format!(
        "UPDATE equipment_profiles SET name=$1, batch_size_liters=$2, \
         batch_volume_target_liters=$3, element_power_watts=$4, boil_size_liters=$5, \
         pre_boil_volume_liters=$6, boil_time_minutes=$7, boil_off_rate_liters_per_hour=$8, \
         boil_temp_c=$9, trub_loss_liters=$10, mash_tun_deadspace_liters=$11, \
         mash_tun_loss_liters=$12, hlt_deadspace_liters=$13, fermenter_loss_liters=$14, \
         top_up_liters=$15, mash_time_minutes=$16, brewhouse_efficiency_pct=$17, \
         mash_efficiency_pct=$18, hop_utilisation_pct=$19, aroma_hop_utilisation_pct=$20, \
         hop_stand_temp_c=$21, altitude_m=$22, cooling_shrinkage_pct=$23, \
         grain_absorption_l_per_kg=$24, water_to_grain_ratio=$25, \
         sparge_water_reminder_liters=$26, notes=$27, updated_at=now() \
         WHERE id=$28 AND tenant_id=$29 RETURNING {EQUIP_COLS}"
    );
    // bind name ($1) .. notes ($27), then id ($28), tenant_id ($29).
    let q = sqlx::query_as::<_, EquipmentProfile>(&sql);
    bind_equipment(q, ep)
        .bind(ep.id)
        .bind(ep.tenant_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a caller-owned equipment profile. Returns the number of rows affected.
pub async fn delete_equipment(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM equipment_profiles WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// ---- Mash profiles ----

const MASH_COLS: &str = "id, tenant_id, name, notes, created_at, updated_at";

const MASH_STEP_COLS: &str = "id, mash_profile_id, step_order, step_type, \
    target_temp_c::float8 AS target_temp_c, hold_minutes, \
    infusion_volume_liters::float8 AS infusion_volume_liters";

/// Loads ordered mash steps for a profile.
pub async fn load_mash_steps(
    pool: &PgPool,
    profile_id: Uuid,
) -> Result<Vec<MashStep>, sqlx::Error> {
    let sql = format!(
        "SELECT {MASH_STEP_COLS} FROM mash_steps WHERE mash_profile_id = $1 ORDER BY step_order"
    );
    sqlx::query_as::<_, MashStep>(&sql)
        .bind(profile_id)
        .fetch_all(pool)
        .await
}

/// Inserts the given step set for a profile within a transaction.
async fn insert_mash_steps_tx(
    tx: &mut Transaction<'_, Postgres>,
    profile_id: Uuid,
    steps: &[MashStepRequest],
) -> Result<(), sqlx::Error> {
    for s in steps {
        sqlx::query(
            "INSERT INTO mash_steps (mash_profile_id, step_order, step_type, target_temp_c, \
             hold_minutes, infusion_volume_liters) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(profile_id)
        .bind(s.step_order)
        .bind(&s.step_type)
        .bind(s.target_temp_c)
        .bind(s.hold_minutes)
        .bind(s.infusion_volume_liters)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Inserts a mash profile with its steps, returning the full profile.
pub async fn insert_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
    notes: Option<&str>,
    steps: &[MashStepRequest],
) -> Result<MashProfile, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let sql = format!(
        "INSERT INTO mash_profiles (tenant_id, name, notes, created_at, updated_at) \
         VALUES ($1, $2, $3, now(), now()) RETURNING {MASH_COLS}"
    );
    let row = sqlx::query_as::<_, MashProfileRow>(&sql)
        .bind(tenant_id)
        .bind(name)
        .bind(notes)
        .fetch_one(&mut *tx)
        .await?;
    insert_mash_steps_tx(&mut tx, row.id, steps).await?;
    tx.commit().await?;

    let loaded = load_mash_steps(pool, row.id).await?;
    Ok(row.with_steps(loaded))
}

/// Lists mash profiles (with steps) in the union of the system and caller tenants.
pub async fn select_mash_profiles(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &MashFilter,
) -> Result<Page<MashProfile>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = parse_sort(&filter.sort, MASH_ALLOWED_SORT, "name")?;

    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM mash_profiles WHERE tenant_id IN (");
    count
        .push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    if let Some(name) = &filter.name {
        count
            .push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let offset = (page - 1) * page_size;
    let mut list: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT {MASH_COLS} FROM mash_profiles WHERE tenant_id IN ("
    ));
    list.push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    if let Some(name) = &filter.name {
        list.push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    list.push(format!(" ORDER BY {order_by} LIMIT "))
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let rows: Vec<MashProfileRow> = list.build_query_as().fetch_all(pool).await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let steps = load_mash_steps(pool, row.id).await?;
        items.push(row.with_steps(steps));
    }
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a mash profile (with steps) from the union of system and caller tenants.
pub async fn select_mash_profile_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<MashProfile>, sqlx::Error> {
    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let sql =
        format!("SELECT {MASH_COLS} FROM mash_profiles WHERE id = $1 AND tenant_id IN ($2, $3)");
    let row = sqlx::query_as::<_, MashProfileRow>(&sql)
        .bind(id)
        .bind(SYSTEM_TENANT_ID)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some(row) => {
            let steps = load_mash_steps(pool, row.id).await?;
            Ok(Some(row.with_steps(steps)))
        }
        None => Ok(None),
    }
}

/// Fetches a caller-owned mash profile header by id (never matches system rows).
pub async fn select_owned_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<MashProfileRow>, sqlx::Error> {
    let sql = format!("SELECT {MASH_COLS} FROM mash_profiles WHERE id = $1 AND tenant_id = $2");
    sqlx::query_as::<_, MashProfileRow>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Updates a caller-owned mash profile header and replaces its step set
/// transactionally, returning the full refreshed profile. `None` when the
/// profile is not owned by the caller.
pub async fn update_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    name: &str,
    notes: Option<&str>,
    steps: &[MashStepRequest],
) -> Result<Option<MashProfile>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let sql = format!(
        "UPDATE mash_profiles SET name=$1, notes=$2, updated_at=now() \
         WHERE id=$3 AND tenant_id=$4 RETURNING {MASH_COLS}"
    );
    let row = sqlx::query_as::<_, MashProfileRow>(&sql)
        .bind(name)
        .bind(notes)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?;
    let row = match row {
        Some(row) => row,
        None => {
            tx.rollback().await?;
            return Ok(None);
        }
    };

    // Always replace steps: delete existing, insert new set.
    sqlx::query("DELETE FROM mash_steps WHERE mash_profile_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    insert_mash_steps_tx(&mut tx, id, steps).await?;
    tx.commit().await?;

    let loaded = load_mash_steps(pool, id).await?;
    Ok(Some(row.with_steps(loaded)))
}

/// Deletes a caller-owned mash profile. Returns the number of rows affected.
pub async fn delete_mash_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM mash_profiles WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// ---- Yeasts ----

const YEAST_COLS: &str = "id, tenant_id, name, manufacturer, product_code, type, form, \
    attenuation_min_pct::float8 AS attenuation_min_pct, \
    attenuation_max_pct::float8 AS attenuation_max_pct, \
    temp_min_c::float8 AS temp_min_c, temp_max_c::float8 AS temp_max_c, \
    flocculation, notes, created_at, updated_at";

/// Inserts a yeast and returns the persisted row.
pub async fn insert_yeast(pool: &PgPool, y: &Yeast) -> Result<Yeast, sqlx::Error> {
    let sql = format!(
        "INSERT INTO yeasts (tenant_id, name, manufacturer, product_code, type, form, \
         attenuation_min_pct, attenuation_max_pct, temp_min_c, temp_max_c, flocculation, notes, \
         created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now()) \
         RETURNING {YEAST_COLS}"
    );
    sqlx::query_as::<_, Yeast>(&sql)
        .bind(y.tenant_id)
        .bind(&y.name)
        .bind(&y.manufacturer)
        .bind(&y.product_code)
        .bind(&y.yeast_type)
        .bind(&y.form)
        .bind(y.attenuation_min_pct)
        .bind(y.attenuation_max_pct)
        .bind(y.temp_min_c)
        .bind(y.temp_max_c)
        .bind(&y.flocculation)
        .bind(&y.notes)
        .fetch_one(pool)
        .await
}

/// Lists yeasts in the union of the system and caller tenants.
pub async fn select_yeasts(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &YeastFilter,
) -> Result<Page<Yeast>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = parse_sort(&filter.sort, YEAST_ALLOWED_SORT, "name")?;

    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM yeasts WHERE tenant_id IN (");
    count
        .push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    apply_yeast_filters(&mut count, filter);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let offset = (page - 1) * page_size;
    let mut list: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT {YEAST_COLS} FROM yeasts WHERE tenant_id IN ("
    ));
    list.push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    apply_yeast_filters(&mut list, filter);
    list.push(format!(" ORDER BY {order_by} LIMIT "))
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let items: Vec<Yeast> = list.build_query_as().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Appends the yeast-specific WHERE conditions to a builder.
fn apply_yeast_filters(qb: &mut QueryBuilder<Postgres>, filter: &YeastFilter) {
    if let Some(name) = &filter.name {
        qb.push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    if let Some(manufacturer) = &filter.manufacturer {
        qb.push(" AND lower(manufacturer) LIKE ")
            .push_bind(format!("%{}%", manufacturer.to_lowercase()));
    }
    if let Some(att_min) = filter.attenuation_min {
        qb.push(" AND attenuation_min_pct >= ").push_bind(att_min);
    }
    if let Some(att_max) = filter.attenuation_max {
        qb.push(" AND attenuation_max_pct <= ").push_bind(att_max);
    }
}

/// Fetches a yeast by id from the union of system and caller tenants.
pub async fn select_yeast_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Yeast>, sqlx::Error> {
    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let sql = format!("SELECT {YEAST_COLS} FROM yeasts WHERE id = $1 AND tenant_id IN ($2, $3)");
    sqlx::query_as::<_, Yeast>(&sql)
        .bind(id)
        .bind(SYSTEM_TENANT_ID)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Fetches a caller-owned yeast by id (never matches system rows).
pub async fn select_owned_yeast(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Yeast>, sqlx::Error> {
    let sql = format!("SELECT {YEAST_COLS} FROM yeasts WHERE id = $1 AND tenant_id = $2");
    sqlx::query_as::<_, Yeast>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Persists a full yeast update and returns the fresh row.
pub async fn update_yeast(pool: &PgPool, y: &Yeast) -> Result<Option<Yeast>, sqlx::Error> {
    let sql = format!(
        "UPDATE yeasts SET name=$1, manufacturer=$2, product_code=$3, type=$4, form=$5, \
         attenuation_min_pct=$6, attenuation_max_pct=$7, temp_min_c=$8, temp_max_c=$9, \
         flocculation=$10, notes=$11, updated_at=now() WHERE id=$12 AND tenant_id=$13 \
         RETURNING {YEAST_COLS}"
    );
    sqlx::query_as::<_, Yeast>(&sql)
        .bind(&y.name)
        .bind(&y.manufacturer)
        .bind(&y.product_code)
        .bind(&y.yeast_type)
        .bind(&y.form)
        .bind(y.attenuation_min_pct)
        .bind(y.attenuation_max_pct)
        .bind(y.temp_min_c)
        .bind(y.temp_max_c)
        .bind(&y.flocculation)
        .bind(&y.notes)
        .bind(y.id)
        .bind(y.tenant_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a caller-owned yeast. Returns the number of rows affected.
pub async fn delete_yeast(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM yeasts WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

// ---- Library fermentables ----

const FERMENTABLE_COLS: &str = "id, tenant_id, name, supplier, type, \
    colour_ebc_min::float8 AS colour_ebc_min, colour_ebc_max::float8 AS colour_ebc_max, \
    extract_litres_per_kg::float8 AS extract_litres_per_kg, \
    moisture_pct_max::float8 AS moisture_pct_max, \
    tn_min::float8 AS tn_min, tn_max::float8 AS tn_max, \
    snr_min::float8 AS snr_min, snr_max::float8 AS snr_max, \
    attributes, notes, created_at, updated_at";

/// Inserts a library fermentable and returns the persisted row.
pub async fn insert_fermentable(
    pool: &PgPool,
    f: &Fermentable,
) -> Result<Fermentable, sqlx::Error> {
    let sql = format!(
        "INSERT INTO library_fermentables (tenant_id, name, supplier, type, colour_ebc_min, \
         colour_ebc_max, extract_litres_per_kg, moisture_pct_max, tn_min, tn_max, snr_min, \
         snr_max, attributes, notes, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, now(), now()) \
         RETURNING {FERMENTABLE_COLS}"
    );
    sqlx::query_as::<_, Fermentable>(&sql)
        .bind(f.tenant_id)
        .bind(&f.name)
        .bind(&f.supplier)
        .bind(&f.fermentable_type)
        .bind(f.colour_ebc_min)
        .bind(f.colour_ebc_max)
        .bind(f.extract_litres_per_kg)
        .bind(f.moisture_pct_max)
        .bind(f.tn_min)
        .bind(f.tn_max)
        .bind(f.snr_min)
        .bind(f.snr_max)
        .bind(&f.attributes)
        .bind(&f.notes)
        .fetch_one(pool)
        .await
}

/// Lists library fermentables in the union of the system and caller tenants.
pub async fn select_fermentables(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &FermentableFilter,
) -> Result<Page<Fermentable>, ApiError> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = parse_sort(&filter.sort, FERMENTABLE_ALLOWED_SORT, "name")?;

    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM library_fermentables WHERE tenant_id IN (");
    count
        .push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    apply_fermentable_filters(&mut count, filter);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let offset = (page - 1) * page_size;
    let mut list: QueryBuilder<Postgres> = QueryBuilder::new(format!(
        "SELECT {FERMENTABLE_COLS} FROM library_fermentables WHERE tenant_id IN ("
    ));
    list.push_bind(SYSTEM_TENANT_ID)
        .push(", ")
        .push_bind(tenant_id)
        .push(")");
    apply_fermentable_filters(&mut list, filter);
    list.push(format!(" ORDER BY {order_by} LIMIT "))
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(offset));
    let items: Vec<Fermentable> = list.build_query_as().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Appends the fermentable-specific WHERE conditions to a builder.
fn apply_fermentable_filters(qb: &mut QueryBuilder<Postgres>, filter: &FermentableFilter) {
    if let Some(name) = &filter.name {
        qb.push(" AND lower(name) LIKE ")
            .push_bind(format!("%{}%", name.to_lowercase()));
    }
    if let Some(supplier) = &filter.supplier {
        qb.push(" AND lower(supplier) LIKE ")
            .push_bind(format!("%{}%", supplier.to_lowercase()));
    }
    if let Some(ftype) = &filter.fermentable_type {
        qb.push(" AND lower(type) = ")
            .push_bind(ftype.to_lowercase());
    }
}

/// Fetches a fermentable by id from the union of system and caller tenants.
pub async fn select_fermentable_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Fermentable>, sqlx::Error> {
    // CROSS-TENANT QUERY: reads include the shared system-tenant library rows.
    let sql = format!(
        "SELECT {FERMENTABLE_COLS} FROM library_fermentables WHERE id = $1 AND tenant_id IN ($2, $3)"
    );
    sqlx::query_as::<_, Fermentable>(&sql)
        .bind(id)
        .bind(SYSTEM_TENANT_ID)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Fetches a caller-owned fermentable by id (never matches system rows).
pub async fn select_owned_fermentable(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Fermentable>, sqlx::Error> {
    let sql = format!(
        "SELECT {FERMENTABLE_COLS} FROM library_fermentables WHERE id = $1 AND tenant_id = $2"
    );
    sqlx::query_as::<_, Fermentable>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Persists a full fermentable update and returns the fresh row.
pub async fn update_fermentable(
    pool: &PgPool,
    f: &Fermentable,
) -> Result<Option<Fermentable>, sqlx::Error> {
    let sql = format!(
        "UPDATE library_fermentables SET name=$1, supplier=$2, type=$3, colour_ebc_min=$4, \
         colour_ebc_max=$5, extract_litres_per_kg=$6, moisture_pct_max=$7, tn_min=$8, tn_max=$9, \
         snr_min=$10, snr_max=$11, attributes=$12, notes=$13, updated_at=now() \
         WHERE id=$14 AND tenant_id=$15 RETURNING {FERMENTABLE_COLS}"
    );
    sqlx::query_as::<_, Fermentable>(&sql)
        .bind(&f.name)
        .bind(&f.supplier)
        .bind(&f.fermentable_type)
        .bind(f.colour_ebc_min)
        .bind(f.colour_ebc_max)
        .bind(f.extract_litres_per_kg)
        .bind(f.moisture_pct_max)
        .bind(f.tn_min)
        .bind(f.tn_max)
        .bind(f.snr_min)
        .bind(f.snr_max)
        .bind(&f.attributes)
        .bind(&f.notes)
        .bind(f.id)
        .bind(f.tenant_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a caller-owned fermentable. Returns the number of rows affected.
pub async fn delete_fermentable(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM library_fermentables WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
