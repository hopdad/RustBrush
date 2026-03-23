//! Hotkey monitoring for pause/resume/cancel during painting.
//!
//! Runs in a background thread, sets atomic flags that the painter checks
//! between each action. No game process interaction - just OS-level key polling.

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

impl PaintControl {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
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

                // F10 toggles pause (on key-down edge)
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

                // ESC cancels (on key-down edge)
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
