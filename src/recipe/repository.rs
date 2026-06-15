//! Data access for recipes and their nested children.
//!
//! Port of the Go `internal/recipe/repository.go`. `NUMERIC` columns are
//! selected as `float8`. Child arrays are replaced wholesale (DELETE + INSERT)
//! inside the caller's transaction.

use sqlx::{PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    CalculatedValues, Fermentable, Hop, ListFilter, MashStep, Page, Recipe, Yeast,
};

const REC_COLS: &str = "id, tenant_id, name, type, style_id, equipment_profile_id, mash_profile_id, \
    batch_size_liters::float8 AS batch_size_liters, boil_size_liters::float8 AS boil_size_liters, \
    boil_time_minutes, efficiency_pct::float8 AS efficiency_pct, calc_og::float8 AS calc_og, \
    calc_fg::float8 AS calc_fg, calc_abv_pct::float8 AS calc_abv_pct, calc_ibu::float8 AS calc_ibu, \
    calc_color_ebc::float8 AS calc_color_ebc, tasting_aroma, tasting_flavour, tasting_mouthfeel, \
    tasting_finish, notes, created_at, updated_at";

/// Scalar columns for inserting/replacing a recipe.
#[derive(Debug, Clone)]
pub struct RecipeWrite {
    pub name: String,
    pub r#type: String,
    pub style_id: Option<Uuid>,
    pub equipment_profile_id: Option<Uuid>,
    pub mash_profile_id: Option<Uuid>,
    pub batch_size_liters: f64,
    pub boil_size_liters: Option<f64>,
    pub boil_time_minutes: Option<i32>,
    pub efficiency_pct: Option<f64>,
    pub tasting_aroma: Option<String>,
    pub tasting_flavour: Option<String>,
    pub tasting_mouthfeel: Option<String>,
    pub tasting_finish: Option<String>,
    pub notes: Option<String>,
}

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

/// Inserts a recipe (scalar columns only) and returns the created row.
pub async fn insert<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    w: &RecipeWrite,
) -> Result<Recipe, sqlx::Error> {
    let sql = format!(
        "INSERT INTO recipes (tenant_id, name, type, style_id, equipment_profile_id, \
         mash_profile_id, batch_size_liters, boil_size_liters, boil_time_minutes, efficiency_pct, \
         tasting_aroma, tasting_flavour, tasting_mouthfeel, tasting_finish, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING {REC_COLS}"
    );
    bind_write(sqlx::query_as::<_, Recipe>(&sql).bind(tenant_id), w)
        .fetch_one(exec)
        .await
}

/// Replaces scalar columns; returns the new row or `None` if not found.
pub async fn update_full<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
    w: &RecipeWrite,
) -> Result<Option<Recipe>, sqlx::Error> {
    let sql = format!(
        "UPDATE recipes SET name=$3, type=$4, style_id=$5, equipment_profile_id=$6, \
         mash_profile_id=$7, batch_size_liters=$8, boil_size_liters=$9, boil_time_minutes=$10, \
         efficiency_pct=$11, tasting_aroma=$12, tasting_flavour=$13, tasting_mouthfeel=$14, \
         tasting_finish=$15, notes=$16, updated_at=now() WHERE tenant_id=$1 AND id=$2 \
         RETURNING {REC_COLS}"
    );
    let q = sqlx::query_as::<_, Recipe>(&sql).bind(tenant_id).bind(id);
    bind_write(q, w).fetch_optional(exec).await
}

fn bind_write<'q>(
    q: sqlx::query::QueryAs<'q, Postgres, Recipe, sqlx::postgres::PgArguments>,
    w: &'q RecipeWrite,
) -> sqlx::query::QueryAs<'q, Postgres, Recipe, sqlx::postgres::PgArguments> {
    q.bind(&w.name)
        .bind(&w.r#type)
        .bind(w.style_id)
        .bind(w.equipment_profile_id)
        .bind(w.mash_profile_id)
        .bind(w.batch_size_liters)
        .bind(w.boil_size_liters)
        .bind(w.boil_time_minutes)
        .bind(w.efficiency_pct)
        .bind(&w.tasting_aroma)
        .bind(&w.tasting_flavour)
        .bind(&w.tasting_mouthfeel)
        .bind(&w.tasting_finish)
        .bind(&w.notes)
}

