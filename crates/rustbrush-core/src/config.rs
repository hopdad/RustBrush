//! User configuration save/load.
//!
//! Settings are stored as JSON in ~/.rustbrush/config.json.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persistent user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Canvas preset index (into all_presets()).
    pub canvas_preset_idx: usize,
    /// Use custom canvas size instead of preset.
    pub use_custom_size: bool,
    /// Custom canvas width.
    pub custom_width: u32,
    /// Custom canvas height.
    pub custom_height: u32,
    /// Color matching algorithm: "ciede2000" or "rgb".
    pub color_match: String,
    /// Dithering mode: "none", "floyd-steinberg", or "ordered".
    pub dither: String,
    /// Painting strategy: "hybrid", "color-grouped", "line-draw", or "scanline".
    pub strategy: String,
    /// Alpha threshold for transparency.
    pub alpha_threshold: u8,
    /// Delay between mouse actions in ms.
    pub delay_ms: u32,
    /// Use hex input mode.
    pub hex_input: bool,
    /// Auto-save session.
    pub save_session: bool,
    /// Use adaptive palette (k-means).
    pub adaptive_palette: bool,
    /// Number of colors for adaptive palette.
    pub adaptive_colors: usize,
    /// Image brightness adjustment (1.0 = unchanged).
    pub brightness: f32,
    /// Image contrast adjustment (1.0 = unchanged).
    pub contrast: f32,
    /// Image saturation adjustment (1.0 = unchanged).
    pub saturation: f32,
    /// Quality preset: "speed", "balanced", "quality", "maximum", or "custom".
    pub quality_preset: String,
    /// Enable Gaussian blur filter.
    pub blur_enabled: bool,
    /// Gaussian blur sigma (0.5–5.0).
    pub blur_sigma: f32,
    /// Enable posterize filter.
    pub posterize_enabled: bool,
    /// Posterize levels per channel (2–32).
    pub posterize_levels: u8,
    /// Enable median filter.
    pub median_enabled: bool,
    /// Median filter radius (1–3).
    pub median_radius: u32,
    /// Enable 2-opt path optimization for segment ordering.
    pub path_optimizer: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            canvas_preset_idx: 1,
            use_custom_size: false,
            custom_width: 256,
            custom_height: 128,
            color_match: "ciede2000".to_string(),
            dither: "none".to_string(),
            strategy: "hybrid".to_string(),
            alpha_threshold: 128,
            delay_ms: 15,
            hex_input: false,
            save_session: true,
            adaptive_palette: false,
            adaptive_colors: 256,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            quality_preset: "balanced".to_string(),
            blur_enabled: false,
            blur_sigma: 1.0,
            posterize_enabled: false,
            posterize_levels: 8,
            median_enabled: false,
            median_radius: 1,
            path_optimizer: true,
        }
    }
}

impl Config {
    /// Default config file path (~/.rustbrush/config.json).
    pub fn default_path() -> PathBuf {
        config_dir().join("config.json")
    }

    /// Save config to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }

    /// Load config from a JSON file. Returns default if file doesn't exist.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Load from the default path.
    pub fn load_default() -> Self {
        Self::load(&Self::default_path())
    }

    /// Save to the default path.
    pub fn save_default(&self) -> Result<(), String> {
        self.save(&Self::default_path())
    }
}

fn config_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        PathBuf::from(home).join(".rustbrush")
    } else {
        PathBuf::from(".rustbrush")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.delay_ms, 15);
        assert_eq!(config.color_match, "ciede2000");
        assert_eq!(config.strategy, "hybrid");
        assert_eq!(config.brightness, 1.0);
    }

    #[test]
    fn test_config_save_load_roundtrip() {
        let mut config = Config::default();
        config.delay_ms = 42;
        config.strategy = "line-draw".to_string();
        config.brightness = 1.5;

        let tmp = std::env::temp_dir().join("rustbrush_test_config.json");
        config.save(&tmp).unwrap();

        let loaded = Config::load(&tmp);
        assert_eq!(loaded.delay_ms, 42);
        assert_eq!(loaded.strategy, "line-draw");
        assert_eq!(loaded.brightness, 1.5);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_config_load_nonexistent_returns_default() {
        let config = Config::load(Path::new("/nonexistent/config.json"));
        assert_eq!(config.delay_ms, 15);
    }
}
