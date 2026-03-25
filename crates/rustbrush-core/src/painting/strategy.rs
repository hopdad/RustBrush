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
        #[allow(clippy::type_complexity)]
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
                optimization_improvement: None,
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

            if use_hex {
                commands.push(PaintCommand::SelectColorByHex { hex: group.hex.clone() });
            }
            commands.push(PaintCommand::Delay { ms: color_switch_delay_ms });

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
                optimization_improvement: None,
            },
            commands,
        }
    }
}

/// Line drawing strategy - detects horizontal runs of same-color pixels (3+)
/// and uses shift-click to draw lines instead of clicking each pixel individually.
/// This provides a massive speedup for images with solid areas.
pub struct LineDrawStrategy {
    /// Minimum run length to use line drawing (default: 3)
    pub min_run_length: u32,
}

impl Default for LineDrawStrategy {
    fn default() -> Self {
        Self { min_run_length: 3 }
    }
}

impl PaintStrategy for LineDrawStrategy {
    fn name(&self) -> &str {
        "line-draw"
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

            if use_hex {
                commands.push(PaintCommand::SelectColorByHex { hex: group.hex.clone() });
            }
            commands.push(PaintCommand::Delay { ms: color_switch_delay_ms });

            let segments = detect_line_segments(&group.pixels, self.min_run_length);
            emit_segments(&segments, canvas, img_width, img_height, &mut commands);
        }

        PaintPlan {
            metadata: PlanMetadata {
                total_pixels,
                total_colors: groups.len(),
                total_commands: commands.len(),
                strategy_name: self.name().to_string(),
                optimization_improvement: None,
            },
            commands,
        }
    }
}

/// Hybrid strategy - combines color grouping with line detection.
/// Groups by color, detects line runs within each group, uses nearest-neighbor
/// ordering for isolated pixels and remaining segments.
pub struct HybridStrategy {
    /// Minimum run length to use line drawing (default: 3)
    pub min_run_length: u32,
    /// Enable 2-opt path optimization (default: true)
    pub optimize: bool,
}

impl Default for HybridStrategy {
    fn default() -> Self {
        Self {
            min_run_length: 3,
            optimize: true,
        }
    }
}

impl PaintStrategy for HybridStrategy {
    fn name(&self) -> &str {
        "hybrid"
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
        let mut total_improvement = 0.0f64;
        let mut total_original = 0.0f64;

        for group in groups {
            if group.pixels.is_empty() {
                continue;
            }

            if use_hex {
                commands.push(PaintCommand::SelectColorByHex { hex: group.hex.clone() });
            }
            commands.push(PaintCommand::Delay { ms: color_switch_delay_ms });

            let segments = detect_line_segments(&group.pixels, self.min_run_length);

            // Order segments by nearest-neighbor on their start points
            let mut ordered = nearest_neighbor_order_segments(&segments);

            // Optionally apply 2-opt local search to reduce travel distance
            if self.optimize && ordered.len() > 2 {
                let max_iter = if ordered.len() > 1000 { 3 } else { 0 };
                let result = super::optimizer::optimize_2opt(&mut ordered, max_iter);
                total_improvement += result.original_distance - result.optimized_distance;
                total_original += result.original_distance;
            }

            emit_segments(&ordered, canvas, img_width, img_height, &mut commands);
        }

        let optimization_improvement = if self.optimize && total_original > 0.0 {
            Some((total_improvement / total_original) * 100.0)
        } else {
            None
        };

        PaintPlan {
            metadata: PlanMetadata {
                total_pixels,
                total_colors: groups.len(),
                total_commands: commands.len(),
                strategy_name: self.name().to_string(),
                optimization_improvement,
            },
            commands,
        }
    }
}

/// A paint segment - either a single pixel or a horizontal line run.
#[derive(Debug, Clone)]
pub(crate) enum PaintSegment {
    /// Single pixel at (x, y).
    Pixel(u32, u32),
    /// Horizontal line from (x_start, y) to (x_end, y) inclusive.
    HLine { y: u32, x_start: u32, x_end: u32 },
}

impl PaintSegment {
    pub(crate) fn start_point(&self) -> (u32, u32) {
        match self {
            PaintSegment::Pixel(x, y) => (*x, *y),
            PaintSegment::HLine { y, x_start, .. } => (*x_start, *y),
        }
    }

    pub(crate) fn end_point(&self) -> (u32, u32) {
        match self {
            PaintSegment::Pixel(x, y) => (*x, *y),
            PaintSegment::HLine { y, x_end, .. } => (*x_end, *y),
        }
    }

