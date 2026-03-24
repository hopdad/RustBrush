# Strategies and Quality

How RustBrush converts your image to paint commands, and how to tune the tradeoff between speed and visual quality.

## Painting Strategies

A strategy determines the order in which pixels are painted. All strategies produce the same final image — they differ in speed and mouse travel efficiency.

### Scanline

Paints row by row, left to right, top to bottom. Switches colors as needed for each pixel.

- **Simplest** approach
- **Most color switches** — changes color for every pixel that differs from the previous
- Best for: debugging, very small images

```bash
rustbrush image.png --strategy scanline
```

### Color-Grouped

Groups all pixels by color, then paints each color group using nearest-neighbor ordering to minimize mouse travel within the group.

- **Fewer color switches** — switches once per color
- **Good mouse efficiency** within each group
- Best for: images with large solid-color areas

```bash
rustbrush image.png --strategy color-grouped
```

### Line-Draw

Detects horizontal runs of 3 or more same-color pixels and uses shift-click to draw lines instead of clicking individual pixels. Falls back to single-pixel clicks for scattered pixels.

- **Faster** for images with horizontal patterns
- Uses Rust's built-in shift-click line drawing
- Best for: text, geometric art, pixel art with horizontal runs

```bash
rustbrush image.png --strategy line-draw
```

### Hybrid (Default)

Combines the best aspects of all strategies:
1. Groups pixels by color (like color-grouped)
2. Detects horizontal runs for line drawing (like line-draw)
3. Orders segments using nearest-neighbor
4. Applies 2-opt path optimization to reduce total mouse travel

- **Fastest** overall — fewest commands and shortest mouse path
- Default and recommended for most images
- Best for: everything

```bash
rustbrush image.png --strategy hybrid
```

## Color Matching

When mapping your image to the in-game palette, RustBrush needs to find the closest available color for each pixel.

### CIEDE2000 (Default)

A perceptually uniform color distance metric. Colors that look similar to humans are measured as "close" even if their RGB values differ significantly.

- **More accurate** color reproduction
- **Slower** to compute (~3x slower than RGB)
- Recommended for photos and artwork

### RGB (Euclidean)

Simple Euclidean distance in RGB color space. Fast but can produce visually wrong matches — two colors that look very different to humans might have similar RGB distance.

- **Faster** computation
- **Less accurate** for some color pairs (especially greens and blues)
- Acceptable for speed-focused presets or images with limited colors

```bash
rustbrush photo.png --color-match ciede2000  # perceptual (default)
rustbrush photo.png --color-match rgb         # fast
```

## Dithering

Dithering distributes quantization error across neighboring pixels, creating the illusion of colors that aren't in the palette. It significantly improves visual quality for photos and gradients.

### None (Default)

No dithering. Each pixel is mapped to its closest palette color independently. Can produce visible color banding in gradients.

### Floyd-Steinberg

Error-diffusion dithering. After mapping a pixel, the quantization error is distributed to neighboring pixels (right, below-left, below, below-right). Produces natural-looking results with minimal visible patterns.

- **Best quality** dithering for photos
- Slightly more scattered pixel distribution (more mouse travel)

### Ordered (Bayer)

Pattern-based dithering using a Bayer matrix. Adds a fixed threshold pattern to each pixel before quantization. Produces a regular, grid-like dithering pattern.

- **Faster** to compute than Floyd-Steinberg
- Visible pattern can look stylized or retro
- Good for pixel art or deliberate aesthetic effects

```bash
rustbrush photo.png --dither none             # no dithering
rustbrush photo.png --dither floyd-steinberg   # best quality
rustbrush photo.png --dither ordered           # patterned/retro
```

## Adaptive Palette

By default, RustBrush uses the fixed Rust in-game palette. The **adaptive palette** feature uses K-means++ clustering in Lab color space to find the optimal subset of colors for your specific image.

- Enable in the GUI with the **Adaptive Palette** toggle
- Configure the number of colors (default: 256, max: 512)
- Produces better color representation for images that don't match the default palette well
- Only available in the GUI

## Path Optimizer (2-opt)

When enabled (default), the Hybrid strategy applies a 2-opt local search after nearest-neighbor ordering. This reverses segments of the path when doing so would reduce total mouse travel distance.

- Typically reduces mouse travel by **10-30%** on dithered or scattered images
- Runs automatically during plan generation — no extra wait
- For large segment counts (>1000), limited to 3 improvement passes to cap computation time
- Toggle in the GUI under advanced settings, or it's always on for CLI Hybrid strategy

## Quality Presets

The GUI provides one-click quality presets that configure multiple settings at once:

| Preset | Strategy | Color Match | Dither | Palette Colors | Notes |
|--------|----------|-------------|--------|---------------|-------|
| **Speed** | Hybrid | RGB | None | 32 | Fastest painting |
| **Balanced** | Hybrid | RGB | Ordered | 32 | Good tradeoff (default) |
| **Quality** | Color-Grouped | CIEDE2000 | Floyd-Steinberg | 128 | High fidelity |
| **Maximum** | Scanline | CIEDE2000 | Floyd-Steinberg | 512 | Best possible output |
| **Custom** | — | — | — | — | Configure each setting individually |

Selecting a preset overrides the individual settings. Switch to **Custom** to tweak each option independently.

### Choosing a Preset

- **Speed** — Quick paints for testing or low-detail signs (small wooden signs, banners)
- **Balanced** — Default, good for most use cases
- **Quality** — Photos, detailed artwork, larger signs
- **Maximum** — When you want the absolute best result and don't mind waiting
