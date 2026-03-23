//! Painting engine - orchestrates color selection and pixel placement.
//!
//! SAFETY APPROACH:
//! - Groups pixels by color to minimize color-picker interactions
//! - Uses only OS-level input simulation (see input.rs)
//! - Provides interruptible painting with progress reporting

use crate::color::MappedPixel;
use crate::input::SafeInput;
use std::collections::HashMap;
use std::time::Duration;

/// A group of pixels that share the same color.
#[derive(Debug)]
pub struct ColorGroup {
    pub color: (u8, u8, u8),
    pub hex: String,
    pub pixels: Vec<(u32, u32)>,
}

/// Group mapped pixels by color for efficient painting.
/// Painting color-by-color avoids constant color-picker switching.
pub fn group_by_color(pixels: &[MappedPixel], _width: u32, _height: u32) -> Vec<ColorGroup> {
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

/// Paint all color groups onto the canvas.
///
/// This assumes the Rust sign editor is open and focused, with the brush
/// tool selected. The painting coordinate system is relative to the
/// canvas top-left corner.
///
/// TODO: Canvas position detection needs to be calibrated per-resolution.
/// For now, users need to configure canvas_origin manually.
pub fn paint(groups: &[ColorGroup], delay: Duration) -> Result<(), String> {
    let mut input = SafeInput::new(delay)?;
    let total_pixels: usize = groups.iter().map(|g| g.pixels.len()).sum();
    let mut painted = 0usize;

    println!("Starting painting: {} colors, {} pixels", groups.len(), total_pixels);

    for (i, group) in groups.iter().enumerate() {
        println!(
            "[{}/{}] Color #{} ({} pixels)",
            i + 1,
            groups.len(),
            group.hex,
            group.pixels.len()
        );

        // Select this color via the hex input in Rust's color picker.
        // The new color picker (Nov 2025) supports hex code entry.
        select_color_by_hex(&mut input, &group.hex)?;

        // Paint each pixel in this color group
        for &(px, py) in &group.pixels {
            // Convert canvas pixel coordinates to screen coordinates.
            // TODO: Make canvas origin configurable via CLI or auto-detect.
            let screen_x = canvas_pixel_to_screen_x(px);
            let screen_y = canvas_pixel_to_screen_y(py);

            input.move_to(screen_x, screen_y)?;
            input.click()?;

            painted += 1;
            if painted % 500 == 0 {
                let pct = (painted as f64 / total_pixels as f64) * 100.0;
                println!("  Progress: {}/{} ({:.1}%)", painted, total_pixels, pct);
            }
        }
    }

    Ok(())
}

/// Select a color by typing its hex code into Rust's color picker.
fn select_color_by_hex(input: &mut SafeInput, hex: &str) -> Result<(), String> {
    // TODO: These coordinates need to be calibrated for the actual UI layout.
    // The hex input field position depends on screen resolution and UI scale.
    // For now, this is a placeholder that will need per-setup calibration.

    // Click the hex input field in the color picker
    // (placeholder coordinates - must be calibrated)
    input.move_to(960, 800)?;
    input.click()?;

    // Select all existing text and replace with new hex code
    input.press_key(enigo::Key::Control)?;
    input.type_text(hex)?;
    input.press_key(enigo::Key::Return)?;

    // Small extra delay after color selection for the UI to update
    std::thread::sleep(std::time::Duration::from_millis(50));

    Ok(())
}

// Placeholder coordinate conversion - these need calibration
// TODO: Auto-detect canvas position or make configurable
fn canvas_pixel_to_screen_x(px: u32) -> i32 {
    // Assumes canvas starts at screen position (400, 200) - placeholder
    400 + px as i32
}

fn canvas_pixel_to_screen_y(py: u32) -> i32 {
    200 + py as i32
}
