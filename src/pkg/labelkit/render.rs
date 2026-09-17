//! Print-ready PDF rendering of a [`RenderModel`].
//!
//! Port of the Go `pkg/labelkit/render.go` (which used `go-pdf/fpdf`). Uses
//! `printpdf`. Note the coordinate-system difference: fpdf measures Y from the
//! top, printpdf from the bottom — text positions are converted accordingly.
//! Content, fonts, colours, and the per-kind/option field logic mirror the Go
//! renderer; a logo that cannot be decoded is skipped (matching Go's behaviour).

use image::{DynamicImage, ImageReader, Limits};
use printpdf::{
    BuiltinFont, Color, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt,
    RawImage, RawImageData, RawImageFormat, Rgb, TextItem, XObjectTransform,
};

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

/// Decodes a stored logo with dimension and allocation limits, so a crafted file
/// cannot exhaust memory while a PDF renders. `None` on any failure.
fn decode_logo(logo: &[u8]) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(std::io::Cursor::new(logo))
        .with_guessed_format()
        .ok()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().ok()
}

/// Adds the logo to the document and returns the op that draws it in the
/// top-right corner (~20 mm wide). Best-effort: a logo that cannot be decoded is
/// omitted, so a bad image never breaks PDF generation.
fn logo_op(doc: &mut PdfDocument, width_mm: f64, height_mm: f64, logo: &[u8]) -> Option<Op> {
    if logo.is_empty() {
        return None;
    }
    let img = decode_logo(logo)?;
    let (w_px, h_px) = (img.width() as f32, img.height() as f32);
    if w_px == 0.0 || h_px == 0.0 {
        return None;
    }
    // Target a 20 mm wide logo; derive the dpi that yields that width.
    let target_w_mm = 20.0_f32;
    let dpi = w_px * 25.4 / target_w_mm;
    let logo_h_mm = h_px * target_w_mm / w_px;
    let (width, height) = (img.width() as usize, img.height() as usize);
    // Keep transparency (printpdf writes the alpha channel as a soft mask).
    let (data_format, pixels) = if img.color().has_alpha() {
        (RawImageFormat::RGBA8, img.into_rgba8().into_raw())
    } else {
        (RawImageFormat::RGB8, img.into_rgb8().into_raw())
    };
    let id = doc.add_image(&RawImage {
        pixels: RawImageData::U8(pixels),
        width,
        height,
        data_format,
        tag: Vec::new(),
    });
    Some(Op::UseXobject {
        id,
        transform: XObjectTransform {
            translate_x: Some(Mm(width_mm as f32 - 25.0).into()),
            translate_y: Some(Mm(height_mm as f32 - 5.0 - logo_h_mm).into()),
            dpi: Some(dpi),
            ..Default::default()
        },
    })
}

