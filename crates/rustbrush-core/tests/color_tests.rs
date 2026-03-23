//! Integration tests for color matching.

use rustbrush_core::color::*;

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
fn test_palette_has_32_colors() {
    let palette = rust_palette();
    assert_eq!(palette.len(), 32);
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
    let img = image::RgbaImage::from_fn(2, 2, |x, _y| {
        if x == 0 {
            image::Rgba([255, 255, 255, 255])
        } else {
            image::Rgba([255, 0, 0, 255])
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
    let img = image::RgbaImage::from_fn(2, 1, |x, _| {
        if x == 0 {
            image::Rgba([255, 0, 0, 200])
        } else {
            image::Rgba([255, 0, 0, 50])
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
    let img = image::RgbaImage::from_fn(4, 4, |_, _| image::Rgba([255, 0, 0, 255]));
    let opts = QuantizeOptions::default();
    let pixels = map_image_to_palette(&img, &palette, &opts);
    let preview = build_preview(&img, &pixels);
    assert_eq!(preview.width(), 4);
    assert_eq!(preview.height(), 4);
    for p in preview.pixels() {
        assert_eq!(p.0[3], 255);
    }
}

#[test]
fn test_floyd_steinberg_dithering() {
    let palette = rust_palette();
    let img = image::RgbaImage::from_fn(4, 4, |_, _| image::Rgba([100, 100, 100, 255]));
    let opts = QuantizeOptions {
        dither: DitherMode::FloydSteinberg,
        ..Default::default()
    };
    let result = map_image_to_palette(&img, &palette, &opts);
    assert_eq!(result.len(), 16);
}

#[test]
fn test_ordered_dithering() {
    let palette = rust_palette();
    let img = image::RgbaImage::from_fn(4, 4, |_, _| image::Rgba([100, 100, 100, 255]));
    let opts = QuantizeOptions {
        dither: DitherMode::Ordered,
        ..Default::default()
    };
    let result = map_image_to_palette(&img, &palette, &opts);
    assert_eq!(result.len(), 16);
}
