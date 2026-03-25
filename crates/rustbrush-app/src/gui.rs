//! RustBrush GUI - egui/eframe application for automatic sign painting.

use eframe::egui;
use rustbrush_core::canvas;
use rustbrush_core::color::{
    build_preview, generate_adaptive_palette, map_image_to_palette, palette_from_rgb, rust_palette,
    ColorMatchAlgo, DitherMode, MappedPixel, QuantizeOptions,
};
use rustbrush_core::config::Config;
use rustbrush_core::image as rb_image;
use rustbrush_core::library;
use rustbrush_core::painting::{self, ColorGroup, PaintCommand, PaintPlan, PaintStrategy, ScreenRect};
use rustbrush_platform::capture::{self, PaletteEntry};
use rustbrush_core::session::Session;
use rustbrush_core::text::{self, TextAlign, TextConfig};
use rustbrush_platform::executor::{self, ExecutionResult, ExecutorConfig, ProgressUpdate};
use rustbrush_platform::hotkey::{PaintControl, region};
use rustbrush_platform::input::{InputDriver, SafeInput};
use portable_atomic::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustBrush - Sign Painter for Rust",
        options,
        Box::new(|cc| Ok(Box::new(RustBrushApp::new(cc)))),
    )
}

/// Application state.
struct RustBrushApp {
    // Image state
    image_path: Option<PathBuf>,
    source_image: Option<image::RgbaImage>,
    source_texture: Option<egui::TextureHandle>,
    preview_image: Option<image::RgbaImage>,
    preview_texture: Option<egui::TextureHandle>,
    mapped_pixels: Option<Vec<MappedPixel>>,
    paint_groups: Option<Vec<ColorGroup>>,

    // Settings
    canvas_preset_idx: usize,
    custom_width: u32,
    custom_height: u32,
    use_custom_size: bool,
    color_match: ColorMatchAlgo,
    dither: DitherMode,
    strategy: StrategyChoice,
    alpha_threshold: u8,
    skip_color_enabled: bool,
    skip_color_hex: String,
    hex_input: bool,
    delay_ms: u32,
    save_session: bool,

    // Image adjustments
    brightness: f32,
    contrast: f32,
    saturation: f32,

    // Crop margins (percentage, 0–50%)
    crop_top: f32,
    crop_bottom: f32,
    crop_left: f32,
    crop_right: f32,

    // Preview mode
    preview_mode: PreviewMode,

    // Image simplification filters
    blur_enabled: bool,
    blur_sigma: f32,
    posterize_enabled: bool,
    posterize_levels: u8,
    median_enabled: bool,
    median_radius: u32,

    // Adaptive palette
    adaptive_palette: bool,
    adaptive_colors: usize,

    // Quality preset
    quality_preset: QualityPreset,

    // Background processing
    bg_result_rx: Option<mpsc::Receiver<ProcessResult>>,
    settings_generation: u64,
    last_settings_change: Instant,
    pending_reprocess: bool,

    // Time estimation
    estimated_time_secs: Option<f64>,
    last_pixel_count: Option<usize>,
    last_color_count: Option<usize>,

    // UI state
    status_message: String,
    processing: bool,
    paint_plan: Option<PaintPlan>,

    // Painting execution state
    painting_active: bool,
    paint_control: Option<Arc<PaintControl>>,
    paint_progress_rx: Option<mpsc::Receiver<ProgressUpdate>>,
    paint_result_rx: Option<mpsc::Receiver<ExecutionResult>>,
    paint_progress: Option<ProgressUpdate>,
    countdown_start: Option<Instant>,
    session_path: Option<PathBuf>,

    // Canvas region (screen coordinates for painting)
    canvas_region: Option<ScreenRect>,
    hex_field_pos: Option<(i32, i32)>,
    size_field_pos: Option<(i32, i32)>,
    two_pass_enabled: bool,
    coarse_brush_size: u32,
    resume_index: usize,

    // Scanned palette state
    palette_region: Option<ScreenRect>,
    scanned_palette: Option<Vec<PaletteEntry>>,

    // GIF / animation state
    gif_frames: Option<Vec<image::RgbaImage>>,
    gif_frame_thumbs: Vec<egui::TextureHandle>,
    selected_frame_indices: Vec<usize>,
    active_frame_slot: usize,
    animation_mode: bool,
    frame_previews: Vec<Option<image::RgbaImage>>,
    frame_preview_textures: Vec<Option<egui::TextureHandle>>,
    frame_groups: Vec<Option<Vec<ColorGroup>>>,
    frame_plans: Vec<Option<PaintPlan>>,
    painting_phase: PaintingPhase,

    // Calibration state
    calibration_rx: Option<mpsc::Receiver<CalibrationResult>>,
    calibrating: bool,
    calibration_label: String,

    // Path optimizer
    path_optimizer: bool,

    // Text builder state
    show_text_builder: bool,
    text_input: String,
    text_font_size: f32,
    text_color: [u8; 3],
    text_bg_color: [u8; 3],
    text_bg_transparent: bool,
    text_alignment: TextAlign,
    text_padding: u32,
    system_fonts: Vec<(String, std::path::PathBuf)>,
    selected_font_idx: usize,
    text_preview_texture: Option<egui::TextureHandle>,

    // Clipart library state
    show_library: bool,
    library_category_idx: usize,
    library_thumbnails: Vec<Option<egui::TextureHandle>>,

    // Update check
    #[cfg(feature = "update-check")]
    update_rx: Option<mpsc::Receiver<rustbrush_core::update::UpdateInfo>>,
    #[cfg(feature = "update-check")]
    update_available: Option<rustbrush_core::update::UpdateInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrategyChoice {
    Hybrid,
    ColorGrouped,
    LineDraw,
    Scanline,
}

impl StrategyChoice {
    fn label(&self) -> &str {
        match self {
            Self::Hybrid => "Hybrid (fastest)",
            Self::ColorGrouped => "Color Grouped",
            Self::LineDraw => "Line Draw",
            Self::Scanline => "Scanline",
        }
    }

    fn as_config_str(&self) -> &str {
        match self {
            Self::Hybrid => "hybrid",
            Self::ColorGrouped => "color-grouped",
            Self::LineDraw => "line-draw",
            Self::Scanline => "scanline",
        }
    }

