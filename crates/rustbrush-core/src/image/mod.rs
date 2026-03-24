//! Image loading, resizing, and adjustment utilities.

use image::RgbaImage;
use image::imageops::FilterType;
use image::AnimationDecoder;

/// Aspect ratio handling when resizing images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectRatio {
    /// Stretch to fill the exact dimensions.
    Stretch,
    /// Scale to fit within dimensions, preserving aspect ratio (may have empty space).
    Fit,
    /// Scale to fill dimensions, preserving aspect ratio (may crop).
    Fill,
}

/// Load an image from a file path and convert to RGBA.
pub fn load_image(path: &std::path::Path) -> Result<RgbaImage, String> {
    image::open(path)
        .map(|img| img.to_rgba8())
        .map_err(|e| format!("Failed to load image '{}': {}", path.display(), e))
}

/// Load all frames from a GIF file, returning them as RGBA images.
/// Caps at `max_frames` to prevent excessive memory usage.
pub fn load_gif_frames(path: &std::path::Path, max_frames: usize) -> Result<Vec<RgbaImage>, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open GIF '{}': {}", path.display(), e))?;
    let reader = std::io::BufReader::new(file);
    let decoder = image::codecs::gif::GifDecoder::new(reader)
        .map_err(|e| format!("Failed to decode GIF '{}': {}", path.display(), e))?;

    let frames: Vec<RgbaImage> = decoder
        .into_frames()
        .take(max_frames)
        .filter_map(|f| f.ok())
        .map(|frame| frame.into_buffer())
        .collect();

    if frames.is_empty() {
        return Err("GIF contains no frames".to_string());
    }

    Ok(frames)
}

/// Returns `true` if the file path has a GIF extension.
pub fn is_gif(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gif"))
        .unwrap_or(false)
}

/// Select `count` evenly-spaced frame indices from a total of `total` frames.
/// Returns indices distributed as evenly as possible across the range.
pub fn select_evenly_spaced(total: usize, count: usize) -> Vec<usize> {
    if total == 0 || count == 0 {
        return vec![];
    }
    let count = count.min(total);
    if count == 1 {
        return vec![0];
    }
    (0..count)
        .map(|i| i * (total - 1) / (count - 1))
        .collect()
}

/// Resize an image to the target dimensions.
pub fn resize(
    img: &RgbaImage,
    width: u32,
    height: u32,
    aspect: AspectRatio,
) -> RgbaImage {
    match aspect {
        AspectRatio::Stretch => {
            image::imageops::resize(img, width, height, FilterType::Lanczos3)
        }
        AspectRatio::Fit => {
            let (fw, fh) = fit_dimensions(img.width(), img.height(), width, height);
            image::imageops::resize(img, fw, fh, FilterType::Lanczos3)
        }
        AspectRatio::Fill => {
            let (fw, fh) = fill_dimensions(img.width(), img.height(), width, height);
            let resized = image::imageops::resize(img, fw, fh, FilterType::Lanczos3);
            // Center crop to target dimensions
            let x_offset = (fw.saturating_sub(width)) / 2;
            let y_offset = (fh.saturating_sub(height)) / 2;
            image::imageops::crop_imm(&resized, x_offset, y_offset, width, height).to_image()
        }
    }
}

/// Adjust brightness of an image. Factor: 0.0 = black, 1.0 = unchanged, 2.0 = double.
pub fn adjust_brightness(img: &RgbaImage, factor: f32) -> RgbaImage {
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        pixel.0[0] = (pixel.0[0] as f32 * factor).clamp(0.0, 255.0) as u8;
        pixel.0[1] = (pixel.0[1] as f32 * factor).clamp(0.0, 255.0) as u8;
        pixel.0[2] = (pixel.0[2] as f32 * factor).clamp(0.0, 255.0) as u8;
    }
    result
}

