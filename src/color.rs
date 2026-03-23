//! Color matching and image quantization for Rust's in-game palette.
//!
//! Supports both RGB Euclidean distance and CIEDE2000 perceptual color matching,
//! with optional Floyd-Steinberg or ordered dithering.

use image::RgbaImage;
use palette::{color_difference::Ciede2000, FromColor, Lab, Srgb};

/// Color matching algorithm to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMatchAlgo {
    /// Fast squared Euclidean distance in RGB space.
    Rgb,
    /// Perceptual CIEDE2000 distance in CIE L*a*b* space.
    Ciede2000,
}

/// Dithering algorithm to apply during quantization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMode {
    /// No dithering - direct nearest-color mapping.
    None,
    /// Floyd-Steinberg error-diffusion dithering.
    FloydSteinberg,
    /// 4x4 Bayer ordered dithering.
    Ordered,
}

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

/// Options controlling how image pixels are mapped to the palette.
pub struct QuantizeOptions {
    pub algorithm: ColorMatchAlgo,
    pub dither: DitherMode,
    pub alpha_threshold: u8,
    /// If set, pixels matching this color (within tolerance) are skipped.
    pub skip_color: Option<(u8, u8, u8)>,
    /// Tolerance for background color skip (Euclidean distance squared).
    pub skip_tolerance: u32,
}

impl Default for QuantizeOptions {
    fn default() -> Self {
        Self {
            algorithm: ColorMatchAlgo::Ciede2000,
            dither: DitherMode::None,
            alpha_threshold: 128,
            skip_color: None,
            skip_tolerance: 30 * 30 + 30 * 30 + 30 * 30, // ~30 per channel
        }
    }
}

/// Returns Rust's in-game sign editor palette (post-November 2025 update).
pub fn rust_palette() -> Vec<PaletteColor> {
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
            hex: Box::leak(hex.to_string().into_boxed_str()),
        })
        .collect()
}

/// Pre-computed Lab values for a palette, used with CIEDE2000.
struct LabPalette {
    entries: Vec<(PaletteColor, Lab)>,
}

impl LabPalette {
    fn new(palette: &[PaletteColor]) -> Self {
        let entries = palette
            .iter()
            .map(|c| {
                let srgb = Srgb::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
                let lab: Lab = Lab::from_color(srgb.into_linear());
                (*c, lab)
            })
            .collect();
        Self { entries }
    }

