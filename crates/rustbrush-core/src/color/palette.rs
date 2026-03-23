//! Rust in-game palette definitions.

use serde::{Deserialize, Serialize};

/// An RGB color from Rust's in-game palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub hex: &'static str,
}

impl PaletteColor {
    pub const fn new(r: u8, g: u8, b: u8, hex: &'static str) -> Self {
        Self { r, g, b, hex }
    }
}

/// Returns Rust's in-game sign editor palette (post-November 2025 update).
///
/// These 32 colors represent the fixed palette available in the sign painting UI.
pub fn rust_palette() -> Vec<PaletteColor> {
    vec![
        // Blacks and grays
        PaletteColor::new(0x00, 0x00, 0x00, "000000"),
        PaletteColor::new(0x40, 0x40, 0x40, "404040"),
        PaletteColor::new(0x80, 0x80, 0x80, "808080"),
        PaletteColor::new(0xC0, 0xC0, 0xC0, "C0C0C0"),
        PaletteColor::new(0xFF, 0xFF, 0xFF, "FFFFFF"),
        // Reds
        PaletteColor::new(0x80, 0x00, 0x00, "800000"),
        PaletteColor::new(0xFF, 0x00, 0x00, "FF0000"),
        PaletteColor::new(0xFF, 0x40, 0x40, "FF4040"),
        // Oranges
        PaletteColor::new(0x80, 0x40, 0x00, "804000"),
        PaletteColor::new(0xFF, 0x80, 0x00, "FF8000"),
        PaletteColor::new(0xFF, 0xBF, 0x00, "FFBF00"),
        // Yellows
        PaletteColor::new(0x80, 0x80, 0x00, "808000"),
        PaletteColor::new(0xFF, 0xFF, 0x00, "FFFF00"),
        PaletteColor::new(0xFF, 0xFF, 0x80, "FFFF80"),
        // Greens
        PaletteColor::new(0x00, 0x80, 0x00, "008000"),
        PaletteColor::new(0x00, 0xFF, 0x00, "00FF00"),
        PaletteColor::new(0x80, 0xFF, 0x80, "80FF80"),
        PaletteColor::new(0x00, 0x40, 0x00, "004000"),
        // Cyans
        PaletteColor::new(0x00, 0x80, 0x80, "008080"),
        PaletteColor::new(0x00, 0xFF, 0xFF, "00FFFF"),
        PaletteColor::new(0x80, 0xFF, 0xFF, "80FFFF"),
        // Blues
        PaletteColor::new(0x00, 0x00, 0x80, "000080"),
        PaletteColor::new(0x00, 0x00, 0xFF, "0000FF"),
        PaletteColor::new(0x80, 0x80, 0xFF, "8080FF"),
        PaletteColor::new(0x00, 0x40, 0xFF, "0040FF"),
        // Purples / Magentas
        PaletteColor::new(0x80, 0x00, 0x80, "800080"),
        PaletteColor::new(0xFF, 0x00, 0xFF, "FF00FF"),
        PaletteColor::new(0xFF, 0x80, 0xFF, "FF80FF"),
        // Browns / Skin tones
        PaletteColor::new(0x80, 0x40, 0x20, "804020"),
        PaletteColor::new(0xC0, 0x80, 0x40, "C08040"),
        PaletteColor::new(0xFF, 0xBF, 0x80, "FFBF80"),
        PaletteColor::new(0x40, 0x20, 0x00, "402000"),
    ]
}