    fn from_config_str(s: &str) -> Self {
        match s {
            "color-grouped" | "grouped" => Self::ColorGrouped,
            "line-draw" | "line" => Self::LineDraw,
            "scanline" => Self::Scanline,
            _ => Self::Hybrid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewMode {
    SideBySide,
    OriginalOnly,
    PreviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QualityPreset {
    Speed,
    Balanced,
    Quality,
    Maximum,
    Custom,
}

impl QualityPreset {
    fn label(&self) -> &str {
        match self {
            Self::Speed => "Speed",
            Self::Balanced => "Balanced",
            Self::Quality => "Quality",
            Self::Maximum => "Maximum",
            Self::Custom => "Custom",
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::Speed => "Hybrid + RGB + No dither (32 colors)",
            Self::Balanced => "Hybrid + RGB + Ordered dither (32 colors)",
            Self::Quality => "Grouped + CIEDE2000 + F-S dither (128 colors)",
            Self::Maximum => "Scanline + CIEDE2000 + F-S dither (512 colors)",
            Self::Custom => "Custom settings",
        }
    }

    fn as_config_str(&self) -> &str {
        match self {
            Self::Speed => "speed",
            Self::Balanced => "balanced",
            Self::Quality => "quality",
            Self::Maximum => "maximum",
            Self::Custom => "custom",
        }
    }

    fn from_config_str(s: &str) -> Self {
        match s {
            "speed" => Self::Speed,
            "quality" => Self::Quality,
            "maximum" => Self::Maximum,
            "custom" => Self::Custom,
            _ => Self::Balanced,
        }
    }
}

/// Phase of multi-frame painting (for animated neon signs).
#[derive(Debug, Clone, PartialEq, Eq)]
enum PaintingPhase {
    Idle,
    PaintingFrame { index: usize },
    WaitingForFrameSwitch { next_index: usize },
    Complete,
}

/// Result from a background calibration capture.
enum CalibrationResult {
    CanvasRegion(Result<region::ScreenRegion, String>),
    HexFieldPoint(Result<(i32, i32), String>),
    PaletteRegion(Result<(region::ScreenRegion, Vec<PaletteEntry>), String>),
    SizeFieldPoint(Result<(i32, i32), String>),
}

/// Result sent back from the background processing thread.
struct ProcessResult {
    preview_image: image::RgbaImage,
    mapped_pixels: Vec<MappedPixel>,
    paint_groups: Vec<ColorGroup>,
    total_pixels: usize,
    total_colors: usize,
    generation: u64,
}

impl RustBrushApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load_default();

        Self {
            image_path: None,
            source_image: None,
            source_texture: None,
            preview_image: None,
            preview_texture: None,
            mapped_pixels: None,
            paint_groups: None,

            canvas_preset_idx: config.canvas_preset_idx,
            custom_width: config.custom_width,
            custom_height: config.custom_height,
            use_custom_size: config.use_custom_size,
            color_match: match config.color_match.as_str() {
                "rgb" => ColorMatchAlgo::Rgb,
                _ => ColorMatchAlgo::Ciede2000,
            },
            dither: match config.dither.as_str() {
                "floyd-steinberg" => DitherMode::FloydSteinberg,
                "ordered" => DitherMode::Ordered,
                _ => DitherMode::None,
            },
            strategy: StrategyChoice::from_config_str(&config.strategy),
            alpha_threshold: config.alpha_threshold,
            skip_color_enabled: false,
            skip_color_hex: "FFFFFF".to_string(),
            hex_input: config.hex_input,
            delay_ms: config.delay_ms,
            save_session: config.save_session,

            brightness: config.brightness,
            contrast: config.contrast,
            saturation: config.saturation,

            crop_top: 0.0,
            crop_bottom: 0.0,
            crop_left: 0.0,
            crop_right: 0.0,

            preview_mode: PreviewMode::SideBySide,

            blur_enabled: config.blur_enabled,
            blur_sigma: config.blur_sigma,
            posterize_enabled: config.posterize_enabled,
            posterize_levels: config.posterize_levels,
            median_enabled: config.median_enabled,
            median_radius: config.median_radius,

            adaptive_palette: config.adaptive_palette,
            adaptive_colors: config.adaptive_colors,

            quality_preset: QualityPreset::from_config_str(&config.quality_preset),

            bg_result_rx: None,
            settings_generation: 0,
            last_settings_change: Instant::now(),
            pending_reprocess: false,

            estimated_time_secs: None,
            last_pixel_count: None,
            last_color_count: None,

            status_message: "Load an image to get started.".to_string(),
            processing: false,
            paint_plan: None,

            painting_active: false,
            paint_control: None,
            paint_progress_rx: None,
            paint_result_rx: None,
            paint_progress: None,
            countdown_start: None,
            session_path: None,

            canvas_region: None,
            hex_field_pos: None,
            size_field_pos: None,
            two_pass_enabled: config.two_pass_enabled,
            coarse_brush_size: config.coarse_brush_size,
            resume_index: 0,

            gif_frames: None,
            gif_frame_thumbs: Vec::new(),
            selected_frame_indices: Vec::new(),
            active_frame_slot: 0,
            animation_mode: false,
            frame_previews: vec![None; 5],
            frame_preview_textures: vec![None; 5],
            frame_groups: vec![None; 5],
            frame_plans: vec![None; 5],
            painting_phase: PaintingPhase::Idle,

            calibration_rx: None,
            calibrating: false,
            calibration_label: String::new(),

            palette_region: None,
            scanned_palette: None,

            // Path optimizer
            path_optimizer: config.path_optimizer,

            // Text builder
            show_text_builder: false,
            text_input: "Hello\nWorld".to_string(),
            text_font_size: 48.0,
            text_color: [255, 255, 255],
            text_bg_color: [0, 0, 0],
            text_bg_transparent: false,
            text_alignment: TextAlign::Center,
            text_padding: 8,
            system_fonts: enumerate_system_fonts(),
            selected_font_idx: 0,
            text_preview_texture: None,

            // Library
            show_library: false,
            library_category_idx: 0,
            library_thumbnails: vec![None; library::all_items().len()],

            // Update check
            #[cfg(feature = "update-check")]
            update_rx: Some(rustbrush_core::update::check_for_update()),
            #[cfg(feature = "update-check")]
            update_available: None,
        }
    }

    fn canvas_width(&self) -> u32 {
        if self.use_custom_size {
            self.custom_width
        } else {
            canvas::all_presets()[self.canvas_preset_idx].width
        }
    }

    fn canvas_height(&self) -> u32 {
        if self.use_custom_size {
            self.custom_height
        } else {
            canvas::all_presets()[self.canvas_preset_idx].height
        }
    }

    fn save_config(&self) {
        let config = Config {
            canvas_preset_idx: self.canvas_preset_idx,
            use_custom_size: self.use_custom_size,
            custom_width: self.custom_width,
            custom_height: self.custom_height,
            color_match: match self.color_match {
                ColorMatchAlgo::Rgb => "rgb",
                ColorMatchAlgo::Ciede2000 => "ciede2000",
            }
            .to_string(),
            dither: match self.dither {
                DitherMode::None => "none",
                DitherMode::FloydSteinberg => "floyd-steinberg",
                DitherMode::Ordered => "ordered",
            }
            .to_string(),
            strategy: self.strategy.as_config_str().to_string(),
            alpha_threshold: self.alpha_threshold,
            delay_ms: self.delay_ms,
            hex_input: self.hex_input,
            save_session: self.save_session,
            adaptive_palette: self.adaptive_palette,
            adaptive_colors: self.adaptive_colors,
            brightness: self.brightness,
            contrast: self.contrast,
            saturation: self.saturation,
            blur_enabled: self.blur_enabled,
            blur_sigma: self.blur_sigma,
            posterize_enabled: self.posterize_enabled,
            posterize_levels: self.posterize_levels,
            median_enabled: self.median_enabled,
            median_radius: self.median_radius,
            path_optimizer: self.path_optimizer,
            quality_preset: self.quality_preset.as_config_str().to_string(),
            two_pass_enabled: self.two_pass_enabled,
            coarse_brush_size: self.coarse_brush_size,
        };
        let _ = config.save_default();
    }

    fn load_image(&mut self, path: PathBuf) {
        // Reset animation state
        self.animation_mode = false;
        self.gif_frames = None;
        self.gif_frame_thumbs.clear();
        self.selected_frame_indices.clear();
        self.active_frame_slot = 0;
        self.frame_previews = vec![None; 5];
        self.frame_preview_textures = vec![None; 5];
        self.frame_groups = vec![None; 5];
        self.frame_plans = vec![None; 5];
        self.painting_phase = PaintingPhase::Idle;

        if rb_image::is_gif(&path) {
            self.load_gif(path);
        } else {
            self.load_static_image(path);
        }
    }

    fn load_static_image(&mut self, path: PathBuf) {
        match rb_image::load_image(&path) {
            Ok(img) => {
                self.source_image = Some(img);
                self.image_path = Some(path);
                self.source_texture = None;
                self.preview_texture = None;
                self.preview_image = None;
                self.mapped_pixels = None;
                self.paint_groups = None;
                self.paint_plan = None;
                self.last_pixel_count = None;
                self.last_color_count = None;
                self.update_time_estimate();
                self.mark_settings_changed(true);
                self.status_message = "Image loaded. Processing...".to_string();
            }
            Err(e) => {
                self.status_message = format!("Failed to load image: {}", e);
            }
        }
    }

    fn load_gif(&mut self, path: PathBuf) {
        match rb_image::load_gif_frames(&path, 500) {
            Ok(frames) => {
                let frame_count = frames.len();
                // Determine max selectable frames from canvas preset
                let max_frames = self.current_preset_frame_count().min(frame_count);
                let selected = rb_image::select_evenly_spaced(frame_count, max_frames);

                // Use the first selected frame as source_image for single-frame compatibility
                if let Some(&first_idx) = selected.first() {
                    self.source_image = Some(frames[first_idx].clone());
                }

                self.selected_frame_indices = selected;
                self.gif_frames = Some(frames);
                self.animation_mode = max_frames > 1;
                self.active_frame_slot = 0;
                self.image_path = Some(path);
                self.source_texture = None;
                self.preview_texture = None;
                self.preview_image = None;
                self.mapped_pixels = None;
                self.paint_groups = None;
                self.paint_plan = None;
                self.last_pixel_count = None;
                self.last_color_count = None;
                self.update_time_estimate();
                self.mark_settings_changed(true);

                if self.animation_mode {
                    self.status_message = format!(
                        "GIF loaded: {} frames, {} selected for animation.",
                        frame_count, self.selected_frame_indices.len()
                    );
                } else {
                    self.status_message = format!(
                        "GIF loaded: {} frames (single-frame mode).", frame_count
                    );
                }
            }
            Err(e) => {
                self.status_message = format!("Failed to load GIF: {}", e);
            }
        }
    }

    fn current_preset_frame_count(&self) -> usize {
        let presets = canvas::all_presets();
        presets[self.canvas_preset_idx].frame_count as usize
    }

    fn generate_plan(&mut self) {
        let Some(ref groups) = self.paint_groups else {
            self.status_message = "Process the image first.".to_string();
            return;
        };

        let canvas = ScreenRect {
            x: 0,
            y: 0,
            width: self.canvas_width(),
            height: self.canvas_height(),
        };

        let strategy: Box<dyn PaintStrategy> = match self.strategy {
            StrategyChoice::Hybrid => Box::new(painting::HybridStrategy {
                optimize: self.path_optimizer,
                ..Default::default()
            }),
            StrategyChoice::ColorGrouped => Box::new(painting::ColorGroupedStrategy),
            StrategyChoice::LineDraw => Box::new(painting::LineDrawStrategy::default()),
            StrategyChoice::Scanline => Box::new(painting::ScanlineStrategy),
        };

        // Generate color selection commands when using hex, adaptive, or scanned palette
        let need_color_selection = self.hex_input || self.adaptive_palette || self.scanned_palette.is_some();

        let mut plan = if self.two_pass_enabled {
            // Build pixel color map from mapped_pixels for coarse block analysis
            let mut all_pixel_colors = std::collections::HashMap::new();
            if let Some(ref mapped) = self.mapped_pixels {
                for mp in mapped {
                    all_pixel_colors.insert((mp.x, mp.y), (mp.color.r, mp.color.g, mp.color.b));
                }
            }

            let size_pos = self.size_field_pos.unwrap_or((0, 0));
            let planner = painting::TwoPassPlanner {
                coarse_brush_size: self.coarse_brush_size,
                size_field_pos: size_pos,
                detail_strategy: strategy,
            };
            planner.plan(
                groups,
                &canvas,
                self.canvas_width(),
                self.canvas_height(),
                need_color_selection,
                30,
                &all_pixel_colors,
            )
        } else {
            strategy.plan(
                groups,
                &canvas,
                self.canvas_width(),
                self.canvas_height(),
                need_color_selection,
                30,
            )
        };

        // When using scanned palette without hex mode, replace hex commands with clicks
        if self.use_palette_clicks() {
            if let Some(positions) = self.scanned_click_positions() {
                for cmd in &mut plan.commands {
                    if let PaintCommand::SelectColorByHex { ref hex } = cmd {
                        if let Some(&(x, y)) = positions.get(hex.as_str()) {
                            *cmd = PaintCommand::SelectColorByClick { x, y };
                        }
                    }
                }
            }
        }

        let est_time = painting::estimate_time(&plan, self.delay_ms as u64);
        self.status_message = format!(
            "Plan: {} commands ({} strategy), est. {:.0}s",
            plan.metadata.total_commands, plan.metadata.strategy_name, est_time
        );
        self.paint_plan = Some(plan);
    }

    fn apply_quality_preset(&mut self, preset: QualityPreset) {
        self.quality_preset = preset;
        match preset {
            QualityPreset::Speed => {
                self.strategy = StrategyChoice::Hybrid;
                self.dither = DitherMode::None;
                self.color_match = ColorMatchAlgo::Rgb;
                self.delay_ms = 5;
                self.adaptive_palette = false;
                self.path_optimizer = true;
            }
            QualityPreset::Balanced => {
                self.strategy = StrategyChoice::Hybrid;
                self.dither = DitherMode::Ordered;
                self.color_match = ColorMatchAlgo::Rgb;
                self.delay_ms = 10;
                self.adaptive_palette = false;
                self.path_optimizer = true;
            }
            QualityPreset::Quality => {
                self.strategy = StrategyChoice::ColorGrouped;
                self.dither = DitherMode::FloydSteinberg;
                self.color_match = ColorMatchAlgo::Ciede2000;
                self.delay_ms = 15;
                self.adaptive_palette = true;
                self.adaptive_colors = 128;
                self.path_optimizer = true;
            }
            QualityPreset::Maximum => {
                self.strategy = StrategyChoice::Scanline;
                self.dither = DitherMode::FloydSteinberg;
                self.color_match = ColorMatchAlgo::Ciede2000;
                self.delay_ms = 30;
                self.adaptive_palette = true;
                self.adaptive_colors = 512;
                self.path_optimizer = true;
            }
            QualityPreset::Custom => {}
        }
        self.mark_settings_changed(preset != QualityPreset::Custom);
    }

    /// Mark that settings have changed; triggers debounced reprocess if `needs_reprocess`.
    fn mark_settings_changed(&mut self, needs_reprocess: bool) {
        self.settings_generation += 1;
        self.last_settings_change = Instant::now();
        if needs_reprocess {
            self.pending_reprocess = true;
        }
        self.update_time_estimate();
    }

    /// Update the approximate time estimate using heuristics.
    fn update_time_estimate(&mut self) {
        let pixels = self.last_pixel_count.unwrap_or_else(|| {
            (self.canvas_width() as usize) * (self.canvas_height() as usize)
        });
        let colors = self.last_color_count.unwrap_or(if self.adaptive_palette {
            self.adaptive_colors
        } else {
            20 // rough guess for fixed palette usage
        });
        let use_hex = self.hex_input || self.adaptive_palette || self.scanned_palette.is_some();
        self.estimated_time_secs = Some(painting::estimate_time_approx(
            pixels,
            colors,
            self.strategy.as_config_str(),
            self.delay_ms as u64,
            use_hex,
        ));
    }

    /// Spawn a background thread to process the image with current settings.
    fn start_background_process(&mut self) {
        let Some(ref source) = self.source_image else {
            return;
        };

        let source = source.clone();
        let w = self.canvas_width();
        let h = self.canvas_height();
        let crop_top = self.crop_top;
        let crop_bottom = self.crop_bottom;
        let crop_left = self.crop_left;
        let crop_right = self.crop_right;
        let brightness = self.brightness;
        let contrast = self.contrast;
        let saturation = self.saturation;
        let blur_enabled = self.blur_enabled;
        let blur_sigma = self.blur_sigma;
        let posterize_enabled = self.posterize_enabled;
        let posterize_levels = self.posterize_levels;
        let median_enabled = self.median_enabled;
        let median_radius = self.median_radius;
        let skip_color_enabled = self.skip_color_enabled;
        let skip_color_hex = self.skip_color_hex.clone();
        let adaptive_palette = self.adaptive_palette;
        let adaptive_colors = self.adaptive_colors;
        let color_match = self.color_match;
        let dither = self.dither;
        let alpha_threshold = self.alpha_threshold;
        let generation = self.settings_generation;
        let scanned_rgb: Option<Vec<(u8, u8, u8)>> = self.scanned_palette.as_ref().map(|entries| {
            entries.iter().map(|e| (e.r, e.g, e.b)).collect()
        });

        let (tx, rx) = mpsc::channel();
        self.bg_result_rx = Some(rx);
        self.processing = true;

        std::thread::spawn(move || {
            // Crop (before resize)
            let cropped = if crop_top > 0.01 || crop_bottom > 0.01 || crop_left > 0.01 || crop_right > 0.01 {
                rb_image::crop_margins(&source, crop_top, crop_bottom, crop_left, crop_right)
            } else {
                source
            };

            // Resize
            let mut resized = rb_image::resize(&cropped, w, h, rb_image::AspectRatio::Stretch);

            // Apply adjustments
            if (brightness - 1.0).abs() > 0.01 {
                resized = rb_image::adjust_brightness(&resized, brightness);
            }
            if (contrast - 1.0).abs() > 0.01 {
                resized = rb_image::adjust_contrast(&resized, contrast);
            }
            if (saturation - 1.0).abs() > 0.01 {
                resized = rb_image::adjust_saturation(&resized, saturation);
            }

            // Apply simplification filters (median → blur → posterize)
            if median_enabled && median_radius > 0 {
                resized = rb_image::median_filter(&resized, median_radius);
            }
            if blur_enabled && blur_sigma > 0.01 {
                resized = rb_image::apply_gaussian_blur(&resized, blur_sigma);
            }
            if posterize_enabled && posterize_levels >= 2 {
                resized = rb_image::posterize(&resized, posterize_levels);
            }

            // Parse skip color
            let skip_color = if skip_color_enabled {
                parse_hex_color(&skip_color_hex).ok()
            } else {
                None
            };

            // Determine palette
            let palette = if adaptive_palette {
                generate_adaptive_palette(&resized, adaptive_colors)
            } else if let Some(ref rgb) = scanned_rgb {
                palette_from_rgb(rgb)
            } else {
                rust_palette()
            };

            // Quantize
            let opts = QuantizeOptions {
                algorithm: color_match,
                dither,
                alpha_threshold,
                skip_color,
                ..Default::default()
            };

            let mapped = map_image_to_palette(&resized, &palette, &opts);
            let preview = build_preview(&resized, &mapped);
            let groups = painting::group_by_color(&mapped);

            let total_pixels: usize = groups.iter().map(|g| g.pixels.len()).sum();
            let total_colors = groups.len();

            let _ = tx.send(ProcessResult {
                preview_image: preview,
                mapped_pixels: mapped,
                paint_groups: groups,
                total_pixels,
                total_colors,
                generation,
            });
        });
    }

    /// Start a 3-second countdown, then begin painting.
    fn start_painting_countdown(&mut self) {
        self.countdown_start = Some(Instant::now());
        self.status_message = "Starting in 3... Switch to the game now!".to_string();
    }

    /// Actually launch the painter worker thread.
    fn start_painting(&mut self) {
        // In animation mode, start painting the current frame
        if self.animation_mode && self.painting_phase == PaintingPhase::Idle {
            self.start_painting_frame(0);
            return;
        }

        let Some(ref plan) = self.paint_plan else {
            self.status_message = "Generate a plan first.".to_string();
            return;
        };
        let Some(canvas_region) = self.canvas_region else {
            self.status_message = "Set the canvas region first.".to_string();
            return;
        };

        let control = Arc::new(PaintControl::new());
        control.start_listener();
        self.paint_control = Some(control.clone());

        let plan = plan.clone();
        let delay_ms = self.delay_ms;
        let save_session = self.save_session;
        // Only focus hex field when actually typing hex codes (not when using palette clicks)
        let hex_input = (self.hex_input || self.adaptive_palette) && !self.use_palette_clicks();

        // Set up session
        let image_path_str = self.image_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let session_path = if save_session {
            let p = Session::generate_path(&image_path_str);
            self.session_path = Some(p.clone());
            Some(p)
        } else {
            None
        };

        let mut session = Session::new(
            plan.clone(),
            canvas_region,
            image_path_str,
            self.canvas_width(),
            self.canvas_height(),
        );
        session.progress = self.resume_index;

        // Progress channel
        let (progress_tx, progress_rx) = mpsc::channel();
        self.paint_progress_rx = Some(progress_rx);

        // Result channel
        let (result_tx, result_rx) = mpsc::channel();
        self.paint_result_rx = Some(result_rx);

        self.painting_active = true;
        self.paint_progress = None;
        self.status_message = "Painting... F10=pause, ESC=cancel".to_string();

        let start_index = self.resume_index;
        self.resume_index = 0; // reset for next time

        let hex_field_pos = self.hex_field_pos;

        std::thread::spawn(move || {
            // Initial delay to let user switch to game
            std::thread::sleep(Duration::from_millis(500));

            let mut input = match SafeInput::new(Duration::from_millis(delay_ms as u64)) {
                Ok(input) => input,
                Err(e) => {
                    let _ = result_tx.send(ExecutionResult::Error {
                        commands_executed: 0,
                        error: e,
                    });
                    return;
                }
            };
            // If hex input and hex field position set, click it first
            if hex_input {
                if let Some((hx, hy)) = hex_field_pos {
                    let _ = input.move_to(hx, hy);
                }
            }

            let config = ExecutorConfig {
                save_interval: if save_session { 500 } else { 0 },
                session_path,
                progress_interval: 100,
                progress_tx: Some(progress_tx),
                drift_tolerance: 5,
            };

            let result = executor::execute_plan(
                &plan, &mut input, &control, &config,
                Some(&mut session), start_index,
            );

            let _ = result_tx.send(result);
        });
    }

    /// Start painting a specific animation frame by slot index.
    fn start_painting_frame(&mut self, frame_slot: usize) {
        let plan = match self.frame_plans.get(frame_slot).and_then(|p| p.as_ref()) {
            Some(plan) => plan.clone(),
            None => {
                self.status_message = format!("Frame {} has no plan.", frame_slot + 1);
                return;
            }
        };
        let Some(canvas_region) = self.canvas_region else {
            self.status_message = "Set the canvas region first.".to_string();
            return;
        };

        let control = Arc::new(PaintControl::new());
        control.start_listener();
        self.paint_control = Some(control.clone());

        let delay_ms = self.delay_ms;
        let hex_input = (self.hex_input || self.adaptive_palette) && !self.use_palette_clicks();
        let hex_field_pos = self.hex_field_pos;

        let image_path_str = self.image_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut session = Session::new(
            plan.clone(), canvas_region, image_path_str,
            self.canvas_width(), self.canvas_height(),
        );

        let (progress_tx, progress_rx) = mpsc::channel();
        self.paint_progress_rx = Some(progress_rx);
        let (result_tx, result_rx) = mpsc::channel();
        self.paint_result_rx = Some(result_rx);

        self.painting_active = true;
        self.paint_progress = None;
        self.painting_phase = PaintingPhase::PaintingFrame { index: frame_slot };
        self.status_message = format!(
            "Painting frame {}/{}... F10=pause, ESC=cancel",
            frame_slot + 1, self.selected_frame_indices.len()
        );

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));

            let mut input = match SafeInput::new(Duration::from_millis(delay_ms as u64)) {
                Ok(input) => input,
                Err(e) => {
                    let _ = result_tx.send(ExecutionResult::Error {
                        commands_executed: 0, error: e,
                    });
                    return;
                }
            };

            if hex_input {
                if let Some((hx, hy)) = hex_field_pos {
                    let _ = input.move_to(hx, hy);
                }
            }

            let config = ExecutorConfig {
                save_interval: 0,
                session_path: None,
                progress_interval: 100,
                progress_tx: Some(progress_tx),
                drift_tolerance: 5,
            };

            let result = executor::execute_plan(
                &plan, &mut input, &control, &config,
                Some(&mut session), 0,
            );
            let _ = result_tx.send(result);
        });
    }

    fn stop_painting(&mut self) {
        if let Some(ref control) = self.paint_control {
            control.cancelled.store(true, Ordering::Relaxed);
        }
    }

    fn toggle_pause(&mut self) {
        if let Some(ref control) = self.paint_control {
            control.paused.fetch_xor(true, Ordering::Relaxed);
        }
    }

    fn is_paused(&self) -> bool {
        self.paint_control
            .as_ref()
            .map(|c| c.paused.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// Start interactive canvas region capture on a background thread.
    fn start_canvas_capture(&mut self) {
        if self.calibrating {
            return;
        }
        self.calibrating = true;
        self.calibration_label = "Canvas".to_string();
        self.status_message = "Press F9, then click and drag to select the canvas area. ESC to cancel.".to_string();

        let (tx, rx) = mpsc::channel();
        self.calibration_rx = Some(rx);

        std::thread::spawn(move || {
            let result = region::capture_region_interactive("Canvas", device_query::Keycode::F9);
            let _ = tx.send(CalibrationResult::CanvasRegion(result));
        });
    }

    /// Start interactive hex field point capture on a background thread.
    fn start_hex_field_capture(&mut self) {
        if self.calibrating {
            return;
        }
        self.calibrating = true;
        self.calibration_label = "Hex Field".to_string();
        self.status_message = "Press F8, then click on the hex input field. ESC to cancel.".to_string();

        let (tx, rx) = mpsc::channel();
        self.calibration_rx = Some(rx);

        std::thread::spawn(move || {
            let result = region::capture_point_interactive("Hex Input Field", device_query::Keycode::F8);
            let _ = tx.send(CalibrationResult::HexFieldPoint(result));
        });
    }

    /// Start interactive brush size field point capture on a background thread.
    fn start_size_field_capture(&mut self) {
        if self.calibrating {
            return;
        }
        self.calibrating = true;
        self.calibration_label = "Size Field".to_string();
        self.status_message = "Press F6, then click on the brush size input field. ESC to cancel.".to_string();

        let (tx, rx) = mpsc::channel();
        self.calibration_rx = Some(rx);

        std::thread::spawn(move || {
            let result = region::capture_point_interactive("Brush Size Field", device_query::Keycode::F6);
            let _ = tx.send(CalibrationResult::SizeFieldPoint(result));
        });
    }

    /// Start interactive palette region capture on a background thread.
    fn start_palette_capture(&mut self) {
        if self.calibrating {
            return;
        }
        self.calibrating = true;
        self.calibration_label = "Palette".to_string();
        self.status_message = "Press F7, then click and drag over the in-game color palette. ESC to cancel.".to_string();

        let (tx, rx) = mpsc::channel();
        self.calibration_rx = Some(rx);

        std::thread::spawn(move || {
            let region_result = region::capture_region_interactive("Palette", device_query::Keycode::F7);
            let result = match region_result {
                Ok(r) => {
                    match capture::sample_palette_colors(r.x, r.y, r.width, r.height) {
                        Ok(entries) => Ok((r, entries)),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(e),
            };
            let _ = tx.send(CalibrationResult::PaletteRegion(result));
        });
    }

    /// Build a hex→click position map from the scanned palette.
    fn scanned_click_positions(&self) -> Option<HashMap<String, (i32, i32)>> {
        self.scanned_palette.as_ref().map(|entries| {
            entries
                .iter()
                .map(|e| {
                    (
                        format!("{:02X}{:02X}{:02X}", e.r, e.g, e.b),
                        (e.screen_x, e.screen_y),
                    )
                })
                .collect()
        })
    }

    /// Whether the plan should use click-based color selection (scanned palette without hex override).
    fn use_palette_clicks(&self) -> bool {
        self.scanned_palette.is_some() && !self.hex_input && !self.adaptive_palette
    }

    /// Re-sample colors from the previously captured palette region.
    fn rescan_palette(&mut self) {
        let Some(region) = self.palette_region else { return };
        match capture::sample_palette_colors(region.x, region.y, region.width, region.height) {
            Ok(entries) => {
                let count = entries.len();
                self.scanned_palette = Some(entries);
                self.status_message = format!("Palette rescanned: {} colors found", count);
                if self.source_image.is_some() {
                    self.pending_reprocess = true;
                    self.last_settings_change = Instant::now();
                }
            }
            Err(e) => {
                self.status_message = format!("Rescan failed: {}", e);
            }
        }
    }

    /// Whether crop margins are active.
    fn has_crop(&self) -> bool {
        self.crop_top > 0.01 || self.crop_bottom > 0.01 || self.crop_left > 0.01 || self.crop_right > 0.01
    }
}

impl eframe::App for RustBrushApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background processing results
        if let Some(ref rx) = self.bg_result_rx {
            if let Ok(result) = rx.try_recv() {
                // Only apply if this result matches current generation
                if result.generation == self.settings_generation {
                    let total_pixels = result.total_pixels;
                    let total_colors = result.total_colors;
                    let w = self.canvas_width();
                    let h = self.canvas_height();

                    self.mapped_pixels = Some(result.mapped_pixels);
                    self.preview_image = Some(result.preview_image);
                    self.preview_texture = None; // force re-upload
                    self.paint_groups = Some(result.paint_groups);
                    self.paint_plan = None;
                    self.last_pixel_count = Some(total_pixels);
                    self.last_color_count = Some(total_colors);
                    self.processing = false;
                    self.bg_result_rx = None;
                    self.update_time_estimate();
                    self.status_message = format!(
                        "Processed: {}x{}, {} colors, {} pixels.",
                        w, h, total_colors, total_pixels
                    );
                } else {
                    // Stale result — discard and re-queue
                    self.processing = false;
                    self.bg_result_rx = None;
                    self.pending_reprocess = true;
                }
            }
        }

        // Poll calibration results
        if let Some(ref rx) = self.calibration_rx {
            if let Ok(result) = rx.try_recv() {
                self.calibrating = false;
                self.calibration_rx = None;
                match result {
                    CalibrationResult::CanvasRegion(Ok(r)) => {
                        self.canvas_region = Some(ScreenRect {
                            x: r.x, y: r.y, width: r.width, height: r.height,
                        });
                        self.status_message = format!(
                            "Canvas region set: {}x{} at ({}, {})",
                            r.width, r.height, r.x, r.y
                        );
                    }
                    CalibrationResult::CanvasRegion(Err(e)) => {
                        self.status_message = format!("Canvas capture cancelled: {}", e);
                    }
                    CalibrationResult::HexFieldPoint(Ok((x, y))) => {
                        self.hex_field_pos = Some((x, y));
                        self.status_message = format!(
                            "Hex field position set: ({}, {})", x, y
                        );
                    }
                    CalibrationResult::HexFieldPoint(Err(e)) => {
                        self.status_message = format!("Hex field capture cancelled: {}", e);
                    }
                    CalibrationResult::PaletteRegion(Ok((r, entries))) => {
                        let count = entries.len();
                        self.palette_region = Some(ScreenRect {
                            x: r.x, y: r.y, width: r.width, height: r.height,
                        });
                        self.scanned_palette = Some(entries);
                        self.status_message = format!(
                            "Palette scanned: {} colors found", count
                        );
                        // Trigger reprocess with scanned palette
                        if self.source_image.is_some() {
                            self.pending_reprocess = true;
                            self.last_settings_change = Instant::now();
                        }
                    }
                    CalibrationResult::PaletteRegion(Err(e)) => {
                        self.status_message = format!("Palette capture cancelled: {}", e);
                    }
                    CalibrationResult::SizeFieldPoint(Ok((x, y))) => {
                        self.size_field_pos = Some((x, y));
                        self.status_message = format!(
                            "Brush size field position set: ({}, {})", x, y
                        );
                    }
                    CalibrationResult::SizeFieldPoint(Err(e)) => {
                        self.status_message = format!("Size field capture cancelled: {}", e);
                    }
                }
            }
        }

        // Poll update check
        #[cfg(feature = "update-check")]
        if let Some(ref rx) = self.update_rx {
            if let Ok(info) = rx.try_recv() {
                self.update_available = Some(info);
                self.update_rx = None;
            }
        }

        // Keep polling while calibrating
        if self.calibrating {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Debounced auto-reprocess
        if self.pending_reprocess
            && self.source_image.is_some()
            && self.bg_result_rx.is_none()
            && self.last_settings_change.elapsed() >= Duration::from_millis(500)
        {
            self.pending_reprocess = false;
            self.start_background_process();
        }

        // Poll painting progress
        if let Some(ref rx) = self.paint_progress_rx {
            // Drain all pending updates, keep the latest
            while let Ok(update) = rx.try_recv() {
                self.paint_progress = Some(update);
            }
        }

        // Poll painting result (completion/cancellation/error)
        if let Some(ref rx) = self.paint_result_rx {
            if let Ok(result) = rx.try_recv() {
                self.painting_active = false;
                self.paint_result_rx = None;
                self.paint_progress_rx = None;
                self.paint_control = None;

                match result {
                    ExecutionResult::Completed { commands_executed } => {
                        // Check if we're in animation mode and have more frames
                        if let PaintingPhase::PaintingFrame { index } = self.painting_phase {
                            let total_frames = self.selected_frame_indices.len();
                            if index + 1 < total_frames {
                                // More frames to paint — wait for user to switch
                                self.painting_phase = PaintingPhase::WaitingForFrameSwitch {
                                    next_index: index + 1,
                                };
                                self.status_message = format!(
                                    "Frame {}/{} complete! Switch to frame {} in-game, then click Continue.",
                                    index + 1, total_frames, index + 2
                                );
                            } else {
                                // All frames done
                                self.painting_phase = PaintingPhase::Complete;
                                self.status_message = format!(
                                    "All {} frames painted! {} commands on last frame.",
                                    total_frames, commands_executed
                                );
                            }
                        } else {
                            // Single-frame mode
                            self.status_message = format!(
                                "Painting complete! {} commands executed.", commands_executed
                            );
                            self.painting_phase = PaintingPhase::Idle;
                        }
                        // Clean up session file on success
                        if let Some(ref path) = self.session_path {
                            let _ = std::fs::remove_file(path);
                        }
                        self.session_path = None;
                    }
                    ExecutionResult::Cancelled { commands_executed, total_commands } => {
                        self.painting_phase = PaintingPhase::Idle;
                        self.status_message = format!(
                            "Painting cancelled at {}/{}.", commands_executed, total_commands
                        );
                    }
                    ExecutionResult::Error { commands_executed, error } => {
                        self.painting_phase = PaintingPhase::Idle;
                        self.status_message = format!(
                            "Painting error after {} commands: {}", commands_executed, error
                        );
                    }
                }
                self.paint_progress = None;
            }
        }

        // Countdown timer
        if let Some(start) = self.countdown_start {
            let elapsed = start.elapsed();
            if elapsed >= Duration::from_secs(3) {
                self.countdown_start = None;
                self.start_painting();
            } else {
                let remaining = 3 - elapsed.as_secs();
                self.status_message = format!(
                    "Starting in {}... Switch to the game now!", remaining
                );
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }

        // Keep polling while background task or painting is active
        if self.bg_result_rx.is_some() || self.painting_active {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Image...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(
                                "Images",
                                &["png", "jpg", "jpeg", "gif", "bmp", "webp"],
                            )
                            .pick_file()
                        {
                            self.load_image(path);
                        }
                        ui.close();
                    }
                    if ui.button("Text Builder...").clicked() {
                        self.show_text_builder = true;
                        self.show_library = false;
                        self.text_preview_texture = None;
                        ui.close();
                    }
                    if ui.button("Clipart Library...").clicked() {
                        self.show_library = true;
                        self.show_text_builder = false;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Save Preview...").clicked() {
                        if let Some(ref preview) = self.preview_image {
                            if let Some(path) =
                                rfd::FileDialog::new().add_filter("PNG", &["png"]).save_file()
                            {
                                match preview.save(&path) {
                                    Ok(()) => {
                                        self.status_message =
                                            format!("Preview saved to {}", path.display())
                                    }
                                    Err(e) => {
                                        self.status_message = format!("Save failed: {}", e)
                                    }
                                }
                            }
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Save Settings").clicked() {
                        self.save_config();
                        self.status_message = "Settings saved.".to_string();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        self.save_config();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);

                #[cfg(feature = "update-check")]
                {
                    let mut dismiss = false;
                    if let Some(ref info) = self.update_available {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").clicked() {
                                dismiss = true;
                            }
                            ui.hyperlink_to("Download", &info.release_url);
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                format!("v{} available!", info.latest_version),
                            );
                        });
                    }
                    if dismiss {
                        self.update_available = None;
                    }
                }
            });
        });

        // Settings panel (left side)
        egui::SidePanel::left("settings_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.settings_ui(ui);
                });
            });

        // Central panel - image preview, text builder, or library
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_text_builder {
                self.text_builder_ui(ui, ctx);
            } else if self.show_library {
                self.library_ui(ui, ctx);
            } else {
                self.preview_ui(ui, ctx);
            }
        });
    }
}

