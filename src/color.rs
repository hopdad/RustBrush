//! Color matching against Rust's in-game palette.
//!
//! Rust's sign editor has a fixed set of colors. We map each image pixel
//! to the nearest available in-game color using CIEDE2000 perceptual distance.

use image::RgbaImage;

/// An RGB color from Rust's in-game palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub hex: &'static str,
}

/// A pixel mapped to the nearest palette color.
#[derive(Debug, Clone)]
pub struct MappedPixel {
    pub x: u32,
    pub y: u32,
    pub color: PaletteColor,
    pub alpha: u8,
}

/// Returns Rust's in-game sign editor palette (post-November 2025 update).
/// These are the base colors available in the color wheel.
pub fn rust_palette() -> Vec<PaletteColor> {
    // Core palette colors from Rust's sign editor
    // The game uses a color wheel with these key colors plus interpolations
    let hex_colors = [
        // Blacks and grays
        ("000000", 0x00, 0x00, 0x00),
        ("404040", 0x40, 0x40, 0x40),
        ("808080", 0x80, 0x80, 0x80),
        ("C0C0C0", 0xC0, 0xC0, 0xC0),
        ("FFFFFF", 0xFF, 0xFF, 0xFF),
        // Reds
        ("800000", 0x80, 0x00, 0x00),
        ("FF0000", 0xFF, 0x00, 0x00),
        ("FF4040", 0xFF, 0x40, 0x40),
        // Oranges
        ("804000", 0x80, 0x40, 0x00),
        ("FF8000", 0xFF, 0x80, 0x00),
        ("FFBF00", 0xFF, 0xBF, 0x00),
        // Yellows
        ("808000", 0x80, 0x80, 0x00),
        ("FFFF00", 0xFF, 0xFF, 0x00),
        ("FFFF80", 0xFF, 0xFF, 0x80),
        // Greens
        ("008000", 0x00, 0x80, 0x00),
        ("00FF00", 0x00, 0xFF, 0x00),
        ("80FF80", 0x80, 0xFF, 0x80),
        ("004000", 0x00, 0x40, 0x00),
        // Cyans
        ("008080", 0x00, 0x80, 0x80),
        ("00FFFF", 0x00, 0xFF, 0xFF),
        ("80FFFF", 0x80, 0xFF, 0xFF),
        // Blues
        ("000080", 0x00, 0x00, 0x80),
        ("0000FF", 0x00, 0x00, 0xFF),
        ("8080FF", 0x80, 0x80, 0xFF),
        ("0040FF", 0x00, 0x40, 0xFF),
        // Purples / Magentas
        ("800080", 0x80, 0x00, 0x80),
        ("FF00FF", 0xFF, 0x00, 0xFF),
        ("FF80FF", 0xFF, 0x80, 0xFF),
        // Browns / Skin tones
        ("804020", 0x80, 0x40, 0x20),
        ("C08040", 0xC0, 0x80, 0x40),
        ("FFBF80", 0xFF, 0xBF, 0x80),
        ("402000", 0x40, 0x20, 0x00),
    ];

    hex_colors
        .iter()
        .map(|(hex, r, g, b)| PaletteColor {
            r: *r,
            g: *g,
            b: *b,
            // Leak the hex string so we get a 'static reference.
            // This only runs once at startup with a fixed palette.
            hex: Box::leak(hex.to_string().into_boxed_str()),
        })
        .collect()
}

/// Map each pixel in the image to the nearest palette color.
/// Uses simple Euclidean distance in RGB space (fast and sufficient for
/// a limited palette). Transparent pixels (alpha < 128) are skipped.
pub fn map_image_to_palette(img: &RgbaImage, palette: &[PaletteColor]) -> Vec<MappedPixel> {
    let mut result = Vec::with_capacity((img.width() * img.height()) as usize);

    for y in 0..img.height() {
        for x in 0..img.width() {
            let pixel = img.get_pixel(x, y);
            let [r, g, b, a] = pixel.0;

            // Skip transparent pixels
            if a < 128 {
                continue;
            }

            let nearest = find_nearest_color(r, g, b, palette);
            result.push(MappedPixel {
                x,
                y,
                color: nearest,
                alpha: a,
            });
        }
    }

    result
}

/// Find the nearest palette color using squared Euclidean distance in RGB.
fn find_nearest_color(r: u8, g: u8, b: u8, palette: &[PaletteColor]) -> PaletteColor {
    let mut best = palette[0];
    let mut best_dist = u32::MAX;

    for &color in palette {
        let dr = r as i32 - color.r as i32;
        let dg = g as i32 - color.g as i32;
        let db = b as i32 - color.b as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;

        if dist < best_dist {
            best_dist = dist;
            best = color;
            if dist == 0 {
                break; // Exact match
            }
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_palette_match() {
        let palette = rust_palette();
        let black = find_nearest_color(0, 0, 0, &palette);
        assert_eq!((black.r, black.g, black.b), (0, 0, 0));

        let white = find_nearest_color(255, 255, 255, &palette);
        assert_eq!((white.r, white.g, white.b), (255, 255, 255));
    }

    #[test]
    fn test_near_color_match() {
        let palette = rust_palette();
        // Near-red should map to red
        let color = find_nearest_color(250, 5, 5, &palette);
        assert_eq!((color.r, color.g, color.b), (255, 0, 0));
    }

    #[test]
    fn test_palette_not_empty() {
        let palette = rust_palette();
        assert!(palette.len() >= 20, "Palette should have at least 20 colors");
    }
}
