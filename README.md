# RustBrush

An automatic sign painter for [Rust](https://rust.facepunch.com/) (the game). Paint any image onto in-game signs automatically.

## Safety

RustBrush is designed with anti-cheat safety as a primary concern:

- **OS-level input only** - Uses `SendInput` (Windows) via the [enigo](https://crates.io/crates/enigo) crate. This is the same approach used by [Rustangelo](https://store.steampowered.com/app/527440/Rustangelo/) and [RustForge](https://store.steampowered.com/app/4266520/RustForge_Sign_Painter/).
- **No game memory access** - Never reads or writes to the Rust process memory.
- **No DLL injection** - No code is injected into the game.
- **No process hooking** - Never opens handles to or hooks into any game process.
- **Screen reading via OS APIs** - Uses standard screen capture APIs (same as Print Screen / Snipping Tool), not game memory.

### Important Disclaimer

> **This tool has NOT been officially whitelisted by Facepunch Studios or Easy Anti-Cheat (EAC).**
>
> According to Facepunch, the only way to get officially whitelisted is to publish on Steam. RustBrush is distributed via GitHub.
>
> While the input method used (OS-level `SendInput`) is the same as whitelisted tools and should not trigger EAC, **use this tool at your own risk**.
>
> **We strongly recommend testing on a private server first** by launching with `+server.secure 0` to disable EAC.

## Features

### Image Processing
- **Perceptual color matching (CIEDE2000)** - Accurate color reproduction using human-perceptual distance
- **Dithering** - Floyd-Steinberg error-diffusion and ordered (Bayer) dithering
- **Adaptive 512-color palette** - K-means++ clustering in Lab space for optimal hex colors
- **Image adjustments** - Brightness, contrast, saturation sliders
- **Image transforms** - Rotate CW/CCW, flip horizontal/vertical
- **Crop margins** - Percentage-based crop sliders applied before resize
- **Simplification filters** - Median filter, Gaussian blur, posterize for reducing detail
- **GIF/animation support** - Load GIF files, select frames, process multi-frame animations

### Painting
- **Interactive region capture** - Mark canvas (F9), palette (F8) areas on screen
- **Hex code input mode** - Type hex colors directly for exact reproduction
- **Palette scanning (F7)** - Detect available colors and click positions from the in-game palette
- **Canvas presets** - Built-in dimensions for all sign types (wooden signs, frames, banners, neon signs, etc.)
- **Painting strategies** - Hybrid, color-grouped, line-draw, and scanline strategies
- **Line drawing optimization** - Detects horizontal runs and uses shift-click for speed
- **Path optimizer (2-opt)** - Reorders paint segments to minimize total mouse travel
- **Adaptive delays** - Auto-tunes painting speed based on success/failure
- **Pause/Resume/Cancel** - F10 to pause/resume, ESC to cancel
- **Progress save/resume** - JSON session files with auto-save, `--resume` flag
- **Quality presets** - Speed/Balanced/Quality/Maximum one-click configurations

### Content Creation
- **Text-to-image builder** - Type text, choose font/size/color/alignment, render as source image
- **Clipart library** - Built-in gallery of 24 icons (arrows, warnings, symbols, borders, text labels)

### GUI
- **Full GUI mode** - egui/eframe window with image preview, settings, and live plan generation
- **Preview A/B toggle** - Side-by-side, original-only, or preview-only display modes
- **Palette color strip** - Visual bar showing proportional color distribution
- **Config file** - Persistent settings saved at `~/.rustbrush/config.json`

## Installation

### Download (Windows)

Grab the latest `rustbrush-windows-x86_64.zip` from [GitHub Releases](https://github.com/hopdad/RustBrush/releases). The zip contains two binaries:

- **`rustbrush.exe`** - CLI mode
- **`rustbrush-gui.exe`** - GUI mode (recommended for most users)

No installer needed — just extract and run.

### Build from Source

```bash
git clone https://github.com/hopdad/RustBrush.git
cd RustBrush
cargo build --release
```

No extra dependencies are needed. Binaries will be at `target/release/rustbrush.exe` and `target/release/rustbrush-gui.exe`.

## Usage

### GUI Mode (Recommended)

```bash
rustbrush-gui
```

The GUI provides image preview, settings panels, live paint plan generation, and interactive region capture — all in one window.

### CLI Mode

```bash
# Dry run - preview what would be painted (no input sent)
rustbrush myimage.png --dry-run

# Paint with default settings
rustbrush myimage.png --accept-risk
# 1. Switch to Rust with sign editor open
# 2. Press F9, then click and drag to select the CANVAS area
# 3. Press F8, then click and drag to select the COLOR PALETTE area
# 4. Painting begins after countdown
# 5. F10 = pause/resume, ESC = cancel

# Use a canvas preset
rustbrush myimage.png --preset "wooden sign" --accept-risk

# Custom canvas size with dithering and hex input
rustbrush myimage.png -W 128 -H 128 --dither floyd-steinberg --hex-input --accept-risk

# Resume an interrupted session
rustbrush myimage.png --resume session.json --accept-risk

# Save a preview image before painting
rustbrush myimage.png --preview preview.png --dry-run
```

## Documentation

- **[Getting Started](docs/getting-started.md)** — Install and paint your first sign
- **[User Guide](docs/user-guide.md)** — Full GUI walkthrough
- **[CLI Reference](docs/cli-reference.md)** — All command-line flags and examples
- **[Canvas Presets](docs/canvas-presets.md)** — All 18 sign types with dimensions
- **[Strategies and Quality](docs/strategies-and-quality.md)** — Painting strategies, color matching, dithering, quality presets
- **[Configuration](docs/config.md)** — Config file location, fields, and defaults
- **[Troubleshooting](docs/troubleshooting.md)** — Common problems and solutions
- **[Contributing](CONTRIBUTING.md)** — Build, test, and submit changes
- **[Changelog](CHANGELOG.md)** — Release history

## Architecture

RustBrush is organized as a three-crate Cargo workspace:

```
crates/
├── rustbrush-core/        # Pure library: color science, image processing, paint planning
│   └── src/
│       ├── color/         # CIEDE2000, palette, color matching, dithering, adaptive palette
│       ├── image/         # Loading, resize, adjustments, transforms, filters
│       ├── painting/      # PaintPlan, PaintCommand, strategies, optimizer
│       ├── canvas/        # Sign dimension presets
│       ├── text/          # Text-to-image rendering
│       ├── library/       # Built-in clipart catalog and generation
│       └── session/       # Save/resume state
├── rustbrush-platform/    # OS-specific: input simulation, screen capture, hotkeys
│   └── src/
│       ├── input/         # InputDriver trait + enigo implementation + dry-run driver
│       ├── capture/       # Screen capture and palette sampling
│       ├── executor/      # PaintPlan execution with pause/cancel/resume
│       └── hotkey/        # Global hotkey listener + interactive region capture
└── rustbrush-app/         # Application (CLI + GUI)
    └── src/
        ├── main.rs        # CLI argument parsing and orchestration
        └── gui.rs         # egui/eframe GUI application
```

**Why three crates:** Core logic is testable without hardware. Platform layer handles OS input. CLI/GUI is decoupled from algorithms.

## How It Works

1. **Load** - Reads the input image and resizes it to the target canvas dimensions (Lanczos3)
2. **Adjust** (optional) - Apply brightness, contrast, saturation, crop, rotation, and simplification filters
3. **Map** - Converts each pixel to the nearest color in the palette (CIEDE2000 or RGB distance)
4. **Dither** (optional) - Applies Floyd-Steinberg or ordered dithering for better visual quality
5. **Plan** - Groups pixels by color, builds paint segments with line detection, optimizes path order
6. **Capture** - User marks the canvas and palette regions on screen with hotkeys
7. **Paint** - Executes the paint plan command-by-command with adaptive delays and pause/cancel support

## License

MIT