impl RustBrushApp {
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        // --- Image Loading ---
        ui.heading("Image");
        if ui.button("Open Image...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
                .pick_file()
            {
                self.load_image(path);
            }
        }
        if let Some(ref path) = self.image_path {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            ui.label(format!("File: {}", name));
            if let Some(ref img) = self.source_image {
                ui.label(format!("Size: {}x{}", img.width(), img.height()));
            }
        }

        ui.separator();

        // --- Animation Frames (GIF) ---
        if self.animation_mode {
            self.animation_ui(ui);
            ui.separator();
        }

        // --- Quality Preset ---
        ui.heading("Quality");
        let mut preset_changed = false;
        ui.horizontal_wrapped(|ui| {
            for preset in [
                QualityPreset::Speed,
                QualityPreset::Balanced,
                QualityPreset::Quality,
                QualityPreset::Maximum,
            ] {
                if ui
                    .selectable_label(self.quality_preset == preset, preset.label())
                    .clicked()
                {
                    self.apply_quality_preset(preset);
                    preset_changed = true;
                }
            }
        });
        if self.quality_preset == QualityPreset::Custom {
            ui.small("Custom settings");
        } else {
            ui.small(self.quality_preset.description());
        }

        // Live time estimate
        if let Some(est) = self.estimated_time_secs {
            let mins = (est / 60.0).floor() as u64;
            let secs = (est % 60.0).round() as u64;
            if mins > 0 {
                ui.label(format!("Est. paint time: ~{}m {}s", mins, secs));
            } else {
                ui.label(format!("Est. paint time: ~{}s", secs));
            }
        }
        if self.processing {
            ui.spinner();
            ui.small("Processing...");
        }

