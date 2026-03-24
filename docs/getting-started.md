# Getting Started

This guide walks you through installing RustBrush and painting your first sign.

## Installation

### Windows (Recommended)

1. Download `rustbrush-windows-x86_64.zip` from [GitHub Releases](https://github.com/hopdad/RustBrush/releases)
2. Extract the zip anywhere (e.g., your Desktop)
3. You'll see two files:
   - **`rustbrush-gui.exe`** — GUI mode (recommended)
   - **`rustbrush.exe`** — CLI mode

No installer needed. Just run the `.exe` files directly.

### Build from Source

```bash
git clone https://github.com/hopdad/RustBrush.git
cd RustBrush
cargo build --release
```

On Linux, install dependencies first:
```bash
sudo apt install libxdo-dev libxcb1-dev
```

Binaries are output to `target/release/rustbrush` and `target/release/rustbrush-gui`.

## Before You Start

> **Anti-cheat warning:** RustBrush has NOT been whitelisted by Facepunch or EAC. We strongly recommend testing on a private server first. Launch your server with `+server.secure 0` to disable EAC.

See [Troubleshooting](troubleshooting.md#anti-cheat--eac) for more details on anti-cheat safety.

## Your First Paint (GUI)

### 1. Launch the GUI

Double-click `rustbrush-gui.exe` (Windows) or run `rustbrush-gui` from the terminal.

### 2. Load an Image

Click **Load Image** and select a PNG, JPG, or other supported image file.

Your image will appear in the preview panel on the left. The right panel shows settings.

### 3. Pick a Canvas Preset

From the **Canvas Preset** dropdown, select the sign type you want to paint (e.g., "Wooden Sign" at 256x128). The preview will update to show how your image looks at that resolution.

See [Canvas Presets](canvas-presets.md) for the full list of sign types.

### 4. Choose a Quality Preset

For your first paint, use the **Balanced** quality preset. This gives good results at reasonable speed. You can experiment with other presets later — see [Strategies and Quality](strategies-and-quality.md).

### 5. Try a Dry Run First

Before painting on a real server, do a dry run to see how the result will look:

1. Check the preview image — does it look acceptable?
2. Look at the **Plan Info** section to see how many commands and estimated time

### 6. Set Up the Game

1. Open Rust and go to the sign you want to paint
2. Open the sign editor (press E on the sign)
3. Select the **Brush** tool
4. Make sure you can see both the canvas and the color palette

### 7. Capture Regions

With the GUI running alongside the game:

1. Press **F9**, then click and drag on the game screen to select the **canvas area** (the paintable surface)
2. Press **F8**, then click and drag to select the **color palette area** (the row of colors at the bottom)
3. If using hex input mode: press **F7** and click on the hex color input field

### 8. Paint

1. Click the **Paint** button in the GUI
2. A countdown starts (default 3 seconds) — switch to the game window
3. Make sure the brush tool is selected and the sign editor is focused
4. Painting begins automatically

### 9. During Painting

- **F10** — Pause or resume painting
- **ESC** — Cancel painting

Don't move the mouse or interact with the game while painting is in progress.

## Your First Paint (CLI)

### Quick Start

```bash
# Dry run to preview (no input sent, no risk)
rustbrush myimage.png --preset "wooden sign" --dry-run

# Paint for real
rustbrush myimage.png --preset "wooden sign" --accept-risk
```

### Step by Step

1. Run the command — RustBrush shows a safety disclaimer (use `--accept-risk` to skip)
2. Switch to the game with the sign editor open
3. Press **F9** and drag to mark the canvas
4. Press **F8** and drag to mark the palette
5. Painting starts after the countdown

See [CLI Reference](cli-reference.md) for all available options.

## What's Next

- [User Guide](user-guide.md) — Full walkthrough of all GUI features
- [CLI Reference](cli-reference.md) — All command-line flags and examples
- [Strategies and Quality](strategies-and-quality.md) — Tune painting speed vs. quality
- [Troubleshooting](troubleshooting.md) — Common issues and solutions