/// Adjust contrast of an image. Factor: 0.0 = gray, 1.0 = unchanged, 2.0 = high contrast.
pub fn adjust_contrast(img: &RgbaImage, factor: f32) -> RgbaImage {
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        pixel.0[0] = ((((pixel.0[0] as f32 / 255.0) - 0.5) * factor + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
        pixel.0[1] = ((((pixel.0[1] as f32 / 255.0) - 0.5) * factor + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
        pixel.0[2] = ((((pixel.0[2] as f32 / 255.0) - 0.5) * factor + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
    }
    result
}

/// Adjust saturation of an image. Factor: 0.0 = grayscale, 1.0 = unchanged, 2.0 = vivid.
pub fn adjust_saturation(img: &RgbaImage, factor: f32) -> RgbaImage {
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        let r = pixel.0[0] as f32;
        let g = pixel.0[1] as f32;
        let b = pixel.0[2] as f32;
        let gray = 0.299 * r + 0.587 * g + 0.114 * b;
        pixel.0[0] = (gray + (r - gray) * factor).clamp(0.0, 255.0) as u8;
        pixel.0[1] = (gray + (g - gray) * factor).clamp(0.0, 255.0) as u8;
        pixel.0[2] = (gray + (b - gray) * factor).clamp(0.0, 255.0) as u8;
    }
    result
}

/// Apply Gaussian blur to smooth the image. Higher sigma = more blur.
/// Sigma range: 0.5–5.0.
pub fn apply_gaussian_blur(img: &RgbaImage, sigma: f32) -> RgbaImage {
    image::imageops::blur(img, sigma)
}

/// Reduce each RGB channel to a fixed number of discrete levels.
/// Levels range: 2–32. Lower = more dramatic simplification.
pub fn posterize(img: &RgbaImage, levels: u8) -> RgbaImage {
    let levels = levels.max(2) as f32;
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        for c in 0..3 {
            let v = pixel.0[c] as f32 / 255.0;
            let quantized = (v * (levels - 1.0)).round() / (levels - 1.0);
            pixel.0[c] = (quantized * 255.0).clamp(0.0, 255.0) as u8;
        }
        // Alpha unchanged
    }
    result
}

/// Median filter: replaces each pixel with the median of its neighborhood.
/// Radius 1 = 3x3 window, radius 2 = 5x5, radius 3 = 7x7.
/// Preserves edges better than Gaussian blur.
pub fn median_filter(img: &RgbaImage, radius: u32) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut result = img.clone();
    let r = radius as i32;
    let window_size = ((2 * r + 1) * (2 * r + 1)) as usize;
    let mut buf_r = Vec::with_capacity(window_size);
    let mut buf_g = Vec::with_capacity(window_size);
    let mut buf_b = Vec::with_capacity(window_size);

    for y in 0..h {
        for x in 0..w {
            buf_r.clear();
            buf_g.clear();
            buf_b.clear();

            for dy in -r..=r {
                for dx in -r..=r {
                    let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    let p = img.get_pixel(nx, ny).0;
                    buf_r.push(p[0]);
                    buf_g.push(p[1]);
                    buf_b.push(p[2]);
                }
            }

            buf_r.sort_unstable();
            buf_g.sort_unstable();
            buf_b.sort_unstable();

            let mid = buf_r.len() / 2;
            let pixel = result.get_pixel_mut(x, y);
            pixel.0[0] = buf_r[mid];
            pixel.0[1] = buf_g[mid];
            pixel.0[2] = buf_b[mid];
            // Alpha unchanged
        }
    }
    result
}

/// Rotate image 90 degrees clockwise.
pub fn rotate_90(img: &RgbaImage) -> RgbaImage {
    image::imageops::rotate90(img)
}

/// Rotate image 180 degrees.
pub fn rotate_180(img: &RgbaImage) -> RgbaImage {
    image::imageops::rotate180(img)
}

/// Rotate image 90 degrees counter-clockwise.
pub fn rotate_270(img: &RgbaImage) -> RgbaImage {
    image::imageops::rotate270(img)
}

/// Flip image horizontally (mirror).
pub fn flip_horizontal(img: &RgbaImage) -> RgbaImage {
    image::imageops::flip_horizontal(img)
}

/// Flip image vertically.
pub fn flip_vertical(img: &RgbaImage) -> RgbaImage {
    image::imageops::flip_vertical(img)
}

/// Crop image by percentage margins (0.0–50.0 for each side).
pub fn crop_margins(img: &RgbaImage, top: f32, bottom: f32, left: f32, right: f32) -> RgbaImage {
    let w = img.width() as f32;
    let h = img.height() as f32;
    let x = (w * left / 100.0).round() as u32;
    let y = (h * top / 100.0).round() as u32;
    let x2 = (w * (1.0 - right / 100.0)).round() as u32;
    let y2 = (h * (1.0 - bottom / 100.0)).round() as u32;
    let cw = x2.saturating_sub(x).max(1);
    let ch = y2.saturating_sub(y).max(1);
    image::imageops::crop_imm(img, x, y, cw, ch).to_image()
}

fn fit_dimensions(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let ratio_w = max_w as f64 / src_w as f64;
    let ratio_h = max_h as f64 / src_h as f64;
    let ratio = ratio_w.min(ratio_h);
    ((src_w as f64 * ratio) as u32, (src_h as f64 * ratio) as u32)
}

fn fill_dimensions(src_w: u32, src_h: u32, min_w: u32, min_h: u32) -> (u32, u32) {
    let ratio_w = min_w as f64 / src_w as f64;
    let ratio_h = min_h as f64 / src_h as f64;
    let ratio = ratio_w.max(ratio_h);
    ((src_w as f64 * ratio).ceil() as u32, (src_h as f64 * ratio).ceil() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit_dimensions() {
        let (w, h) = fit_dimensions(200, 100, 100, 100);
        assert_eq!((w, h), (100, 50));
    }

    #[test]
    fn test_fill_dimensions() {
        let (w, h) = fill_dimensions(200, 100, 100, 100);
        assert_eq!((w, h), (200, 100));
    }

    #[test]
    fn test_resize_stretch() {
        let img = RgbaImage::new(100, 50);
        let resized = resize(&img, 200, 200, AspectRatio::Stretch);
        assert_eq!(resized.width(), 200);
        assert_eq!(resized.height(), 200);
    }

    #[test]
    fn test_brightness_zero() {
        let img = RgbaImage::from_fn(2, 2, |_, _| image::Rgba([128, 128, 128, 255]));
        let dark = adjust_brightness(&img, 0.0);
        for p in dark.pixels() {
            assert_eq!(p.0[0], 0);
        }
    }

    #[test]
    fn test_saturation_zero_is_grayscale() {
        let img = RgbaImage::from_fn(1, 1, |_, _| image::Rgba([255, 0, 0, 255]));
        let gray = adjust_saturation(&img, 0.0);
        let p = gray.get_pixel(0, 0).0;
        // All channels should be equal (grayscale)
        assert_eq!(p[0], p[1]);
        assert_eq!(p[1], p[2]);
    }

    #[test]
    fn test_select_evenly_spaced_basic() {
        assert_eq!(select_evenly_spaced(30, 5), vec![0, 7, 14, 21, 29]);
        assert_eq!(select_evenly_spaced(5, 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(select_evenly_spaced(10, 3), vec![0, 4, 9]);
        assert_eq!(select_evenly_spaced(1, 5), vec![0]);
        assert_eq!(select_evenly_spaced(2, 5), vec![0, 1]);
    }

    #[test]
    fn test_select_evenly_spaced_edge_cases() {
        assert_eq!(select_evenly_spaced(0, 5), Vec::<usize>::new());
        assert_eq!(select_evenly_spaced(10, 0), Vec::<usize>::new());
        assert_eq!(select_evenly_spaced(10, 1), vec![0]);
    }

    #[test]
    fn test_is_gif() {
        assert!(is_gif(std::path::Path::new("test.gif")));
        assert!(is_gif(std::path::Path::new("test.GIF")));
        assert!(!is_gif(std::path::Path::new("test.png")));
        assert!(!is_gif(std::path::Path::new("test")));
    }

    #[test]
    fn test_gaussian_blur_preserves_dimensions() {
        let img = RgbaImage::from_fn(10, 10, |_, _| image::Rgba([128, 64, 32, 255]));
        let blurred = apply_gaussian_blur(&img, 1.0);
        assert_eq!(blurred.width(), 10);
        assert_eq!(blurred.height(), 10);
    }

    #[test]
    fn test_posterize_two_levels() {
        // With 2 levels, each channel should be either 0 or 255
        let img = RgbaImage::from_fn(2, 2, |x, _| {
            if x == 0 {
                image::Rgba([64, 200, 128, 255])
            } else {
                image::Rgba([192, 30, 250, 255])
            }
        });
        let result = posterize(&img, 2);
        for p in result.pixels() {
            for c in 0..3 {
                assert!(p.0[c] == 0 || p.0[c] == 255, "channel {} was {}", c, p.0[c]);
            }
            assert_eq!(p.0[3], 255, "alpha should be unchanged");
        }
    }

    #[test]
    fn test_median_filter_uniform_image() {
        let img = RgbaImage::from_fn(5, 5, |_, _| image::Rgba([100, 150, 200, 255]));
        let result = median_filter(&img, 1);
        for p in result.pixels() {
            assert_eq!(p.0, [100, 150, 200, 255]);
        }
    }
}
