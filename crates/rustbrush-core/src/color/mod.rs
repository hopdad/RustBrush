//! Color matching and image quantization for Rust's in-game palette.
//!
//! Supports both RGB Euclidean distance and CIEDE2000 perceptual color matching,
//! with optional Floyd-Steinberg or ordered dithering.

mod palette;
mod matching;
mod dithering;
mod adaptive;

pub use palette::{PaletteColor, rust_palette};
pub use matching::{ColorMatchAlgo, find_nearest_color_rgb, LabPalette};
pub use dithering::DitherMode;
pub use adaptive::generate_adaptive_palette;

use image::RgbaImage;

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
            skip_tolerance: 30 * 30 + 30 * 30 + 30 * 30,
        }
    }
}

/// Map each pixel in the image to the nearest palette color.
pub fn map_image_to_palette(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    match opts.dither {
        DitherMode::None => dithering::quantize_direct(img, palette, opts),
        DitherMode::FloydSteinberg => dithering::quantize_floyd_steinberg(img, palette, opts),
        DitherMode::Ordered => dithering::quantize_ordered(img, palette, opts),
    }
}

/// Build a preview image showing the quantized result.
pub fn build_preview(img: &RgbaImage, pixels: &[MappedPixel]) -> RgbaImage {
    let mut preview = RgbaImage::new(img.width(), img.height());
    for p in preview.pixels_mut() {
        *p = image::Rgba([0, 0, 0, 0]);
    }
    for mp in pixels {
        preview.put_pixel(mp.x, mp.y, image::Rgba([mp.color.r, mp.color.g, mp.color.b, 255]));
    }
    preview
}

/// Create PaletteColors from scanned RGB values (e.g. from screen palette capture).
pub fn palette_from_rgb(colors: &[(u8, u8, u8)]) -> Vec<PaletteColor> {
    colors
        .iter()
        .map(|&(r, g, b)| {
            let hex = format!("{:02X}{:02X}{:02X}", r, g, b);
            PaletteColor {
                r,
                g,
                b,
                hex: Box::leak(hex.into_boxed_str()),
            }
        })
        .collect()
}

pub(crate) fn should_skip(r: u8, g: u8, b: u8, opts: &QuantizeOptions) -> bool {
    if let Some((sr, sg, sb)) = opts.skip_color {
        let dr = r as i32 - sr as i32;
        let dg = g as i32 - sg as i32;
        let db = b as i32 - sb as i32;
        (dr * dr + dg * dg + db * db) as u32 <= opts.skip_tolerance
    } else {
        false
    }
}
