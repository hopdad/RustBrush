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

- **Interactive region capture** - Mark your canvas and palette areas on screen (like RustForge's F9/F10)
- **Auto palette sampling** - Reads actual colors from your screen instead of hardcoding
- **Color-grouped painting** - Groups pixels by color to minimize palette switches
- **Pause/Resume/Cancel** - F10 to pause/resume, ESC to cancel mid-paint
- **Dry-run mode** - Preview what would be painted without sending any input
- **Configurable speed** - Adjust delay between mouse actions
- **Progress reporting** - Real-time progress updates during painting

## Usage

```bash
# Dry run - preview what would be painted (no input sent)
rustbrush myimage.png --dry-run

# Paint with default settings
rustbrush myimage.png --accept-risk
# 1. Switch to Rust with sign editor open
# 2. Press F9 at top-left of canvas, then F9 at bottom-right
# 3. Press F8 at top-left of palette, then F8 at bottom-right
# 4. Painting begins after countdown
# 5. F10 = pause/resume, ESC = cancel

# Custom canvas size and speed
rustbrush myimage.png -W 128 -H 128 --delay-ms 20 --accept-risk
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

## How It Works

1. **Load** - Reads the input image and resizes it to the target dimensions
2. **Map** - Converts each pixel to the nearest color in Rust's in-game palette
3. **Group** - Groups pixels by color to minimize palette switches
4. **Capture** - User marks the canvas and palette regions on screen with hotkeys
5. **Sample** - Reads actual palette colors from the screen
6. **Paint** - For each color group:
   - Clicks the matching color in the on-screen palette
   - Clicks each pixel position on the canvas
   - Checks for pause/cancel between actions

## Architecture

```
main.rs      - CLI, image loading, orchestration
color.rs     - Palette definition, color matching (RGB Euclidean distance)
input.rs     - OS-level input simulation via enigo (SendInput/xdotool)
screen.rs    - Screen capture via OS APIs (never touches game memory)
region.rs    - Interactive canvas/palette region capture
hotkeys.rs   - Background hotkey listener (F10 pause, ESC cancel)
painter.rs   - Painting engine with color grouping and progress tracking
```

## Development Phases

See [PLAN.md](PLAN.md) for the full roadmap. Current status:

- [x] **Phase 1: Core Foundation** - End-to-end painting with interactive region capture
- [ ] **Phase 2: Image Quality** - CIEDE2000 color matching, dithering, previews
- [ ] **Phase 3: Performance** - Line drawing optimization, adaptive delays, save/resume
- [ ] **Phase 4: Polish** - GUI mode, multiple sign sizes, built-in editor

## License

MIT
