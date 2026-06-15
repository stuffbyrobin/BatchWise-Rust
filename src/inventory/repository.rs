//! Data access for inventory (ingredient lots + stock movements).
//!
//! Port of the Go `internal/inventory/repository.go`. `NUMERIC` columns are
//! selected as `float8`; `best_before_date` is rendered with `to_char`.
//! Functions are executor-generic so the FIFO deduction can run inside the
//! caller's transaction with `FOR UPDATE` row locks.

use chrono::NaiveDate;
use sqlx::{PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    Ingredient, ListFilter, MovementFilter, Page, StockMovement, SummaryFilter, SummaryRow,
};

/// Selected columns for `ingredients`, casting NUMERIC → float8 and the date to text.
const ING_COLS: &str = "id, tenant_id, type, name, amount::float8 AS amount, unit, lot_number, \
    to_char(best_before_date, 'YYYY-MM-DD') AS best_before_date, cost_pence, cost_currency, \
    supplier, origin, color_ebc::float8 AS color_ebc, alpha_acid_pct::float8 AS alpha_acid_pct, \
    attenuation_pct::float8 AS attenuation_pct, allergens, notes, created_at, updated_at";

const MV_COLS: &str = "id, tenant_id, ingredient_id, amount_delta::float8 AS amount_delta, \
    balance_after::float8 AS balance_after, reference_type, reference_id, notes, \
    created_by_user_id, created_at";

/// Column values for inserting/replacing a lot.
#[derive(Debug, Clone)]
pub struct IngredientWrite {
    pub r#type: String,
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub lot_number: String,
    pub best_before_date: Option<NaiveDate>,
    pub cost_pence: i64,
    pub cost_currency: String,
    pub supplier: Option<String>,
    pub origin: Option<String>,
    pub color_ebc: Option<f64>,
    pub alpha_acid_pct: Option<f64>,
    pub attenuation_pct: Option<f64>,
    pub allergens: Vec<String>,
    pub notes: Option<String>,
}

/// A stock-movement row to insert.
#[derive(Debug, Clone)]
pub struct MovementWrite {
    pub tenant_id: Uuid,
    pub ingredient_id: Uuid,
    pub amount_delta: f64,
    pub balance_after: f64,
    pub reference_type: String,
    pub reference_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_by_user_id: Option<Uuid>,
}

/// Clamps pagination to page ≥ 1 and 1 ≤ page_size ≤ 100 (default 20).
pub fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
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

/// Inserts a lot and returns the created row.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    w: &IngredientWrite,
) -> Result<Ingredient, sqlx::Error> {
    let sql = format!(
        "INSERT INTO ingredients (tenant_id, type, name, amount, unit, lot_number, \
         best_before_date, cost_pence, cost_currency, supplier, origin, color_ebc, \
         alpha_acid_pct, attenuation_pct, allergens, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING {ING_COLS}"
    );
    bind_write(sqlx::query_as::<_, Ingredient>(&sql).bind(tenant_id), w)
        .fetch_one(exec)
        .await
}

/// Replaces all mutable columns of a lot; returns the new row, or `None` if the
/// lot does not exist for this tenant.
pub async fn update_full<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    w: &IngredientWrite,
) -> Result<Option<Ingredient>, sqlx::Error> {
    let sql = format!(
        "UPDATE ingredients SET type=$3, name=$4, amount=$5, unit=$6, lot_number=$7, \
         best_before_date=$8, cost_pence=$9, cost_currency=$10, supplier=$11, origin=$12, \
         color_ebc=$13, alpha_acid_pct=$14, attenuation_pct=$15, allergens=$16, notes=$17, \
         updated_at=now() WHERE id=$1 AND tenant_id=$2 RETURNING {ING_COLS}"
    );
    let q = sqlx::query_as::<_, Ingredient>(&sql)
        .bind(id)
        .bind(tenant_id);
    bind_write(q, w).fetch_optional(exec).await
}

/// Binds the 15 `IngredientWrite` columns in order onto a query.
fn bind_write<'q>(
    q: sqlx::query::QueryAs<'q, Postgres, Ingredient, sqlx::postgres::PgArguments>,
    w: &'q IngredientWrite,
) -> sqlx::query::QueryAs<'q, Postgres, Ingredient, sqlx::postgres::PgArguments> {
    q.bind(&w.r#type)
        .bind(&w.name)
        .bind(w.amount)
        .bind(&w.unit)
        .bind(&w.lot_number)
        .bind(w.best_before_date)
        .bind(w.cost_pence)
        .bind(&w.cost_currency)
        .bind(&w.supplier)
        .bind(&w.origin)
        .bind(w.color_ebc)
        .bind(w.alpha_acid_pct)
        .bind(w.attenuation_pct)
        .bind(&w.allergens)
        .bind(&w.notes)
}

/// Fetches a lot by id, tenant-scoped.
pub async fn select_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Ingredient>, sqlx::Error> {
    let sql = format!("SELECT {ING_COLS} FROM ingredients WHERE id=$1 AND tenant_id=$2");
    sqlx::query_as::<_, Ingredient>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
}

/// Fetches a lot `FOR UPDATE` (within a transaction), tenant-scoped.
pub async fn select_by_id_for_update<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Ingredient>, sqlx::Error> {
    let sql = format!("SELECT {ING_COLS} FROM ingredients WHERE id=$1 AND tenant_id=$2 FOR UPDATE");
    sqlx::query_as::<_, Ingredient>(&sql)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(exec)
        .await
}

