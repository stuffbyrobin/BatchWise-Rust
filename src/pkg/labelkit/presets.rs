//! Label size and template presets.
//!
//! Port of the Go `pkg/labelkit/presets.go`.

/// Physical dimensions and classification of a label size.
#[derive(Debug, Clone, Copy)]
pub struct SizeSpec {
    pub key: &'static str,
    pub kind: &'static str,
    pub shape: &'static str,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// A label template with its description.
#[derive(Debug, Clone, Copy)]
pub struct TemplateSpec {
    pub key: &'static str,
    pub description: &'static str,
}

const SIZES: [SizeSpec; 5] = [
    SizeSpec { key: "bottle_front_90x120", kind: "bottle", shape: "rect", width_mm: 90.0, height_mm: 120.0 },
    SizeSpec { key: "can_wrap_200x100", kind: "can", shape: "rect", width_mm: 200.0, height_mm: 100.0 },
    SizeSpec { key: "pumpclip_round_114", kind: "pump_clip", shape: "circle", width_mm: 114.0, height_mm: 114.0 },
    SizeSpec { key: "pumpclip_rect_140x90", kind: "pump_clip", shape: "rect", width_mm: 140.0, height_mm: 90.0 },
    SizeSpec { key: "lens_round_100", kind: "cask_lens", shape: "circle", width_mm: 100.0, height_mm: 100.0 },
];

const TEMPLATES: [TemplateSpec; 2] = [
    TemplateSpec { key: "compliance_standard", description: "Full compliance layout for bottle/can" },
    TemplateSpec { key: "clip_standard", description: "Pump clip / cask lens layout" },
];

/// Returns the [`SizeSpec`] for the given key if it exists.
pub fn size_preset(key: &str) -> Option<SizeSpec> {
    SIZES.iter().find(|s| s.key == key).copied()
}

/// Returns the [`TemplateSpec`] for the given key if it exists.
pub fn template_preset(key: &str) -> Option<TemplateSpec> {
    TEMPLATES.iter().find(|t| t.key == key).copied()
}

/// True only if `size_key` exists and its kind matches `kind`.
pub fn valid_size_for_kind(size_key: &str, kind: &str) -> bool {
    size_preset(size_key).is_some_and(|s| s.kind == kind)
}
