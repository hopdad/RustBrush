//! Screen capture for reading pixel colors.
//!
//! SAFETY GUARANTEES:
//! - Uses OS-level screen capture APIs only (e.g., BitBlt on Windows)
//! - Captures from the display framebuffer, NOT from game process memory
//! - Never opens handles to the game process
//! - This is the same mechanism as the Snipping Tool or Print Screen

use screenshots::Screen;

/// Capture a pixel color at the given screen coordinates.
/// Uses OS-level screen capture - reads from the display, not game memory.
pub fn get_pixel_color(x: i32, y: i32) -> Result<(u8, u8, u8), String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens.first().ok_or("No display found")?;

    // Capture a small region around the target pixel
    let capture = screen
        .capture_area(x, y, 1, 1)
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    let pixels = capture.as_raw();
    if pixels.len() >= 4 {
        // RGBA format
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
