//! Safe input simulation using enigo (OS-level SendInput).
//!
//! SAFETY GUARANTEES:
//! - Uses only the OS-provided SendInput API (Windows)
//! - Never reads or writes game process memory
//! - Never injects DLLs or hooks into any process
//! - All input appears identical to physical mouse/keyboard actions

use enigo::{
    Enigo, Keyboard, Mouse, Settings,
    {Coordinate, Direction, Key},
};
use std::time::Duration;

/// Trait for input drivers, allowing mock implementations for testing.
pub trait InputDriver {
    fn move_to(&mut self, x: i32, y: i32) -> Result<(), String>;
    fn click(&mut self) -> Result<(), String>;
    fn shift_click(&mut self) -> Result<(), String>;
    fn select_all(&mut self) -> Result<(), String>;
    fn type_text(&mut self, text: &str) -> Result<(), String>;
    fn press_key_return(&mut self) -> Result<(), String>;
}

/// Real input driver using enigo for OS-level input simulation.
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
}

impl InputDriver for SafeInput {
    fn move_to(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| format!("Mouse move failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    fn click(&mut self) -> Result<(), String> {
        self.enigo
            .button(enigo::Button::Left, Direction::Click)
            .map_err(|e| format!("Mouse click failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    fn shift_click(&mut self) -> Result<(), String> {
        self.enigo
            .key(Key::Shift, Direction::Press)
            .map_err(|e| format!("Key press failed: {}", e))?;
        self.enigo
            .button(enigo::Button::Left, Direction::Click)
            .map_err(|e| format!("Mouse click failed: {}", e))?;
        self.enigo
            .key(Key::Shift, Direction::Release)
            .map_err(|e| format!("Key release failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    fn select_all(&mut self) -> Result<(), String> {
        self.enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| format!("Key press failed: {}", e))?;
        self.enigo
            .key(Key::Unicode('a'), Direction::Click)
            .map_err(|e| format!("Key press failed: {}", e))?;
        self.enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| format!("Key press failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<(), String> {
        self.enigo
            .text(text)
            .map_err(|e| format!("Text input failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }

    fn press_key_return(&mut self) -> Result<(), String> {
        self.enigo
            .key(Key::Return, Direction::Click)
            .map_err(|e| format!("Key press failed: {}", e))?;
        std::thread::sleep(self.delay);
        Ok(())
    }
}

/// Dry-run input driver that logs actions without sending real input.
#[derive(Default)]
pub struct DryRunInput {
    pub log: Vec<String>,
}

impl DryRunInput {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InputDriver for DryRunInput {
    fn move_to(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.log.push(format!("MOVE({}, {})", x, y));
        Ok(())
    }

    fn click(&mut self) -> Result<(), String> {
        self.log.push("CLICK".to_string());
        Ok(())
    }

    fn shift_click(&mut self) -> Result<(), String> {
        self.log.push("SHIFT_CLICK".to_string());
        Ok(())
    }

    fn select_all(&mut self) -> Result<(), String> {
        self.log.push("SELECT_ALL".to_string());
        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<(), String> {
        self.log.push(format!("TYPE({})", text));
        Ok(())
    }

    fn press_key_return(&mut self) -> Result<(), String> {
        self.log.push("ENTER".to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run_input() {
        let mut input = DryRunInput::new();
        input.move_to(100, 200).unwrap();
        input.click().unwrap();
        input.type_text("FF0000").unwrap();
        input.press_key_return().unwrap();
        assert_eq!(input.log.len(), 4);
        assert_eq!(input.log[0], "MOVE(100, 200)");
        assert_eq!(input.log[1], "CLICK");
        assert_eq!(input.log[2], "TYPE(FF0000)");
        assert_eq!(input.log[3], "ENTER");
    }
}
