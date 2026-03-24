# Canvas Presets

RustBrush includes built-in dimension presets for every paintable sign type in Rust.

## Preset Table

| # | Preset Name | Width | Height | Notes |
|---|-------------|-------|--------|-------|
| 1 | Small Wooden Sign | 128 | 64 | |
| 2 | Wooden Sign | 256 | 128 | Default |
| 3 | Large Wooden Sign | 256 | 128 | |
| 4 | Huge Wooden Sign | 512 | 128 | |
| 5 | Two Sided Hanging Sign | 256 | 128 | |
| 6 | Small Banner | 64 | 256 | Vertical |
| 7 | Large Banner | 64 | 512 | Vertical |
| 8 | Portrait Frame | 256 | 256 | Square |
| 9 | Landscape Frame | 256 | 128 | |
| 10 | Tall Picture Frame | 128 | 256 | Vertical |
| 11 | XL Picture Frame | 512 | 512 | Square |
| 12 | XXL Picture Frame | 1024 | 512 | Largest sign |
| 13 | Spinning Wheel | 256 | 256 | Square |
| 14 | Small Neon Sign | 128 | 128 | Square |
| 15 | Neon Sign | 256 | 128 | |
| 16 | Large Neon Sign | 256 | 256 | Square |
| 17 | Large Animated Neon Sign | 256 | 256 | 5 frames |
| 18 | Photo Frame | 320 | 240 | 4:3 ratio |

## Usage

### GUI

Select a preset from the **Canvas Preset** dropdown in the settings panel. The canvas dimensions update automatically.

You can also check **Use custom size** to override the preset dimensions with your own width and height.

### CLI

```bash
rustbrush myimage.png --preset "wooden sign"
rustbrush myimage.png --preset "portrait frame"
rustbrush myimage.png --preset "large animated neon sign"
```

Preset names are case-insensitive. If you specify an invalid name, RustBrush prints all available presets.

You can also set dimensions manually with `-W` and `-H`:

```bash
rustbrush myimage.png -W 512 -H 512
```

## Animated Signs

The **Large Animated Neon Sign** has 5 animation frames. In the GUI, load a GIF file and RustBrush will extract individual frames for painting. Each frame is painted as a separate pass on the same canvas.

In CLI mode, animated signs are not directly supported — use the GUI for multi-frame painting.
