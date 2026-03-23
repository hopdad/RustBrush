//! Screen capture for reading pixel colors.
//!
//! SAFETY GUARANTEES:
//! - Uses OS-level screen capture APIs only (e.g., BitBlt on Windows)
//! - Captures from the display framebuffer, NOT from game process memory
//! - This is the same mechanism as the Snipping Tool or Print Screen

use screenshots::Screen;

/// Capture a pixel color at the given screen coordinates.
pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens.first().ok_or("No display found")?;

    let capture = screen
        .capture_area(x, y, 1, 1)
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    let pixels = capture.as_raw();
    if pixels.len() >= 4 {
        Ok((pixels[0], pixels[1], pixels[2]))
    } else {
        Err("Failed to read pixel data".to_string())
    }
}

/// Capture a region of the screen and return it as raw RGBA bytes.
pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens.first().ok_or("No display found")?;

    let capture = screen
        .capture_area(x, y, width, height)
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    Ok(capture.as_raw().to_vec())
}

/// Palette entry sampled from the screen.
#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub screen_x: i32,
    pub screen_y: i32,
}

/// Sample colors from a captured palette region by reading screen pixels.
pub fn sample_palette_colors(
    region_x: i32,
    region_y: i32,
    region_width: u32,
    region_height: u32,
) -> Result<Vec<PaletteEntry>, String> {
    let capture = capture_region(region_x, region_y, region_width, region_height)?;

    let mut entries: Vec<PaletteEntry> = Vec::new();
    let stride = region_width as usize * 4;

    let step_x = (region_width / 16).max(4) as usize;
    let step_y = (region_height / 4).max(4) as usize;

    for py in (0..region_height as usize).step_by(step_y) {
        for px in (0..region_width as usize).step_by(step_x) {
            let offset = py * stride + px * 4;
            if offset + 2 >= capture.len() {
                continue;
            }

            let r = capture[offset];
            let g = capture[offset + 1];
            let b = capture[offset + 2];

            if is_ui_background(r, g, b) {
                continue;
            }

            if !has_similar_color(&entries, r, g, b, 15) {
                entries.push(PaletteEntry {
                    r,
                    g,
                    b,
                    screen_x: region_x + px as i32,
                    screen_y: region_y + py as i32,
                });
            }
        }
    }

    if entries.is_empty() {
        return Err("No colors found in palette region. Make sure the palette is visible.".into());
    }

    Ok(entries)
}

fn is_ui_background(r: u8, g: u8, b: u8) -> bool {
    let brightness = r as u32 + g as u32 + b as u32;
    brightness < 30
        && (r as i32 - g as i32).unsigned_abs() < 10
        && (g as i32 - b as i32).unsigned_abs() < 10
}

fn has_similar_color(entries: &[PaletteEntry], r: u8, g: u8, b: u8, threshold: u32) -> bool {
    entries.iter().any(|e| {
        let dr = r as i32 - e.r as i32;
        let dg = g as i32 - e.g as i32;
        let db = b as i32 - e.b as i32;
        (dr * dr + dg * dg + db * db) as u32 <= threshold * threshold
    })
}
