//! 2-opt path optimizer for paint segments.
//!
//! Improves the greedy nearest-neighbor ordering by reversing sub-paths
//! that reduce total mouse travel distance.

use super::strategy::PaintSegment;

/// Result of a 2-opt optimization pass.
#[derive(Debug, Clone)]
pub struct OptimizeResult {
    pub original_distance: f64,
    pub optimized_distance: f64,
    pub improvement_pct: f64,
}

/// Compute total travel distance for a segment ordering.
///
/// Travel distance is the sum of distances from the end of each segment
/// to the start of the next segment.
pub(crate) fn total_travel_distance(segments: &[PaintSegment]) -> f64 {
    if segments.len() <= 1 {
        return 0.0;
    }
    let mut total = 0.0f64;
    for i in 0..segments.len() - 1 {
        let (ex, ey) = segments[i].end_point();
        let (sx, sy) = segments[i + 1].start_point();
        let dx = ex as f64 - sx as f64;
        let dy = ey as f64 - sy as f64;
        total += (dx * dx + dy * dy).sqrt();
    }
    total
}

/// Squared distance between two points (avoids sqrt for comparisons).
fn dist_sq(x0: u32, y0: u32, x1: u32, y1: u32) -> i64 {
    let dx = x0 as i64 - x1 as i64;
    let dy = y0 as i64 - y1 as i64;
    dx * dx + dy * dy
}

