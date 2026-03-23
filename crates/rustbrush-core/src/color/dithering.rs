//! Dithering algorithms for image quantization.

use image::RgbaImage;
use super::{MappedPixel, PaletteColor, QuantizeOptions, should_skip};
use super::matching::{ColorMatchAlgo, LabPalette, find_nearest_color_rgb};

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

/// Direct quantization without dithering.
pub fn quantize_direct(
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
pub fn quantize_floyd_steinberg(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    let w = img.width() as usize;
    let h = img.height() as usize;

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

            let err_r = old_r - nearest.r as f32;
            let err_g = old_g - nearest.g as f32;
            let err_b = old_b - nearest.b as f32;

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
pub fn quantize_ordered(
    img: &RgbaImage,
    palette: &[PaletteColor],
    opts: &QuantizeOptions,
) -> Vec<MappedPixel> {
    const BAYER4: [[f32; 4]; 4] = [
        [0.0 / 16.0 - 0.5, 8.0 / 16.0 - 0.5, 2.0 / 16.0 - 0.5, 10.0 / 16.0 - 0.5],
        [12.0 / 16.0 - 0.5, 4.0 / 16.0 - 0.5, 14.0 / 16.0 - 0.5, 6.0 / 16.0 - 0.5],
        [3.0 / 16.0 - 0.5, 11.0 / 16.0 - 0.5, 1.0 / 16.0 - 0.5, 9.0 / 16.0 - 0.5],
        [15.0 / 16.0 - 0.5, 7.0 / 16.0 - 0.5, 13.0 / 16.0 - 0.5, 5.0 / 16.0 - 0.5],
    ];
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
