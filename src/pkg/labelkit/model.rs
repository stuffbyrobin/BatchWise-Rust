//! Render model types — the resolved, locked data the renderer consumes.
//!
//! Port of the Go `pkg/labelkit/model.go`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-design voluntary-field toggles, stored in the `label_designs.options`
/// JSONB column and surfaced on the design and render model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignOptions {
    #[serde(default)]
    pub show_ingredient_list: bool,
    #[serde(default)]
    pub show_energy: bool,
    #[serde(default)]
    pub show_units: bool,
    #[serde(default)]
    pub show_responsible_drinking: bool,
    #[serde(default)]
    pub show_tasting_notes: bool,
}

/// The resolved data the renderer needs, merged from the design, brand profile,
/// size/template preset, and the source compliance/recipe record.
#[derive(Debug, Clone, Serialize)]
pub struct RenderModel {
    pub design_id: Uuid,
    pub kind: String,
    pub size_key: String,
    pub template_key: String,
    pub width_mm: f64,
    pub height_mm: f64,
    /// "rect" | "circle"
    pub shape: String,
    pub brand: RenderBrand,
    pub fields: RenderFields,
    pub options: DesignOptions,
}

/// Branding applied to a rendered artifact.
#[derive(Debug, Clone, Serialize)]
pub struct RenderBrand {
    pub brewery_name: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub font_family: String,
    pub logo_asset_id: Option<Uuid>,
}

/// Resolved content fields. Compliance fields originate from the approved label
/// record (bottle/can) and are locked; recipe fields originate from the recipe
/// (pump_clip/cask_lens).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RenderFields {
    pub product_name: String,
    pub style: Option<String>,
    pub abv_percent: f64,
    pub allergens: Vec<String>,
    pub net_volume_ml: Option<i32>,
    pub responsible_party: Option<String>,
    pub country_of_origin: Option<String>,
    pub best_before_date: Option<String>,
    pub lot_identifier: Option<String>,
    pub ingredient_list: Option<String>,
    pub energy_kj_per_100ml: Option<f64>,
    pub energy_kcal_per_100ml: Option<f64>,
    pub alcohol_units_per_serving: Option<f64>,
    pub tasting: Option<RenderTasting>,
}

/// Structured tasting descriptors for clip/lens output.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RenderTasting {
    pub aroma: Option<String>,
    pub flavour: Option<String>,
    pub mouthfeel: Option<String>,
    pub finish: Option<String>,
}