/// Apply 2-opt local search to improve segment ordering.
///
/// Takes a mutable vector of segments (typically already ordered by nearest-neighbor)
/// and repeatedly reverses sub-paths that reduce total travel distance.
///
/// `max_iterations`: maximum number of full passes (0 = unlimited, run until convergence).
pub(crate) fn optimize_2opt(segments: &mut Vec<PaintSegment>, max_iterations: usize) -> OptimizeResult {
    let original_distance = total_travel_distance(segments);

    let n = segments.len();
    if n <= 2 {
        return OptimizeResult {
            original_distance,
            optimized_distance: original_distance,
            improvement_pct: 0.0,
        };
    }

    // Pre-compute start/end points for fast access
    let mut starts: Vec<(u32, u32)> = segments.iter().map(|s| s.start_point()).collect();
    let mut ends: Vec<(u32, u32)> = segments.iter().map(|s| s.end_point()).collect();

    let mut iteration = 0;
    loop {
        if max_iterations > 0 && iteration >= max_iterations {
            break;
        }
        iteration += 1;

        let mut improved = false;

        // Try all 2-opt swaps
        for i in 0..n - 1 {
            for j in (i + 2)..n {
                // Current cost of edges: end[i]->start[i+1] and end[j]->start[j+1]
                let old_cost = dist_sq(ends[i].0, ends[i].1, starts[i + 1].0, starts[i + 1].1)
                    + if j + 1 < n {
                        dist_sq(ends[j].0, ends[j].1, starts[j + 1].0, starts[j + 1].1)
                    } else {
                        0
                    };

                // New cost if we reverse [i+1..=j]: end[i]->start[j] and end[i+1]->start[j+1]
                let new_cost = dist_sq(ends[i].0, ends[i].1, starts[j].0, starts[j].1)
                    + if j + 1 < n {
                        dist_sq(ends[i + 1].0, ends[i + 1].1, starts[j + 1].0, starts[j + 1].1)
                    } else {
                        0
                    };

                if new_cost < old_cost {
                    // Reverse the sub-path [i+1..=j]
                    segments[i + 1..=j].reverse();
                    starts[i + 1..=j].reverse();
                    ends[i + 1..=j].reverse();

                    // After reversing, the start/end of each segment within the reversed
                    // range are still correct (segments are atomic units), but their
                    // positions in the arrays are swapped.
                    improved = true;
                }
            }
        }

        if !improved {
            break;
        }
    }

    let optimized_distance = total_travel_distance(segments);
    let improvement_pct = if original_distance > 0.0 {
        (1.0 - optimized_distance / original_distance) * 100.0
    } else {
        0.0
    };

    OptimizeResult {
        original_distance,
        optimized_distance,
        improvement_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_on_optimal_path() {
        // Three pixels in a straight line — already optimal
        let mut segments = vec![
            PaintSegment::Pixel(0, 0),
            PaintSegment::Pixel(1, 0),
            PaintSegment::Pixel(2, 0),
        ];
        let result = optimize_2opt(&mut segments, 0);
        // Should stay in same order
        assert_eq!(segments[0].start_point(), (0, 0));
        assert_eq!(segments[1].start_point(), (1, 0));
        assert_eq!(segments[2].start_point(), (2, 0));
        assert!(result.improvement_pct.abs() < 0.01);
    }

    #[test]
    fn test_improvement_on_bad_ordering() {
        // Zigzag: (0,0) -> (100,0) -> (1,0) -> (99,0) -> (2,0)
        // This is terrible ordering; 2-opt should fix it
        let mut segments = vec![
            PaintSegment::Pixel(0, 0),
            PaintSegment::Pixel(100, 0),
            PaintSegment::Pixel(1, 0),
            PaintSegment::Pixel(99, 0),
            PaintSegment::Pixel(2, 0),
        ];

        let before = total_travel_distance(&segments);
        let result = optimize_2opt(&mut segments, 0);
        let after = total_travel_distance(&segments);

        assert!(after < before, "2-opt should reduce distance: {} -> {}", before, after);
        assert!(result.improvement_pct > 0.0);
    }

    #[test]
    fn test_preservation() {
        // Verify no segments are lost or duplicated
        let mut segments = vec![
            PaintSegment::Pixel(10, 20),
            PaintSegment::Pixel(50, 80),
            PaintSegment::Pixel(5, 5),
            PaintSegment::HLine { y: 30, x_start: 0, x_end: 15 },
            PaintSegment::Pixel(90, 10),
        ];

        let original_count = segments.len();
        let mut original_starts: Vec<(u32, u32)> =
            segments.iter().map(|s| s.start_point()).collect();
        original_starts.sort();

        optimize_2opt(&mut segments, 0);

        assert_eq!(segments.len(), original_count);
        let mut new_starts: Vec<(u32, u32)> = segments.iter().map(|s| s.start_point()).collect();
        new_starts.sort();
        assert_eq!(original_starts, new_starts);
    }

    #[test]
    fn test_large_input() {
        // 2000 random-ish segments spread across a 256x128 canvas
        let mut segments: Vec<PaintSegment> = (0..2000)
            .map(|i| {
                let x = ((i * 97) % 256) as u32;
                let y = ((i * 53) % 128) as u32;
                PaintSegment::Pixel(x, y)
            })
            .collect();

        let before = total_travel_distance(&segments);
        let result = optimize_2opt(&mut segments, 3); // Cap iterations for speed
        let after = total_travel_distance(&segments);

        assert!(after <= before, "Should not worsen: {} -> {}", before, after);
        assert_eq!(segments.len(), 2000, "No segments lost");
        assert!(result.improvement_pct >= 0.0);
    }

    #[test]
    fn test_hline_segments() {
        // Mix of HLines and Pixels — verify HLine end_point is used correctly
        let mut segments = vec![
            PaintSegment::HLine { y: 0, x_start: 0, x_end: 50 },
            PaintSegment::Pixel(200, 100),
            PaintSegment::HLine { y: 1, x_start: 0, x_end: 50 },
        ];

        // The optimal order should put the two HLines adjacent
        let before = total_travel_distance(&segments);
        optimize_2opt(&mut segments, 0);
        let after = total_travel_distance(&segments);

        assert!(after <= before);
    }

    #[test]
    fn test_two_segments() {
        // Edge case: only 2 segments, should not crash
        let mut segments = vec![
            PaintSegment::Pixel(0, 0),
            PaintSegment::Pixel(10, 10),
        ];
        let result = optimize_2opt(&mut segments, 0);
        assert_eq!(result.improvement_pct, 0.0);
    }

    #[test]
    fn test_empty_and_single() {
        let mut empty: Vec<PaintSegment> = vec![];
        let r = optimize_2opt(&mut empty, 0);
        assert_eq!(r.improvement_pct, 0.0);

        let mut single = vec![PaintSegment::Pixel(5, 5)];
        let r = optimize_2opt(&mut single, 0);
        assert_eq!(r.improvement_pct, 0.0);
    }
}