    fn find_nearest(&self, r: f32, g: f32, b: f32) -> PaletteColor {
        let srgb = Srgb::new(r, g, b);
        let lab: Lab = Lab::from_color(srgb.into_linear());
        self.entries
            .iter()
            .min_by(|(_, a), (_, b)| {
                let da = a.difference(lab);
                let db = b.difference(lab);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(c, _)| *c)
            .unwrap()
    }
}

/// Map each pixel in the image to the nearest palette color.
pub fn map_image_to_palette(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    match opts.dither {
        DitherMode::None => quantize_direct(img, palette, opts),
        DitherMode::FloydSteinberg => quantize_floyd_steinberg(img, palette, opts),
        DitherMode::Ordered => quantize_ordered(img, palette, opts),
    }
}

/// Build a preview image showing the quantized result.
pub fn build_preview(img: &RgbaImage, pixels: &[MappedPixel]) -> RgbaImage {
    let mut preview = RgbaImage::new(img.width(), img.height());
    // Fill with transparent
    for p in preview.pixels_mut() {
        *p = image::Rgba([0, 0, 0, 0]);
    }
    for mp in pixels {
        preview.put_pixel(mp.x, mp.y, image::Rgba([mp.color.r, mp.color.g, mp.color.b, 255]));
    }
    preview
}

fn should_skip(r: u8, g: u8, b: u8, opts: &QuantizeOptions) -> bool {
    if let Some((sr, sg, sb)) = opts.skip_color {
        let dr = r as i32 - sr as i32;
        let dg = g as i32 - sg as i32;
        let db = b as i32 - sb as i32;
        (dr * dr + dg * dg + db * db) as u32 <= opts.skip_tolerance
    } else {
        false
    }
}

/// Direct quantization without dithering.
fn quantize_direct(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    let lab_palette = if opts.algorithm == ColorMatchAlgo::Ciede2000 {
        Some(LabPalette::new(palette))
    } else {
        None
    };

    let mut result = Vec::with_capacity((img.width() * img.height()) as usize);
    for y in 0..img.height() {
        for x in 0..img.width() {
            let [r, g, b, a] = img.get_pixel(x, y).0;
            if a < opts.alpha_threshold {
                continue;
            }
            if should_skip(r, g, b, opts) {
                continue;
            }
            let nearest = match &lab_palette {
                Some(lp) => lp.find_nearest(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
                None => find_nearest_color_rgb(r, g, b, palette),
            };
            result.push(MappedPixel { x, y, color: nearest, alpha: a });
        }
    }
    result
}

/// Floyd-Steinberg error-diffusion dithering.
fn quantize_floyd_steinberg(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    let w = img.width() as usize;
    let h = img.height() as usize;

    // Work in f32 to accumulate error
    let mut buf_r = vec![0.0f32; w * h];
    let mut buf_g = vec![0.0f32; w * h];
    let mut buf_b = vec![0.0f32; w * h];
    let mut alphas = vec![0u8; w * h];

    for y in 0..h {
        for x in 0..w {
            let [r, g, b, a] = img.get_pixel(x as u32, y as u32).0;
            let idx = y * w + x;
            buf_r[idx] = r as f32;
            buf_g[idx] = g as f32;
            buf_b[idx] = b as f32;
            alphas[idx] = a;
        }
    }

    let lab_palette = if opts.algorithm == ColorMatchAlgo::Ciede2000 {
        Some(LabPalette::new(palette))
    } else {
        None
    };

    let mut result = Vec::with_capacity(w * h);

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let a = alphas[idx];
            if a < opts.alpha_threshold {
                continue;
            }

            let old_r = buf_r[idx].clamp(0.0, 255.0);
            let old_g = buf_g[idx].clamp(0.0, 255.0);
            let old_b = buf_b[idx].clamp(0.0, 255.0);

            let r8 = old_r.round() as u8;
            let g8 = old_g.round() as u8;
            let b8 = old_b.round() as u8;

            if should_skip(r8, g8, b8, opts) {
                continue;
            }

            let nearest = match &lab_palette {
                Some(lp) => lp.find_nearest(old_r / 255.0, old_g / 255.0, old_b / 255.0),
                None => find_nearest_color_rgb(r8, g8, b8, palette),
            };

            result.push(MappedPixel {
                x: x as u32,
                y: y as u32,
                color: nearest,
                alpha: a,
            });

            // Compute quantization error
            let err_r = old_r - nearest.r as f32;
            let err_g = old_g - nearest.g as f32;
            let err_b = old_b - nearest.b as f32;

            // Distribute error to neighbors (Floyd-Steinberg coefficients)
            let neighbors: [(isize, isize, f32); 4] = [
                (1, 0, 7.0 / 16.0),
                (-1, 1, 3.0 / 16.0),
                (0, 1, 5.0 / 16.0),
                (1, 1, 1.0 / 16.0),
            ];

            for (dx, dy, weight) in &neighbors {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx >= 0 && nx < w as isize && ny >= 0 && ny < h as isize {
                    let ni = ny as usize * w + nx as usize;
                    buf_r[ni] += err_r * weight;
                    buf_g[ni] += err_g * weight;
                    buf_b[ni] += err_b * weight;
                }
            }
        }
    }
    result
}

