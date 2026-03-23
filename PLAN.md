# RustBrush - Phased Build Plan

## Phase 1: Core Foundation (get painting working end-to-end)
- [x] **Canvas capture overlay** - User draws a rectangle over the in-game canvas (F9)
- [x] **Palette capture overlay** - User draws a rectangle over the in-game color palette (F8)
- [x] **Coordinate mapping** - Map image pixels to screen coordinates based on captured canvas region
- [x] **Color selection via palette clicking** - Click the nearest color in the captured palette region
- [x] **Basic pixel-by-pixel painting** - Group by color to minimize palette switches
- [x] **Pause/Resume/Cancel** - F10 pause, ESC cancel

## Phase 2: Image Quality
- [x] **Perceptual color matching (CIEDE2000)** - Accurate color mapping in CIE L*a*b* space
- [x] **Dithering support** - Floyd-Steinberg and ordered (Bayer 4x4) dithering
- [x] **Transparency handling** - Configurable alpha threshold
- [x] **Background color skip** - Skip pixels matching a specified color
- [x] **Image preview** - Save quantized preview before painting

## Phase 3: Architecture & Paint Planning
- [x] **Three-crate workspace** - rustbrush-core (pure logic), rustbrush-platform (OS), rustbrush-app (CLI)
- [x] **PaintCommand/PaintPlan** - Formal command representation for painting actions
- [x] **Painting strategies** - Scanline and color-grouped strategies with nearest-neighbor ordering
- [x] **Canvas presets** - All Rust sign types with correct dimensions
- [x] **InputDriver trait** - Abstracted input with dry-run driver for testing
- [x] **Image adjustments** - Brightness, contrast, saturation controls
- [x] **Hex code input mode** - Type exact hex colors via the game's color input field

## Phase 4: Painting Performance
- [ ] **Line drawing optimization** - Detect horizontal runs and use shift-click
- [ ] **Hybrid strategy** - Combine color grouping with line detection
- [ ] **Adaptive delays** - Auto-tune click delays based on server responsiveness
- [ ] **Progress save/resume** - Track painted pixels, resume from interruption

## Phase 5: GUI & Polish
- [ ] **GUI mode** - egui/eframe window with image preview, settings, start/stop
- [ ] **Image editor** - Built-in crop, rotate, brightness/contrast adjustments
- [ ] **Config file** - Save/load user settings and calibration data
- [ ] **Adaptive 512-color palette** - K-means clustering in Lab space for optimal hex colors
- [ ] **GitHub Actions CI** - Build + test on Linux, release Windows binary
