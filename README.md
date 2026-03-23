# RustBrush

An automatic sign painter for [Rust](https://rust.facepunch.com/) (the game). Paint any image onto in-game signs automatically.

## Safety

RustBrush is designed with anti-cheat safety as a primary concern:

- **OS-level input only** - Uses `SendInput` (Windows) / `xdotool` (Linux) via the [enigo](https://crates.io/crates/enigo) crate. This is the same approach used by [Rustangelo](https://store.steampowered.com/app/527440/Rustangelo/) and [RustForge](https://store.steampowered.com/app/4266520/RustForge_Sign_Painter/).
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

- **Perceptual color matching (CIEDE2000)** - Accurate color reproduction using human-perceptual distance
- **Dithering** - Floyd-Steinberg error-diffusion and ordered (Bayer) dithering for quality with limited palettes
- **Interactive region capture** - Mark your canvas and palette areas on screen (F9/F8 hotkeys)
- **Hex code input mode** - Type hex colors directly for exact reproduction (--hex-input)
- **Auto palette sampling** - Reads actual colors from your screen
- **Color-grouped painting** - Groups pixels by color with nearest-neighbor ordering to minimize palette switches and mouse travel
- **Canvas presets** - Built-in dimensions for all sign types (wooden signs, frames, banners, neon signs, etc.)
- **Pause/Resume/Cancel** - F10 to pause/resume, ESC to cancel mid-paint
- **Dry-run mode** - Preview what would be painted without sending any input
- **Image adjustments** - Brightness, contrast, and saturation controls
- **Configurable speed** - Adjust delay between mouse actions

## Usage

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

# Fast RGB color matching (less accurate but faster)
rustbrush myimage.png --color-match rgb --accept-risk

# Save a preview image before painting
rustbrush myimage.png --preview preview.png --dry-run
```

## Building

```bash
# Dependencies (Linux)
sudo apt install libxdo-dev libxcb1-dev

# Clone and build
git clone https://github.com/hopdad/RustBrush.git
cd RustBrush
cargo build --release

# The binary will be at target/release/rustbrush
```

## Architecture

RustBrush is organized as a three-crate Cargo workspace:

```
crates/
├── rustbrush-core/        # Pure library: color science, image processing, paint planning
│   └── src/
│       ├── color/         # CIEDE2000, palette definitions, color matching, dithering
│       ├── image/         # Loading, resize, brightness/contrast/saturation
│       ├── painting/      # PaintPlan, PaintCommand, strategies (scanline, color-grouped)
│       ├── canvas/        # Sign dimension presets for all sign types
│       └── session/       # Save/resume state
├── rustbrush-platform/    # OS-specific: input simulation, screen capture, hotkeys
│   └── src/
│       ├── input/         # InputDriver trait + enigo implementation + dry-run driver
│       ├── capture/       # Screen capture and palette sampling
│       └── hotkey/        # Global hotkey listener + interactive region capture
└── rustbrush-app/         # CLI application
    └── src/
        └── main.rs        # CLI argument parsing and orchestration
```

**Why three crates:** Core logic is testable on any OS (CI on Linux). Platform layer is swappable. CLI is decoupled from algorithms.

## How It Works

1. **Load** - Reads the input image and resizes it to the target canvas dimensions (Lanczos3)
2. **Map** - Converts each pixel to the nearest color in Rust's 32-color palette (CIEDE2000 or RGB distance)
3. **Dither** (optional) - Applies Floyd-Steinberg or ordered dithering for better visual quality
4. **Group** - Groups pixels by color, sorts by frequency, orders within groups by nearest-neighbor
5. **Capture** - User marks the canvas and palette regions on screen with hotkeys
6. **Paint** - For each color group: selects color (palette click or hex input) → paints all pixels → checks for pause/cancel

## License

MIT
