# User Guide

Complete reference for the RustBrush GUI. For a quick tutorial, see [Getting Started](getting-started.md).

## Overview

The GUI window is split into two main areas:
- **Left panel** — Image preview with source/preview toggle
- **Right panel** — Settings, controls, and paint plan info

## Loading Images

### From File

Click **Load Image** to open a file picker. Supported formats: PNG, JPG, BMP, GIF, TIFF, WebP.

### Text Builder

Create text-based signs without an external image editor:

1. Open the **Text Builder** section
2. Type your text in the input field
3. Configure:
   - **Font** — Select from system fonts
   - **Font size** — Adjust with the slider
   - **Text color** — Click to pick a color
   - **Background color** — Click to pick, or enable **transparent background**
   - **Alignment** — Left, center, or right
   - **Padding** — Space around the text in pixels
4. The preview updates in real time
5. Click **Use Text Image** to load it as the source image

### Clipart Library

RustBrush includes 24 built-in clipart icons:

| Category | Items |
|----------|-------|
| **Arrows** | Up, Down, Left, Right |
| **Warning Signs** | Danger Triangle, Radiation, No Entry, High Voltage, Skull, Biohazard |
| **Symbols** | Checkmark, X Mark, Heart, Star, Peace, Crosshair |
| **Borders** | Simple Border, Double Border, Corner Frame, Dashed Border |
| **Text Labels** | Keep Out, Danger, Private, Shop |

Select a category, click a clipart thumbnail, and it loads as the source image.

### GIF / Animation

For animated signs (Large Animated Neon Sign):

1. Load a `.gif` file
2. RustBrush extracts all frames
3. Use the frame selector to preview individual frames
4. Each frame is painted as a separate pass

## Image Adjustments

### Brightness, Contrast, Saturation

Three sliders that modify the image before color mapping:

- **Brightness** — Multiplier on pixel luminance. `1.0` = unchanged, `>1.0` = brighter
- **Contrast** — Multiplier on contrast around the midpoint. `1.0` = unchanged
- **Saturation** — Multiplier on color intensity. `0.0` = grayscale, `1.0` = unchanged

### Crop

Four percentage-based crop sliders (0-50% each):
- **Top / Bottom / Left / Right** — Trim edges before resizing

Cropping is applied before the image is resized to canvas dimensions, so it affects the final composition.

### Rotation and Flip

Available as image transforms (via the GUI):
- Rotate 90° clockwise / counter-clockwise / 180°
- Flip horizontal / vertical

## Filters

Simplification filters reduce image detail, which can improve painting results for complex photos.

### Gaussian Blur

Smooths the image by averaging nearby pixels with a Gaussian kernel.

- **Sigma** (0.5–5.0) — Controls blur strength. Higher = smoother.
- Good for: reducing noise, softening sharp transitions

### Median Filter

Replaces each pixel with the median of its neighbors. Preserves edges better than blur.

- **Radius** (1–3) — Size of the neighborhood.
- Good for: removing noise while keeping edges sharp

### Posterize

Reduces the number of color levels per channel, creating a flat/poster-like appearance.

- **Levels** (2–32) — Fewer levels = more dramatic effect.
- Good for: simplifying complex images, creating stylized looks

## Painting Settings

### Canvas Preset

Select from 18 built-in sign presets. See [Canvas Presets](canvas-presets.md) for the full list.

Check **Use custom size** to enter arbitrary width and height values.

### Strategy

Choose a painting strategy. See [Strategies and Quality](strategies-and-quality.md) for detailed descriptions.

- **Hybrid** (default) — Fastest, combines all techniques
- **Color-Grouped** — Groups by color, good for solid areas
- **Line-Draw** — Uses shift-click for horizontal runs
- **Scanline** — Simple row-by-row

### Color Matching

- **CIEDE2000** (default) — Perceptual color distance, more accurate
- **RGB** — Euclidean distance, faster but less accurate

### Dithering

- **None** (default) — Direct color mapping
- **Floyd-Steinberg** — Error diffusion, best visual quality
- **Ordered** — Bayer pattern, stylized look

### Alpha Threshold

Slider (0–255). Pixels with alpha below this value are skipped. Default: 128.

Useful for images with transparency — set higher to skip more semi-transparent pixels.

### Skip Color

Enable and enter a hex color code (e.g., `FFFFFF`) to skip all pixels matching that color. Useful for removing solid backgrounds.

### Hex Input

When enabled, RustBrush types hex color codes directly into the game's color input field instead of clicking palette positions. More accurate but requires marking the hex input field location (F7).

### Painting Delay

Slider controlling the delay between mouse actions in milliseconds. Default: 15ms.

Lower = faster painting but more risk of missed inputs. The executor also uses adaptive delay — it automatically slows down if it detects failures.

### Adaptive Palette

Toggle to use K-means++ clustering to find the optimal color subset for your image. Configure the number of colors (default: 256).

### Path Optimizer

Toggle the 2-opt path optimizer. Reduces mouse travel distance by reordering paint segments. Enabled by default.

### Quality Presets

One-click configurations. See [Strategies and Quality](strategies-and-quality.md#quality-presets) for what each configures.

## Preview

### Display Modes

Toggle between three preview modes:
- **Side by side** — Original and quantized preview next to each other
- **Original only** — Full-size source image
- **Preview only** — Full-size quantized result

### Palette Strip

A color bar showing the proportional distribution of colors in the paint plan. Wider segments = more pixels of that color.

## Painting

### Region Capture

Before painting, you must mark regions on the game screen:

| Hotkey | Region | Description |
|--------|--------|-------------|
| **F9** | Canvas | Click and drag to mark the paintable canvas area |
| **F8** | Palette | Click and drag to mark the color palette area |
| **F7** | Hex Input | Click the hex color input field (only if hex input is enabled) |

After pressing the hotkey, click one corner and drag to the opposite corner to define the region.

### Palette Scanning

After marking the palette region (F8), RustBrush captures the screen and samples the palette to detect available colors and their click positions.

### Painting Execution

1. Click **Paint** in the GUI
2. A countdown timer starts (default: 3 seconds)
3. Switch to the game window — make sure the brush tool is selected
4. Painting proceeds automatically

### Controls During Painting

| Hotkey | Action |
|--------|--------|
| **F10** | Pause or resume painting |
| **ESC** | Cancel painting immediately |

The progress bar shows completion percentage and estimated time remaining.

### Session Save / Resume

When **Save session** is enabled (default), RustBrush periodically saves progress to a JSON file. If painting is interrupted (crash, ESC, disconnect), you can resume later from where it left off.

Session files are saved in `~/.rustbrush/sessions/`.

## Plan Info

After generating a paint plan, the GUI shows:
- **Strategy** used
- **Total commands** in the plan
- **Estimated time** based on current delay settings
- **Path optimization** improvement (if the optimizer ran)
- **Color breakdown** — number of pixels per color