        ui.separator();

        // --- Image Transforms ---
        ui.heading("Transform");
        let has_image = self.source_image.is_some();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(has_image, |ui| {
                if ui.button("Rotate CW").on_hover_text("Rotate 90\u{00b0} clockwise").clicked() {
                    if let Some(ref img) = self.source_image {
                        self.source_image = Some(rb_image::rotate_90(img));
                        self.source_texture = None;
                        self.mark_settings_changed(true);
                    }
                }
                if ui.button("Rotate CCW").on_hover_text("Rotate 90\u{00b0} counter-clockwise").clicked() {
                    if let Some(ref img) = self.source_image {
                        self.source_image = Some(rb_image::rotate_270(img));
                        self.source_texture = None;
                        self.mark_settings_changed(true);
                    }
                }
                if ui.button("Flip H").on_hover_text("Mirror horizontally").clicked() {
                    if let Some(ref img) = self.source_image {
                        self.source_image = Some(rb_image::flip_horizontal(img));
                        self.source_texture = None;
                        self.mark_settings_changed(true);
                    }
                }
                if ui.button("Flip V").on_hover_text("Flip vertically").clicked() {
                    if let Some(ref img) = self.source_image {
                        self.source_image = Some(rb_image::flip_vertical(img));
                        self.source_texture = None;
                        self.mark_settings_changed(true);
                    }
                }
            });
        });

        // --- Crop Margins ---
        let mut crop_changed = false;
        egui::CollapsingHeader::new("Crop")
            .default_open(self.has_crop())
            .show(ui, |ui| {
                crop_changed |= ui.add(egui::Slider::new(&mut self.crop_top, 0.0..=45.0).text("Top %")).changed();
                crop_changed |= ui.add(egui::Slider::new(&mut self.crop_bottom, 0.0..=45.0).text("Bottom %")).changed();
                crop_changed |= ui.add(egui::Slider::new(&mut self.crop_left, 0.0..=45.0).text("Left %")).changed();
                crop_changed |= ui.add(egui::Slider::new(&mut self.crop_right, 0.0..=45.0).text("Right %")).changed();
                if self.has_crop() && ui.button("Reset Crop").clicked() {
                    self.crop_top = 0.0;
                    self.crop_bottom = 0.0;
                    self.crop_left = 0.0;
                    self.crop_right = 0.0;
                    crop_changed = true;
                }
            });
        if crop_changed {
            self.mark_settings_changed(true);
        }

        ui.separator();

        // --- Image Adjustments ---
        ui.heading("Adjustments");
        let mut adj_changed = false;
        adj_changed |= ui
            .add(egui::Slider::new(&mut self.brightness, 0.2..=3.0).text("Brightness"))
            .changed();
        adj_changed |= ui
            .add(egui::Slider::new(&mut self.contrast, 0.2..=3.0).text("Contrast"))
            .changed();
        adj_changed |= ui
            .add(egui::Slider::new(&mut self.saturation, 0.0..=3.0).text("Saturation"))
            .changed();
        if ui.button("Reset Adjustments").clicked() {
            self.brightness = 1.0;
            self.contrast = 1.0;
            self.saturation = 1.0;
            adj_changed = true;
        }
        if adj_changed {
            self.mark_settings_changed(true);
        }

        ui.separator();

        // --- Simplification Filters ---
        ui.heading("Simplify");
        ui.small("Reduce detail for faster painting");
        let mut filter_changed = false;

        filter_changed |= ui.checkbox(&mut self.median_enabled, "Median filter").changed();
        if self.median_enabled {
            filter_changed |= ui
                .add(egui::Slider::new(&mut self.median_radius, 1..=3).text("Radius"))
                .changed();
        }

        filter_changed |= ui.checkbox(&mut self.blur_enabled, "Blur").changed();
        if self.blur_enabled {
            filter_changed |= ui
                .add(egui::Slider::new(&mut self.blur_sigma, 0.5..=5.0).text("Sigma"))
                .changed();
        }

        filter_changed |= ui.checkbox(&mut self.posterize_enabled, "Posterize").changed();
        if self.posterize_enabled {
            filter_changed |= ui
                .add(egui::Slider::new(&mut self.posterize_levels, 2..=32u8).text("Levels"))
                .changed();
        }

        if ui.button("Reset Filters").clicked() {
            self.blur_enabled = false;
            self.blur_sigma = 1.0;
            self.posterize_enabled = false;
            self.posterize_levels = 8;
            self.median_enabled = false;
            self.median_radius = 1;
            filter_changed = true;
        }
        if filter_changed {
            self.quality_preset = QualityPreset::Custom;
            self.mark_settings_changed(true);
        }

        ui.separator();

        // --- Canvas Size ---
        ui.heading("Canvas");
        let mut canvas_changed = false;
        canvas_changed |= ui.checkbox(&mut self.use_custom_size, "Custom size").changed();

        if self.use_custom_size {
            ui.horizontal(|ui| {
                canvas_changed |= ui
                    .add(egui::DragValue::new(&mut self.custom_width).range(8..=2048).prefix("W: "))
                    .changed();
                canvas_changed |= ui
                    .add(egui::DragValue::new(&mut self.custom_height).range(8..=2048).prefix("H: "))
                    .changed();
            });
        } else {
            let presets = canvas::all_presets();
            let before = self.canvas_preset_idx;
            egui::ComboBox::from_label("Preset")
                .selected_text(presets[self.canvas_preset_idx].to_string())
                .show_ui(ui, |ui| {
                    for (i, preset) in presets.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.canvas_preset_idx,
                            i,
                            preset.to_string(),
                        );
                    }
                });
            canvas_changed |= self.canvas_preset_idx != before;
        }
        ui.label(format!(
            "Canvas: {}x{}",
            self.canvas_width(),
            self.canvas_height()
        ));
        if canvas_changed {
            self.mark_settings_changed(true);
        }

        ui.separator();

        // --- Advanced Settings (collapsible) ---
        let advanced_label = if self.quality_preset == QualityPreset::Custom {
            "Advanced Settings (customized)"
        } else {
            "Advanced Settings"
        };
        egui::CollapsingHeader::new(advanced_label)
            .default_open(self.quality_preset == QualityPreset::Custom)
            .show(ui, |ui| {
                self.advanced_settings_ui(ui);
            });

        ui.separator();

        // --- Actions ---
        ui.heading("Actions");

        let has_image = self.source_image.is_some();
        let has_groups = self.paint_groups.is_some();
        let has_plan = self.paint_plan.is_some();

        ui.add_enabled_ui(has_image && !self.processing && !self.painting_active, |ui| {
            if ui
                .button("Reprocess Now")
                .on_hover_text("Manually re-run image processing")
                .clicked()
            {
                self.start_background_process();
            }
        });

        ui.add_enabled_ui(has_groups && !self.painting_active, |ui| {
            if ui
                .button("Generate Plan")
                .on_hover_text("Create the painting command plan")
                .clicked()
            {
                self.generate_plan();
            }
        });

        if let Some(ref plan) = self.paint_plan {
            ui.separator();
            ui.heading("Plan Info");
            ui.label(format!("Strategy: {}", plan.metadata.strategy_name));
            ui.label(format!("Commands: {}", plan.metadata.total_commands));
            ui.label(format!("Colors: {}", plan.metadata.total_colors));
            ui.label(format!("Pixels: {}", plan.metadata.total_pixels));
            ui.label(format!(
                "Est. time: {:.0}s",
                painting::estimate_time(plan, self.delay_ms as u64)
            ));
            if let Some(pct) = plan.metadata.optimization_improvement {
                ui.label(format!("Path optimized: {:.0}% less travel", pct));
            }
        }

        // --- Calibration & Paint Setup ---
        if has_plan && !self.painting_active {
            ui.separator();
            ui.heading("Calibration");

            if self.calibrating {
                ui.spinner();
                ui.label(format!("Capturing {}...", self.calibration_label));
                ui.small("Press the indicated key, then click/drag. ESC to cancel.");
            } else {
                // Canvas region capture
                ui.horizontal(|ui| {
                    if ui.button("Select Canvas Region (F9)")
                        .on_hover_text("Press F9, then click and drag over the in-game canvas area")
                        .clicked()
                    {
                        self.start_canvas_capture();
                    }
                    if self.canvas_region.is_some() {
                        ui.label("Set");
                    }
                });

                if let Some(ref region) = self.canvas_region {
                    ui.small(format!(
                        "  {}x{} at ({}, {})", region.width, region.height, region.x, region.y
                    ));
                }

                // Palette region scan
                ui.horizontal(|ui| {
                    if ui.button("Scan Palette (F7)")
                        .on_hover_text("Press F7, then click and drag over the in-game color palette swatches")
                        .clicked()
                    {
                        self.start_palette_capture();
                    }
                    if self.palette_region.is_some()
                        && self.scanned_palette.is_some()
                        && ui.button("Rescan")
                            .on_hover_text("Re-sample colors from the same palette region")
                            .clicked()
                    {
                        self.rescan_palette();
                    }
                    if let Some(ref entries) = self.scanned_palette {
                        ui.label(format!("{} colors", entries.len()));
                    }
                });

                if let Some(ref region) = self.palette_region {
                    ui.small(format!(
                        "  {}x{} at ({}, {})", region.width, region.height, region.x, region.y
                    ));
                }
                if self.scanned_palette.is_some() && !self.hex_input && !self.adaptive_palette {
                    ui.small("  Colors will be selected by clicking the palette");
                }

                // Hex field point capture (needed for hex input or adaptive palette modes)
                if self.hex_input || self.adaptive_palette {
                    ui.horizontal(|ui| {
                        if ui.button("Select Hex Field (F8)")
                            .on_hover_text("Press F8, then click on the hex color input field in-game")
                            .clicked()
                        {
                            self.start_hex_field_capture();
                        }
                        if self.hex_field_pos.is_some() {
                            ui.label("Set");
                        }
                    });

                    if let Some((x, y)) = self.hex_field_pos {
                        ui.small(format!("  Position: ({}, {})", x, y));
                    }
                }

                // Two-pass painting (coarse fill + detail)
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.two_pass_enabled, "Two-pass painting")
                        .on_hover_text("Use a large brush for uniform regions first, then size 1 for detail")
                        .changed()
                    {
                        self.pending_reprocess = true;
                        self.last_settings_change = Instant::now();
                    }
                });
                if self.two_pass_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Coarse brush size:");
                        ui.add(
                            egui::DragValue::new(&mut self.coarse_brush_size)
                                .range(2..=100u32)
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Select Size Field (F6)")
                            .on_hover_text("Press F6, then click on the brush size input field in-game")
                            .clicked()
                        {
                            self.start_size_field_capture();
                        }
                        if self.size_field_pos.is_some() {
                            ui.label("Set");
                        }
                    });
                    if let Some((x, y)) = self.size_field_pos {
                        ui.small(format!("  Position: ({}, {})", x, y));
                    }
                }

                // Manual override (collapsible)
                egui::CollapsingHeader::new("Manual Coordinates")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("Canvas region:");
                        let mut region = self.canvas_region.unwrap_or(ScreenRect {
                            x: 0, y: 0, width: 800, height: 600,
                        });
                        let mut region_changed = false;
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            region_changed |= ui.add(egui::DragValue::new(&mut region.x)).changed();
                            ui.label("Y:");
                            region_changed |= ui.add(egui::DragValue::new(&mut region.y)).changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label("W:");
                            region_changed |= ui.add(
                                egui::DragValue::new(&mut region.width).range(10..=4096u32)
                            ).changed();
                            ui.label("H:");
                            region_changed |= ui.add(
                                egui::DragValue::new(&mut region.height).range(10..=4096u32)
                            ).changed();
                        });
                        if region_changed || self.canvas_region.is_none() {
                            self.canvas_region = Some(region);
                        }

                        if self.hex_input || self.adaptive_palette {
                            ui.label("Hex field position:");
                            let mut pos = self.hex_field_pos.unwrap_or((0, 0));
                            ui.horizontal(|ui| {
                                ui.label("X:");
                                ui.add(egui::DragValue::new(&mut pos.0));
                                ui.label("Y:");
                                ui.add(egui::DragValue::new(&mut pos.1));
                            });
                            self.hex_field_pos = Some(pos);
                        }
                    });
            }

            ui.separator();

            // Start painting button
            let can_start = self.canvas_region.is_some()
                && self.countdown_start.is_none()
                && !self.calibrating;
            ui.add_enabled_ui(can_start, |ui| {
                if ui
                    .button("Start Painting (3s countdown)")
                    .on_hover_text("Starts a 3-second countdown, then begins painting. Switch to the game!")
                    .clicked()
                {
                    self.start_painting_countdown();
                }
            });

            if self.countdown_start.is_some() {
                ui.spinner();
                ui.label("Switch to the game window now!");
            }
        }

        // --- Painting Progress ---
        if let PaintingPhase::WaitingForFrameSwitch { next_index } = self.painting_phase {
            ui.separator();
            ui.heading("Frame Switch");
            ui.label(format!(
                "Switch to frame {} in the game's sign UI, then click Continue.",
                next_index + 1
            ));
            if ui.button("Continue Painting").clicked() {
                self.start_painting_frame(next_index);
            }
            if ui.button("Cancel").clicked() {
                self.painting_phase = PaintingPhase::Idle;
                self.status_message = "Animation painting cancelled.".to_string();
            }
        }

        if self.painting_phase == PaintingPhase::Complete {
            ui.separator();
            ui.heading("Complete");
            ui.label("All animation frames have been painted!");
            if ui.button("Done").clicked() {
                self.painting_phase = PaintingPhase::Idle;
            }
        }

        if self.painting_active {
            ui.separator();
            ui.heading("Painting");

            // Show frame indicator in animation mode
            if let PaintingPhase::PaintingFrame { index } = self.painting_phase {
                ui.label(format!(
                    "Frame {}/{}", index + 1, self.selected_frame_indices.len()
                ));
            }

            if let Some(ref progress) = self.paint_progress {
                let pct = progress.percent as f32 / 100.0;
                ui.add(egui::ProgressBar::new(pct).show_percentage());
                ui.label(format!(
                    "{} / {} commands",
                    progress.commands_executed, progress.total_commands
                ));

                // ETA calculation
                if progress.percent > 0.0 && progress.percent < 100.0 {
                    let remaining_cmds = progress.total_commands - progress.commands_executed;
                    let est_remaining = remaining_cmds as f64 * self.delay_ms as f64 / 1000.0;
                    let mins = (est_remaining / 60.0).floor() as u64;
                    let secs = (est_remaining % 60.0).round() as u64;
                    if mins > 0 {
                        ui.label(format!("ETA: ~{}m {}s remaining", mins, secs));
                    } else {
                        ui.label(format!("ETA: ~{}s remaining", secs));
                    }
                }

                if let Some(ref color) = progress.current_color {
                    ui.horizontal(|ui| {
                        if let Ok((r, g, b)) = parse_hex_color(color) {
                            let c = egui::Color32::from_rgb(r, g, b);
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(14.0, 14.0), egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2.0, c);
                        }
                        ui.label(format!("Current: #{}", color));
                    });
                }
            } else {
                ui.spinner();
                ui.label("Starting...");
            }

            // Pause / Cancel buttons
            ui.horizontal(|ui| {
                if self.is_paused() {
                    if ui.button("Resume (F10)").clicked() {
                        self.toggle_pause();
                    }
                } else if ui.button("Pause (F10)").clicked() {
                    self.toggle_pause();
                }
                if ui.button("Cancel (ESC)").clicked() {
                    self.stop_painting();
                }
            });

            if self.is_paused() {
                ui.label("PAUSED");
            }
        }

        // Color palette display
        if let Some(ref groups) = self.paint_groups {
            ui.separator();
            ui.heading("Colors Used");
            let max_show = 20;
            for group in groups.iter().take(max_show) {
                let (r, g, b) = group.color;
                let color = egui::Color32::from_rgb(r, g, b);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color);
                    ui.label(format!("#{} ({}px)", group.hex, group.pixels.len()));
                });
            }
            if groups.len() > max_show {
                ui.label(format!("...and {} more", groups.len() - max_show));
            }
        }

        // --- Session Recovery ---
        if !self.painting_active {
            let sessions_dir = Session::default_sessions_dir();
            if sessions_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
                    let mut session_files: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
                        .collect();
                    session_files.sort_by(|a, b| b.cmp(a)); // newest first

                    if !session_files.is_empty() {
                        ui.separator();
                        egui::CollapsingHeader::new(format!(
                            "Saved Sessions ({})", session_files.len()
                        ))
                        .default_open(false)
                        .show(ui, |ui| {
                            let mut to_delete = None;
                            let mut to_resume = None;

                            for (idx, path) in session_files.iter().take(10).enumerate() {
                                let fname = path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("?");

                                ui.horizontal(|ui| {
                                    ui.label(fname);
                                    if ui.small_button("Resume").clicked() {
                                        to_resume = Some(path.clone());
                                    }
                                    if ui.small_button("Delete").clicked() {
                                        to_delete = Some(idx);
                                    }
                                });

                                // Show session info on hover
                                if let Ok(session) = Session::load(path) {
                                    ui.small(format!(
                                        "  {:.0}% ({}/{})",
                                        session.progress_percent(),
                                        session.progress,
                                        session.plan.commands.len(),
                                    ));
                                }
                            }

                            if let Some(idx) = to_delete {
                                let _ = std::fs::remove_file(&session_files[idx]);
                            }

                            if let Some(path) = to_resume {
                                if let Ok(session) = Session::load(&path) {
                                    self.resume_index = session.progress;
                                    self.canvas_region = Some(session.canvas_region);
                                    self.paint_plan = Some(session.plan.clone());
                                    self.session_path = Some(path);
                                    self.status_message = format!(
                                        "Session loaded. Resuming from {:.0}%.",
                                        session.progress_percent()
                                    );
                                    // Auto-start with countdown
                                    self.start_painting_countdown();
                                }
                            }
                        });
                    }
                }
            }
        }
    }

    fn animation_ui(&mut self, ui: &mut egui::Ui) {
        let num_slots = self.selected_frame_indices.len();
        ui.heading(format!("Animation ({} frames)", num_slots));

        // Frame slot selector — numbered buttons
        ui.horizontal(|ui| {
            ui.label("Slots:");
            for slot in 0..num_slots {
                let label = format!("{}", slot + 1);
                let selected = slot == self.active_frame_slot;
                if ui.selectable_label(selected, &label).clicked() {
                    self.active_frame_slot = slot;
                    // Update source_image to match this slot's GIF frame
                    if let Some(ref frames) = self.gif_frames {
                        if let Some(&idx) = self.selected_frame_indices.get(slot) {
                            if idx < frames.len() {
                                self.source_image = Some(frames[idx].clone());
                                self.source_texture = None;
                                // Show this slot's preview if available
                                self.preview_image = self.frame_previews[slot].clone();
                                self.preview_texture = None;
                            }
                        }
                    }
                }
            }
        });

        // Filmstrip — scrollable row of all GIF frame thumbnails
        let total_frames = self.gif_frames.as_ref().map(|f| f.len()).unwrap_or(0);
        if total_frames > 0 {
            ui.label(format!("All frames ({}):", total_frames));

            // Collect click in filmstrip without mutating self
            let mut clicked_frame: Option<usize> = None;
            egui::ScrollArea::horizontal().max_height(56.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    for i in 0..total_frames {
                        let is_selected = self.selected_frame_indices.contains(&i);
                        let btn_text = format!("{}", i + 1);
                        if ui.selectable_label(is_selected, &btn_text).clicked() {
                            clicked_frame = Some(i);
                        }
                    }
                });
            });

            // Apply filmstrip click
            if let Some(frame_idx) = clicked_frame {
                if let Some(slot) = self.selected_frame_indices.get_mut(self.active_frame_slot) {
                    *slot = frame_idx;
                }
                if let Some(ref frames) = self.gif_frames {
                    if let Some(frame) = frames.get(frame_idx) {
                        self.source_image = Some(frame.clone());
                    }
                }
                self.source_texture = None;
                let slot = self.active_frame_slot;
                self.frame_previews[slot] = None;
                self.frame_preview_textures[slot] = None;
                self.frame_groups[slot] = None;
                self.frame_plans[slot] = None;
                self.preview_image = None;
                self.preview_texture = None;
                self.mark_settings_changed(true);
            }

            // Auto-select button
            let mut do_auto_select = false;
            if ui.button("Auto-select evenly spaced").clicked() {
                do_auto_select = true;
            }
            if do_auto_select {
                let max_frames = self.current_preset_frame_count().min(total_frames);
                self.selected_frame_indices = rb_image::select_evenly_spaced(total_frames, max_frames);
                self.active_frame_slot = 0;
                self.frame_previews = vec![None; 5];
                self.frame_preview_textures = vec![None; 5];
                self.frame_groups = vec![None; 5];
                self.frame_plans = vec![None; 5];
                if let Some(&first_idx) = self.selected_frame_indices.first() {
                    if let Some(ref frames) = self.gif_frames {
                        if let Some(frame) = frames.get(first_idx) {
                            self.source_image = Some(frame.clone());
                            self.source_texture = None;
                        }
                    }
                }
                self.preview_image = None;
                self.preview_texture = None;
                self.mark_settings_changed(true);
            }

            // Process All Frames / Generate All Plans buttons
            let mut do_process_all = false;
            let mut do_generate_all = false;
            ui.horizontal(|ui| {
                if ui.button("Process All Frames").clicked() {
                    do_process_all = true;
                }
                if ui.button("Generate All Plans").clicked() {
                    do_generate_all = true;
                }
            });
            if do_process_all {
                self.process_all_animation_frames();
            }
            if do_generate_all {
                self.generate_all_plans();
            }

            // Show per-frame status
            for (slot, idx) in self.selected_frame_indices.iter().enumerate() {
                let has_preview = self.frame_previews.get(slot).and_then(|p| p.as_ref()).is_some();
                let has_plan = self.frame_plans.get(slot).and_then(|p| p.as_ref()).is_some();
                let status = if has_plan { "planned" } else if has_preview { "processed" } else { "pending" };
                ui.small(format!("  Slot {} (frame {}): {}", slot + 1, idx + 1, status));
            }
        }
    }

    fn process_all_animation_frames(&mut self) {
        let Some(ref frames) = self.gif_frames else { return };

        for slot in 0..self.selected_frame_indices.len() {
            let Some(&frame_idx) = self.selected_frame_indices.get(slot) else { continue };
            let Some(frame) = frames.get(frame_idx) else { continue };

            // Process this frame synchronously (could be optimized to background later)
            let w = self.canvas_width();
            let h = self.canvas_height();
            let cropped = if self.has_crop() {
                rb_image::crop_margins(frame, self.crop_top, self.crop_bottom, self.crop_left, self.crop_right)
            } else {
                frame.clone()
            };
            let mut resized = rb_image::resize(&cropped, w, h, rb_image::AspectRatio::Stretch);

            if (self.brightness - 1.0).abs() > 0.01 {
                resized = rb_image::adjust_brightness(&resized, self.brightness);
            }
            if (self.contrast - 1.0).abs() > 0.01 {
                resized = rb_image::adjust_contrast(&resized, self.contrast);
            }
            if (self.saturation - 1.0).abs() > 0.01 {
                resized = rb_image::adjust_saturation(&resized, self.saturation);
            }

            // Apply simplification filters (median → blur → posterize)
            if self.median_enabled && self.median_radius > 0 {
                resized = rb_image::median_filter(&resized, self.median_radius);
            }
            if self.blur_enabled && self.blur_sigma > 0.01 {
                resized = rb_image::apply_gaussian_blur(&resized, self.blur_sigma);
            }
            if self.posterize_enabled && self.posterize_levels >= 2 {
                resized = rb_image::posterize(&resized, self.posterize_levels);
            }

            let skip_color = if self.skip_color_enabled {
                parse_hex_color(&self.skip_color_hex).ok()
            } else {
                None
            };

            let palette = if self.adaptive_palette {
                generate_adaptive_palette(&resized, self.adaptive_colors)
            } else if let Some(ref entries) = self.scanned_palette {
                let rgb: Vec<(u8, u8, u8)> = entries.iter().map(|e| (e.r, e.g, e.b)).collect();
                palette_from_rgb(&rgb)
            } else {
                rust_palette()
            };

            let opts = QuantizeOptions {
                algorithm: self.color_match,
                dither: self.dither,
                alpha_threshold: self.alpha_threshold,
                skip_color,
                ..Default::default()
            };

            let mapped = map_image_to_palette(&resized, &palette, &opts);
            let preview = build_preview(&resized, &mapped);
            let groups = painting::group_by_color(&mapped);

            self.frame_previews[slot] = Some(preview);
            self.frame_preview_textures[slot] = None;
            self.frame_groups[slot] = Some(groups);
        }

        // Show the active slot's preview
        self.preview_image = self.frame_previews[self.active_frame_slot].clone();
        self.preview_texture = None;
        self.status_message = format!(
            "All {} frames processed.", self.selected_frame_indices.len()
        );
    }

    fn generate_all_plans(&mut self) {
        let canvas = ScreenRect {
            x: 0, y: 0,
            width: self.canvas_width(),
            height: self.canvas_height(),
        };

        let need_color_selection = self.hex_input || self.adaptive_palette || self.scanned_palette.is_some();
        let click_positions = if self.use_palette_clicks() {
            self.scanned_click_positions()
        } else {
            None
        };
        let mut total_commands = 0usize;

        for slot in 0..self.selected_frame_indices.len() {
            let Some(ref groups) = self.frame_groups[slot] else { continue };

            let strategy: Box<dyn PaintStrategy> = match self.strategy {
                StrategyChoice::Hybrid => Box::new(painting::HybridStrategy {
                    optimize: self.path_optimizer,
                    ..Default::default()
                }),
                StrategyChoice::ColorGrouped => Box::new(painting::ColorGroupedStrategy),
                StrategyChoice::LineDraw => Box::new(painting::LineDrawStrategy::default()),
                StrategyChoice::Scanline => Box::new(painting::ScanlineStrategy),
            };

            let mut plan = strategy.plan(
                groups, &canvas,
                self.canvas_width(), self.canvas_height(),
                need_color_selection, 30,
            );

            // Replace hex commands with clicks when using scanned palette
            if let Some(ref positions) = click_positions {
                for cmd in &mut plan.commands {
                    if let PaintCommand::SelectColorByHex { ref hex } = cmd {
                        if let Some(&(x, y)) = positions.get(hex.as_str()) {
                            *cmd = PaintCommand::SelectColorByClick { x, y };
                        }
                    }
                }
            }

            total_commands += plan.metadata.total_commands;
            self.frame_plans[slot] = Some(plan);
        }

        let total_time: f64 = self.frame_plans.iter()
            .filter_map(|p| p.as_ref())
            .map(|p| painting::estimate_time(p, self.delay_ms as u64))
            .sum();

        self.status_message = format!(
            "All plans generated: {} total commands, est. {:.0}s",
            total_commands, total_time
        );
    }

    fn advanced_settings_ui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        // --- Color Matching ---
        ui.label("Color Matching");
        let cm_before = self.color_match;
        egui::ComboBox::from_label("Matching")
            .selected_text(match self.color_match {
                ColorMatchAlgo::Ciede2000 => "CIEDE2000 (perceptual)",
                ColorMatchAlgo::Rgb => "RGB (fast)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.color_match,
                    ColorMatchAlgo::Ciede2000,
                    "CIEDE2000 (perceptual)",
                );
                ui.selectable_value(&mut self.color_match, ColorMatchAlgo::Rgb, "RGB (fast)");
            });
        changed |= self.color_match != cm_before;

        let dither_before = self.dither;
        egui::ComboBox::from_label("Dithering")
            .selected_text(match self.dither {
                DitherMode::None => "None",
                DitherMode::FloydSteinberg => "Floyd-Steinberg",
                DitherMode::Ordered => "Ordered (Bayer)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.dither, DitherMode::None, "None");
                ui.selectable_value(
                    &mut self.dither,
                    DitherMode::FloydSteinberg,
                    "Floyd-Steinberg",
                );
                ui.selectable_value(&mut self.dither, DitherMode::Ordered, "Ordered (Bayer)");
            });
        changed |= self.dither != dither_before;

        let alpha_before = self.alpha_threshold;
        ui.add(
            egui::Slider::new(&mut self.alpha_threshold, 0..=255).text("Alpha threshold"),
        );
        changed |= self.alpha_threshold != alpha_before;

        let skip_before = self.skip_color_enabled;
        ui.checkbox(&mut self.skip_color_enabled, "Skip background color");
        changed |= self.skip_color_enabled != skip_before;
        if self.skip_color_enabled {
            ui.horizontal(|ui| {
                ui.label("#");
                ui.text_edit_singleline(&mut self.skip_color_hex);
            });
        }

        ui.separator();

        // --- Adaptive Palette ---
        ui.label("Palette");
        let ap_before = self.adaptive_palette;
        ui.checkbox(&mut self.adaptive_palette, "Adaptive palette (k-means)")
            .on_hover_text("Generate optimal colors from the image via clustering");
        changed |= self.adaptive_palette != ap_before;
        if self.adaptive_palette {
            let ac_before = self.adaptive_colors;
            ui.add(
                egui::Slider::new(&mut self.adaptive_colors, 16..=512)
                    .text("Colors")
                    .logarithmic(true),
            );
            changed |= self.adaptive_colors != ac_before;
            ui.small("Requires hex input mode");
        }

        ui.separator();

        // --- Strategy ---
        ui.label("Painting");
        let strat_before = self.strategy;
        egui::ComboBox::from_label("Strategy")
            .selected_text(self.strategy.label())
            .show_ui(ui, |ui| {
                for s in [
                    StrategyChoice::Hybrid,
                    StrategyChoice::ColorGrouped,
                    StrategyChoice::LineDraw,
                    StrategyChoice::Scanline,
                ] {
                    ui.selectable_value(&mut self.strategy, s, s.label());
                }
            });
        changed |= self.strategy != strat_before;

        let opt_before = self.path_optimizer;
        ui.checkbox(&mut self.path_optimizer, "Path optimizer (2-opt)")
            .on_hover_text("Optimize segment order to minimize mouse travel distance");
        changed |= self.path_optimizer != opt_before;

        let delay_before = self.delay_ms;
        ui.add(egui::Slider::new(&mut self.delay_ms, 5..=100).text("Delay (ms)"));
        let delay_changed = self.delay_ms != delay_before;

        let hex_before = self.hex_input;
        ui.checkbox(&mut self.hex_input, "Hex input mode");
        changed |= self.hex_input != hex_before;

        ui.checkbox(&mut self.save_session, "Auto-save session");

        // If any coordinated setting changed, switch to Custom preset
        if changed {
            self.quality_preset = QualityPreset::Custom;
            self.mark_settings_changed(true);
        } else if delay_changed {
            // delay only affects time estimate, not preview
            self.quality_preset = QualityPreset::Custom;
            self.mark_settings_changed(false);
        }
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Painting warning overlay
        if self.painting_active || self.painting_phase != PaintingPhase::Idle {
            let drift_detected = self.paint_progress
                .as_ref()
                .map(|p| p.mouse_drift)
                .unwrap_or(false);

            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                if drift_detected {
                    ui.heading("MOUSE DRIFT DETECTED");
                    ui.add_space(10.0);
                    ui.label("Your mouse was moved during painting!");
                    ui.label("Painting has been auto-paused to prevent errors.");
                    ui.add_space(10.0);
                    ui.label("Move your mouse away, then click Resume in the sidebar.");
                } else if self.painting_active {
                    ui.heading("PAINTING IN PROGRESS");
                    ui.add_space(10.0);
                    ui.label("Do not move the mouse or use the keyboard!");
                    ui.label("The mouse is being controlled by RustBrush.");
                    ui.add_space(10.0);
                    ui.label("F10 = Pause/Resume    ESC = Cancel");
                } else if let PaintingPhase::WaitingForFrameSwitch { next_index } = self.painting_phase {
                    ui.heading("SWITCH FRAMES IN-GAME");
                    ui.add_space(10.0);
                    ui.label(format!(
                        "Switch to frame {} in the game's sign UI.",
                        next_index + 1
                    ));
                    ui.label("Then click 'Continue Painting' in the sidebar.");
                }
                ui.add_space(20.0);
            });
            return;
        }

        if self.source_image.is_none() {
            ui.centered_and_justified(|ui| {
                ui.heading("Drag & drop an image or click Open Image");
            });
            return;
        }

        // Upload textures if needed
        if self.source_texture.is_none() {
            if let Some(ref img) = self.source_image {
                self.source_texture = Some(upload_texture(ctx, "source", img));
            }
        }
        if self.preview_texture.is_none() {
            if let Some(ref img) = self.preview_image {
                self.preview_texture = Some(upload_texture(ctx, "preview", img));
            }
        }

        let has_preview = self.preview_texture.is_some();

        // Preview mode toggle + palette strip
        ui.horizontal(|ui| {
            if has_preview {
                if ui.selectable_label(self.preview_mode == PreviewMode::SideBySide, "Side by Side").clicked() {
                    self.preview_mode = PreviewMode::SideBySide;
                }
                if ui.selectable_label(self.preview_mode == PreviewMode::OriginalOnly, "Original").clicked() {
                    self.preview_mode = PreviewMode::OriginalOnly;
                }
                if ui.selectable_label(self.preview_mode == PreviewMode::PreviewOnly, "Preview").clicked() {
                    self.preview_mode = PreviewMode::PreviewOnly;
                }
            } else {
                ui.heading("Source");
            }
        });

        // Palette color strip — show color distribution
        if let Some(ref groups) = self.paint_groups {
            let total_px: usize = groups.iter().map(|g| g.pixels.len()).sum();
            if total_px > 0 {
                let strip_height = 14.0;
                let strip_width = ui.available_width().min(800.0);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(strip_width, strip_height),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(rect);
                let mut x = rect.left();
                for group in groups.iter() {
                    let frac = group.pixels.len() as f32 / total_px as f32;
                    let w = (frac * strip_width).max(1.0);
                    let (r, g, b) = group.color;
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, strip_height)),
                        0.0,
                        egui::Color32::from_rgb(r, g, b),
                    );
                    x += w;
                }
                ui.small(format!("{} colors, {} pixels", groups.len(), total_px));
            }
        }

        ui.separator();

        let available = ui.available_size();
        let show_source = !has_preview || self.preview_mode != PreviewMode::PreviewOnly;
        let show_preview = has_preview && self.preview_mode != PreviewMode::OriginalOnly;
        let both = show_source && show_preview;
        let img_width = if both {
            (available.x - 10.0) / 2.0
        } else {
            available.x
        };

        ui.horizontal(|ui| {
            if show_source {
                if let Some(ref tex) = self.source_texture {
                    let size = fit_image_size(tex.size_vec2(), img_width, available.y - 30.0);
                    ui.image(egui::load::SizedTexture::new(tex.id(), size));
                }
            }

            if both {
                ui.separator();
            }

            if show_preview {
                if let Some(ref tex) = self.preview_texture {
                    let size = fit_image_size(tex.size_vec2(), img_width, available.y - 30.0);
                    ui.image(egui::load::SizedTexture::new(tex.id(), size));
                }
            }
        });

        // Handle drag and drop
        let dropped: Option<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .first()
                .and_then(|f| f.path.clone())
        });
        if let Some(path) = dropped {
            self.load_image(path);
        }
    }
}

