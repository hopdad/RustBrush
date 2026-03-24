# Configuration File

RustBrush saves your GUI settings to a JSON configuration file that persists between sessions.

## File Location

| Path |
|------|
| `C:\Users\<username>\.rustbrush\config.json` |

The file is created automatically when you first change a setting in the GUI.

## Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `canvas_preset_idx` | integer | `1` | Index into the preset list (0-based). Default is Wooden Sign. |
| `use_custom_size` | boolean | `false` | Use custom dimensions instead of the selected preset. |
| `custom_width` | integer | `256` | Custom canvas width in pixels. |
| `custom_height` | integer | `128` | Custom canvas height in pixels. |
| `color_match` | string | `"ciede2000"` | Color matching algorithm. `"ciede2000"` or `"rgb"`. |
| `dither` | string | `"none"` | Dithering mode. `"none"`, `"floyd-steinberg"`, or `"ordered"`. |
| `strategy` | string | `"hybrid"` | Painting strategy. `"hybrid"`, `"color-grouped"`, `"line-draw"`, or `"scanline"`. |
| `alpha_threshold` | integer | `128` | Pixels with alpha below this value are skipped (0-255). |
| `delay_ms` | integer | `15` | Delay between mouse actions in milliseconds. |
| `hex_input` | boolean | `false` | Type hex codes instead of clicking palette colors. |
| `save_session` | boolean | `true` | Auto-save session progress for crash recovery. |
| `adaptive_palette` | boolean | `false` | Use K-means++ adaptive palette instead of the fixed Rust palette. |
| `adaptive_colors` | integer | `256` | Number of colors for the adaptive palette. |
| `brightness` | float | `1.0` | Image brightness multiplier. `1.0` = unchanged. |
| `contrast` | float | `1.0` | Image contrast multiplier. `1.0` = unchanged. |
| `saturation` | float | `1.0` | Image saturation multiplier. `1.0` = unchanged. |
| `quality_preset` | string | `"balanced"` | Quality preset. `"speed"`, `"balanced"`, `"quality"`, `"maximum"`, or `"custom"`. |
| `blur_enabled` | boolean | `false` | Enable Gaussian blur filter. |
| `blur_sigma` | float | `1.0` | Gaussian blur strength (0.5-5.0). |
| `posterize_enabled` | boolean | `false` | Enable posterize filter. |
| `posterize_levels` | integer | `8` | Posterize levels per channel (2-32). |
| `median_enabled` | boolean | `false` | Enable median filter. |
| `median_radius` | integer | `1` | Median filter radius (1-3). |
| `path_optimizer` | boolean | `true` | Enable 2-opt path optimization to minimize mouse travel. |

## Example

```json
{
  "canvas_preset_idx": 1,
  "use_custom_size": false,
  "custom_width": 256,
  "custom_height": 128,
  "color_match": "ciede2000",
  "dither": "none",
  "strategy": "hybrid",
  "alpha_threshold": 128,
  "delay_ms": 15,
  "hex_input": false,
  "save_session": true,
  "adaptive_palette": false,
  "adaptive_colors": 256,
  "brightness": 1.0,
  "contrast": 1.0,
  "saturation": 1.0,
  "quality_preset": "balanced",
  "blur_enabled": false,
  "blur_sigma": 1.0,
  "posterize_enabled": false,
  "posterize_levels": 8,
  "median_enabled": false,
  "median_radius": 1,
  "path_optimizer": true
}
```

## Resetting to Defaults

Delete the config file and relaunch the GUI:

```
del %USERPROFILE%\.rustbrush\config.json
```

The CLI does not use the config file — all settings are passed as command-line arguments.
