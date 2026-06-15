//! Label/clip/lens render model, size & template presets, field formatting, and
//! print-ready PDF rendering.
//!
//! Port of the Go `pkg/labelkit` package. Pure aside from PDF generation (via
//! `printpdf`); no dependency on the rest of the crate.

pub mod format;
pub mod model;
pub mod presets;
pub mod render;

pub use format::{format_allergens, format_best_before};
pub use model::{DesignOptions, RenderBrand, RenderFields, RenderModel, RenderTasting};
pub use presets::{size_preset, template_preset, valid_size_for_kind, SizeSpec, TemplateSpec};
pub use render::render_pdf;
