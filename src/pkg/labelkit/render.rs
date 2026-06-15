//! Print-ready PDF rendering of a [`RenderModel`].
//!
//! Port of the Go `pkg/labelkit/render.go` (which used `go-pdf/fpdf`). Uses
//! `printpdf`. Note the coordinate-system difference: fpdf measures Y from the
//! top, printpdf from the bottom — text positions are converted accordingly.
//! Content, fonts, colours, and the per-kind/option field logic mirror the Go
//! renderer; a logo that cannot be decoded is skipped (matching Go's behaviour).

use printpdf::{BuiltinFont, Color, Image, ImageTransform, IndirectFontRef, Mm, PdfDocumentReference, PdfLayerReference, Rgb};

use super::format::format_allergens;
use super::model::RenderModel;

/// Parses a `#rrggbb` hex string to an RGB triple (0–255). Invalid input → black.
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let b = hex.as_bytes();
    if b.len() != 7 || b[0] != b'#' {
        return (0, 0, 0);
    }
    let parse = |s: &str| u8::from_str_radix(s, 16);
    match (parse(&hex[1..3]), parse(&hex[3..5]), parse(&hex[5..7])) {
        (Ok(r), Ok(g), Ok(b)) => (r, g, b),
        _ => (0, 0, 0),
    }
}

/// The three builtin font variants (regular, bold, italic) for a family.
fn font_variants(family: &str) -> (BuiltinFont, BuiltinFont, BuiltinFont) {
    match family {
        "times" => (
            BuiltinFont::TimesRoman,
            BuiltinFont::TimesBold,
            BuiltinFont::TimesItalic,
        ),
        "courier" => (
            BuiltinFont::Courier,
            BuiltinFont::CourierBold,
            BuiltinFont::CourierOblique,
        ),
        _ => (
            BuiltinFont::Helvetica,
            BuiltinFont::HelveticaBold,
            BuiltinFont::HelveticaOblique,
        ),
    }
}

/// Embeds the logo in the top-right corner (~20 mm wide), best-effort: any decode
/// failure simply omits the logo so a bad image never breaks PDF generation.
fn embed_logo(layer: &PdfLayerReference, width_mm: f64, height_mm: f64, logo: &[u8]) {
    if logo.is_empty() {
        return;
    }
    let Ok(img) = printpdf::image_crate::load_from_memory(logo) else {
        return;
    };
    let (w_px, h_px) = (img.width() as f32, img.height() as f32);
    if w_px == 0.0 || h_px == 0.0 {
        return;
    }
    // Target a 20 mm wide logo; derive the dpi that yields that width.
    let target_w_mm = 20.0_f32;
    let dpi = w_px * 25.4 / target_w_mm;
    let logo_h_mm = h_px * target_w_mm / w_px;
    let image = Image::from_dynamic_image(&img);
    image.add_to_layer(
        layer.clone(),
        ImageTransform {
            translate_x: Some(Mm(width_mm as f32 - 25.0)),
            translate_y: Some(Mm(height_mm as f32 - 5.0 - logo_h_mm)),
            dpi: Some(dpi),
            ..Default::default()
        },
    );
}

