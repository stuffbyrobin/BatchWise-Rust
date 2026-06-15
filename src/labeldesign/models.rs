//! Label-design domain types and DTOs.
//!
//! Port of the Go `internal/labeldesign` types. The render-model value types
//! live in [`crate::pkg::labelkit`] (pkg cannot import internal); they are
//! re-exported here so the domain surface reads naturally. `options` is the
//! `label_designs.options` JSONB column, decoded via `#[sqlx(json)]`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::pkg::labelkit::DesignOptions;

/// Generic paginated response envelope.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 && total > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        Page {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

// ---- domain types ----

/// An uploaded logo image (metadata only; bytes fetched separately).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BrandAsset {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub byte_size: i32,
    pub created_at: DateTime<Utc>,
}

/// A per-tenant branding configuration.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BrandProfile {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub font_family: String,
    pub logo_asset_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A label/clip/lens design instance.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LabelDesign {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub brand_profile_id: Option<Uuid>,
    pub size_key: String,
    pub template_key: String,
    #[sqlx(json)]
    pub options: DesignOptions,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateBrandProfileRequest {
    #[validate(length(min = 1, max = 120))]
    pub name: String,
    #[validate(custom(function = "validate_hexcolor_opt"))]
    pub primary_color: Option<String>,
    #[validate(custom(function = "validate_hexcolor_opt"))]
    pub secondary_color: Option<String>,
    #[validate(custom(function = "validate_font_family_opt"))]
    pub font_family: Option<String>,
    pub logo_asset_id: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchBrandProfileRequest {
    #[validate(length(min = 1, max = 120))]
    pub name: Option<String>,
    #[validate(custom(function = "validate_hexcolor_opt"))]
    pub primary_color: Option<String>,
    #[validate(custom(function = "validate_hexcolor_opt"))]
    pub secondary_color: Option<String>,
    #[validate(custom(function = "validate_font_family_opt"))]
    pub font_family: Option<String>,
    pub logo_asset_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateLabelDesignRequest {
    #[validate(custom(function = "validate_kind"))]
    pub kind: String,
    #[validate(length(min = 1, max = 120))]
    pub name: String,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub brand_profile_id: Option<Uuid>,
    #[validate(length(min = 1))]
    pub size_key: String,
    #[validate(length(min = 1))]
    pub template_key: String,
    pub options: Option<DesignOptions>,
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PatchLabelDesignRequest {
    #[validate(length(min = 1, max = 120))]
    pub name: Option<String>,
    pub brand_profile_id: Option<Uuid>,
    pub size_key: Option<String>,
    pub template_key: Option<String>,
    pub options: Option<DesignOptions>,
}

// ---- filters ----

#[derive(Debug, Default)]
pub struct ListFilter {
    pub kind: Option<String>,
    pub batch_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
}

// ---- validators ----

const KINDS: [&str; 4] = ["bottle", "can", "pump_clip", "cask_lens"];
const FONT_FAMILIES: [&str; 3] = ["helvetica", "times", "courier"];

/// `#[a-fA-F0-9]{6}` hex colour (matches the Go `hexcolor` validator shape).
fn is_hexcolor(v: &str) -> bool {
    let bytes = v.as_bytes();
    bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

fn validate_hexcolor_opt(v: &str) -> Result<(), ValidationError> {
    if is_hexcolor(v) {
        Ok(())
    } else {
        Err(ValidationError::new("hexcolor"))
    }
}

fn validate_font_family_opt(v: &str) -> Result<(), ValidationError> {
    if FONT_FAMILIES.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("oneof"))
    }
}

fn validate_kind(v: &str) -> Result<(), ValidationError> {
    if KINDS.contains(&v) {
        Ok(())
    } else {
        Err(ValidationError::new("oneof"))
    }
}
