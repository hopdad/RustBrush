//! Painting strategies that generate PaintPlans from color groups.

use super::*;

/// Trait for painting strategies that generate paint plans.
pub trait PaintStrategy {
    fn name(&self) -> &str;

    /// Generate a paint plan from color groups.
    fn plan(
        &self,
        groups: &[ColorGroup],
        canvas: &ScreenRect,
        img_width: u32,
        img_height: u32,
        use_hex: bool,
        color_switch_delay_ms: u32,
    ) -> PaintPlan;
}

/// Simple scanline strategy - paints row by row, switching colors as needed.
pub struct ScanlineStrategy;

impl PaintStrategy for ScanlineStrategy {
    fn name(&self) -> &str {
        "scanline"
    }

    fn plan(
        &self,
        groups: &[ColorGroup],
        canvas: &ScreenRect,
        img_width: u32,
        img_height: u32,
        use_hex: bool,
        color_switch_delay_ms: u32,
    ) -> PaintPlan {
        // Flatten all pixels and sort by (y, x) for scanline order
        let mut all_pixels: Vec<(u32, u32, (u8, u8, u8), String)> = Vec::new();
        for group in groups {
            for &(px, py) in &group.pixels {
                all_pixels.push((px, py, group.color, group.hex.clone()));
            }
        }
        all_pixels.sort_by_key(|&(px, py, _, _)| (py, px));

        let total_pixels = all_pixels.len();
        let mut commands = Vec::new();
        let mut current_color: Option<(u8, u8, u8)> = None;

        for (px, py, color, hex) in &all_pixels {
            // Switch color if needed
            if current_color != Some(*color) {
                if use_hex {
                    commands.push(PaintCommand::SelectColorByHex { hex: hex.clone() });
                }
                commands.push(PaintCommand::Delay { ms: color_switch_delay_ms });
                current_color = Some(*color);
            }

            let (sx, sy) = pixel_to_screen(*px, *py, img_width, img_height, canvas);
            commands.push(PaintCommand::MoveTo { x: sx, y: sy });
            commands.push(PaintCommand::Click);
        }

        let unique_colors: std::collections::HashSet<(u8, u8, u8)> =
            all_pixels.iter().map(|(_, _, c, _)| *c).collect();

        PaintPlan {
            metadata: PlanMetadata {
                total_pixels,
                total_colors: unique_colors.len(),
                total_commands: commands.len(),
                strategy_name: self.name().to_string(),
            },
            commands,
        }
    }
}

/// Color-grouped strategy - groups pixels by color, paints all same-color pixels
/// before switching. Within each group, uses nearest-neighbor ordering to minimize
/// mouse travel distance.
pub struct ColorGroupedStrategy;

impl PaintStrategy for ColorGroupedStrategy {
    fn name(&self) -> &str {
        "color-grouped"
    }

    fn plan(
        &self,
        groups: &[ColorGroup],
        canvas: &ScreenRect,
        img_width: u32,
        img_height: u32,
        use_hex: bool,
        color_switch_delay_ms: u32,
    ) -> PaintPlan {
        let total_pixels: usize = groups.iter().map(|g| g.pixels.len()).sum();
        let mut commands = Vec::new();

        for group in groups {
            if group.pixels.is_empty() {
                continue;
            }

            // Select color
            if use_hex {
                commands.push(PaintCommand::SelectColorByHex { hex: group.hex.clone() });
            }
            commands.push(PaintCommand::Delay { ms: color_switch_delay_ms });

            // Order pixels by nearest-neighbor to minimize mouse travel
            let ordered = nearest_neighbor_order(&group.pixels);

            for (px, py) in ordered {
                let (sx, sy) = pixel_to_screen(px, py, img_width, img_height, canvas);
                commands.push(PaintCommand::MoveTo { x: sx, y: sy });
                commands.push(PaintCommand::Click);
            }
        }

        PaintPlan {
            metadata: PlanMetadata {
                total_pixels,
                total_colors: groups.len(),
                total_commands: commands.len(),
                strategy_name: self.name().to_string(),
            },
            commands,
        }
    }
}

/// Order pixels using nearest-neighbor heuristic to minimize total mouse travel.
fn nearest_neighbor_order(pixels: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if pixels.len() <= 1 {
        return pixels.to_vec();
    }

    let mut remaining: Vec<(u32, u32)> = pixels.to_vec();
    let mut result = Vec::with_capacity(remaining.len());

    // Start with the first pixel
    result.push(remaining.swap_remove(0));

    while !remaining.is_empty() {
        let (last_x, last_y) = *result.last().unwrap();
        let mut best_idx = 0;
        let mut best_dist = u64::MAX;

        for (i, &(px, py)) in remaining.iter().enumerate() {
            let dx = px as i64 - last_x as i64;
            let dy = py as i64 - last_y as i64;
            let dist = (dx * dx + dy * dy) as u64;
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }

        result.push(remaining.swap_remove(best_idx));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nearest_neighbor_order() {
        let pixels = vec![(0, 0), (10, 10), (1, 1), (9, 9)];
        let ordered = nearest_neighbor_order(&pixels);
        assert_eq!(ordered.len(), 4);
        // First should be (0,0), second should be (1,1) since it's closest
        assert_eq!(ordered[0], (0, 0));
        assert_eq!(ordered[1], (1, 1));
    }

    #[test]
    fn test_scanline_strategy_generates_commands() {
        let groups = vec![
            ColorGroup {
                color: (255, 0, 0),
                hex: "FF0000".to_string(),
                pixels: vec![(0, 0), (1, 0)],
            },
        ];
        let canvas = ScreenRect { x: 100, y: 100, width: 200, height: 200 };
        let strategy = ScanlineStrategy;
        let plan = strategy.plan(&groups, &canvas, 10, 10, true, 30);
        assert_eq!(plan.metadata.total_pixels, 2);
        assert!(!plan.commands.is_empty());
    }

    #[test]
    fn test_color_grouped_strategy_generates_commands() {
        let groups = vec![
            ColorGroup {
                color: (255, 0, 0),
                hex: "FF0000".to_string(),
                pixels: vec![(0, 0), (5, 5)],
            },
            ColorGroup {
                color: (0, 0, 255),
                hex: "0000FF".to_string(),
                pixels: vec![(1, 1)],
            },
        ];
        let canvas = ScreenRect { x: 0, y: 0, width: 100, height: 100 };
        let strategy = ColorGroupedStrategy;
        let plan = strategy.plan(&groups, &canvas, 10, 10, true, 30);
        assert_eq!(plan.metadata.total_pixels, 3);
        assert_eq!(plan.metadata.total_colors, 2);
    }
}
