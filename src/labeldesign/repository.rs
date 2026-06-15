//! Data access for brand assets, brand profiles, and label designs.
//!
//! Port of the Go `internal/labeldesign/repository.go`. `brand_assets.data` is
//! `BYTEA` (bound/selected as `Vec<u8>`); `label_designs.options` is JSONB
//! (decoded via `#[sqlx(json)]`). Every query is tenant-scoped.

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{BrandAsset, BrandProfile, LabelDesign, ListFilter, Page};
use crate::pkg::labelkit::DesignOptions;
use crate::platform::errors::ApiError;

const ASSET_COLS: &str = "id, tenant_id, filename, content_type, byte_size, created_at";

const PROFILE_COLS: &str =
    "id, tenant_id, name, primary_color, secondary_color, font_family, logo_asset_id, \
    created_at, updated_at";

const DESIGN_COLS: &str = "id, tenant_id, kind, name, batch_id, recipe_id, brand_profile_id, \
    size_key, template_key, options, created_at, updated_at";

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

fn design_order_by(sort: &str) -> &'static str {
    match sort {
        "created_at" => "created_at ASC",
        "name" => "name ASC",
        "-name" => "name DESC",
        _ => "created_at DESC",
    }
}

// ---- brand assets ----

/// Inserts a brand asset (with its bytes) and returns the metadata.
pub async fn insert_asset(
    pool: &PgPool,
    tenant_id: Uuid,
    filename: &str,
    content_type: &str,
    byte_size: i32,
    data: &[u8],
) -> Result<BrandAsset, sqlx::Error> {
    let sql = format!(
        "INSERT INTO brand_assets (tenant_id, filename, content_type, byte_size, data) \
         VALUES ($1,$2,$3,$4,$5) RETURNING {ASSET_COLS}"
    );
    sqlx::query_as::<_, BrandAsset>(&sql)
        .bind(tenant_id)
        .bind(filename)
        .bind(content_type)
        .bind(byte_size)
        .bind(data)
        .fetch_one(pool)
        .await
}

