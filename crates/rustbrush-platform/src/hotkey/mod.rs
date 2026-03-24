//! Hotkey monitoring for pause/resume/cancel during painting.
//!
//! Runs in a background thread, sets atomic flags that the painter checks
//! between each action.

use device_query::{DeviceQuery, DeviceState, Keycode};
use portable_atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Shared state for hotkey-driven painting control.
pub struct PaintControl {
    pub paused: Arc<AtomicBool>,
    pub cancelled: Arc<AtomicBool>,
}

impl Default for PaintControl {
    fn default() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl PaintControl {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start the background hotkey listener thread.
    /// F10 = toggle pause/resume, ESC = cancel painting.
    pub fn start_listener(&self) {
        let paused = self.paused.clone();
        let cancelled = self.cancelled.clone();

        thread::spawn(move || {
            let device_state = DeviceState::new();
            let mut f10_was_pressed = false;
            let mut esc_was_pressed = false;

            loop {
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }

                let keys = device_state.get_keys();

                let f10_pressed = keys.contains(&Keycode::F10);
                if f10_pressed && !f10_was_pressed {
                    let was_paused = paused.fetch_xor(true, Ordering::Relaxed);
                    if was_paused {
                        println!("\n[RESUMED] Painting resumed. Press F10 to pause, ESC to cancel.");
                    } else {
                        println!("\n[PAUSED] Painting paused. Press F10 to resume, ESC to cancel.");
                    }
                }
                f10_was_pressed = f10_pressed;

                let esc_pressed = keys.contains(&Keycode::Escape);
                if esc_pressed && !esc_was_pressed {
                    cancelled.store(true, Ordering::Relaxed);
                    println!("\n[CANCELLED] Painting cancelled by user.");
                    break;
                }
                esc_was_pressed = esc_pressed;

                thread::sleep(Duration::from_millis(50));
            }
        });
    }

    /// Check if painting should continue. Blocks while paused.
    /// Returns false if cancelled.
    pub fn check(&self) -> bool {
        if self.cancelled.load(Ordering::Relaxed) {
            return false;
        }

        while self.paused.load(Ordering::Relaxed) {
            if self.cancelled.load(Ordering::Relaxed) {
                return false;
            }
            thread::sleep(Duration::from_millis(100));
        }

        true
    }
}

/// Interactive region capture using device_query.
pub mod region {
    use super::*;

    /// A rectangular region on screen.
    #[derive(Debug, Clone, Copy)]
    pub struct ScreenRegion {
        pub x: i32,
        pub y: i32,
        pub width: u32,
        pub height: u32,
    }

    /// Wait for user to press a key, then click-drag to define a rectangular region.
    /// Displays a visual overlay rectangle during the drag on supported platforms.
    pub fn capture_region_interactive(label: &str, key: Keycode) -> Result<ScreenRegion, String> {
        let device_state = DeviceState::new();

        println!("\n--- {} Capture ---", label);
        println!("Press {:?} to start, then click and drag to select the {} area.", key, label);

        wait_for_key_press_and_release(&device_state, key)?;
        println!("  Now click and drag to draw the {} bounding box...", label);

        let p1 = wait_for_mouse_down(&device_state)?;
        println!("  Start: ({}, {})", p1.0, p1.1);

        let p2 = wait_for_mouse_up_with_overlay(&device_state, p1)?;
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

    /// Wait for user to press a key, then click to mark a point.
    pub fn capture_point_interactive(label: &str, key: Keycode) -> Result<(i32, i32), String> {
        let device_state = DeviceState::new();

        println!("\n--- {} Capture ---", label);
        println!("Press {:?} to start, then click on the {}.", key, label);

        wait_for_key_press_and_release(&device_state, key)?;
        println!("  Now click on the {}...", label);

        let pos = wait_for_mouse_down(&device_state)?;
        wait_for_mouse_up(&device_state)?;
        println!("  {}: ({}, {})", label, pos.0, pos.1);
        Ok(pos)
    }

    fn wait_for_key_press_and_release(
        device_state: &DeviceState,
        key: Keycode,
    ) -> Result<(), String> {
        while device_state.get_keys().contains(&key) {
            std::thread::sleep(Duration::from_millis(50));
        }

        loop {
            let keys = device_state.get_keys();
            if keys.contains(&key) {
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

    fn wait_for_mouse_down(device_state: &DeviceState) -> Result<(i32, i32), String> {
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

    /// Wait for mouse release while showing a visual overlay rectangle.
    /// The overlay tracks the mouse position in real-time during the drag.
    fn wait_for_mouse_up_with_overlay(
        device_state: &DeviceState,
        anchor: (i32, i32),
    ) -> Result<(i32, i32), String> {
        let mut overlay = crate::overlay::create_overlay();

        let result = loop {
            if device_state.get_keys().contains(&Keycode::Escape) {
                break Err("Cancelled by user".into());
            }

            let mouse = device_state.get_mouse();

            // Update the overlay rectangle with current mouse position.
            if let Some(ref mut ov) = overlay {
                ov.update(anchor.0, anchor.1, mouse.coords.0, mouse.coords.1);
            }

            if !is_left_button_pressed(device_state) {
                break Ok((mouse.coords.0, mouse.coords.1));
            }

            std::thread::sleep(Duration::from_millis(16)); // ~60fps
        };

        // Clean up overlay in all exit paths.
        if let Some(mut ov) = overlay {
            ov.destroy();
        }

        result
    }

    fn is_left_button_pressed(device_state: &DeviceState) -> bool {
        let mouse = device_state.get_mouse();
        mouse.button_pressed.get(1).copied().unwrap_or(false)
    }
}