// ── Text Builder UI ──────────────────────────────────────────────────

impl RustBrushApp {
    fn text_builder_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Text Builder");
        ui.separator();

        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Font:");
            let current_name = self
                .system_fonts
                .get(self.selected_font_idx)
                .map(|(name, _)| name.as_str())
                .unwrap_or("(none)");
            egui::ComboBox::from_id_salt("font_selector")
                .selected_text(current_name)
                .width(250.0)
                .show_ui(ui, |ui| {
                    for (i, (name, _)) in self.system_fonts.iter().enumerate() {
                        if ui
                            .selectable_value(&mut self.selected_font_idx, i, name)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            if ui
                .add(egui::Slider::new(&mut self.text_font_size, 8.0..=200.0).suffix("px"))
                .changed()
            {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Text color:");
            if ui.color_edit_button_srgb(&mut self.text_color).changed() {
                changed = true;
            }
            ui.separator();
            ui.label("Background:");
            if ui.checkbox(&mut self.text_bg_transparent, "Transparent").changed() {
                changed = true;
            }
            if !self.text_bg_transparent
                && ui.color_edit_button_srgb(&mut self.text_bg_color).changed()
            {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Alignment:");
            if ui
                .selectable_value(&mut self.text_alignment, TextAlign::Left, "Left")
                .changed()
            {
                changed = true;
            }
            if ui
                .selectable_value(&mut self.text_alignment, TextAlign::Center, "Center")
                .changed()
            {
                changed = true;
            }
            if ui
                .selectable_value(&mut self.text_alignment, TextAlign::Right, "Right")
                .changed()
            {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Padding:");
            if ui
                .add(egui::Slider::new(&mut self.text_padding, 0..=64).suffix("px"))
                .changed()
            {
                changed = true;
            }
        });

        ui.separator();
        ui.label("Text (use Enter for new lines):");
        if ui
            .add(
                egui::TextEdit::multiline(&mut self.text_input)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4),
            )
            .changed()
        {
            changed = true;
        }

        // Render preview when settings change
        if changed {
            self.text_preview_texture = None;
        }

        ui.separator();

        let cw = self.canvas_width();
        let ch = self.canvas_height();
        ui.small(format!("Preview at canvas size: {}x{}", cw, ch));

        // Lazy render preview
        if self.text_preview_texture.is_none() && !self.system_fonts.is_empty() {
            if let Some(img) = self.render_text_preview() {
                self.text_preview_texture = Some(upload_texture(ctx, "text_preview", &img));
            }
        }

        // Show preview
        if let Some(ref tex) = self.text_preview_texture {
            let available = ui.available_size();
            let size = fit_image_size(tex.size_vec2(), available.x, available.y - 40.0);
            ui.image(egui::load::SizedTexture::new(tex.id(), size));
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Apply as Source Image").clicked() {
                if let Some(img) = self.render_text_preview() {
                    self.source_image = Some(img);
                    self.image_path = None;
                    self.source_texture = None;
                    self.preview_texture = None;
                    self.preview_image = None;
                    self.mapped_pixels = None;
                    self.paint_groups = None;
                    self.paint_plan = None;
                    self.last_pixel_count = None;
                    self.last_color_count = None;
                    self.update_time_estimate();
                    self.mark_settings_changed(true);
                    self.status_message = "Text image applied. Processing...".to_string();
                    self.show_text_builder = false;
                }
            }
            if ui.button("Cancel").clicked() {
                self.show_text_builder = false;
            }
        });
    }

