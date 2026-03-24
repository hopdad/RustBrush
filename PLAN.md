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
- [x] **Line drawing optimization** - Detect horizontal runs (3+ pixels) and use shift-click
- [x] **Hybrid strategy** - Color grouping + line detection + nearest-neighbor segment ordering
- [x] **Paint plan executor** - Command-by-command execution with pause/cancel/resume support
- [x] **Progress save/resume** - JSON session files with auto-save interval, --resume flag
- [x] **Adaptive delays** - AdaptiveDelay controller auto-tunes speed based on success/failure

## Phase 5: GUI & Polish
- [x] **GUI mode** - egui/eframe window with image preview, settings, plan generation
- [x] **GitHub Actions CI** - Build + test on Linux, release Windows binary
- [x] **Image editor** - Brightness, contrast, saturation sliders with reset
- [x] **Config file** - JSON config save/load at ~/.rustbrush/config.json
- [x] **Adaptive 512-color palette** - K-means++ clustering in Lab space for optimal hex colors

## Phase 6: Content Creation Tools
- [x] **Text-to-image builder** - Type text, choose system font/size/color/alignment, render as source image
- [x] **Stock image library** - Built-in gallery of 24 programmatically generated clipart items (arrows, warnings, symbols, borders, text labels)

## Phase 7: Advanced Features (bonus)
- [x] **GIF/animation support** - Load GIF files, select frames, process and paint multi-frame animations on animated signs
- [x] **Image simplification filters** - Median filter, Gaussian blur, posterize for reducing detail before painting
- [x] **Quality presets** - Speed/Balanced/Quality/Maximum presets that configure strategy, matching, dithering, and palette in one click
- [x] **Path optimizer (2-opt)** - Reorder paint segments to minimize total mouse travel distance
- [x] **Palette scanning** - Scan the in-game palette (F7) to detect available colors and click positions; click-based color selection as faster alternative to hex typing
- [x] **Preview A/B toggle** - Switch between side-by-side, original-only, or preview-only display modes
- [x] **Palette color strip** - Horizontal bar showing proportional color distribution after processing
- [x] **Image transforms** - Rotate CW/CCW, flip horizontal/vertical
- [x] **Crop margins** - Percentage-based crop sliders applied before resize
- [x] **Rescan palette** - Re-sample palette colors from stored region without re-capturing
