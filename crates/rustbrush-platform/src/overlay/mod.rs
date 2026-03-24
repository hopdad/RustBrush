//! Screen overlay for visualizing region selection during capture.
//!
//! Draws a visible rectangle on screen while the user clicks and drags
//! to define a region. The overlay is click-through so mouse events
//! still reach the game underneath.

#[cfg(target_os = "windows")]
mod windows;

/// A handle to an overlay window that draws a selection rectangle on screen.
/// Dropping the handle destroys the overlay window.
pub trait SelectionOverlay: Send {
    /// Update the rectangle drawn on screen. Coordinates are absolute screen pixels.
    /// Pass the anchor point (mouse-down) and the current mouse position.
    fn update(&mut self, x1: i32, y1: i32, x2: i32, y2: i32);

    /// Hide and destroy the overlay.
    fn destroy(&mut self);
}

/// Create a platform-appropriate overlay. Returns None if overlay creation fails
/// (capture still works, just without visual feedback).
pub fn create_overlay() -> Option<Box<dyn SelectionOverlay>> {
    #[cfg(target_os = "windows")]
    {
        windows::WindowsOverlay::new()
            .map(|o| Box::new(o) as Box<dyn SelectionOverlay>)
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
