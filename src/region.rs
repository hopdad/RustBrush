//! Interactive region capture for canvas and palette areas.
//!
//! Instead of hardcoding screen coordinates (which break across resolutions),
//! we let the user capture regions by clicking two corner points.
//! This is the same approach used by RustForge (F9/F10 hotkeys).

use crate::screen;
use device_query::{DeviceQuery, DeviceState, Keycode};
use std::time::Duration;

/// A rectangular region on screen, defined by two corner points.
#[derive(Debug, Clone, Copy)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRegion {
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }
}

/// Configuration captured from the user's screen.
#[derive(Debug, Clone)]
pub struct CapturedLayout {
    /// The in-game sign canvas area
    pub canvas: ScreenRegion,
    /// The in-game color palette area
    pub palette: ScreenRegion,
    /// Colors sampled from the palette region, with their screen positions
    pub palette_colors: Vec<PaletteEntry>,
    /// Screen position of the hex color input field (if using hex mode)
    pub hex_input_pos: Option<(i32, i32)>,
}

/// A color found in the palette, with its screen click position.
#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub screen_x: i32,
    pub screen_y: i32,
}

/// Wait for the user to press a hotkey, then click and drag to define a
/// rectangular region on screen.
pub fn capture_region_interactive(label: &str, key: Keycode) -> Result<ScreenRegion, String> {
    let device_state = DeviceState::new();

    println!("\n--- {} Capture ---", label);
    println!("Press {:?} to start, then click and drag to select the {} area.", key, label);

    // Wait for the hotkey to signal readiness
    wait_for_key_press_and_release(&device_state, key)?;
    println!("  Now click and drag to draw the {} bounding box...", label);

    // Wait for mouse button down (start of drag)
    let p1 = wait_for_mouse_down(&device_state)?;
    println!("  Start: ({}, {})", p1.0, p1.1);

    // Wait for mouse button up (end of drag)
    let p2 = wait_for_mouse_up(&device_state)?;
    println!("  End:   ({}, {})", p2.0, p2.1);

    let x = p1.0.min(p2.0);
    let y = p1.1.min(p2.1);
    let width = (p1.0 - p2.0).unsigned_abs();
    let height = (p1.1 - p2.1).unsigned_abs();

    if width < 10 || height < 10 {
        return Err(format!(
            "Region too small ({}x{}). Please select a larger area.",
            width, height
        ));
    }

    let region = ScreenRegion { x, y, width, height };
    println!("  {} region: {}x{} at ({}, {})", label, width, height, x, y);
    Ok(region)
}

/// Wait for the user to press a hotkey and click to mark a single point on screen.
pub fn capture_point_interactive(label: &str, key: Keycode) -> Result<(i32, i32), String> {
    let device_state = DeviceState::new();

    println!("\n--- {} Capture ---", label);
    println!("Press {:?} to start, then click on the {}.", key, label);

    wait_for_key_press_and_release(&device_state, key)?;
    println!("  Now click on the {}...", label);

    let pos = wait_for_mouse_down(&device_state)?;
    // Wait for release so the click doesn't interfere
    wait_for_mouse_up(&device_state)?;
    println!("  {}: ({}, {})", label, pos.0, pos.1);
    Ok(pos)
}

