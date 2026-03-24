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
- [ ] **Text-to-image builder** - Type text, choose system font/size/color/alignment, render as source image
- [ ] **Stock image library** - Built-in gallery of embedded clipart (arrows, warnings, icons, borders) for signs

### Phase 6 Implementation Details

#### Feature A: Text-to-Image Builder

**Goal**: Let users type text and render it directly as the source image, sized to the current canvas dimensions. Uses system fonts.

**New dependencies** (rustbrush-core):
- `ab_glyph = "0.2"` — Pure-Rust font rasterization (renders glyphs to pixels)
- `imageproc = "0.25"` — Provides `draw_text_mut()` for rendering text onto an `RgbaImage`

**New dependencies** (rustbrush-app):
- `font-kit = "0.14"` — Cross-platform system font discovery (enumerate installed fonts)

**New module**: `crates/rustbrush-core/src/text/mod.rs`
```rust
pub struct TextConfig {
    pub text: String,
    pub font_data: Vec<u8>,        // Raw TTF/OTF bytes loaded from system
    pub font_size: f32,            // In pixels
    pub text_color: [u8; 4],       // RGBA
    pub bg_color: [u8; 4],         // RGBA (supports transparent)
    pub alignment: TextAlign,      // Left, Center, Right
    pub padding: u32,              // Pixels of padding around text
}

pub enum TextAlign { Left, Center, Right }

/// Render text to an RgbaImage of the given dimensions.
/// Handles multi-line text (split on \n), word wrapping, and vertical centering.
pub fn render_text(config: &TextConfig, width: u32, height: u32) -> RgbaImage;
```

**Implementation approach**:
1. Create `RgbaImage` of canvas dimensions, fill with `bg_color`
2. Parse `font_data` into `ab_glyph::FontArc`
3. Split text on `\n`, compute line widths for alignment
4. Auto-size font if text doesn't fit (scale down to fit canvas)
5. Use `imageproc::drawing::draw_text_mut()` for each line
6. Return the `RgbaImage`

**GUI changes** (`gui.rs`):
- New fields on `RustBrushApp`:
  ```rust
  show_text_builder: bool,
  text_input: String,
  text_font_size: f32,
  text_color: [u8; 4],
  text_bg_color: [u8; 4],
  text_alignment: TextAlign,
  text_padding: u32,
  system_fonts: Vec<(String, PathBuf)>,  // (display name, path)
  selected_font_idx: usize,
  text_preview_texture: Option<egui::TextureHandle>,
  ```
- Font discovery: On app init, use `font-kit` to enumerate system fonts → populate `system_fonts` list
- New "Text Builder" button in the File menu (next to "Open Image...")
- When `show_text_builder` is true, show a modal panel over the preview area with:
  - Text input field (multi-line)
  - Font dropdown (system fonts list)
  - Font size slider (8-128px, default 32)
  - Text color picker
  - Background color picker (with transparency option)
  - Alignment radio buttons (Left/Center/Right)
  - Padding slider
  - Live preview of rendered text
  - "Apply" button → renders text, sets as `source_image`, closes builder
  - "Cancel" button → closes builder

**Integration**: The "Apply" action calls `render_text()`, then:
```rust
self.source_image = Some(rendered_img);
self.image_path = None;  // No file path for generated images
self.source_texture = None;
// ... same reset pattern as load_static_image()
self.mark_settings_changed(true);
```

#### Feature B: Stock Image / Clipart Library

**Goal**: Built-in gallery of pre-made images useful for Rust game signs. Embedded in binary via `include_bytes!`.

**New directory**: `crates/rustbrush-core/src/library/`
- `mod.rs` — Library catalog, image loading from embedded bytes
- Images embedded as PNG bytes

**New module**: `crates/rustbrush-core/src/library/mod.rs`
```rust
pub struct LibraryItem {
    pub name: &'static str,
    pub category: &'static str,
    pub data: &'static [u8],       // PNG bytes via include_bytes!
}

pub fn all_items() -> &'static [LibraryItem];
pub fn categories() -> Vec<&'static str>;
pub fn items_in_category(category: &str) -> Vec<&'static LibraryItem>;
pub fn load_item(item: &LibraryItem) -> Result<RgbaImage, String>;
```

**Stock image categories & items** (programmatically generated as simple vector graphics rendered to PNG):
1. **Warning Signs** — Radiation, skull/crossbones, high voltage, biohazard, no entry, exclamation
2. **Arrows** — Up, down, left, right, curved arrows, double arrows
3. **Symbols** — Checkmark, X mark, heart, star, peace sign, question mark
4. **Text Labels** — "KEEP OUT", "DANGER", "PRIVATE", "SHOP", "EXIT", "HELP"
5. **Borders/Frames** — Simple border, double border, decorative corner frame

**Asset generation approach**: Rather than designing images by hand, we generate them programmatically:
- Create a build script or a one-time generator using the `image` crate
- Draw simple shapes (circles, triangles, lines, text) to create clean, recognizable icons
- Save as small PNGs (64×64 or 128×128) in `assets/library/`
- Reference via `include_bytes!("../../../assets/library/arrow_up.png")` etc.

**GUI changes** (`gui.rs`):
- New fields:
  ```rust
  show_library: bool,
  library_category_idx: usize,
  library_thumbnails: Vec<Option<egui::TextureHandle>>,
  ```
- New "Clipart Library" button in File menu
- When `show_library` is true, overlay panel with:
  - Category tabs/buttons across the top
  - Grid of thumbnail images (lazy-loaded textures)
  - Click a thumbnail → loads full image as `source_image`, closes library
  - "Close" button
- Thumbnails are generated lazily (only load textures for visible items)

### Implementation Order

1. **Add dependencies** to both Cargo.toml files
2. **Create `text/mod.rs`** with `render_text()` function
3. **Create `library/mod.rs`** with catalog structure
4. **Generate stock images** — small PNG files in `assets/library/`
5. **Wire up Text Builder UI** in gui.rs (fields, modal panel, font discovery)
6. **Wire up Library UI** in gui.rs (fields, gallery panel, thumbnail loading)
7. **Test both features** — verify generated images feed correctly into painting pipeline
8. **Polish** — keyboard shortcuts, remembering last-used settings
