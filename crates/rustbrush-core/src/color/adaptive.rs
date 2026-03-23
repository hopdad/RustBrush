//! Adaptive palette generation using k-means clustering in Lab color space.
//!
//! Analyzes an image's colors and selects an optimal set of N colors (up to 512)
//! for hex-code input mode, providing much better color reproduction than the
//! fixed 32-color palette.

use image::RgbaImage;
use palette::{FromColor, Lab, Srgb};
use super::PaletteColor;

/// Generate an adaptive palette of up to `max_colors` from the image's actual colors.
/// Uses k-means clustering in CIE L*a*b* space for perceptually uniform results.
pub fn generate_adaptive_palette(img: &RgbaImage, max_colors: usize) -> Vec<PaletteColor> {
    // Sample unique colors from the image
    let mut color_counts: std::collections::HashMap<(u8, u8, u8), u32> =
        std::collections::HashMap::new();
    for pixel in img.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 128 {
            continue;
        }
        *color_counts.entry((r, g, b)).or_insert(0) += 1;
    }

    let unique_colors: Vec<((u8, u8, u8), u32)> =
        color_counts.into_iter().collect();

    if unique_colors.len() <= max_colors {
        // Fewer unique colors than requested - use them all
        return unique_colors
            .into_iter()
            .map(|((r, g, b), _)| make_palette_color(r, g, b))
            .collect();
    }

    // Convert to Lab for clustering
    let lab_colors: Vec<(Lab, u32)> = unique_colors
        .iter()
        .map(|&((r, g, b), count)| {
            let srgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            (Lab::from_color(srgb.into_linear()), count)
        })
        .collect();

    // K-means clustering
    let centroids = kmeans_lab(&lab_colors, max_colors, 20);

    // Convert centroids back to RGB PaletteColors
    centroids
        .into_iter()
        .map(|lab| {
            let srgb: Srgb = Srgb::from_color(palette::IntoColor::<palette::LinSrgb>::into_color(lab));
            let r = (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8;
            make_palette_color(r, g, b)
        })
        .collect()
}

fn make_palette_color(r: u8, g: u8, b: u8) -> PaletteColor {
    // Leak the hex string for 'static lifetime (acceptable for a small palette)
    let hex = format!("{:02X}{:02X}{:02X}", r, g, b);
    PaletteColor {
        r,
        g,
        b,
        hex: Box::leak(hex.into_boxed_str()),
    }
}

/// K-means clustering in Lab color space.
/// Returns `k` centroids representing optimal color clusters.
fn kmeans_lab(colors: &[(Lab, u32)], k: usize, max_iterations: usize) -> Vec<Lab> {
    if colors.is_empty() || k == 0 {
        return Vec::new();
    }

    let k = k.min(colors.len());

    // Initialize centroids using k-means++ seeding
    let mut centroids = kmeans_plus_plus_init(colors, k);
    let mut assignments = vec![0usize; colors.len()];

    for _iter in 0..max_iterations {
        let mut changed = false;

        // Assignment step: assign each color to nearest centroid
        for (i, (lab, _)) in colors.iter().enumerate() {
            let nearest = find_nearest_centroid(lab, &centroids);
            if nearest != assignments[i] {
                assignments[i] = nearest;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Update step: recompute centroids as weighted mean of assigned colors
        let mut sums_l = vec![0.0f64; k];
        let mut sums_a = vec![0.0f64; k];
        let mut sums_b = vec![0.0f64; k];
        let mut weights = vec![0.0f64; k];

        for (i, (lab, count)) in colors.iter().enumerate() {
            let c = assignments[i];
            let w = *count as f64;
            sums_l[c] += lab.l as f64 * w;
            sums_a[c] += lab.a as f64 * w;
            sums_b[c] += lab.b as f64 * w;
            weights[c] += w;
        }

        for c in 0..k {
            if weights[c] > 0.0 {
                centroids[c] = Lab::new(
                    (sums_l[c] / weights[c]) as f32,
                    (sums_a[c] / weights[c]) as f32,
                    (sums_b[c] / weights[c]) as f32,
                );
            }
        }
    }

    centroids
}

/// K-means++ initialization: pick initial centroids that are spread out.
fn kmeans_plus_plus_init(colors: &[(Lab, u32)], k: usize) -> Vec<Lab> {
    let mut centroids = Vec::with_capacity(k);

    // Pick the most common color as the first centroid
    let first = colors
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lab, _)| *lab)
        .unwrap();
    centroids.push(first);

    // For each subsequent centroid, pick the color with the largest
    // minimum distance to existing centroids (weighted by count)
    for _ in 1..k {
        let mut best_lab = colors[0].0;
        let mut best_dist = 0.0f64;

        for (lab, count) in colors {
            let min_dist = centroids
                .iter()
                .map(|c| lab_distance_sq(lab, c) as f64)
                .fold(f64::MAX, f64::min);
            let weighted = min_dist * (*count as f64);
            if weighted > best_dist {
                best_dist = weighted;
                best_lab = *lab;
            }
        }

        centroids.push(best_lab);
    }

    centroids
}

fn find_nearest_centroid(lab: &Lab, centroids: &[Lab]) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = lab_distance_sq(lab, a);
            let db = lab_distance_sq(lab, b);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn lab_distance_sq(a: &Lab, b: &Lab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl * dl + da * da + db * db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_adaptive_palette_single_color() {
        let img = RgbaImage::from_fn(10, 10, |_, _| image::Rgba([255, 0, 0, 255]));
        let palette = generate_adaptive_palette(&img, 16);
        assert_eq!(palette.len(), 1); // Only one unique color
        assert_eq!((palette[0].r, palette[0].g, palette[0].b), (255, 0, 0));
    }

    #[test]
    fn test_generate_adaptive_palette_few_colors() {
        let img = RgbaImage::from_fn(10, 10, |x, _| {
            if x < 5 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            }
        });
        let palette = generate_adaptive_palette(&img, 16);
        assert_eq!(palette.len(), 2);
    }

    #[test]
    fn test_generate_adaptive_palette_respects_max() {
        // Create image with many unique colors (gradient)
        let img = RgbaImage::from_fn(256, 1, |x, _| {
            image::Rgba([x as u8, 0, 0, 255])
        });
        let palette = generate_adaptive_palette(&img, 8);
        assert_eq!(palette.len(), 8);
    }

    #[test]
    fn test_generate_adaptive_palette_skips_transparent() {
        let img = RgbaImage::from_fn(10, 10, |x, _| {
            if x < 5 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 255, 0, 0]) // transparent
            }
        });
        let palette = generate_adaptive_palette(&img, 16);
        assert_eq!(palette.len(), 1); // Only red
    }

    #[test]
    fn test_kmeans_converges() {
        let colors = vec![
            (Lab::new(50.0, 0.0, 0.0), 100),
            (Lab::new(51.0, 0.0, 0.0), 100),
            (Lab::new(90.0, 0.0, 0.0), 100),
            (Lab::new(91.0, 0.0, 0.0), 100),
        ];
        let centroids = kmeans_lab(&colors, 2, 50);
        assert_eq!(centroids.len(), 2);
        // Two clusters should form around L=50 and L=90
        let mut ls: Vec<f32> = centroids.iter().map(|c| c.l).collect();
        ls.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((ls[0] - 50.5).abs() < 2.0);
        assert!((ls[1] - 90.5).abs() < 2.0);
    }
}
