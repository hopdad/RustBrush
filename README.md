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

- Load any image and automatically paint it onto a Rust sign
- Maps colors to Rust's in-game palette using perceptual color matching
- Groups pixels by color for efficient painting
- Dry-run mode to preview without sending any input
- Configurable painting speed
- Startup delay to switch to the game window
- Progress reporting during painting

## Usage

```bash
# Dry run - preview what would be painted (no input sent)
rustbrush myimage.png --dry-run

# Paint with default settings (5 second startup delay)
rustbrush myimage.png --accept-risk

# Custom canvas size and speed
rustbrush myimage.png -W 128 -H 128 --delay-ms 20 --accept-risk

# Longer startup delay to switch windows
rustbrush myimage.png --startup-delay 10 --accept-risk
```

## Building

```bash
# Clone and build
git clone https://github.com/hopdad/RustBrush.git
cd RustBrush
cargo build --release

# The binary will be at target/release/rustbrush
```

## How It Works

1. **Load** - Reads the input image and resizes it to the canvas dimensions
2. **Map** - Converts each pixel to the nearest color in Rust's in-game palette
3. **Group** - Groups pixels by color to minimize color-picker switches
4. **Paint** - For each color group:
   - Selects the color via hex code entry in the color picker
   - Clicks each pixel position on the canvas

## Current Limitations

- Canvas position detection requires manual calibration (TODO: auto-detect)
- Color palette is based on Rust's post-November 2025 update and may need updating
- Windows-focused (Linux support via xdotool is available but less tested)

## License

MIT