/// Renders the model to a print-ready PDF sized to the model's mm dimensions.
pub fn render_pdf(m: &RenderModel, logo: &[u8]) -> Result<Vec<u8>, printpdf::Error> {
    let (doc, page, layer_idx) = printpdf::PdfDocument::new(
        &m.fields.product_name,
        Mm(m.width_mm as f32),
        Mm(m.height_mm as f32),
        "Layer 1",
    );
    let layer = doc.get_page(page).get_layer(layer_idx);

    let (reg, bold, italic) = font_variants(&m.brand.font_family);
    let f_reg = doc.add_builtin_font(reg)?;
    let f_bold = doc.add_builtin_font(bold)?;
    let f_italic = doc.add_builtin_font(italic)?;

    let (r, g, b) = hex_to_rgb(&m.brand.primary_color);
    layer.set_fill_color(Color::Rgb(Rgb::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        None,
    )));

    embed_logo(&layer, m.width_mm, m.height_mm, logo);

    let h = m.height_mm;
    // Converts an fpdf-style top offset + point size to a printpdf baseline Y.
    let baseline = |y_top: f64, size_pt: f64| Mm((h - y_top - size_pt * 0.3528) as f32);
    let put = |y_top: f64, size: f64, font: &IndirectFontRef, text: &str| {
        layer.use_text(text, size as f32, Mm(5.0), baseline(y_top, size), font);
    };

    // Title (product name).
    put(8.0, 16.0, &f_bold, &m.fields.product_name);

    let mut y = 16.0_f64;

    match m.kind.as_str() {
        "bottle" | "can" => {
            if let Some(style) = &m.fields.style {
                put(y, 9.0, &f_reg, &format!("Style: {style}"));
                y += 5.0;
            }
            put(y, 10.0, &f_reg, &format!("ABV {:.1}%", m.fields.abv_percent));
            y += 5.0;
            if !m.fields.allergens.is_empty() {
                put(
                    y,
                    9.0,
                    &f_bold,
                    &format!("Allergens: {}", format_allergens(&m.fields.allergens)),
                );
                y += 5.0;
            }
            if let Some(v) = m.fields.net_volume_ml {
                put(y, 9.0, &f_reg, &format!("{v} ml"));
                y += 5.0;
            }
            if let Some(rp) = &m.fields.responsible_party {
                put(y, 9.0, &f_reg, rp);
                y += 5.0;
            }
            if let Some(co) = &m.fields.country_of_origin {
                put(y, 9.0, &f_reg, &format!("Origin: {co}"));
                y += 5.0;
            }
            if let Some(bb) = &m.fields.best_before_date {
                put(y, 9.0, &f_reg, &format!("Best before: {bb}"));
                y += 5.0;
            }
            if let Some(lot) = &m.fields.lot_identifier {
                put(y, 9.0, &f_reg, &format!("Lot: {lot}"));
                y += 5.0;
            }
            if m.options.show_ingredient_list {
                if let Some(il) = &m.fields.ingredient_list {
                    put(y, 9.0, &f_reg, &format!("Ingredients: {il}"));
                    y += 5.0;
                }
            }
            if m.options.show_energy {
                if let (Some(kj), Some(kcal)) =
                    (m.fields.energy_kj_per_100ml, m.fields.energy_kcal_per_100ml)
                {
                    put(
                        y,
                        9.0,
                        &f_reg,
                        &format!("Energy: {kj:.0} kJ / {kcal:.0} kcal per 100ml"),
                    );
                    y += 5.0;
                }
            }
            if m.options.show_units {
                if let Some(u) = m.fields.alcohol_units_per_serving {
                    put(y, 9.0, &f_reg, &format!("Units: {u:.1} per serving"));
                    y += 5.0;
                }
            }
            if m.options.show_responsible_drinking {
                put(y, 9.0, &f_reg, "Please drink responsibly");
            }
        }
        "pump_clip" | "cask_lens" => {
            if let Some(style) = &m.fields.style {
                put(y, 10.0, &f_italic, &format!("Style: {style}"));
                y += 5.0;
            }
            put(y, 12.0, &f_reg, &format!("ABV {:.1}%", m.fields.abv_percent));
            y += 5.0;
            if !m.brand.brewery_name.is_empty() {
                put(y, 9.0, &f_reg, &m.brand.brewery_name);
                y += 5.0;
            }
            if m.options.show_tasting_notes {
                if let Some(t) = &m.fields.tasting {
                    if let Some(a) = &t.aroma {
                        put(y, 9.0, &f_reg, &format!("Aroma: {a}"));
                        y += 5.0;
                    }
                    if let Some(fl) = &t.flavour {
                        put(y, 9.0, &f_reg, &format!("Flavour: {fl}"));
                        y += 5.0;
                    }
                    if let Some(mf) = &t.mouthfeel {
                        put(y, 9.0, &f_reg, &format!("Mouthfeel: {mf}"));
                        y += 5.0;
                    }
                    if let Some(fin) = &t.finish {
                        put(y, 9.0, &f_reg, &format!("Finish: {fin}"));
                    }
                }
            }
        }
        _ => {}
    }

    save_to_bytes(doc)
}

fn save_to_bytes(doc: PdfDocumentReference) -> Result<Vec<u8>, printpdf::Error> {
    doc.save_to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::labelkit::model::{DesignOptions, RenderBrand, RenderFields};
    use uuid::Uuid;

    fn model(kind: &str) -> RenderModel {
        RenderModel {
            design_id: Uuid::new_v4(),
            kind: kind.into(),
            size_key: "bottle_front_90x120".into(),
            template_key: "compliance_standard".into(),
            width_mm: 90.0,
            height_mm: 120.0,
            shape: "rect".into(),
            brand: RenderBrand {
                brewery_name: "Test Brewery".into(),
                primary_color: "#112233".into(),
                secondary_color: "#ffffff".into(),
                font_family: "helvetica".into(),
                logo_asset_id: None,
            },
            fields: RenderFields {
                product_name: "Test Ale".into(),
                abv_percent: 5.2,
                allergens: vec!["gluten".into()],
                net_volume_ml: Some(500),
                ..Default::default()
            },
            options: DesignOptions {
                show_responsible_drinking: true,
                ..Default::default()
            },
        }
    }

    #[test]
    fn renders_valid_pdf_bytes() {
        let bytes = render_pdf(&model("bottle"), &[]).expect("render");
        assert!(bytes.starts_with(b"%PDF"), "should be a PDF");
        assert!(bytes.len() > 500);
    }

    #[test]
    fn renders_clip_kind() {
        let mut m = model("pump_clip");
        m.size_key = "pumpclip_round_114".into();
        let bytes = render_pdf(&m, &[]).expect("render");
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn invalid_logo_is_skipped() {
        let bytes = render_pdf(&model("bottle"), b"not-an-image").expect("render");
        assert!(bytes.starts_with(b"%PDF"));
    }
}