    fn render_text_preview(&self) -> Option<image::RgbaImage> {
        let (_, font_path) = self.system_fonts.get(self.selected_font_idx)?;
        let font_data = std::fs::read(font_path).ok()?;
        let bg_color = if self.text_bg_transparent {
            [0, 0, 0, 0]
        } else {
            [self.text_bg_color[0], self.text_bg_color[1], self.text_bg_color[2], 255]
        };
        let config = TextConfig {
            text: self.text_input.clone(),
            font_data,
            font_size: self.text_font_size,
            text_color: [self.text_color[0], self.text_color[1], self.text_color[2], 255],
            bg_color,
            alignment: self.text_alignment,
            padding: self.text_padding,
        };
        text::render_text(&config, self.canvas_width(), self.canvas_height()).ok()
    }
}

// ── Clipart Library UI ──────────────────────────────────────────────

impl RustBrushApp {
    fn library_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Clipart Library");
        ui.small("Click an image to use it as the source image.");
        ui.separator();

        let categories = library::categories();

        // Category tabs
        ui.horizontal_wrapped(|ui| {
            for (i, cat) in categories.iter().enumerate() {
                if ui
                    .selectable_label(self.library_category_idx == i, *cat)
                    .clicked()
                {
                    self.library_category_idx = i;
                }
            }
        });
        ui.separator();