/// Sample colors from a captured palette region by reading screen pixels.
/// Scans the region in a grid pattern and deduplicates similar colors.
pub fn sample_palette_colors(region: &ScreenRegion) -> Result<Vec<PaletteEntry>, String> {
    let capture = screen::capture_region(region.x, region.y, region.width, region.height)?;

    let mut entries: Vec<PaletteEntry> = Vec::new();
    let stride = region.width as usize * 4; // RGBA

    // Sample at regular intervals across the palette region.
    // Use a step size that covers the area without being too dense.
    let step_x = (region.width / 16).max(4) as usize;
    let step_y = (region.height / 4).max(4) as usize;

    for py in (0..region.height as usize).step_by(step_y) {
        for px in (0..region.width as usize).step_by(step_x) {
            let offset = py * stride + px * 4;
            if offset + 2 >= capture.len() {
                continue;
            }

            let r = capture[offset];
            let g = capture[offset + 1];
            let b = capture[offset + 2];

            // Skip very dark pixels (likely UI borders/background)
            // and skip near-identical colors we've already captured
            if is_ui_background(r, g, b) {
                continue;
            }

            if !has_similar_color(&entries, r, g, b, 15) {
                entries.push(PaletteEntry {
                    r,
                    g,
                    b,
                    screen_x: region.x + px as i32,
                    screen_y: region.y + py as i32,
                });
            }
        }
    }

    if entries.is_empty() {
        return Err("No colors found in palette region. Make sure the palette is visible.".into());
    }

    println!("Sampled {} unique colors from palette", entries.len());
    Ok(entries)
}

/// Check if a color is likely a UI background element (dark gray/black borders).
fn is_ui_background(r: u8, g: u8, b: u8) -> bool {
    // Rust's UI uses dark backgrounds around the palette
    let brightness = r as u32 + g as u32 + b as u32;
    // Also skip very uniform grays that are likely UI chrome
    brightness < 30 && (r as i32 - g as i32).unsigned_abs() < 10 && (g as i32 - b as i32).unsigned_abs() < 10
}

/// Check if we already have a sufficiently similar color.
fn has_similar_color(entries: &[PaletteEntry], r: u8, g: u8, b: u8, threshold: u32) -> bool {
    entries.iter().any(|e| {
        let dr = r as i32 - e.r as i32;
        let dg = g as i32 - e.g as i32;
        let db = b as i32 - e.b as i32;
        (dr * dr + dg * dg + db * db) as u32 <= threshold * threshold
    })
}

/// Wait for the user to press and release the specified key.
fn wait_for_key_press_and_release(
    device_state: &DeviceState,
    key: Keycode,
) -> Result<(), String> {
    // Wait for key to be released first (in case it's already held)
    while device_state.get_keys().contains(&key) {
        std::thread::sleep(Duration::from_millis(50));
    }

    // Wait for key press
    loop {
        let keys = device_state.get_keys();
        if keys.contains(&key) {
            // Wait for release to avoid interfering with the drag
            while device_state.get_keys().contains(&key) {
                std::thread::sleep(Duration::from_millis(50));
            }
            std::thread::sleep(Duration::from_millis(200));
            return Ok(());
        }

        if keys.contains(&Keycode::Escape) {
            return Err("Cancelled by user".into());
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Wait for the left mouse button to be pressed; return the position.
fn wait_for_mouse_down(device_state: &DeviceState) -> Result<(i32, i32), String> {
    // Make sure mouse is currently up before we start
    while is_left_button_pressed(device_state) {
        std::thread::sleep(Duration::from_millis(50));
    }

    loop {
        if device_state.get_keys().contains(&Keycode::Escape) {
            return Err("Cancelled by user".into());
        }

        if is_left_button_pressed(device_state) {
            let mouse = device_state.get_mouse();
            return Ok((mouse.coords.0, mouse.coords.1));
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Wait for the left mouse button to be released; return the position.
fn wait_for_mouse_up(device_state: &DeviceState) -> Result<(i32, i32), String> {
    loop {
        if device_state.get_keys().contains(&Keycode::Escape) {
            return Err("Cancelled by user".into());
        }

        if !is_left_button_pressed(device_state) {
            let mouse = device_state.get_mouse();
            return Ok((mouse.coords.0, mouse.coords.1));
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Check if the left mouse button (button index 1) is currently pressed.
fn is_left_button_pressed(device_state: &DeviceState) -> bool {
    let mouse = device_state.get_mouse();
    mouse.button_pressed.get(1).copied().unwrap_or(false)
}