/// Updates only the cached calculated values.
pub async fn update_calculations<'e, E: PgExecutor<'e>>(
    exec: E,
    id: Uuid,
    c: &CalculatedValues,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE recipes SET calc_og=$1, calc_fg=$2, calc_abv_pct=$3, calc_ibu=$4, \
         calc_color_ebc=$5, updated_at=now() WHERE id=$6",
    )
    .bind(c.calc_og)
    .bind(c.calc_fg)
    .bind(c.calc_abv_pct)
    .bind(c.calc_ibu)
    .bind(c.calc_color_ebc)
    .bind(id)
    .execute(exec)
    .await
    .map(|_| ())
}

/// Fetches the recipe row (no children) by id, tenant-scoped.
pub async fn select_row(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Recipe>, sqlx::Error> {
    let sql = format!("SELECT {REC_COLS} FROM recipes WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Recipe>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Fetches the recipe row (no children) by name, tenant-scoped.
pub async fn select_row_by_name(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
) -> Result<Option<Recipe>, sqlx::Error> {
    let sql = format!("SELECT {REC_COLS} FROM recipes WHERE tenant_id=$1 AND name=$2 LIMIT 1");
    sqlx::query_as::<_, Recipe>(&sql)
        .bind(tenant_id)
        .bind(name)
        .fetch_optional(pool)
        .await
}

/// Lists recipes with filters and a pre-validated `order_by` clause.
pub async fn select_list(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
    order_by: &str,
) -> Result<Page<Recipe>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);

    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(n) = &filter.name {
            qb.push(" AND name ILIKE ").push_bind(format!("%{n}%"));
        }
        if let Some(t) = &filter.r#type {
            qb.push(" AND type = ").push_bind(t.clone());
        }
        if let Some(s) = filter.style_id {
            qb.push(" AND style_id = ").push_bind(s);
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM recipes");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut list_qb = QueryBuilder::<Postgres>::new(format!("SELECT {REC_COLS} FROM recipes"));
    push_where(&mut list_qb);
    list_qb.push(format!(" ORDER BY {order_by} "));
    list_qb.push(" LIMIT ").push_bind(page_size);
    list_qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = list_qb.build_query_as::<Recipe>().fetch_all(pool).await?;

    Ok(Page::new(items, total, page, page_size))
}

/// Deletes a recipe; returns true if a row was removed.
pub async fn delete_by_id<'e, E: PgExecutor<'e>>(
    exec: E,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM recipes WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// True if any batch references this recipe. Runs on the pool so a missing
/// `batches` table (early phases) does not poison the caller's transaction.
pub async fn is_referenced_by_batch(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM batches WHERE recipe_id=$1)")
        .bind(id)
        .fetch_one(pool)
        .await
    {
        Ok(exists) => Ok(exists),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42P01") => Ok(false),
        Err(e) => Err(e),
    }
}

// ---- child replacement (DELETE + INSERT inside the tx) ----

/// Replaces all fermentables for a recipe.
pub async fn replace_fermentables(
    conn: &mut PgConnection,
    recipe_id: Uuid,
    rows: &[Fermentable],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM recipe_fermentables WHERE recipe_id=$1")
        .bind(recipe_id)
        .execute(&mut *conn)
        .await?;
    for f in rows {
        sqlx::query(
            "INSERT INTO recipe_fermentables (recipe_id, step_order, name, amount, unit, \
             color_ebc, potential_ppg, type, addition) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(recipe_id)
        .bind(f.step_order)
        .bind(&f.name)
        .bind(f.amount)
        .bind(&f.unit)
        .bind(f.color_ebc)
        .bind(f.potential_ppg)
        .bind(&f.r#type)
        .bind(&f.addition)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Replaces all hops for a recipe.
pub async fn replace_hops(
    conn: &mut PgConnection,
    recipe_id: Uuid,
    rows: &[Hop],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM recipe_hops WHERE recipe_id=$1")
        .bind(recipe_id)
        .execute(&mut *conn)
        .await?;
    for h in rows {
        sqlx::query(
            "INSERT INTO recipe_hops (recipe_id, step_order, name, amount, unit, alpha_acid_pct, \
             boil_time_minutes, form, use) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(recipe_id)
        .bind(h.step_order)
        .bind(&h.name)
        .bind(h.amount)
        .bind(&h.unit)
        .bind(h.alpha_acid_pct)
        .bind(h.boil_time_minutes)
        .bind(&h.form)
        .bind(&h.r#use)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Replaces all yeasts for a recipe.
pub async fn replace_yeasts(
    conn: &mut PgConnection,
    recipe_id: Uuid,
    rows: &[Yeast],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM recipe_yeasts WHERE recipe_id=$1")
        .bind(recipe_id)
        .execute(&mut *conn)
        .await?;
    for y in rows {
        sqlx::query(
            "INSERT INTO recipe_yeasts (recipe_id, yeast_id, name, amount, unit, attenuation_pct) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(recipe_id)
        .bind(y.yeast_id)
        .bind(&y.name)
        .bind(y.amount)
        .bind(&y.unit)
        .bind(y.attenuation_pct)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Replaces all mash steps for a recipe.
pub async fn replace_mash_steps(
    conn: &mut PgConnection,
    recipe_id: Uuid,
    rows: &[MashStep],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM recipe_mash_steps WHERE recipe_id=$1")
        .bind(recipe_id)
        .execute(&mut *conn)
        .await?;
    for ms in rows {
        sqlx::query(
            "INSERT INTO recipe_mash_steps (recipe_id, step_order, step_type, target_temp_c, \
             hold_minutes, infusion_volume_liters) VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(recipe_id)
        .bind(ms.step_order)
        .bind(&ms.step_type)
        .bind(ms.target_temp_c)
        .bind(ms.hold_minutes)
        .bind(ms.infusion_volume_liters)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

// ---- child selects ----

/// Fetches a recipe's fermentables ordered by step_order.
pub async fn select_fermentables(
    pool: &PgPool,
    recipe_id: Uuid,
) -> Result<Vec<Fermentable>, sqlx::Error> {
    sqlx::query_as::<_, Fermentable>(
        "SELECT id, recipe_id, step_order, name, amount::float8 AS amount, unit, \
         color_ebc::float8 AS color_ebc, potential_ppg::float8 AS potential_ppg, type, addition \
         FROM recipe_fermentables WHERE recipe_id=$1 ORDER BY step_order",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await
}

/// Fetches a recipe's hops ordered by step_order.
pub async fn select_hops(pool: &PgPool, recipe_id: Uuid) -> Result<Vec<Hop>, sqlx::Error> {
    sqlx::query_as::<_, Hop>(
        "SELECT id, recipe_id, step_order, name, amount::float8 AS amount, unit, \
         alpha_acid_pct::float8 AS alpha_acid_pct, boil_time_minutes::float8 AS boil_time_minutes, \
         form, use FROM recipe_hops WHERE recipe_id=$1 ORDER BY step_order",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await
}

/// Fetches a recipe's yeasts.
pub async fn select_yeasts(pool: &PgPool, recipe_id: Uuid) -> Result<Vec<Yeast>, sqlx::Error> {
    sqlx::query_as::<_, Yeast>(
        "SELECT id, recipe_id, yeast_id, name, amount::float8 AS amount, unit, \
         attenuation_pct::float8 AS attenuation_pct FROM recipe_yeasts WHERE recipe_id=$1",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await
}

/// Fetches a recipe's mash steps ordered by step_order.
pub async fn select_mash_steps(
    pool: &PgPool,
    recipe_id: Uuid,
) -> Result<Vec<MashStep>, sqlx::Error> {
    sqlx::query_as::<_, MashStep>(
        "SELECT id, recipe_id, step_order, step_type, target_temp_c::float8 AS target_temp_c, \
         hold_minutes, infusion_volume_liters::float8 AS infusion_volume_liters \
         FROM recipe_mash_steps WHERE recipe_id=$1 ORDER BY step_order",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await
}