        let items = library::all_items();
        let cat = categories.get(self.library_category_idx).copied().unwrap_or("");
        let filtered: Vec<(usize, &library::LibraryItem)> = items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.category == cat)
            .collect();

        // Grid of thumbnails
        let thumb_size = 80.0;
        let mut selected_idx: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (global_idx, item) in &filtered {
                    // Lazy-load thumbnails
                    if self.library_thumbnails[*global_idx].is_none() {
                        let thumb = item.thumbnail();
                        self.library_thumbnails[*global_idx] =
                            Some(upload_texture(ctx, &format!("lib_{}", global_idx), &thumb));
                    }

                    ui.vertical(|ui| {
                        if let Some(ref tex) = self.library_thumbnails[*global_idx] {
                            let response = ui.add(
                                egui::Image::new(egui::load::SizedTexture::new(
                                    tex.id(),
                                    egui::vec2(thumb_size, thumb_size),
                                ))
                                .sense(egui::Sense::click()),
                            );
                            if response.clicked() {
                                selected_idx = Some(*global_idx);
                            }
                            response.on_hover_text(item.name);
                        }
                        ui.small(item.name);
                    });
                }
            });
        });

        // Handle selection
        if let Some(idx) = selected_idx {
            let item = &items[idx];
            let cw = self.canvas_width();
            let ch = self.canvas_height();
            let img = item.render(cw, ch);
            self.source_image = Some(img);
            self.image_path = None;
            self.source_texture = None;
            self.preview_texture = None;
            self.preview_image = None;
            self.mapped_pixels = None;
            self.paint_groups = None;
            self.paint_plan = None;
            self.last_pixel_count = None;
            self.last_color_count = None;
            self.update_time_estimate();
            self.mark_settings_changed(true);
            self.status_message = format!("Loaded '{}' from library. Processing...", item.name);
            self.show_library = false;
        }

        ui.separator();
        if ui.button("Close").clicked() {
            self.show_library = false;
        }
    }
}

// ── System font discovery ───────────────────────────────────────────

fn enumerate_system_fonts() -> Vec<(String, std::path::PathBuf)> {
    let mut fonts = Vec::new();

    if let Ok(source) = font_kit::source::SystemSource::new().all_fonts() {
        for handle in source {
            if let font_kit::handle::Handle::Path { path, font_index: 0 } = handle {
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy().to_string();
                    fonts.push((name, path));
                }
            }
        }
    }

    fonts.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    fonts.dedup_by(|a, b| a.0 == b.0);
    fonts
}

fn upload_texture(
    ctx: &egui::Context,
    name: &str,
    img: &image::RgbaImage,
) -> egui::TextureHandle {
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.as_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels);
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}

fn fit_image_size(image_size: egui::Vec2, max_width: f32, max_height: f32) -> egui::Vec2 {
    let ratio = (max_width / image_size.x)
        .min(max_height / image_size.y)
        .min(1.0);
    image_size * ratio
}

fn parse_hex_color(s: &str) -> Result<(u8, u8, u8), String> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err("Hex color must be 6 characters".to_string());
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| e.to_string())?;
    Ok((r, g, b))
}
