//! Painting engine - orchestrates color selection and pixel placement.
//!
//! SAFETY APPROACH:
//! - Groups pixels by color to minimize color-picker interactions
//! - Uses only OS-level input simulation (see input.rs)
//! - Interruptible via hotkeys (F10 pause, ESC cancel)
//! - All coordinates come from user-captured screen regions

use crate::color::MappedPixel;
use crate::hotkeys::PaintControl;
use crate::input::SafeInput;
use crate::region::{CapturedLayout, PaletteEntry, ScreenRegion};
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

/// Convert an image pixel coordinate to a screen coordinate within the canvas.
fn pixel_to_screen(
    px: u32,
    py: u32,
    img_width: u32,
    img_height: u32,
    canvas: &ScreenRegion,
) -> (i32, i32) {
    let sx = canvas.x + (px as f64 / img_width as f64 * canvas.width as f64) as i32;
    let sy = canvas.y + (py as f64 / img_height as f64 * canvas.height as f64) as i32;
    (sx, sy)
}

/// Find the palette entry closest to the target color.
fn find_nearest_palette_entry(r: u8, g: u8, b: u8, palette: &[PaletteEntry]) -> &PaletteEntry {
    palette
        .iter()
        .min_by_key(|e| {
            let dr = r as i32 - e.r as i32;
            let dg = g as i32 - e.g as i32;
            let db = b as i32 - e.b as i32;
            (dr * dr + dg * dg + db * db) as u32
        })
        .unwrap()
}

/// Select a color by typing its hex code into the game's hex input field.
fn select_color_by_hex(
    input: &mut SafeInput,
    hex: &str,
    hex_input_pos: (i32, i32),
) -> Result<(), String> {
    // Click the hex input field
    input.move_to(hex_input_pos.0, hex_input_pos.1)?;
    input.click()?;
    std::thread::sleep(Duration::from_millis(30));

    // Select all existing text and replace with new hex code
    input.select_all()?;
    std::thread::sleep(Duration::from_millis(20));
    input.type_text(hex)?;
    std::thread::sleep(Duration::from_millis(20));

    // Press Enter to apply the color
    input.press_key(enigo::Key::Return)?;
    std::thread::sleep(Duration::from_millis(30));
    Ok(())
}

/// Paint all color groups onto the canvas using captured screen regions.
///
/// This is the main painting function. It:
/// 1. Selects each color by clicking its position in the palette (or typing hex)
/// 2. Paints all pixels of that color on the canvas
/// 3. Checks hotkeys between actions for pause/cancel
pub fn paint(
    groups: &[ColorGroup],
    layout: &CapturedLayout,
    img_width: u32,
    img_height: u32,
    delay: Duration,
    control: &PaintControl,
) -> Result<PaintResult, String> {
    let mut input = SafeInput::new(delay)?;
    let total_pixels: usize = groups.iter().map(|g| g.pixels.len()).sum();
    let mut painted = 0usize;
    let use_hex = layout.hex_input_pos.is_some();

    println!(
        "Starting painting: {} colors, {} pixels{}",
        groups.len(),
        total_pixels,
        if use_hex { " (hex input mode)" } else { "" }
    );
    println!("Controls: F10 = pause/resume, ESC = cancel");

    for (i, group) in groups.iter().enumerate() {
        if !control.check() {
            return Ok(PaintResult::Cancelled { painted, total_pixels });
        }

        println!(
            "[{}/{}] Color #{} ({} pixels)",
            i + 1,
            groups.len(),
            group.hex,
            group.pixels.len()
        );

        // Select the color - either by hex input or palette click
        if let Some(hex_pos) = layout.hex_input_pos {
            select_color_by_hex(&mut input, &group.hex, hex_pos)?;
        } else {
            let entry = find_nearest_palette_entry(
                group.color.0,
                group.color.1,
                group.color.2,
                &layout.palette_colors,
            );
            input.move_to(entry.screen_x, entry.screen_y)?;
            input.click()?;
        }

        // Small delay for color picker UI to update
        std::thread::sleep(Duration::from_millis(30));

        // Paint each pixel in this color group
        for &(px, py) in &group.pixels {
            if !control.check() {
                return Ok(PaintResult::Cancelled { painted, total_pixels });
            }

            let (screen_x, screen_y) =
                pixel_to_screen(px, py, img_width, img_height, &layout.canvas);

            input.move_to(screen_x, screen_y)?;
            input.click()?;

            painted += 1;
            if painted % 500 == 0 {
                let pct = (painted as f64 / total_pixels as f64) * 100.0;
                println!("  Progress: {}/{} ({:.1}%)", painted, total_pixels, pct);
            }
        }
    }

    Ok(PaintResult::Completed { painted })
}

/// Result of a painting session.
pub enum PaintResult {
    Completed { painted: usize },
    Cancelled { painted: usize, total_pixels: usize },
}