    #[cfg(test)]
    fn pixel_count(&self) -> u32 {
        match self {
            PaintSegment::Pixel(_, _) => 1,
            PaintSegment::HLine { x_start, x_end, .. } => x_end - x_start + 1,
        }
    }
}

/// Detect horizontal runs of pixels that can be drawn as lines.
/// Returns a list of PaintSegments (single pixels + horizontal lines).
fn detect_line_segments(pixels: &[(u32, u32)], min_run_length: u32) -> Vec<PaintSegment> {
    if pixels.is_empty() {
        return Vec::new();
    }

    let mut rows: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
    for &(x, y) in pixels {
        rows.entry(y).or_default().push(x);
    }

    let mut segments = Vec::new();
    let mut used: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();

    // Find horizontal runs in each row
    for (&y, xs) in &mut rows {
        xs.sort_unstable();
        xs.dedup();

        let mut i = 0;
        while i < xs.len() {
            // Find the end of this contiguous run
            let mut j = i;
            while j + 1 < xs.len() && xs[j + 1] == xs[j] + 1 {
                j += 1;
            }

            let run_length = (j - i + 1) as u32;
            if run_length >= min_run_length {
                // This is a line segment
                let x_start = xs[i];
                let x_end = xs[j];
                segments.push(PaintSegment::HLine { y, x_start, x_end });
                for x in &xs[i..=j] {
                    used.insert((*x, y));
                }
            }

            i = j + 1;
        }
    }

    // Add remaining pixels as single-pixel segments
    for &(x, y) in pixels {
        if !used.contains(&(x, y)) {
            segments.push(PaintSegment::Pixel(x, y));
        }
    }

    segments
}

/// Emit paint commands for a list of segments.
fn emit_segments(
    segments: &[PaintSegment],
    canvas: &ScreenRect,
    img_width: u32,
    img_height: u32,
    commands: &mut Vec<PaintCommand>,
) {
    for segment in segments {
        match segment {
            PaintSegment::Pixel(x, y) => {
                let (sx, sy) = pixel_to_screen(*x, *y, img_width, img_height, canvas);
                commands.push(PaintCommand::MoveTo { x: sx, y: sy });
                commands.push(PaintCommand::Click);
            }
            PaintSegment::HLine { y, x_start, x_end } => {
                // Click at line start
                let (sx1, sy1) = pixel_to_screen(*x_start, *y, img_width, img_height, canvas);
                commands.push(PaintCommand::MoveTo { x: sx1, y: sy1 });
                commands.push(PaintCommand::Click);
                // Shift-click at line end to draw the line
                let (sx2, _sy2) = pixel_to_screen(*x_end, *y, img_width, img_height, canvas);
                commands.push(PaintCommand::MoveTo { x: sx2, y: sy1 });
                commands.push(PaintCommand::ShiftClick);
            }
        }
    }
}