/// 4x4 Bayer ordered dithering.
fn quantize_ordered(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    // 4x4 Bayer matrix normalized to [-0.5, 0.5] range, scaled by strength
    const BAYER4: [[f32; 4]; 4] = [
        [0.0 / 16.0 - 0.5, 8.0 / 16.0 - 0.5, 2.0 / 16.0 - 0.5, 10.0 / 16.0 - 0.5],
        [12.0 / 16.0 - 0.5, 4.0 / 16.0 - 0.5, 14.0 / 16.0 - 0.5, 6.0 / 16.0 - 0.5],
        [3.0 / 16.0 - 0.5, 11.0 / 16.0 - 0.5, 1.0 / 16.0 - 0.5, 9.0 / 16.0 - 0.5],
        [15.0 / 16.0 - 0.5, 7.0 / 16.0 - 0.5, 13.0 / 16.0 - 0.5, 5.0 / 16.0 - 0.5],
    ];
    // Dither strength - how many color levels to spread across
    let strength = 48.0f32;

    let lab_palette = if opts.algorithm == ColorMatchAlgo::Ciede2000 {
        Some(LabPalette::new(palette))
    } else {
        None
    };

    let mut result = Vec::with_capacity((img.width() * img.height()) as usize);

    for y in 0..img.height() {
        for x in 0..img.width() {
            let [r, g, b, a] = img.get_pixel(x, y).0;
            if a < opts.alpha_threshold {
                continue;
            }
            if should_skip(r, g, b, opts) {
                continue;
            }

            let threshold = BAYER4[(y % 4) as usize][(x % 4) as usize] * strength;
            let dr = (r as f32 + threshold).clamp(0.0, 255.0);
            let dg = (g as f32 + threshold).clamp(0.0, 255.0);
            let db = (b as f32 + threshold).clamp(0.0, 255.0);

            let nearest = match &lab_palette {
                Some(lp) => lp.find_nearest(dr / 255.0, dg / 255.0, db / 255.0),
                None => find_nearest_color_rgb(dr.round() as u8, dg.round() as u8, db.round() as u8, palette),
            };

            result.push(MappedPixel { x, y, color: nearest, alpha: a });
        }
    }
    result
}

/// Find nearest palette color using squared Euclidean distance in RGB.
fn find_nearest_color_rgb(r: u8, g: u8, b: u8, palette: &[PaletteColor]) -> PaletteColor {
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
                break;
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
        let black = find_nearest_color_rgb(0, 0, 0, &palette);
        assert_eq!((black.r, black.g, black.b), (0, 0, 0));

        let white = find_nearest_color_rgb(255, 255, 255, &palette);
        assert_eq!((white.r, white.g, white.b), (255, 255, 255));
    }

    #[test]
    fn test_near_color_match() {
        let palette = rust_palette();
        let color = find_nearest_color_rgb(250, 5, 5, &palette);
        assert_eq!((color.r, color.g, color.b), (255, 0, 0));
    }

    #[test]
    fn test_palette_not_empty() {
        let palette = rust_palette();
        assert!(palette.len() >= 20, "Palette should have at least 20 colors");
    }

    #[test]
    fn test_ciede2000_matching() {
        let palette = rust_palette();
        let lab = LabPalette::new(&palette);
        // Black should match black
        let c = lab.find_nearest(0.0, 0.0, 0.0);
        assert_eq!((c.r, c.g, c.b), (0, 0, 0));
        // White should match white
        let c = lab.find_nearest(1.0, 1.0, 1.0);
        assert_eq!((c.r, c.g, c.b), (255, 255, 255));
    }

    #[test]
    fn test_skip_color() {
        let palette = rust_palette();
        let img = RgbaImage::from_fn(2, 2, |x, _y| {
            if x == 0 {
                image::Rgba([255, 255, 255, 255]) // white - should be skipped
            } else {
                image::Rgba([255, 0, 0, 255]) // red
            }
        });
        let opts = QuantizeOptions {
            skip_color: Some((255, 255, 255)),
            ..Default::default()
        };
        let result = map_image_to_palette(&img, &palette, &opts);
        assert_eq!(result.len(), 2); // only the red pixels
    }

    #[test]
    fn test_alpha_threshold() {
        let palette = rust_palette();
        let img = RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                image::Rgba([255, 0, 0, 200]) // above threshold
            } else {
                image::Rgba([255, 0, 0, 50]) // below threshold
            }
        });
        let opts = QuantizeOptions {
            alpha_threshold: 100,
            ..Default::default()
        };
        let result = map_image_to_palette(&img, &palette, &opts);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_build_preview() {
        let palette = rust_palette();
        let img = RgbaImage::from_fn(4, 4, |_, _| image::Rgba([255, 0, 0, 255]));
        let opts = QuantizeOptions::default();
        let pixels = map_image_to_palette(&img, &palette, &opts);
        let preview = build_preview(&img, &pixels);
        assert_eq!(preview.width(), 4);
        assert_eq!(preview.height(), 4);
        // All pixels should be opaque
        for p in preview.pixels() {
            assert_eq!(p.0[3], 255);
        }
    }
}