/// Renders the model to a print-ready PDF sized to the model's mm dimensions.
pub fn render_pdf(m: &RenderModel, logo: &[u8]) -> Vec<u8> {
    let mut doc = PdfDocument::new(&m.fields.product_name);
    let mut ops: Vec<Op> = logo_op(&mut doc, m.width_mm, m.height_mm, logo)
        .into_iter()
        .collect();

    let (r, g, b) = hex_to_rgb(&m.brand.primary_color);
    ops.push(Op::SetFillColor {
        col: Color::Rgb(Rgb {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            icc_profile: None,
        }),
    });

    let (f_reg, f_bold, f_italic) = font_variants(&m.brand.font_family);
    let h = m.height_mm;
    // Converts an fpdf-style top offset + point size to a printpdf baseline Y.
    let baseline = |y_top: f64, size_pt: f64| Mm((h - y_top - size_pt * 0.3528) as f32);
    // One text section per line: `Td` in a fresh section is an absolute position.
    let mut put = |y_top: f64, size: f64, font: BuiltinFont, text: &str| {
        ops.extend([
            Op::StartTextSection,
            Op::SetFont {
                font: PdfFontHandle::Builtin(font),
                size: Pt(size as f32),
            },
            Op::SetTextCursor {
                pos: Point::new(Mm(5.0), baseline(y_top, size)),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
        ]);
    };

    // Title (product name).
    put(8.0, 16.0, f_bold, &m.fields.product_name);

    let mut y = 16.0_f64;

    match m.kind.as_str() {
        "bottle" | "can" => {
            if let Some(style) = &m.fields.style {
                put(y, 9.0, f_reg, &format!("Style: {style}"));
                y += 5.0;
            }
            put(y, 10.0, f_reg, &format!("ABV {:.1}%", m.fields.abv_percent));
            y += 5.0;
            if !m.fields.allergens.is_empty() {
                put(
                    y,
                    9.0,
                    f_bold,
                    &format!("Allergens: {}", format_allergens(&m.fields.allergens)),
                );
                y += 5.0;
            }
            if let Some(v) = m.fields.net_volume_ml {
                put(y, 9.0, f_reg, &format!("{v} ml"));
                y += 5.0;
            }
            if let Some(rp) = &m.fields.responsible_party {
                put(y, 9.0, f_reg, rp);
                y += 5.0;
            }
            if let Some(co) = &m.fields.country_of_origin {
                put(y, 9.0, f_reg, &format!("Origin: {co}"));
                y += 5.0;
            }
            if let Some(bb) = &m.fields.best_before_date {
                put(y, 9.0, f_reg, &format!("Best before: {bb}"));
                y += 5.0;
            }
            if let Some(lot) = &m.fields.lot_identifier {
                put(y, 9.0, f_reg, &format!("Lot: {lot}"));
                y += 5.0;
            }
            if m.options.show_ingredient_list {
                if let Some(il) = &m.fields.ingredient_list {
                    put(y, 9.0, f_reg, &format!("Ingredients: {il}"));
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
                        f_reg,
                        &format!("Energy: {kj:.0} kJ / {kcal:.0} kcal per 100ml"),
                    );
                    y += 5.0;
                }
            }
            if m.options.show_units {
                if let Some(u) = m.fields.alcohol_units_per_serving {
                    put(y, 9.0, f_reg, &format!("Units: {u:.1} per serving"));
                    y += 5.0;
                }
            }
            if m.options.show_responsible_drinking {
                put(y, 9.0, f_reg, "Please drink responsibly");
            }
        }
        "pump_clip" | "cask_lens" => {
            if let Some(style) = &m.fields.style {
                put(y, 10.0, f_italic, &format!("Style: {style}"));
                y += 5.0;
            }
            put(y, 12.0, f_reg, &format!("ABV {:.1}%", m.fields.abv_percent));
            y += 5.0;
            if !m.brand.brewery_name.is_empty() {
                put(y, 9.0, f_reg, &m.brand.brewery_name);
                y += 5.0;
            }
            if m.options.show_tasting_notes {
                if let Some(t) = &m.fields.tasting {
                    if let Some(a) = &t.aroma {
                        put(y, 9.0, f_reg, &format!("Aroma: {a}"));
                        y += 5.0;
                    }
                    if let Some(fl) = &t.flavour {
                        put(y, 9.0, f_reg, &format!("Flavour: {fl}"));
                        y += 5.0;
                    }
                    if let Some(mf) = &t.mouthfeel {
                        put(y, 9.0, f_reg, &format!("Mouthfeel: {mf}"));
                        y += 5.0;
                    }
                    if let Some(fin) = &t.finish {
                        put(y, 9.0, f_reg, &format!("Finish: {fin}"));
                    }
                }
            }
        }
        _ => {}
    }

    let page = PdfPage::new(Mm(m.width_mm as f32), Mm(m.height_mm as f32), ops);
    doc.with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut Vec::new())
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

    fn contains(bytes: &[u8], needle: &str) -> bool {
        bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
    }

    fn has_image(bytes: &[u8]) -> bool {
        contains(bytes, "/Subtype/Image") || contains(bytes, "/Subtype /Image")
    }

    /// Width and height in points from the first `/MediaBox [0 0 w h]`.
    fn media_box(bytes: &[u8]) -> Option<(f64, f64)> {
        let s = String::from_utf8_lossy(bytes);
        let rest = &s[s.find("/MediaBox")? + "/MediaBox".len()..];
        let inner = &rest[rest.find('[')? + 1..rest.find(']')?];
        let nums: Vec<f64> = inner
            .split_whitespace()
            .filter_map(|n| n.parse().ok())
            .collect();
        match nums[..] {
            [_, _, w, h] => Some((w, h)),
            _ => None,
        }
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 40]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn page_is_the_model_size_in_points() {
        let bytes = render_pdf(&model("bottle"), &[]);
        let (w, h) = media_box(&bytes).expect("media box");
        // 90 × 120 mm at 72 pt per inch.
        assert!((w - 255.118).abs() < 0.01, "width {w}");
        assert!((h - 340.157).abs() < 0.01, "height {h}");
    }

    #[test]
    fn uses_the_brand_font_family() {
        let mut m = model("pump_clip");
        m.brand.font_family = "times".into();
        m.fields.style = Some("Bitter".into());
        let bytes = render_pdf(&m, &[]);
        for font in ["Times-Roman", "Times-Bold", "Times-Italic"] {
            assert!(contains(&bytes, font), "missing {font}");
        }
        assert!(!contains(&bytes, "Helvetica"));

        let mut m = model("bottle");
        m.brand.font_family = "courier".into();
        let bytes = render_pdf(&m, &[]);
        assert!(contains(&bytes, "Courier-Bold"), "missing Courier-Bold");
    }

    #[test]
    fn valid_logo_is_embedded() {
        let bytes = render_pdf(&model("bottle"), &png(40, 20));
        assert!(has_image(&bytes), "logo image missing");
        let bytes = render_pdf(&model("bottle"), &[]);
        assert!(!has_image(&bytes), "no logo expected");
    }

    #[test]
    fn transparent_logo_keeps_its_alpha() {
        let img = image::RgbaImage::from_pixel(40, 20, image::Rgba([200, 40, 40, 128]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        let bytes = render_pdf(&model("bottle"), &out.into_inner());
        assert!(has_image(&bytes), "logo image missing");
        assert!(
            contains(&bytes, "/SMask"),
            "alpha must be kept as a soft mask"
        );
    }

    #[test]
    fn oversized_logo_is_skipped() {
        let bytes = render_pdf(&model("bottle"), &png(4100, 1));
        assert!(bytes.starts_with(b"%PDF"));
        assert!(!has_image(&bytes), "a logo over 4096 px must be skipped");
    }

    #[test]
    fn renders_valid_pdf_bytes() {
        let bytes = render_pdf(&model("bottle"), &[]);
        assert!(bytes.starts_with(b"%PDF"), "should be a PDF");
        assert!(bytes.len() > 500);
    }

    #[test]
    fn renders_clip_kind() {
        let mut m = model("pump_clip");
        m.size_key = "pumpclip_round_114".into();
        let bytes = render_pdf(&m, &[]);
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn invalid_logo_is_skipped() {
        let bytes = render_pdf(&model("bottle"), b"not-an-image");
        assert!(bytes.starts_with(b"%PDF"));
    }
}