/// Deletes a lot; returns true if a row was removed.
pub async fn delete_by_id<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM ingredients WHERE id=$1 AND tenant_id=$2")
        .bind(id)
        .bind(tenant_id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Selects candidate lots for FIFO deduction, locked `FOR UPDATE`, ordered
/// best-before-date (NULLS LAST), then created_at, then lot_number.
pub async fn select_for_deduct<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    r#type: &str,
    name: &str,
    unit: &str,
) -> Result<Vec<Ingredient>, sqlx::Error> {
    let sql = format!(
        "SELECT {ING_COLS} FROM ingredients \
         WHERE tenant_id=$1 AND type=$2 AND lower(name)=lower($3) AND unit=$4 AND amount > 0 \
         ORDER BY best_before_date ASC NULLS LAST, created_at ASC, lot_number ASC FOR UPDATE"
    );
    sqlx::query_as::<_, Ingredient>(&sql)
        .bind(tenant_id)
        .bind(r#type)
        .bind(name)
        .bind(unit)
        .fetch_all(exec)
        .await
}

/// Sets a lot's amount.
pub async fn update_amount<'e, E: PgExecutor<'e>>(
    exec: E,
    id: Uuid,
    new_amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE ingredients SET amount=$1, updated_at=now() WHERE id=$2")
        .bind(new_amount)
        .bind(id)
        .execute(exec)
        .await
        .map(|_| ())
}

/// True if the lot is referenced by any batch (blocks deletion).
pub async fn is_referenced_by_batch<'e, E: PgExecutor<'e>>(
    exec: E,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM batch_ingredients WHERE ingredient_id=$1)",
    )
    .bind(id)
    .fetch_one(exec)
    .await
}

/// Inserts an immutable stock-movement record.
pub async fn insert_movement<'e, E: PgExecutor<'e>>(
    exec: E,
    m: &MovementWrite,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO stock_movements (tenant_id, ingredient_id, amount_delta, balance_after, \
         reference_type, reference_id, notes, created_by_user_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(m.tenant_id)
    .bind(m.ingredient_id)
    .bind(m.amount_delta)
    .bind(m.balance_after)
    .bind(&m.reference_type)
    .bind(m.reference_id)
    .bind(&m.notes)
    .bind(m.created_by_user_id)
    .execute(exec)
    .await
    .map(|_| ())
}

/// Lists lots with filters, pagination, and a pre-validated `order_by` clause.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
    order_by: &str,
) -> Result<Page<Ingredient>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    // Shared WHERE builder used for both count and list.
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if !filter.out_of_stock {
            qb.push(" AND amount > 0");
        }
        if let Some(t) = &filter.r#type {
            qb.push(" AND type = ").push_bind(t.clone());
        }
        if let Some(n) = &filter.name {
            qb.push(" AND lower(name) LIKE ")
                .push_bind(format!("%{}%", n.to_lowercase()));
        }
        if let Some(l) = &filter.lot_number {
            qb.push(" AND lot_number = ").push_bind(l.clone());
        }
        if let Some(d) = &filter.expiring_before {
            qb.push(" AND best_before_date IS NOT NULL AND best_before_date <= ")
                .push_bind(d.clone())
                .push("::date");
        }
        if let Some(days) = filter.expiring_within_days {
            qb.push(" AND best_before_date IS NOT NULL AND best_before_date <= (now() + make_interval(days => ")
                .push_bind(days)
                .push("))::date");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM ingredients");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {ING_COLS} FROM ingredients"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb
        .build_query_as::<Ingredient>()
        .fetch_all(pool)
        .await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Lists stock movements with filters and pagination.
pub async fn select_movements(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &MovementFilter,
) -> Result<Page<StockMovement>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(i) = filter.ingredient_id {
            qb.push(" AND ingredient_id = ").push_bind(i);
        }
        if let Some(rt) = &filter.reference_type {
            qb.push(" AND reference_type = ").push_bind(rt.clone());
        }
        if let Some(ri) = filter.reference_id {
            qb.push(" AND reference_id = ").push_bind(ri);
        }
        if let Some(f) = &filter.from_date {
            qb.push(" AND created_at >= ")
                .push_bind(f.clone())
                .push("::date");
        }
        if let Some(t) = &filter.to_date {
            qb.push(" AND created_at < (")
                .push_bind(t.clone())
                .push("::date + 1)");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM stock_movements");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order = if filter.sort == "created_at" {
        "created_at ASC"
    } else {
        "created_at DESC"
    };
    let mut list_qb =
        QueryBuilder::<Postgres>::new(format!("SELECT {MV_COLS} FROM stock_movements"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb
        .build_query_as::<StockMovement>()
        .fetch_all(pool)
        .await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Aggregated inventory summary grouped by (type, name, unit).
pub async fn select_summary(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &SummaryFilter,
) -> Result<Page<SummaryRow>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(t) = &filter.r#type {
            qb.push(" AND type = ").push_bind(t.clone());
        }
    };

    let mut count_qb =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM (SELECT 1 FROM ingredients");
    push_where(&mut count_qb);
    count_qb.push(" GROUP BY type, lower(name), unit) AS sub");
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(
        "SELECT type, name, unit, SUM(amount)::float8 AS total_amount, \
         COUNT(*)::int8 AS lot_count, \
         to_char(MIN(best_before_date), 'YYYY-MM-DD') AS earliest_best_before_date, \
         COALESCE(ROUND(SUM(cost_pence::numeric * amount) / NULLIF(SUM(amount), 0)), 0)::int8 \
         AS weighted_avg_cost_pence_per_unit FROM ingredients",
    );
    push_where(&mut qb);
    qb.push(" GROUP BY type, lower(name), name, unit ORDER BY type, name");
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<SummaryRow>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Counts (type, name, unit) groups whose total amount is below 1.0.
pub async fn count_low_stock(pool: &PgPool, tenant_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM (SELECT 1 FROM ingredients WHERE tenant_id=$1 \
         GROUP BY type, lower(name), unit HAVING SUM(amount) < 1.0) AS sub",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
}
