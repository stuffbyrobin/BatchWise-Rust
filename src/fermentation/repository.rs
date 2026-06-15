//! Data access for fermentation readings.
//!
//! Port of the Go `internal/fermentation/repository.go`. `gravity`, `temp_c` and
//! `ph` are `NUMERIC` columns selected as `float8`. Every query is scoped by
//! `tenant_id` and `batch_id`.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{Page, Reading, ReadingFilter};

const COLS: &str = "id, tenant_id, batch_id, recorded_at, stage, gravity::float8 AS gravity, \
    temp_c::float8 AS temp_c, ph::float8 AS ph, notes, created_at";

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

fn order_by(sort: &str) -> &'static str {
    match sort {
        "recorded_at" => "recorded_at ASC",
        _ => "recorded_at DESC",
    }
}

/// True if the batch exists for this tenant.
pub async fn batch_exists(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM batches WHERE id = $1 AND tenant_id = $2)",
    )
    .bind(batch_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await
}

/// Inserts a fermentation reading and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_reading(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
    recorded_at: DateTime<Utc>,
    stage: &str,
    gravity: Option<f64>,
    temp_c: Option<f64>,
    ph: Option<f64>,
    notes: Option<&str>,
) -> Result<Reading, sqlx::Error> {
    let sql = format!(
        "INSERT INTO fermentation_readings \
         (tenant_id, batch_id, recorded_at, stage, gravity, temp_c, ph, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING {COLS}"
    );
    sqlx::query_as::<_, Reading>(&sql)
        .bind(tenant_id)
        .bind(batch_id)
        .bind(recorded_at)
        .bind(stage)
        .bind(gravity)
        .bind(temp_c)
        .bind(ph)
        .bind(notes)
        .fetch_one(pool)
        .await
}

/// Lists readings for a batch, with an optional stage filter.
pub async fn select_readings(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
    filter: &ReadingFilter,
) -> Result<Page<Reading>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        qb.push(" AND batch_id = ").push_bind(batch_id);
        if let Some(s) = &filter.stage {
            qb.push(" AND stage = ").push_bind(s.clone());
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM fermentation_readings");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {COLS} FROM fermentation_readings"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {}", order_by(&filter.sort)));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<Reading>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a reading by (tenant, batch, id).
pub async fn select_reading_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
    id: Uuid,
) -> Result<Option<Reading>, sqlx::Error> {
    let sql = format!(
        "SELECT {COLS} FROM fermentation_readings \
         WHERE id = $1 AND batch_id = $2 AND tenant_id = $3"
    );
    sqlx::query_as::<_, Reading>(&sql)
        .bind(id)
        .bind(batch_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Updates a reading's mutable fields; returns true if a row was changed.
#[allow(clippy::too_many_arguments)]
pub async fn update_reading(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
    id: Uuid,
    recorded_at: DateTime<Utc>,
    stage: &str,
    gravity: Option<f64>,
    temp_c: Option<f64>,
    ph: Option<f64>,
    notes: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE fermentation_readings \
         SET recorded_at=$1, stage=$2, gravity=$3, temp_c=$4, ph=$5, notes=$6 \
         WHERE id=$7 AND batch_id=$8 AND tenant_id=$9",
    )
    .bind(recorded_at)
    .bind(stage)
    .bind(gravity)
    .bind(temp_c)
    .bind(ph)
    .bind(notes)
    .bind(id)
    .bind(batch_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a reading; returns true if a row was removed.
pub async fn delete_reading(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "DELETE FROM fermentation_readings WHERE id=$1 AND batch_id=$2 AND tenant_id=$3",
    )
    .bind(id)
    .bind(batch_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}
