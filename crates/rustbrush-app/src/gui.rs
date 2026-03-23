//! RustBrush GUI - egui/eframe application for automatic sign painting.

use eframe::egui;
use rustbrush_core::canvas;
use rustbrush_core::color::{
    build_preview, generate_adaptive_palette, map_image_to_palette, rust_palette,
    ColorMatchAlgo, DitherMode, MappedPixel, QuantizeOptions,
};
use rustbrush_core::config::Config;
use rustbrush_core::image as rb_image;
use rustbrush_core::painting::{self, ColorGroup, PaintPlan, PaintStrategy, ScreenRect};
use std::path::PathBuf;
use std::sync::mpsc;
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

    fn to_config_str(&self) -> &str {
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
            Self::Speed => "Hybrid + RGB + No dither",
            Self::Balanced => "Hybrid + RGB + Ordered dither",
            Self::Quality => "Grouped + CIEDE2000 + F-S dither + Adaptive 128",
            Self::Maximum => "Scanline + CIEDE2000 + F-S dither + Adaptive 512",
            Self::Custom => "Custom settings",
        }
    }

    fn to_config_str(&self) -> &str {
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
            strategy: self.strategy.to_config_str().to_string(),
            alpha_threshold: self.alpha_threshold,
            delay_ms: self.delay_ms,
            hex_input: self.hex_input,
            save_session: self.save_session,
            adaptive_palette: self.adaptive_palette,
            adaptive_colors: self.adaptive_colors,
            brightness: self.brightness,
            contrast: self.contrast,
            saturation: self.saturation,
            quality_preset: self.quality_preset.to_config_str().to_string(),
        };
        let _ = config.save_default();
    }

    fn load_image(&mut self, path: PathBuf) {
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
            StrategyChoice::Hybrid => Box::new(painting::HybridStrategy::default()),
            StrategyChoice::ColorGrouped => Box::new(painting::ColorGroupedStrategy),
            StrategyChoice::LineDraw => Box::new(painting::LineDrawStrategy::default()),
            StrategyChoice::Scanline => Box::new(painting::ScanlineStrategy),
        };

        let plan = strategy.plan(
            groups,
            &canvas,
            self.canvas_width(),
            self.canvas_height(),
            self.hex_input || self.adaptive_palette,
            30,
        );

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
            }
            QualityPreset::Balanced => {
                self.strategy = StrategyChoice::Hybrid;
                self.dither = DitherMode::Ordered;
                self.color_match = ColorMatchAlgo::Rgb;
                self.delay_ms = 10;
                self.adaptive_palette = false;
            }
            QualityPreset::Quality => {
                self.strategy = StrategyChoice::ColorGrouped;
                self.dither = DitherMode::FloydSteinberg;
                self.color_match = ColorMatchAlgo::Ciede2000;
                self.delay_ms = 15;
                self.adaptive_palette = true;
                self.adaptive_colors = 128;
            }
            QualityPreset::Maximum => {
                self.strategy = StrategyChoice::Scanline;
                self.dither = DitherMode::FloydSteinberg;
                self.color_match = ColorMatchAlgo::Ciede2000;
                self.delay_ms = 30;
                self.adaptive_palette = true;
                self.adaptive_colors = 512;
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
        let use_hex = self.hex_input || self.adaptive_palette;
        self.estimated_time_secs = Some(painting::estimate_time_approx(
            pixels,
            colors,
            self.strategy.to_config_str(),
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
        let brightness = self.brightness;
        let contrast = self.contrast;
        let saturation = self.saturation;
        let skip_color_enabled = self.skip_color_enabled;
        let skip_color_hex = self.skip_color_hex.clone();
        let adaptive_palette = self.adaptive_palette;
        let adaptive_colors = self.adaptive_colors;
        let color_match = self.color_match;
        let dither = self.dither;
        let alpha_threshold = self.alpha_threshold;
        let generation = self.settings_generation;

        let (tx, rx) = mpsc::channel();
        self.bg_result_rx = Some(rx);
        self.processing = true;

        std::thread::spawn(move || {
            // Resize
            let mut resized = rb_image::resize(&source, w, h, rb_image::AspectRatio::Stretch);

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

            // Parse skip color
            let skip_color = if skip_color_enabled {
                parse_hex_color(&skip_color_hex).ok()
            } else {
                None
            };

            // Determine palette
            let palette = if adaptive_palette {
                generate_adaptive_palette(&resized, adaptive_colors)
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

        // Debounced auto-reprocess
        if self.pending_reprocess
            && self.source_image.is_some()
            && self.bg_result_rx.is_none()
            && self.last_settings_change.elapsed() >= Duration::from_millis(500)
        {
            self.pending_reprocess = false;
            self.start_background_process();
        }

        // Keep polling while background task is active
        if self.bg_result_rx.is_some() {
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

        // Central panel - image preview
        egui::CentralPanel::default().show(ctx, |ui| {
            self.preview_ui(ui, ctx);
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

        ui.add_enabled_ui(has_image && !self.processing, |ui| {
            if ui
                .button("Reprocess Now")
                .on_hover_text("Manually re-run image processing")
                .clicked()
            {
                self.start_background_process();
            }
        });

        ui.add_enabled_ui(has_groups, |ui| {
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

        ui.horizontal(|ui| {
            ui.heading("Source");
            if has_preview {
                ui.separator();
                ui.heading("Preview (quantized)");
            }
        });

        ui.separator();

        let available = ui.available_size();
        let half_width = if has_preview {
            (available.x - 10.0) / 2.0
        } else {
            available.x
        };

        ui.horizontal(|ui| {
            if let Some(ref tex) = self.source_texture {
                let size = fit_image_size(tex.size_vec2(), half_width, available.y - 30.0);
                ui.image(egui::load::SizedTexture::new(tex.id(), size));
            }

            if has_preview {
                ui.separator();

                if let Some(ref tex) = self.preview_texture {
                    let size = fit_image_size(tex.size_vec2(), half_width, available.y - 30.0);
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
