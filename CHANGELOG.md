# Changelog

All notable changes to RustBrush are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] - 2026-03-24

### Added
- Visual selection overlay during region capture — a semi-transparent green bounding box now appears in real-time as you click and drag to select canvas/palette areas
- Comprehensive documentation: getting-started guide, user guide, CLI reference, canvas presets, strategies and quality, configuration, troubleshooting
- LICENSE file (MIT)
- CONTRIBUTING.md with build/test/PR instructions
- CHANGELOG.md

### Changed
- Overlay is click-through so mouse events still reach the game
- README updated with documentation links section

### Removed
- Linux support — RustBrush now targets Windows only (Rust the game is Windows-only)

## [0.1.0] - 2025-01-15

### Added
- GUI mode with egui/eframe: image preview, settings panels, live plan generation
- CLI mode with full argument parsing
- CIEDE2000 perceptual color matching and RGB Euclidean matching
- Floyd-Steinberg and ordered (Bayer) dithering
- Adaptive palette with K-means++ clustering in Lab color space
- 4 painting strategies: Scanline, Color-Grouped, Line-Draw, Hybrid
- 2-opt path optimizer for minimizing mouse travel
- 18 canvas presets for all Rust sign types
- Interactive region capture with hotkeys (F7, F8, F9)
- Painting controls: pause/resume (F10), cancel (ESC)
- Session save and resume for crash recovery
- Image adjustments: brightness, contrast, saturation
- Image transforms: rotate, flip, crop
- Simplification filters: Gaussian blur, median, posterize
- Text-to-image builder with font selection and styling
- Clipart library with 24 built-in icons
- GIF/animation support with frame selection
- Quality presets: Speed, Balanced, Quality, Maximum
- Hex input mode for exact color reproduction
- Adaptive delay controller
- Config file persistence at ~/.rustbrush/config.json
- Preview A/B toggle and palette color strip
- Alpha threshold and skip-color options
