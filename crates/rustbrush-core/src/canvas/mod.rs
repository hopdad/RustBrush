//! Canvas dimension presets for all Rust in-game sign types.

use serde::{Deserialize, Serialize};

/// A canvas preset with width and height in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasPreset {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
}

impl CanvasPreset {
    pub const fn new(name: &'static str, width: u32, height: u32) -> Self {
        Self { name, width, height }
    }
}

impl std::fmt::Display for CanvasPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}x{})", self.name, self.width, self.height)
    }
}

// Sign presets based on Rust's in-game canvas dimensions
pub const SMALL_WOODEN_SIGN: CanvasPreset = CanvasPreset::new("Small Wooden Sign", 128, 64);
pub const WOODEN_SIGN: CanvasPreset = CanvasPreset::new("Wooden Sign", 256, 128);
pub const LARGE_WOODEN_SIGN: CanvasPreset = CanvasPreset::new("Large Wooden Sign", 256, 128);
pub const HUGE_WOODEN_SIGN: CanvasPreset = CanvasPreset::new("Huge Wooden Sign", 512, 128);
pub const TWO_SIDED_SIGN: CanvasPreset = CanvasPreset::new("Two Sided Hanging Sign", 256, 128);
pub const SMALL_BANNER: CanvasPreset = CanvasPreset::new("Small Banner", 64, 256);
pub const LARGE_BANNER: CanvasPreset = CanvasPreset::new("Large Banner", 64, 512);
pub const PORTRAIT_FRAME: CanvasPreset = CanvasPreset::new("Portrait Frame", 256, 256);
pub const LANDSCAPE_FRAME: CanvasPreset = CanvasPreset::new("Landscape Frame", 256, 128);
pub const TALL_FRAME: CanvasPreset = CanvasPreset::new("Tall Picture Frame", 128, 256);
pub const XL_FRAME: CanvasPreset = CanvasPreset::new("XL Picture Frame", 512, 512);
pub const XXL_FRAME: CanvasPreset = CanvasPreset::new("XXL Picture Frame", 1024, 512);
pub const SPINNING_WHEEL: CanvasPreset = CanvasPreset::new("Spinning Wheel", 256, 256);
pub const SMALL_NEON_SIGN: CanvasPreset = CanvasPreset::new("Small Neon Sign", 128, 128);
pub const NEON_SIGN: CanvasPreset = CanvasPreset::new("Neon Sign", 256, 128);
pub const LARGE_NEON_SIGN: CanvasPreset = CanvasPreset::new("Large Neon Sign", 256, 256);
pub const LARGE_ANIMATED_NEON_SIGN: CanvasPreset = CanvasPreset::new("Large Animated Neon Sign", 256, 256);
pub const PHOTO_FRAME: CanvasPreset = CanvasPreset::new("Photo Frame", 320, 240);

/// All available canvas presets.
pub fn all_presets() -> Vec<CanvasPreset> {
    vec![
        SMALL_WOODEN_SIGN,
        WOODEN_SIGN,
        LARGE_WOODEN_SIGN,
        HUGE_WOODEN_SIGN,
        TWO_SIDED_SIGN,
        SMALL_BANNER,
        LARGE_BANNER,
        PORTRAIT_FRAME,
        LANDSCAPE_FRAME,
        TALL_FRAME,
        XL_FRAME,
        XXL_FRAME,
        SPINNING_WHEEL,
        SMALL_NEON_SIGN,
        NEON_SIGN,
        LARGE_NEON_SIGN,
        LARGE_ANIMATED_NEON_SIGN,
        PHOTO_FRAME,
    ]
}

/// Find a preset by name (case-insensitive).
pub fn find_preset(name: &str) -> Option<CanvasPreset> {
    let name_lower = name.to_lowercase();
    all_presets().into_iter().find(|p| p.name.to_lowercase() == name_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_not_empty() {
        assert!(!all_presets().is_empty());
    }

    #[test]
    fn test_find_preset() {
        let preset = find_preset("wooden sign").unwrap();
        assert_eq!(preset.width, 256);
        assert_eq!(preset.height, 128);
    }

    #[test]
    fn test_all_presets_have_positive_dimensions() {
        for p in all_presets() {
            assert!(p.width > 0 && p.height > 0, "Preset {} has zero dimension", p.name);
        }
    }
}
