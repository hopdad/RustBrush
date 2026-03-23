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
                self.status_message =
                    "Image loaded. Adjust settings and click Process.".to_string();
            }
            Err(e) => {
                self.status_message = format!("Failed to load image: {}", e);
            }
        }
    }

    fn process_image(&mut self) {
        let Some(ref source) = self.source_image else {
            self.status_message = "No image loaded.".to_string();
            return;
        };

        self.processing = true;
        let w = self.canvas_width();
        let h = self.canvas_height();

        // Resize
        let mut resized = rb_image::resize(source, w, h, rb_image::AspectRatio::Stretch);

        // Apply image adjustments
        if (self.brightness - 1.0).abs() > 0.01 {
            resized = rb_image::adjust_brightness(&resized, self.brightness);
        }
        if (self.contrast - 1.0).abs() > 0.01 {
            resized = rb_image::adjust_contrast(&resized, self.contrast);
        }
        if (self.saturation - 1.0).abs() > 0.01 {
            resized = rb_image::adjust_saturation(&resized, self.saturation);
        }

        // Parse skip color
        let skip_color = if self.skip_color_enabled {
            parse_hex_color(&self.skip_color_hex).ok()
        } else {
            None
        };

        // Determine palette
        let palette = if self.adaptive_palette {
            generate_adaptive_palette(&resized, self.adaptive_colors)
        } else {
            rust_palette()
        };

        // Quantize
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

        let total_pixels: usize = groups.iter().map(|g| g.pixels.len()).sum();
        let total_colors = groups.len();

        self.mapped_pixels = Some(mapped);
        self.preview_image = Some(preview);
        self.preview_texture = None;
        self.paint_groups = Some(groups);
        self.paint_plan = None;

        self.status_message = format!(
            "Processed: {}x{}, {} colors ({}), {} pixels.",
            w,
            h,
            total_colors,
            if self.adaptive_palette {
                format!("adaptive {}", self.adaptive_colors)
            } else {
                "fixed 32".to_string()
            },
            total_pixels
        );
        self.processing = false;
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
}

impl eframe::App for RustBrushApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            // Clear preview so user re-processes
            self.preview_image = None;
            self.preview_texture = None;
            self.mapped_pixels = None;
            self.paint_groups = None;
            self.paint_plan = None;
        }

        ui.separator();

        // --- Canvas Size ---
        ui.heading("Canvas");
        ui.checkbox(&mut self.use_custom_size, "Custom size");

        if self.use_custom_size {
            ui.horizontal(|ui| {
                ui.label("W:");
                ui.add(egui::DragValue::new(&mut self.custom_width).range(8..=2048));
                ui.label("H:");
                ui.add(egui::DragValue::new(&mut self.custom_height).range(8..=2048));
            });
        } else {
            let presets = canvas::all_presets();
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
        }
        ui.label(format!(
            "Canvas: {}x{}",
            self.canvas_width(),
            self.canvas_height()
        ));

        ui.separator();

        // --- Color Matching ---
        ui.heading("Color");
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

        ui.add(
            egui::Slider::new(&mut self.alpha_threshold, 0..=255).text("Alpha threshold"),
        );

        ui.checkbox(&mut self.skip_color_enabled, "Skip background color");
        if self.skip_color_enabled {
            ui.horizontal(|ui| {
                ui.label("#");
                ui.text_edit_singleline(&mut self.skip_color_hex);
            });
        }

        ui.separator();

        // --- Adaptive Palette ---
        ui.heading("Palette");
        ui.checkbox(&mut self.adaptive_palette, "Adaptive palette (k-means)")
            .on_hover_text("Generate optimal colors from the image via clustering");
        if self.adaptive_palette {
            ui.add(
                egui::Slider::new(&mut self.adaptive_colors, 16..=512)
                    .text("Colors")
                    .logarithmic(true),
            );
            ui.label("Requires hex input mode");
        }

        ui.separator();

        // --- Strategy ---
        ui.heading("Painting");
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

        ui.add(egui::Slider::new(&mut self.delay_ms, 5..=100).text("Delay (ms)"));
        ui.checkbox(&mut self.hex_input, "Hex input mode");
        ui.checkbox(&mut self.save_session, "Auto-save session");

        ui.separator();

        // --- Actions ---
        ui.heading("Actions");

        let has_image = self.source_image.is_some();
        let has_groups = self.paint_groups.is_some();

        ui.add_enabled_ui(has_image && !self.processing, |ui| {
            if ui
                .button("Process Image")
                .on_hover_text("Resize, adjust, color-match, and dither the image")
                .clicked()
            {
                self.process_image();
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
