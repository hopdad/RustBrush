use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
pub struct TextConfig {
    pub text: String,
    pub font_data: Vec<u8>,
    pub font_size: f32,
    pub text_color: [u8; 4],
    pub bg_color: [u8; 4],
    pub alignment: TextAlign,
    pub padding: u32,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_data: Vec::new(),
            font_size: 32.0,
            text_color: [255, 255, 255, 255],
            bg_color: [0, 0, 0, 255],
            alignment: TextAlign::Center,
            padding: 8,
        }
    }
}

/// Render text to an RgbaImage of the given canvas dimensions.
///
/// Supports multi-line text (split on `\n`), alignment, and auto font-size
/// scaling if text doesn't fit.
pub fn render_text(config: &TextConfig, width: u32, height: u32) -> Result<RgbaImage, String> {
    if config.font_data.is_empty() {
        return Err("No font data provided".to_string());
    }

    let font = FontArc::try_from_vec(config.font_data.clone())
        .map_err(|_| "Failed to parse font data".to_string())?;

    let mut img = RgbaImage::from_pixel(width, height, Rgba(config.bg_color));

    if config.text.is_empty() {
        return Ok(img);
    }

    let lines: Vec<&str> = config.text.lines().collect();
    let usable_w = width.saturating_sub(config.padding * 2) as f32;
    let usable_h = height.saturating_sub(config.padding * 2) as f32;

    // Find the best font size: start with requested size, scale down if needed
    let font_size = find_fitting_size(&font, &lines, config.font_size, usable_w, usable_h);
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);

    let line_height = scaled_font.height().ceil();
    let total_text_height = line_height * lines.len() as f32;

    // Vertical centering
    let y_start = config.padding as f32 + (usable_h - total_text_height).max(0.0) / 2.0;

    let text_color = Rgba(config.text_color);

    for (i, line) in lines.iter().enumerate() {
        let line_width = text_width(&scaled_font, line);
        let x = match config.alignment {
            TextAlign::Left => config.padding as f32,
            TextAlign::Center => config.padding as f32 + (usable_w - line_width).max(0.0) / 2.0,
            TextAlign::Right => config.padding as f32 + (usable_w - line_width).max(0.0),
        };
        let y = y_start + i as f32 * line_height;

        draw_text_mut(&mut img, text_color, x as i32, y as i32, scale, &font, line);
    }

    Ok(img)
}

/// Measure the width of a text string using the scaled font.
fn text_width(scaled_font: &ab_glyph::PxScaleFont<&FontArc>, text: &str) -> f32 {
    let mut width = 0.0f32;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
    for c in text.chars() {
        let glyph_id = scaled_font.glyph_id(c);
        if let Some(prev) = prev_glyph {
            width += scaled_font.kern(prev, glyph_id);
        }
        width += scaled_font.h_advance(glyph_id);
        prev_glyph = Some(glyph_id);
    }
    width
}

/// Find the largest font size (up to max_size) where all lines fit within
/// the given dimensions.
fn find_fitting_size(font: &FontArc, lines: &[&str], max_size: f32, max_w: f32, max_h: f32) -> f32 {
    let mut size = max_size;
    let min_size = 6.0;

    loop {
        let scale = PxScale::from(size);
        let scaled = font.as_scaled(scale);
        let line_h = scaled.height().ceil();
        let total_h = line_h * lines.len() as f32;

        if total_h <= max_h {
            let max_line_w = lines.iter().map(|l| text_width(&scaled, l)).fold(0.0f32, f32::max);
            if max_line_w <= max_w {
                return size;
            }
        }

        size *= 0.9;
        if size < min_size {
            return min_size;
        }
    }
}
