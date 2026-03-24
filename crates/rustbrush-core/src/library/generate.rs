use image::{Rgba, RgbaImage};

use super::{BLACK, GREEN, RED, TRANSPARENT, WHITE, YELLOW};

// ── helpers ──────────────────────────────────────────────────────────

fn filled_rect(img: &mut RgbaImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    for y in y0.max(0)..=y1.min(h - 1) {
        for x in x0.max(0)..=x1.min(w - 1) {
            img.put_pixel(x as u32, y as u32, color);
        }
    }
}

fn filled_circle(img: &mut RgbaImage, cx: f32, cy: f32, r: f32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let r2 = r * r;
    for y in ((cy - r) as i32).max(0)..=((cy + r) as i32).min(h - 1) {
        for x in ((cx - r) as i32).max(0)..=((cx + r) as i32).min(w - 1) {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                img.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn ring(img: &mut RgbaImage, cx: f32, cy: f32, r_outer: f32, r_inner: f32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let ro2 = r_outer * r_outer;
    let ri2 = r_inner * r_inner;
    for y in ((cy - r_outer) as i32).max(0)..=((cy + r_outer) as i32).min(h - 1) {
        for x in ((cx - r_outer) as i32).max(0)..=((cx + r_outer) as i32).min(w - 1) {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d2 = dx * dx + dy * dy;
            if d2 <= ro2 && d2 >= ri2 {
                img.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn draw_line(img: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return;
    }
    let nx = -dy / len;
    let ny = dx / len;
    let half_t = thickness / 2.0;

    let min_x = (x0.min(x1) - half_t) as i32;
    let max_x = (x0.max(x1) + half_t) as i32;
    let min_y = (y0.min(y1) - half_t) as i32;
    let max_y = (y0.max(y1) + half_t) as i32;

    for py in min_y.max(0)..=max_y.min(h - 1) {
        for px in min_x.max(0)..=max_x.min(w - 1) {
            let fx = px as f32;
            let fy = py as f32;
            // Distance from point to line
            let perp = ((fx - x0) * nx + (fy - y0) * ny).abs();
            // Projection along line
            let along = (fx - x0) * dx / len + (fy - y0) * dy / len;
            if perp <= half_t && along >= -half_t && along <= len + half_t {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

fn filled_triangle(
    img: &mut RgbaImage,
    x0: f32, y0: f32,
    x1: f32, y1: f32,
    x2: f32, y2: f32,
    color: Rgba<u8>,
) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let min_x = x0.min(x1).min(x2) as i32;
    let max_x = x0.max(x1).max(x2) as i32;
    let min_y = y0.min(y1).min(y2) as i32;
    let max_y = y0.max(y1).max(y2) as i32;

    for py in min_y.max(0)..=max_y.min(h - 1) {
        for px in min_x.max(0)..=max_x.min(w - 1) {
            let fx = px as f32;
            let fy = py as f32;
            if point_in_triangle(fx, fy, x0, y0, x1, y1, x2, y2) {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

fn point_in_triangle(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
    let d1 = sign(px, py, x0, y0, x1, y1);
    let d2 = sign(px, py, x1, y1, x2, y2);
    let d3 = sign(px, py, x2, y2, x0, y0);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn sign(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2)
}

fn new_canvas(w: u32, h: u32, bg: Rgba<u8>) -> RgbaImage {
    RgbaImage::from_pixel(w, h, bg)
}

// Scale factor helpers — all drawing is relative to image size
fn s(dim: u32, frac: f32) -> f32 {
    dim as f32 * frac
}

// ── Arrows ───────────────────────────────────────────────────────────

pub fn arrow_up(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let t = s(w.min(h), 0.12);
    // Shaft
    filled_rect(&mut img, (cx - t) as i32, s(h, 0.35) as i32, (cx + t) as i32, s(h, 0.85) as i32, WHITE);
    // Arrowhead
    filled_triangle(&mut img, cx, s(h, 0.1), s(w, 0.2), s(h, 0.42), s(w, 0.8), s(h, 0.42), WHITE);
    img
}

pub fn arrow_down(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let t = s(w.min(h), 0.12);
    filled_rect(&mut img, (cx - t) as i32, s(h, 0.15) as i32, (cx + t) as i32, s(h, 0.65) as i32, WHITE);
    filled_triangle(&mut img, cx, s(h, 0.9), s(w, 0.2), s(h, 0.58), s(w, 0.8), s(h, 0.58), WHITE);
    img
}

pub fn arrow_left(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cy = h as f32 / 2.0;
    let t = s(w.min(h), 0.12);
    filled_rect(&mut img, s(w, 0.35) as i32, (cy - t) as i32, s(w, 0.85) as i32, (cy + t) as i32, WHITE);
    filled_triangle(&mut img, s(w, 0.1), cy, s(w, 0.42), s(h, 0.2), s(w, 0.42), s(h, 0.8), WHITE);
    img
}

pub fn arrow_right(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cy = h as f32 / 2.0;
    let t = s(w.min(h), 0.12);
    filled_rect(&mut img, s(w, 0.15) as i32, (cy - t) as i32, s(w, 0.65) as i32, (cy + t) as i32, WHITE);
    filled_triangle(&mut img, s(w, 0.9), cy, s(w, 0.58), s(h, 0.2), s(w, 0.58), s(h, 0.8), WHITE);
    img
}

// ── Warning Signs ────────────────────────────────────────────────────

pub fn danger_triangle(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    // Yellow triangle
    filled_triangle(&mut img, cx, s(h, 0.08), s(w, 0.08), s(h, 0.92), s(w, 0.92), s(h, 0.92), YELLOW);
    // Black inner triangle
    filled_triangle(&mut img, cx, s(h, 0.2), s(w, 0.18), s(h, 0.85), s(w, 0.82), s(h, 0.85), BLACK);
    // Exclamation mark
    let ew = s(w, 0.06);
    filled_rect(&mut img, (cx - ew) as i32, s(h, 0.35) as i32, (cx + ew) as i32, s(h, 0.62) as i32, YELLOW);
    filled_rect(&mut img, (cx - ew) as i32, s(h, 0.68) as i32, (cx + ew) as i32, s(h, 0.76) as i32, YELLOW);
    img
}

pub fn radiation(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, YELLOW);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = s(w.min(h), 0.42);

    // Draw three sectors of the radiation trefoil
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    let inner_r = r * 0.22;
    let outer_r = r;
    for py in 0..ih {
        for px in 0..iw {
            let dx = px as f32 - cx;
            let dy = py as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist >= inner_r && dist <= outer_r {
                let angle = dy.atan2(dx).to_degrees() + 180.0; // 0-360
                // Three 60-degree sectors at 0, 120, 240
                let sector = angle % 120.0;
                if sector >= 10.0 && sector <= 70.0 {
                    img.put_pixel(px as u32, py as u32, BLACK);
                }
            }
        }
    }
    filled_circle(&mut img, cx, cy, inner_r * 0.6, BLACK);
    img
}

pub fn no_entry(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = s(w.min(h), 0.42);
    filled_circle(&mut img, cx, cy, r, RED);
    filled_circle(&mut img, cx, cy, r * 0.82, WHITE);
    // Horizontal bar
    let bar_h = r * 0.22;
    filled_rect(&mut img, (cx - r * 0.7) as i32, (cy - bar_h) as i32, (cx + r * 0.7) as i32, (cy + bar_h) as i32, RED);
    img
}

pub fn high_voltage(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, YELLOW);
    let cx = w as f32 / 2.0;
    // Lightning bolt
    let t = s(w, 0.12);
    filled_triangle(&mut img, cx + t, s(h, 0.08), cx - s(w, 0.2), s(h, 0.52), cx + t * 0.5, s(h, 0.48), BLACK);
    filled_triangle(&mut img, cx - t, s(h, 0.92), cx + s(w, 0.2), s(h, 0.48), cx - t * 0.5, s(h, 0.52), BLACK);
    img
}

pub fn skull(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, BLACK);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Head
    filled_circle(&mut img, cx, cy * 0.8, s(w.min(h), 0.32), WHITE);
    // Jaw
    filled_rect(
        &mut img,
        s(w, 0.3) as i32, (cy * 0.9) as i32,
        s(w, 0.7) as i32, s(h, 0.72) as i32,
        WHITE,
    );
    // Eyes
    filled_circle(&mut img, cx - s(w, 0.1), cy * 0.72, s(w.min(h), 0.08), BLACK);
    filled_circle(&mut img, cx + s(w, 0.1), cy * 0.72, s(w.min(h), 0.08), BLACK);
    // Nose
    filled_triangle(&mut img, cx, cy * 0.8, cx - s(w, 0.04), cy * 0.92, cx + s(w, 0.04), cy * 0.92, BLACK);
    // Teeth lines
    let tooth_w = s(w, 0.02);
    for i in 0..3 {
        let tx = cx - s(w, 0.1) + s(w, 0.1) * i as f32;
        filled_rect(&mut img, (tx - tooth_w) as i32, s(h, 0.58) as i32, (tx + tooth_w) as i32, s(h, 0.72) as i32, BLACK);
    }
    // Crossbones
    let bone_t = s(w.min(h), 0.04);
    draw_line(&mut img, s(w, 0.15), s(h, 0.75), s(w, 0.85), s(h, 0.95), bone_t, WHITE);
    draw_line(&mut img, s(w, 0.85), s(h, 0.75), s(w, 0.15), s(h, 0.95), bone_t, WHITE);
    img
}

pub fn biohazard(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, BLACK);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = s(w.min(h), 0.25);

    // Three overlapping circles forming the biohazard trefoil
    let offsets: [(f32, f32); 3] = [
        (0.0, -r * 0.55),
        (-r * 0.48, r * 0.28),
        (r * 0.48, r * 0.28),
    ];
    for &(ox, oy) in &offsets {
        ring(&mut img, cx + ox, cy + oy, r, r * 0.55, YELLOW);
    }
    // Center dot
    filled_circle(&mut img, cx, cy, r * 0.15, YELLOW);
    img
}

// ── Symbols ──────────────────────────────────────────────────────────

pub fn checkmark(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let t = s(w.min(h), 0.1);
    draw_line(&mut img, s(w, 0.15), s(h, 0.55), s(w, 0.4), s(h, 0.8), t, GREEN);
    draw_line(&mut img, s(w, 0.4), s(h, 0.8), s(w, 0.85), s(h, 0.2), t, GREEN);
    img
}

pub fn x_mark(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let t = s(w.min(h), 0.1);
    draw_line(&mut img, s(w, 0.15), s(h, 0.15), s(w, 0.85), s(h, 0.85), t, RED);
    draw_line(&mut img, s(w, 0.85), s(h, 0.15), s(w, 0.15), s(h, 0.85), t, RED);
    img
}

pub fn heart(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let r = s(w.min(h), 0.2);
    // Two circles for the top bumps
    filled_circle(&mut img, cx - r * 0.65, s(h, 0.35), r, RED);
    filled_circle(&mut img, cx + r * 0.65, s(h, 0.35), r, RED);
    // Triangle for the bottom point
    filled_triangle(
        &mut img,
        cx - r * 1.65, s(h, 0.4),
        cx + r * 1.65, s(h, 0.4),
        cx, s(h, 0.88),
        RED,
    );
    img
}

pub fn star(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r_outer = s(w.min(h), 0.42);
    let r_inner = r_outer * 0.38;

    // Generate 5-pointed star vertices
    let mut points = Vec::new();
    for i in 0..10 {
        let angle = std::f32::consts::FRAC_PI_2 * -1.0 + std::f32::consts::PI * 2.0 * i as f32 / 10.0;
        let r = if i % 2 == 0 { r_outer } else { r_inner };
        points.push((cx + r * angle.cos(), cy + r * angle.sin()));
    }
    // Fill using triangles from center
    for i in 0..10 {
        let j = (i + 1) % 10;
        filled_triangle(&mut img, cx, cy, points[i].0, points[i].1, points[j].0, points[j].1, YELLOW);
    }
    img
}

pub fn peace(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = s(w.min(h), 0.4);
    let t = s(w.min(h), 0.06);

    ring(&mut img, cx, cy, r, r - t * 2.0, WHITE);
    // Vertical line
    draw_line(&mut img, cx, cy - r, cx, cy + r, t, WHITE);
    // Two diagonal lines from center downward
    let diag_r = r * 0.7;
    draw_line(&mut img, cx, cy, cx - diag_r * 0.7, cy + diag_r * 0.7, t, WHITE);
    draw_line(&mut img, cx, cy, cx + diag_r * 0.7, cy + diag_r * 0.7, t, WHITE);
    img
}

pub fn crosshair(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = s(w.min(h), 0.35);
    let t = s(w.min(h), 0.04);

    ring(&mut img, cx, cy, r, r - t * 2.0, WHITE);
    // Cross lines with gap in center
    let gap = r * 0.25;
    draw_line(&mut img, cx - r, cy, cx - gap, cy, t, WHITE);
    draw_line(&mut img, cx + gap, cy, cx + r, cy, t, WHITE);
    draw_line(&mut img, cx, cy - r, cx, cy - gap, t, WHITE);
    draw_line(&mut img, cx, cy + gap, cx, cy + r, t, WHITE);
    // Center dot
    filled_circle(&mut img, cx, cy, t, WHITE);
    img
}

// ── Borders ──────────────────────────────────────────────────────────

pub fn simple_border(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let t = s(w.min(h), 0.06).max(2.0) as i32;
    // Top, bottom
    filled_rect(&mut img, 0, 0, w as i32 - 1, t, WHITE);
    filled_rect(&mut img, 0, h as i32 - 1 - t, w as i32 - 1, h as i32 - 1, WHITE);
    // Left, right
    filled_rect(&mut img, 0, 0, t, h as i32 - 1, WHITE);
    filled_rect(&mut img, w as i32 - 1 - t, 0, w as i32 - 1, h as i32 - 1, WHITE);
    img
}

pub fn double_border(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let t = s(w.min(h), 0.04).max(1.0) as i32;
    let gap = t * 2;
    // Outer border
    filled_rect(&mut img, 0, 0, w as i32 - 1, t, WHITE);
    filled_rect(&mut img, 0, h as i32 - 1 - t, w as i32 - 1, h as i32 - 1, WHITE);
    filled_rect(&mut img, 0, 0, t, h as i32 - 1, WHITE);
    filled_rect(&mut img, w as i32 - 1 - t, 0, w as i32 - 1, h as i32 - 1, WHITE);
    // Inner border
    let o = t + gap;
    filled_rect(&mut img, o, o, w as i32 - 1 - o, o + t, WHITE);
    filled_rect(&mut img, o, h as i32 - 1 - o - t, w as i32 - 1 - o, h as i32 - 1 - o, WHITE);
    filled_rect(&mut img, o, o, o + t, h as i32 - 1 - o, WHITE);
    filled_rect(&mut img, w as i32 - 1 - o - t, o, w as i32 - 1 - o, h as i32 - 1 - o, WHITE);
    img
}

pub fn corner_frame(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let t = s(w.min(h), 0.05).max(2.0) as i32;
    let l = s(w.min(h), 0.25) as i32;
    // Top-left
    filled_rect(&mut img, 0, 0, l, t, WHITE);
    filled_rect(&mut img, 0, 0, t, l, WHITE);
    // Top-right
    filled_rect(&mut img, w as i32 - 1 - l, 0, w as i32 - 1, t, WHITE);
    filled_rect(&mut img, w as i32 - 1 - t, 0, w as i32 - 1, l, WHITE);
    // Bottom-left
    filled_rect(&mut img, 0, h as i32 - 1 - t, l, h as i32 - 1, WHITE);
    filled_rect(&mut img, 0, h as i32 - 1 - l, t, h as i32 - 1, WHITE);
    // Bottom-right
    filled_rect(&mut img, w as i32 - 1 - l, h as i32 - 1 - t, w as i32 - 1, h as i32 - 1, WHITE);
    filled_rect(&mut img, w as i32 - 1 - t, h as i32 - 1 - l, w as i32 - 1, h as i32 - 1, WHITE);
    img
}

pub fn dashed_border(w: u32, h: u32) -> RgbaImage {
    let mut img = new_canvas(w, h, TRANSPARENT);
    let t = s(w.min(h), 0.05).max(2.0) as i32;
    let dash = s(w.min(h), 0.08).max(4.0) as i32;
    let gap = dash;

    // Top and bottom dashes
    let mut x = 0;
    while x < w as i32 {
        filled_rect(&mut img, x, 0, (x + dash).min(w as i32 - 1), t, WHITE);
        filled_rect(&mut img, x, h as i32 - 1 - t, (x + dash).min(w as i32 - 1), h as i32 - 1, WHITE);
        x += dash + gap;
    }
    // Left and right dashes
    let mut y = 0;
    while y < h as i32 {
        filled_rect(&mut img, 0, y, t, (y + dash).min(h as i32 - 1), WHITE);
        filled_rect(&mut img, w as i32 - 1 - t, y, w as i32 - 1, (y + dash).min(h as i32 - 1), WHITE);
        y += dash + gap;
    }
    img
}

// ── Text Labels ──────────────────────────────────────────────────────
// These use simple block-pixel text since we can't depend on fonts in core.

fn draw_block_text(img: &mut RgbaImage, text: &str, color: Rgba<u8>, bg: Rgba<u8>) {
    // Fill background
    for p in img.pixels_mut() {
        *p = bg;
    }

    let (w, h) = (img.width(), img.height());
    let char_patterns = block_font();

    let chars: Vec<char> = text.chars().collect();
    let char_w = 5;
    let char_h = 7;
    let spacing = 1;
    let total_w = chars.len() as u32 * (char_w + spacing);

    // Scale to fit
    let scale_x = (w as f32 * 0.85) / total_w as f32;
    let scale_y = (h as f32 * 0.5) / char_h as f32;
    let scale = scale_x.min(scale_y).max(1.0);

    let scaled_char_w = (char_w as f32 * scale) as u32;
    let scaled_char_h = (char_h as f32 * scale) as u32;
    let scaled_spacing = (spacing as f32 * scale) as u32;
    let total_scaled_w = chars.len() as u32 * (scaled_char_w + scaled_spacing);

    let start_x = (w.saturating_sub(total_scaled_w)) / 2;
    let start_y = (h.saturating_sub(scaled_char_h)) / 2;

    for (ci, ch) in chars.iter().enumerate() {
        let pattern = char_patterns.iter().find(|p| p.0 == *ch).map(|p| p.1);
        if let Some(rows) = pattern {
            let ox = start_x + ci as u32 * (scaled_char_w + scaled_spacing);
            for (row_idx, &row) in rows.iter().enumerate() {
                for bit in 0..char_w {
                    if (row >> (char_w - 1 - bit)) & 1 == 1 {
                        // Scale up
                        for sy in 0..scale as u32 {
                            for sx in 0..scale as u32 {
                                let px = ox + bit as u32 * scale as u32 + sx;
                                let py = start_y + row_idx as u32 * scale as u32 + sy;
                                if px < w && py < h {
                                    img.put_pixel(px, py, color);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Minimal 5x7 block font for uppercase + common characters.
fn block_font() -> Vec<(char, [u8; 7])> {
    vec![
        ('A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        ('B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
        ('C', [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        ('D', [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100]),
        ('E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        ('F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        ('G', [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110]),
        ('H', [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        ('I', [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        ('K', [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        ('L', [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        ('N', [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        ('O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        ('P', [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        ('R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        ('S', [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
        ('T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        ('U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        ('V', [0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100]),
        ('X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
        (' ', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
    ]
}

pub fn label_keep_out(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    draw_block_text(&mut img, "KEEP OUT", WHITE, RED);
    img
}

pub fn label_danger(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    draw_block_text(&mut img, "DANGER", BLACK, YELLOW);
    img
}

pub fn label_private(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    draw_block_text(&mut img, "PRIVATE", WHITE, Rgba([60, 60, 60, 255]));
    img
}

pub fn label_shop(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    draw_block_text(&mut img, "SHOP", WHITE, Rgba([40, 100, 180, 255]));
    img
}
