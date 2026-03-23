//! Image loading, resizing, and adjustment utilities.

use image::RgbaImage;
use image::imageops::FilterType;

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
}
