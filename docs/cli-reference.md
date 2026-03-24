# CLI Reference

Full reference for `rustbrush` command-line options.

## Synopsis

```
rustbrush <IMAGE> [OPTIONS]
```

## Arguments

### Required

| Argument | Description |
|----------|-------------|
| `<IMAGE>` | Path to the image file to paint (PNG, JPG, BMP, GIF, etc.) |

### Canvas Size

| Flag | Default | Description |
|------|---------|-------------|
| `-W, --canvas-width <N>` | `256` | Canvas width in pixels |
| `-H, --canvas-height <N>` | `256` | Canvas height in pixels |
| `--preset <NAME>` | — | Use a canvas preset by name (e.g., `"wooden sign"`) |

When `--preset` is set, it overrides `-W` and `-H`. See [Canvas Presets](canvas-presets.md) for all available presets.

### Color Processing

| Flag | Default | Description |
|------|---------|-------------|
| `--color-match <ALG>` | `ciede2000` | Color matching: `ciede2000` (perceptual) or `rgb` (fast) |
| `--dither <MODE>` | `none` | Dithering: `none`, `floyd-steinberg`, or `ordered` |
| `--alpha-threshold <N>` | `128` | Pixels with alpha below this are skipped (0-255) |
| `--skip-color <HEX>` | — | Skip a background color (e.g., `FFFFFF` or `#FF0000`) |

### Painting

| Flag | Default | Description |
|------|---------|-------------|
| `--strategy <NAME>` | `hybrid` | Painting strategy: `hybrid`, `color-grouped`, `line-draw`, or `scanline` |
| `-d, --delay-ms <N>` | `15` | Delay between mouse actions in milliseconds |
| `--hex-input` | off | Type hex codes instead of clicking palette colors |
| `-s, --startup-delay <N>` | `3` | Seconds to wait before painting starts |

See [Strategies and Quality](strategies-and-quality.md) for strategy details.

### Session Management

| Flag | Default | Description |
|------|---------|-------------|
| `--save-session` | off | Auto-save session progress for crash recovery |
| `--resume <FILE>` | — | Resume a previously interrupted session from a JSON file |

### Output & Safety

| Flag | Default | Description |
|------|---------|-------------|
| `--dry-run` | off | Process image and show stats without capturing or painting |
| `--preview <PATH>` | — | Save a preview of the quantized image before painting |
| `--accept-risk` | off | Skip the safety disclaimer confirmation |

## Examples

### Basic Usage

```bash
# Paint an image onto a wooden sign
rustbrush photo.png --preset "wooden sign" --accept-risk

# Dry run to see how it would look (no input sent)
rustbrush logo.png --preset "portrait frame" --dry-run
```

### Quality Tuning

```bash
# High quality: perceptual color matching + Floyd-Steinberg dithering
rustbrush photo.png --preset "xl picture frame" \
  --color-match ciede2000 --dither floyd-steinberg --accept-risk

# Fast mode: RGB matching, no dithering
rustbrush banner.png --preset "large banner" \
  --color-match rgb --accept-risk

# Ordered dithering for a retro pattern effect
rustbrush pixel-art.png -W 128 -H 128 --dither ordered --accept-risk
```

### Hex Input Mode

For exact color reproduction, use hex input mode. This types hex codes directly into the game's color input field instead of clicking palette positions:

```bash
rustbrush photo.png --preset "neon sign" --hex-input --accept-risk
```

During region capture, you'll be asked to mark the hex input field (F7) in addition to the canvas (F9) and palette (F8).

### Transparent Images

```bash
# Skip pixels with alpha < 200 (more aggressive transparency)
rustbrush logo.png --alpha-threshold 200 --accept-risk

# Skip white background pixels
rustbrush logo.png --skip-color FFFFFF --accept-risk
```

### Session Save & Resume

```bash
# Start painting with auto-save enabled
rustbrush huge-mural.png --preset "xxl picture frame" \
  --save-session --accept-risk

# If interrupted (crash, ESC, disconnect), resume later:
rustbrush huge-mural.png --resume ~/.rustbrush/sessions/huge-mural_20250115.json \
  --accept-risk
```

### Preview Export

```bash
# Save a preview image to see the quantized result before painting
rustbrush photo.png --preset "wooden sign" --preview preview.png --dry-run
```

## Workflow

1. Run `rustbrush` with your image and options
2. Accept the safety disclaimer (or use `--accept-risk`)
3. Switch to the Rust game with the sign editor open
4. Press **F9**, click and drag to select the canvas area
5. If using `--hex-input`: press **F7**, click the hex input field
6. Press **F8**, click and drag to select the palette area
7. Painting begins after the startup delay countdown
8. During painting: **F10** = pause/resume, **ESC** = cancel

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (or dry run completed) |
| 1 | Error (invalid args, image load failure, unknown preset) |
