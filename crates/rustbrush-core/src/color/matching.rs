//! Color matching algorithms: RGB Euclidean and CIEDE2000.

use palette::{color_difference::Ciede2000, FromColor, Lab, Srgb};
use super::PaletteColor;

/// Color matching algorithm to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMatchAlgo {
    /// Fast squared Euclidean distance in RGB space.
    Rgb,
    /// Perceptual CIEDE2000 distance in CIE L*a*b* space.
    Ciede2000,
}

/// Pre-computed Lab values for a palette, used with CIEDE2000.
pub struct LabPalette {
    entries: Vec<(PaletteColor, Lab)>,
}

impl LabPalette {
    pub fn new(palette: &[PaletteColor]) -> Self {
        let entries = palette
            .iter()
            .map(|c| {
                let srgb = Srgb::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
                let lab: Lab = Lab::from_color(srgb.into_linear());
                (*c, lab)
            })
            .collect();
        Self { entries }
    }

    pub fn find_nearest(&self, r: f32, g: f32, b: f32) -> PaletteColor {
        let srgb = Srgb::new(r, g, b);
        let lab: Lab = Lab::from_color(srgb.into_linear());
        self.entries
            .iter()
            .min_by(|(_, a), (_, b)| {
                let da = a.difference(lab);
                let db = b.difference(lab);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(c, _)| *c)
            .unwrap()
    }
}

/// Find nearest palette color using squared Euclidean distance in RGB.
pub fn find_nearest_color_rgb(r: u8, g: u8, b: u8, palette: &[PaletteColor]) -> PaletteColor {
    let mut best = palette[0];
    let mut best_dist = u32::MAX;

    for &color in palette {
        let dr = r as i32 - color.r as i32;
        let dg = g as i32 - color.g as i32;
        let db = b as i32 - color.b as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;

        if dist < best_dist {
            best_dist = dist;
            best = color;
            if dist == 0 {
                break;
            }
        }
    }

    best
}
