//! Paint planning and command generation.
//!
//! Converts a grid of mapped pixels into an ordered sequence of paint commands
//! that can be executed by the platform layer's input driver.

mod strategy;

pub use strategy::{PaintStrategy, ScanlineStrategy, ColorGroupedStrategy, LineDrawStrategy, HybridStrategy};

use crate::color::MappedPixel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The atomic unit of painting work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaintCommand {
    /// Move mouse to absolute screen coordinates.
    MoveTo { x: i32, y: i32 },
    /// Click the left mouse button.
    Click,
    /// Shift-click for line drawing.
    ShiftClick,
    /// Select a color by typing its hex code.
    SelectColorByHex { hex: String },
    /// Select a color by clicking a palette position.
    SelectColorByClick { x: i32, y: i32 },
    /// Wait for a specified duration.
    Delay { ms: u32 },
}

/// Metadata about a paint plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub total_pixels: usize,
    pub total_colors: usize,
    pub total_commands: usize,
    pub strategy_name: String,
}

/// A complete, executable painting plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaintPlan {
    pub commands: Vec<PaintCommand>,
    pub metadata: PlanMetadata,
}

/// A group of pixels that share the same color.
#[derive(Debug, Clone)]
pub struct ColorGroup {
    pub color: (u8, u8, u8),
    pub hex: String,
    pub pixels: Vec<(u32, u32)>,
}

/// Group mapped pixels by color for efficient painting.
/// Sorts groups by pixel count descending (most common colors first).
pub fn group_by_color(pixels: &[MappedPixel]) -> Vec<ColorGroup> {
    let mut groups: HashMap<(u8, u8, u8), (String, Vec<(u32, u32)>)> = HashMap::new();

    for pixel in pixels {
        let key = (pixel.color.r, pixel.color.g, pixel.color.b);
        groups
            .entry(key)
            .or_insert_with(|| (pixel.color.hex.to_string(), Vec::new()))
            .1
            .push((pixel.x, pixel.y));
    }

    let mut result: Vec<ColorGroup> = groups
        .into_iter()
        .map(|(color, (hex, pixels))| ColorGroup { color, hex, pixels })
        .collect();

    // Sort by pixel count descending - paint most common colors first
    result.sort_by(|a, b| b.pixels.len().cmp(&a.pixels.len()));
    result
}

/// Screen region definition for coordinate mapping.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Convert an image pixel coordinate to screen coordinates within a canvas region.
pub fn pixel_to_screen(
    px: u32,
    py: u32,
    img_width: u32,
    img_height: u32,
    canvas: &ScreenRect,
) -> (i32, i32) {
    let sx = canvas.x + (px as f64 / img_width as f64 * canvas.width as f64) as i32;
    let sy = canvas.y + (py as f64 / img_height as f64 * canvas.height as f64) as i32;
    (sx, sy)
}

/// Estimate painting time in seconds based on command count and delay.
pub fn estimate_time(plan: &PaintPlan, delay_ms: u64) -> f64 {
    plan.commands.len() as f64 * delay_ms as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::PaletteColor;

    #[test]
    fn test_group_by_color() {
        let pixels = vec![
            MappedPixel { x: 0, y: 0, color: PaletteColor::new(255, 0, 0, "FF0000"), alpha: 255 },
            MappedPixel { x: 1, y: 0, color: PaletteColor::new(0, 0, 255, "0000FF"), alpha: 255 },
            MappedPixel { x: 2, y: 0, color: PaletteColor::new(255, 0, 0, "FF0000"), alpha: 255 },
        ];
        let groups = group_by_color(&pixels);
        assert_eq!(groups.len(), 2);
        // Red group should be first (2 pixels > 1 pixel)
        assert_eq!(groups[0].pixels.len(), 2);
        assert_eq!(groups[1].pixels.len(), 1);
    }

    #[test]
    fn test_pixel_to_screen() {
        let canvas = ScreenRect { x: 100, y: 200, width: 400, height: 300 };
        let (sx, sy) = pixel_to_screen(0, 0, 100, 100, &canvas);
        assert_eq!((sx, sy), (100, 200));

        let (sx, sy) = pixel_to_screen(50, 50, 100, 100, &canvas);
        assert_eq!((sx, sy), (300, 350));
    }

    #[test]
    fn test_empty_pixels_group() {
        let groups = group_by_color(&[]);
        assert!(groups.is_empty());
    }
}
