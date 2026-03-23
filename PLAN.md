# RustBrush - Phased Build Plan

## Phase 1: Core Foundation (get painting working end-to-end)
- [ ] **Canvas capture overlay** - User draws a rectangle over the in-game canvas (like RustForge's F9). Capture screen coordinates of the canvas area.
- [ ] **Palette capture overlay** - User draws a rectangle over the in-game color palette (like RustForge's F10). Sample actual on-screen colors instead of hardcoding.
- [ ] **Coordinate mapping** - Map image pixels to screen coordinates based on captured canvas region. Handle scaling.
- [ ] **Color selection via palette clicking** - Click the nearest color in the captured palette region instead of typing hex codes. More reliable across UI changes.
- [ ] **Basic pixel-by-pixel painting** - Paint each pixel by: select color → click canvas position. Group by color to minimize palette switches.
- [ ] **Pause/Resume/Cancel** - Hotkey support (e.g., F10 pause, ESC cancel) so users can interrupt painting safely.

## Phase 2: Image Quality
- [ ] **Perceptual color matching (CIEDE2000)** - Upgrade from RGB Euclidean distance to CIEDE2000 for more accurate color mapping, especially for skin tones and gradients.
- [ ] **Dithering support** - Floyd-Steinberg or ordered dithering to improve visual quality with the limited palette.
- [ ] **Transparency handling** - Configurable alpha threshold for skipping transparent pixels.
- [ ] **Background color skip** - Allow users to specify a color to ignore (e.g., white borders).
- [ ] **Image preview** - Show the quantized image before painting so users can verify quality.

## Phase 3: Painting Performance
- [ ] **Line drawing optimization** - Detect horizontal/vertical runs of same-color pixels and drag instead of clicking each one. Major speed improvement.
- [ ] **Adaptive delays** - Auto-tune click delays based on server responsiveness. Add stability buffer for laggy servers.
- [ ] **Progress save/resume** - Track which pixels have been painted. If interrupted (death, crash, disconnect), resume from where it stopped.

## Phase 4: Polish & UX
- [ ] **GUI mode** - Simple window with image preview, settings, and start/stop controls (egui or similar).
- [ ] **Multiple sign sizes** - Presets for wooden signs, metal signs, picture frames, neon signs, pumpkins.
- [ ] **Image editor** - Basic crop, rotate, brightness/contrast adjustments built-in.
- [ ] **Config file** - Save/load user settings and calibration data.
