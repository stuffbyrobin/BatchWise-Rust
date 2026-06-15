//! Data access for yeast kinetics.
//!
//! Port of the Go `internal/yeastkinetics/repository.go`. Free async functions
//! over a `&PgPool` (or `&mut PgConnection` inside transactions). All queries are
//! parameterised and tenant-scoped. `NUMERIC` columns are selected as `float8` so
//! they decode into `f64`.

use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Kinetics, ListFilter, Page, SYSTEM_TENANT_ID};

const COLS: &str = "id, tenant_id, yeast_id, \
    fermentation_temp_c::float8 AS fermentation_temp_c, \
    primary_fermentation_days, conditioning_days, lag_phase_hours, \
    attenuation_pct::float8 AS attenuation_pct, notes, created_at, updated_at";

/// Clamps page (>=1) and page_size (1..=100, default 20).
fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
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

/// Scalar columns for inserting/updating a kinetics row.
#[derive(Debug, Clone)]
pub struct KineticsWrite {
    pub yeast_id: Uuid,
    pub fermentation_temp_c: f64,
    pub primary_fermentation_days: i32,
    pub conditioning_days: i32,
    pub lag_phase_hours: Option<i32>,
    pub attenuation_pct: Option<f64>,
    pub notes: Option<String>,
}

/// Inserts a kinetics row and returns the created row.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    w: &KineticsWrite,
) -> Result<Kinetics, sqlx::Error> {
    let sql = format!(
        "INSERT INTO yeast_kinetics (tenant_id, yeast_id, fermentation_temp_c, \
         primary_fermentation_days, conditioning_days, lag_phase_hours, attenuation_pct, notes, \
         created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), now()) RETURNING {COLS}"
    );
    sqlx::query_as::<_, Kinetics>(&sql)
        .bind(tenant_id)
        .bind(w.yeast_id)
        .bind(w.fermentation_temp_c)
        .bind(w.primary_fermentation_days)
        .bind(w.conditioning_days)
        .bind(w.lag_phase_hours)
        .bind(w.attenuation_pct)
        .bind(&w.notes)
        .fetch_one(exec)
        .await
}

/// Replaces scalar columns; returns the new row or `None` if not found.
pub async fn update<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    w: &KineticsWrite,
) -> Result<Option<Kinetics>, sqlx::Error> {
    let sql = format!(
        "UPDATE yeast_kinetics SET yeast_id=$1, fermentation_temp_c=$2, \
         primary_fermentation_days=$3, conditioning_days=$4, lag_phase_hours=$5, \
         attenuation_pct=$6, notes=$7, updated_at=now() WHERE id=$8 AND tenant_id=$9 \
         RETURNING {COLS}"
    );
    sqlx::query_as::<_, Kinetics>(&sql)
        .bind(w.yeast_id)
        .bind(w.fermentation_temp_c)
        .bind(w.primary_fermentation_days)
        .bind(w.conditioning_days)
        .bind(w.lag_phase_hours)
        .bind(w.attenuation_pct)
        .bind(&w.notes)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(exec)
        .await
}

/// Fetches a kinetics row by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Kinetics>, sqlx::Error> {
    let sql = format!("SELECT {COLS} FROM yeast_kinetics WHERE id = $1 AND tenant_id = $2");
    sqlx::query_as::<_, Kinetics>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

const ALLOWED_SORT: &[(&str, &str)] = &[
    ("created_at", "created_at"),
    ("fermentation_temp_c", "fermentation_temp_c"),
];

/// Builds a safe `ORDER BY` from the sort spec (default `created_at ASC`).
/// Unknown fields fall back to `created_at`, matching the Go allow-list.
fn build_order_by(sort: &str) -> String {
    let spec = sort.trim();
    if spec.is_empty() {
        return "created_at ASC".to_string();
    }
    let (name, dir) = match spec.strip_prefix('-') {
        Some(rest) => (rest, "DESC"),
        None => (spec, "ASC"),
    };
    let col = ALLOWED_SORT
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, c)| *c)
        .unwrap_or("created_at");
    format!("{col} {dir}")
}

/// Lists kinetics rows with filters, tenant-scoped.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
) -> Result<Page<Kinetics>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let order_by = build_order_by(&filter.sort);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(y) = filter.yeast_id {
            qb.push(" AND yeast_id = ").push_bind(y);
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM yeast_kinetics");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {COLS} FROM yeast_kinetics"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Kinetics>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Deletes a kinetics row; returns true if a row was removed.
pub async fn delete_by_id<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM yeast_kinetics WHERE id = $1 AND tenant_id = $2")
        .bind(id)
        .bind(tenant_id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// True if the yeast exists in either the caller's tenant or the system tenant.
pub async fn yeast_exists(
    pool: &PgPool,
    tenant_id: Uuid,
    yeast_id: Uuid,
) -> Result<bool, sqlx::Error> {
    // CROSS-TENANT QUERY: a yeast may be a shared system-tenant library row.
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM yeasts WHERE id = $1 AND tenant_id IN ($2, $3))",
    )
    .bind(yeast_id)
    .bind(tenant_id)
    .bind(SYSTEM_TENANT_ID)
    .fetch_one(pool)
    .await
}

/// Returns the kinetics row for the yeast whose `fermentation_temp_c` is closest
/// to `preferred_temp_c`, searching only within `tenant_id`. `None` if no rows.
pub async fn find_closest_for_yeast(
    pool: &PgPool,
    tenant_id: Uuid,
    yeast_id: Uuid,
    preferred_temp_c: f64,
) -> Result<Option<Kinetics>, sqlx::Error> {
    let sql = format!(
        "SELECT {COLS} FROM yeast_kinetics WHERE tenant_id = $1 AND yeast_id = $2 \
         ORDER BY ABS(fermentation_temp_c - $3) LIMIT 1"
    );
    sqlx::query_as::<_, Kinetics>(&sql)
        .bind(tenant_id)
        .bind(yeast_id)
        .bind(preferred_temp_c)
        .fetch_optional(pool)
        .await
}