/// Order segments using nearest-neighbor heuristic based on start points.
fn nearest_neighbor_order_segments(segments: &[PaintSegment]) -> Vec<PaintSegment> {
    if segments.len() <= 1 {
        return segments.to_vec();
    }

    let mut remaining: Vec<PaintSegment> = segments.to_vec();
    let mut result = Vec::with_capacity(remaining.len());

    result.push(remaining.swap_remove(0));

    while !remaining.is_empty() {
        let (last_x, last_y) = match result.last().unwrap() {
            PaintSegment::Pixel(x, y) => (*x, *y),
            PaintSegment::HLine { y, x_end, .. } => (*x_end, *y), // End of last line drawn
        };

        let mut best_idx = 0;
        let mut best_dist = u64::MAX;

        for (i, seg) in remaining.iter().enumerate() {
            let (px, py) = seg.start_point();
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

/// Order pixels using nearest-neighbor heuristic to minimize total mouse travel.
fn nearest_neighbor_order(pixels: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if pixels.len() <= 1 {
        return pixels.to_vec();
    }

    let mut remaining: Vec<(u32, u32)> = pixels.to_vec();
    let mut result = Vec::with_capacity(remaining.len());

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

/// Two-pass painter: coarse fill with large brush, then detail with brush size 1.
///
/// Pass 1: For each color group, finds NxN blocks that are entirely one color
/// and paints them with a single click using brush size N.
/// Pass 2: Remaining pixels are painted at brush size 1 using the inner strategy.
pub struct TwoPassPlanner {
    /// Brush size for the coarse fill pass (2..=100).
    pub coarse_brush_size: u32,
    /// Screen position of the brush size input field.
    pub size_field_pos: (i32, i32),
    /// The inner strategy used for detail pass ordering.
    pub detail_strategy: Box<dyn PaintStrategy>,
}

impl TwoPassPlanner {
    /// Generate a two-pass paint plan.
    ///
    /// `all_pixel_colors` maps every (x,y) in the image to its assigned color.
    /// This is needed to verify that NxN blocks are entirely one color.
    pub fn plan(
        &self,
        groups: &[ColorGroup],
        canvas: &ScreenRect,
        img_width: u32,
        img_height: u32,
        use_hex: bool,
        color_switch_delay_ms: u32,
        all_pixel_colors: &std::collections::HashMap<(u32, u32), (u8, u8, u8)>,
    ) -> PaintPlan {
        let total_pixels: usize = groups.iter().map(|g| g.pixels.len()).sum();
        let n = self.coarse_brush_size;
        let mut commands = Vec::new();

        // Collect coarse clicks and detail pixels per group
        let mut coarse_groups: Vec<(&ColorGroup, Vec<(u32, u32)>)> = Vec::new();
        let mut detail_groups: Vec<ColorGroup> = Vec::new();

        for group in groups {
            let analysis = analyze_coarse_blocks(
                &group.pixels,
                all_pixel_colors,
                group.color,
                n,
                img_width,
                img_height,
            );
            if !analysis.coarse_clicks.is_empty() {
                coarse_groups.push((group, analysis.coarse_clicks));
            }
            if !analysis.detail_pixels.is_empty() {
                detail_groups.push(ColorGroup {
                    color: group.color,
                    hex: group.hex.clone(),
                    pixels: analysis.detail_pixels,
                });
            }
        }

        // === Pass 1: Coarse fill ===
        if !coarse_groups.is_empty() {
            // Set brush size to N
            commands.push(PaintCommand::MoveTo {
                x: self.size_field_pos.0,
                y: self.size_field_pos.1,
            });
            commands.push(PaintCommand::SetBrushSize { size: n });
            commands.push(PaintCommand::Delay { ms: 50 });

            for (group, coarse_clicks) in &coarse_groups {
                if use_hex {
                    commands.push(PaintCommand::SelectColorByHex { hex: group.hex.clone() });
                }
                commands.push(PaintCommand::Delay { ms: color_switch_delay_ms });

                // Order coarse clicks by nearest-neighbor for less mouse travel
                let ordered = nearest_neighbor_order(coarse_clicks);
                for (px, py) in ordered {
                    let (sx, sy) = pixel_to_screen(px, py, img_width, img_height, canvas);
                    commands.push(PaintCommand::MoveTo { x: sx, y: sy });
                    commands.push(PaintCommand::Click);
                }
            }
        }

        // === Pass 2: Detail at brush size 1 ===
        if !detail_groups.is_empty() {
            // Set brush size back to 1
            commands.push(PaintCommand::MoveTo {
                x: self.size_field_pos.0,
                y: self.size_field_pos.1,
            });
            commands.push(PaintCommand::SetBrushSize { size: 1 });
            commands.push(PaintCommand::Delay { ms: 50 });

            // Use inner strategy for detail pass
            let detail_plan = self.detail_strategy.plan(
                &detail_groups,
                canvas,
                img_width,
                img_height,
                use_hex,
                color_switch_delay_ms,
            );
            commands.extend(detail_plan.commands);
        }

        PaintPlan {
            metadata: PlanMetadata {
                total_pixels,
                total_colors: groups.len(),
                total_commands: commands.len(),
                strategy_name: format!("two-pass({}+{})", n, self.detail_strategy.name()),
                optimization_improvement: None,
            },
            commands,
        }
    }
}

/// Result of coarse block analysis for a single color group.
struct CoarseAnalysis {
    /// Center points of NxN blocks that can be painted with one coarse click.
    coarse_clicks: Vec<(u32, u32)>,
    /// Pixels not covered by any coarse block (need detail painting).
    detail_pixels: Vec<(u32, u32)>,
}

/// Analyze which NxN blocks within a color group are entirely one color.
///
/// Sweeps non-overlapping NxN blocks across the image. If every pixel in a block
/// matches `target_color`, the block center becomes a coarse click and all pixels
/// are marked as covered. Remaining pixels go to the detail list.
fn analyze_coarse_blocks(
    group_pixels: &[(u32, u32)],
    all_pixel_colors: &std::collections::HashMap<(u32, u32), (u8, u8, u8)>,
    target_color: (u8, u8, u8),
    brush_size: u32,
    img_width: u32,
    img_height: u32,
) -> CoarseAnalysis {
    let group_set: std::collections::HashSet<(u32, u32)> =
        group_pixels.iter().cloned().collect();
    let mut covered: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    let mut coarse_clicks = Vec::new();
    let n = brush_size;

    // Sweep non-overlapping NxN blocks
    let mut by = 0;
    while by + n <= img_height {
        let mut bx = 0;
        while bx + n <= img_width {
            // Check if ALL pixels in this NxN block are the target color
            let mut all_match = true;
            'block: for dy in 0..n {
                for dx in 0..n {
                    let pos = (bx + dx, by + dy);
                    // Must be in this color group AND match the target color
                    if !group_set.contains(&pos) {
                        all_match = false;
                        break 'block;
                    }
                    if let Some(&color) = all_pixel_colors.get(&pos) {
                        if color != target_color {
                            all_match = false;
                            break 'block;
                        }
                    } else {
                        all_match = false;
                        break 'block;
                    }
                }
            }

            if all_match {
                // Click at center of the NxN block
                let cx = bx + n / 2;
                let cy = by + n / 2;
                coarse_clicks.push((cx, cy));
                for dy in 0..n {
                    for dx in 0..n {
                        covered.insert((bx + dx, by + dy));
                    }
                }
            }
            bx += n;
        }
        by += n;
    }

    let detail_pixels: Vec<(u32, u32)> = group_pixels
        .iter()
        .filter(|p| !covered.contains(p))
        .cloned()
        .collect();

    CoarseAnalysis {
        coarse_clicks,
        detail_pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nearest_neighbor_order() {
        let pixels = vec![(0, 0), (10, 10), (1, 1), (9, 9)];
        let ordered = nearest_neighbor_order(&pixels);
        assert_eq!(ordered.len(), 4);
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

    #[test]
    fn test_detect_line_segments_horizontal_run() {
        // 5 consecutive horizontal pixels at y=0 -> should detect as one line
        let pixels: Vec<(u32, u32)> = (0..5).map(|x| (x, 0)).collect();
        let segments = detect_line_segments(&pixels, 3);

        let lines: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::HLine { .. }))
            .collect();
        assert_eq!(lines.len(), 1);

        if let PaintSegment::HLine { y, x_start, x_end } = lines[0] {
            assert_eq!(*y, 0);
            assert_eq!(*x_start, 0);
            assert_eq!(*x_end, 4);
        }

        // No isolated pixels should remain
        let singles: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::Pixel(_, _)))
            .collect();
        assert_eq!(singles.len(), 0);
    }

    #[test]
    fn test_detect_line_segments_short_run_stays_as_pixels() {
        // 2 consecutive pixels -> below min_run_length of 3
        let pixels = vec![(0, 0), (1, 0)];
        let segments = detect_line_segments(&pixels, 3);

        let singles: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::Pixel(_, _)))
            .collect();
        assert_eq!(singles.len(), 2);

        let lines: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::HLine { .. }))
            .collect();
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_detect_line_segments_mixed() {
        // Row with a gap: pixels at x=0,1,2,3 (line) and x=6 (isolated)
        let pixels = vec![(0, 0), (1, 0), (2, 0), (3, 0), (6, 0)];
        let segments = detect_line_segments(&pixels, 3);

        let total_pixels: u32 = segments.iter().map(|s| s.pixel_count()).sum();
        assert_eq!(total_pixels, 5);

        let lines: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::HLine { .. }))
            .collect();
        assert_eq!(lines.len(), 1);

        let singles: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::Pixel(_, _)))
            .collect();
        assert_eq!(singles.len(), 1);
    }

    #[test]
    fn test_line_draw_strategy_uses_shift_click() {
        let groups = vec![
            ColorGroup {
                color: (255, 0, 0),
                hex: "FF0000".to_string(),
                pixels: (0..10).map(|x| (x, 0)).collect(), // 10 consecutive pixels
            },
        ];
        let canvas = ScreenRect { x: 0, y: 0, width: 100, height: 100 };
        let strategy = LineDrawStrategy::default();
        let plan = strategy.plan(&groups, &canvas, 100, 100, true, 30);

        // Should contain ShiftClick commands (line drawing)
        let shift_clicks = plan.commands.iter()
            .filter(|c| matches!(c, PaintCommand::ShiftClick))
            .count();
        assert!(shift_clicks > 0, "Line draw strategy should use ShiftClick");

        // Should have fewer commands than 10 individual clicks
        let clicks = plan.commands.iter()
            .filter(|c| matches!(c, PaintCommand::Click))
            .count();
        // One click at start of line
        assert_eq!(clicks, 1);
        assert_eq!(shift_clicks, 1);
    }

    #[test]
    fn test_hybrid_strategy_fewer_commands_than_color_grouped() {
        // Image with a solid horizontal band - hybrid should use line drawing
        let pixels: Vec<(u32, u32)> = (0..50).map(|x| (x, 5)).collect();
        let groups = vec![
            ColorGroup {
                color: (255, 0, 0),
                hex: "FF0000".to_string(),
                pixels,
            },
        ];
        let canvas = ScreenRect { x: 0, y: 0, width: 500, height: 500 };

        let grouped_plan = ColorGroupedStrategy.plan(&groups, &canvas, 100, 100, true, 30);
        let hybrid_plan = HybridStrategy::default().plan(&groups, &canvas, 100, 100, true, 30);

        assert!(
            hybrid_plan.metadata.total_commands < grouped_plan.metadata.total_commands,
            "Hybrid ({}) should produce fewer commands than color-grouped ({}) for solid runs",
            hybrid_plan.metadata.total_commands,
            grouped_plan.metadata.total_commands
        );
    }

    #[test]
    fn test_detect_line_segments_multiple_rows() {
        let mut pixels = Vec::new();
        for y in 0..3 {
            for x in 0..5 {
                pixels.push((x, y));
            }
        }
        let segments = detect_line_segments(&pixels, 3);

        let lines: Vec<&PaintSegment> = segments
            .iter()
            .filter(|s| matches!(s, PaintSegment::HLine { .. }))
            .collect();
        assert_eq!(lines.len(), 3, "Should detect 3 horizontal line segments (one per row)");
    }

    #[test]
    fn test_analyze_coarse_blocks_uniform() {
        // 4x4 image, all red, brush size 2 -> 4 coarse blocks, 0 detail
        let color = (255, 0, 0);
        let mut pixels = Vec::new();
        let mut all_colors = std::collections::HashMap::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.push((x, y));
                all_colors.insert((x, y), color);
            }
        }
        let result = analyze_coarse_blocks(&pixels, &all_colors, color, 2, 4, 4);
        assert_eq!(result.coarse_clicks.len(), 4);
        assert_eq!(result.detail_pixels.len(), 0);
    }

    #[test]
    fn test_analyze_coarse_blocks_mixed() {
        // 4x4 image, mostly red but (1,1) is blue -> only 3 of 4 blocks are pure red
        let red = (255, 0, 0);
        let blue = (0, 0, 255);
        let mut red_pixels = Vec::new();
        let mut all_colors = std::collections::HashMap::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                if x == 1 && y == 1 {
                    all_colors.insert((x, y), blue);
                } else {
                    red_pixels.push((x, y));
                    all_colors.insert((x, y), red);
                }
            }
        }
        let result = analyze_coarse_blocks(&red_pixels, &all_colors, red, 2, 4, 4);
        // Block (0,0)-(1,1) has a blue pixel at (1,1), so not fully red
        assert_eq!(result.coarse_clicks.len(), 3);
        // 3 red pixels in the broken block become detail
        assert_eq!(result.detail_pixels.len(), 3);
    }

    #[test]
    fn test_analyze_coarse_blocks_not_divisible() {
        // 5x5 image, all red, brush size 2 -> 4 coarse blocks (covers 4x4), 9 detail pixels on edges
        let color = (255, 0, 0);
        let mut pixels = Vec::new();
        let mut all_colors = std::collections::HashMap::new();
        for y in 0..5u32 {
            for x in 0..5u32 {
                pixels.push((x, y));
                all_colors.insert((x, y), color);
            }
        }
        let result = analyze_coarse_blocks(&pixels, &all_colors, color, 2, 5, 5);
        assert_eq!(result.coarse_clicks.len(), 4); // 4 blocks in 4x4 area
        assert_eq!(result.detail_pixels.len(), 9); // row 4 (5 px) + column 4 rows 0-3 (4 px)
    }

    #[test]
    fn test_detect_line_segments_preserves_all_pixels() {
        let pixels = vec![
            (0, 0), (1, 0), (2, 0), (3, 0),  // line
            (0, 1), (2, 1),                     // isolated (gap)
            (5, 3), (6, 3), (7, 3), (8, 3), (9, 3),  // line
        ];
        let segments = detect_line_segments(&pixels, 3);

        let total: u32 = segments.iter().map(|s| s.pixel_count()).sum();
        assert_eq!(total, pixels.len() as u32);
    }
}
