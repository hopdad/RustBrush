//! Safe input simulation using enigo (OS-level SendInput).
//!
//! SAFETY GUARANTEES:
//! - Uses only the OS-provided SendInput API (Windows) or xdotool (Linux)
//! - Never reads or writes game process memory
//! - Never injects DLLs or hooks into any process
//! - Never opens handles to the game process
//! - All input appears identical to physical mouse/keyboard actions

use enigo::{
    Enigo, Keyboard, Mouse, Settings,
    {Coordinate, Direction, Key},
};
use std::time::Duration;

pub struct SafeInput {
    enigo: Enigo,
    delay: Duration,
}

impl SafeInput {
    pub fn new(delay: Duration) -> Result<Self, String> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to initialize input simulation: {}", e))?;
        Ok(Self { enigo, delay })
    }

    /// Move the mouse to absolute screen coordinates.
    /// Uses OS-level SendInput - identical to a physical mouse move.
    pub fn move_to(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| format!("Mouse move failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    /// Click the left mouse button.
    /// Uses OS-level SendInput - identical to a physical click.
    pub fn click(&mut self) -> Result<(), String> {
        self.enigo
            .button(enigo::Button::Left, Direction::Click)
            .map_err(|e| format!("Mouse click failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    /// Press and hold the left mouse button.
    pub fn mouse_down(&mut self) -> Result<(), String> {
        self.enigo
            .button(enigo::Button::Left, Direction::Press)
            .map_err(|e| format!("Mouse down failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    /// Release the left mouse button.
    pub fn mouse_up(&mut self) -> Result<(), String> {
        self.enigo
            .button(enigo::Button::Left, Direction::Release)
            .map_err(|e| format!("Mouse up failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    /// Type a text string (for entering hex codes in the color picker).
    pub fn type_text(&mut self, text: &str) -> Result<(), String> {
        self.enigo
            .text(text)
            .map_err(|e| format!("Text input failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    /// Press a single key.
    pub fn press_key(&mut self, key: Key) -> Result<(), String> {
        self.enigo
            .key(key, Direction::Click)
            .map_err(|e| format!("Key press failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }
}