/// Row carrying a brand asset's metadata plus its bytes.
#[derive(sqlx::FromRow)]
struct AssetWithData {
    id: Uuid,
    tenant_id: Uuid,
    filename: String,
    content_type: String,
    byte_size: i32,
    data: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Fetches a brand asset's metadata and bytes, tenant-scoped. 404 if missing.
pub async fn select_asset(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<(BrandAsset, Vec<u8>), ApiError> {
    let row: Option<AssetWithData> = sqlx::query_as(
        "SELECT id, tenant_id, filename, content_type, byte_size, data, created_at \
             FROM brand_assets WHERE tenant_id=$1 AND id=$2",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok((
            BrandAsset {
                id: r.id,
                tenant_id: r.tenant_id,
                filename: r.filename,
                content_type: r.content_type,
                byte_size: r.byte_size,
                created_at: r.created_at,
            },
            r.data,
        )),
        None => Err(ApiError::not_found("brand_asset")),
    }
}

/// Deletes a brand asset; 404 if no row was removed.
pub async fn delete_asset(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let r = sqlx::query("DELETE FROM brand_assets WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(ApiError::not_found("brand_asset"));
    }
    Ok(())
}

// ---- brand profiles ----

/// Inserts a brand profile and returns it.
pub async fn insert_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
    primary_color: &str,
    secondary_color: &str,
    font_family: &str,
    logo_asset_id: Option<Uuid>,
) -> Result<BrandProfile, sqlx::Error> {
    let sql = format!(
        "INSERT INTO brand_profiles \
         (tenant_id, name, primary_color, secondary_color, font_family, logo_asset_id) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING {PROFILE_COLS}"
    );
    sqlx::query_as::<_, BrandProfile>(&sql)
        .bind(tenant_id)
        .bind(name)
        .bind(primary_color)
        .bind(secondary_color)
        .bind(font_family)
        .bind(logo_asset_id)
        .fetch_one(pool)
        .await
}

/// Lists all brand profiles for the tenant (ordered by name).
pub async fn select_profiles(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<BrandProfile>, sqlx::Error> {
    let sql =
        format!("SELECT {PROFILE_COLS} FROM brand_profiles WHERE tenant_id=$1 ORDER BY name ASC");
    sqlx::query_as::<_, BrandProfile>(&sql)
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

/// Fetches a brand profile by id, tenant-scoped.
pub async fn select_profile_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<BrandProfile>, sqlx::Error> {
    let sql = format!("SELECT {PROFILE_COLS} FROM brand_profiles WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, BrandProfile>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Updates a brand profile's mutable fields and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn update_profile(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    name: &str,
    primary_color: &str,
    secondary_color: &str,
    font_family: &str,
    logo_asset_id: Option<Uuid>,
) -> Result<Option<BrandProfile>, sqlx::Error> {
    let sql = format!(
        "UPDATE brand_profiles SET name=$3, primary_color=$4, secondary_color=$5, \
         font_family=$6, logo_asset_id=$7, updated_at=now() \
         WHERE tenant_id=$1 AND id=$2 RETURNING {PROFILE_COLS}"
    );
    sqlx::query_as::<_, BrandProfile>(&sql)
        .bind(tenant_id)
        .bind(id)
        .bind(name)
        .bind(primary_color)
        .bind(secondary_color)
        .bind(font_family)
        .bind(logo_asset_id)
        .fetch_optional(pool)
        .await
}

/// Deletes a brand profile; returns true if a row was removed.
pub async fn delete_profile(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM brand_profiles WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- designs ----

/// Inserts a label design and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_design(
    pool: &PgPool,
    tenant_id: Uuid,
    kind: &str,
    name: &str,
    batch_id: Option<Uuid>,
    recipe_id: Option<Uuid>,
    brand_profile_id: Option<Uuid>,
    size_key: &str,
    template_key: &str,
    options: &DesignOptions,
) -> Result<LabelDesign, sqlx::Error> {
    let sql = format!(
        "INSERT INTO label_designs \
         (tenant_id, kind, name, batch_id, recipe_id, brand_profile_id, \
          size_key, template_key, options) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING {DESIGN_COLS}"
    );
    sqlx::query_as::<_, LabelDesign>(&sql)
        .bind(tenant_id)
        .bind(kind)
        .bind(name)
        .bind(batch_id)
        .bind(recipe_id)
        .bind(brand_profile_id)
        .bind(size_key)
        .bind(template_key)
        .bind(sqlx::types::Json(options))
        .fetch_one(pool)
        .await
}

/// Lists label designs with filters; paginated.
pub async fn select_designs(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &ListFilter,
) -> Result<Page<LabelDesign>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(k) = &filter.kind {
            qb.push(" AND kind = ").push_bind(k.clone());
        }
        if let Some(b) = filter.batch_id {
            qb.push(" AND batch_id = ").push_bind(b);
        }
        if let Some(r) = filter.recipe_id {
            qb.push(" AND recipe_id = ").push_bind(r);
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM label_designs");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = design_order_by(&filter.sort);
    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {DESIGN_COLS} FROM label_designs"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<LabelDesign>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Fetches a label design by id, tenant-scoped.
pub async fn select_design_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<LabelDesign>, sqlx::Error> {
    let sql = format!("SELECT {DESIGN_COLS} FROM label_designs WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, LabelDesign>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Updates a design's mutable fields (name, brand_profile_id, size/template,
/// options) and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn update_design(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    name: &str,
    brand_profile_id: Option<Uuid>,
    size_key: &str,
    template_key: &str,
    options: &DesignOptions,
) -> Result<Option<LabelDesign>, sqlx::Error> {
    let sql = format!(
        "UPDATE label_designs SET name=$3, brand_profile_id=$4, size_key=$5, \
         template_key=$6, options=$7, updated_at=now() \
         WHERE tenant_id=$1 AND id=$2 RETURNING {DESIGN_COLS}"
    );
    sqlx::query_as::<_, LabelDesign>(&sql)
        .bind(tenant_id)
        .bind(id)
        .bind(name)
        .bind(brand_profile_id)
        .bind(size_key)
        .bind(template_key)
        .bind(sqlx::types::Json(options))
        .fetch_optional(pool)
        .await
}

/// Deletes a label design; returns true if a row was removed.
pub async fn delete_design(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM label_designs WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}
