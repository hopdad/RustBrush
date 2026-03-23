//! RustBrush - Automatic sign painter for Rust (the game)
//!
//! SAFETY: This tool uses ONLY OS-level input simulation (SendInput/xdotool).
//! It does NOT read or write game memory, inject DLLs, or hook into any process.
//! However, it has NOT been whitelisted by Facepunch/EAC. Use at your own risk.

use clap::Parser;
use rustbrush_core::color::{ColorMatchAlgo, DitherMode, QuantizeOptions};
use rustbrush_core::painting::{self, PaintStrategy, ScreenRect};
use rustbrush_core::session::Session;
use rustbrush_platform::capture;
use rustbrush_platform::executor::{self, ExecutionResult, ExecutorConfig};
use rustbrush_platform::hotkey::{PaintControl, region};
use rustbrush_platform::input::SafeInput;
use std::path::PathBuf;
use std::time::Duration;

use device_query::Keycode;

#[derive(Parser, Debug)]
#[command(name = "rustbrush")]
#[command(about = "Automatic sign painter for Rust (the game)")]
#[command(after_help = "\
SAFETY DISCLAIMER:
  RustBrush uses only OS-level input simulation (SendInput/xdotool).
  It does NOT inject into the game, read game memory, or modify game files.

  However, this tool has NOT been officially whitelisted by Facepunch or EAC.
  Use at your own risk. We recommend testing on a private server first
  (launch with +server.secure 0 to disable EAC).

WORKFLOW:
  1. Open the sign editor in Rust and select the brush tool
  2. Run rustbrush with your image
  3. Press F9, then click and drag to select the CANVAS area
  4. Press F8, then click and drag to select the COLOR PALETTE area
  5. Painting begins after a countdown
  6. During painting: F10 = pause/resume, ESC = cancel

STRATEGIES:
  --strategy scanline        Row-by-row painting (simplest)
  --strategy color-grouped   Group by color, nearest-neighbor ordering (fewer switches)
  --strategy line-draw       Detect horizontal runs, use shift-click lines
  --strategy hybrid          Color grouping + line detection (default, fastest)

HEX INPUT MODE (--hex-input):
  For exact color reproduction, use --hex-input to type hex codes
  directly into the game's color input field instead of clicking
  the palette.
")]
struct Cli {
    /// Path to the image file to paint
    image: PathBuf,

    /// Canvas width in pixels for image scaling
    #[arg(short = 'W', long, default_value_t = 256)]
    canvas_width: u32,

    /// Canvas height in pixels for image scaling
    #[arg(short = 'H', long, default_value_t = 256)]
    canvas_height: u32,

    /// Delay between mouse actions in milliseconds
    #[arg(short, long, default_value_t = 15)]
    delay_ms: u64,

    /// Dry-run mode: process image without capturing regions or painting
    #[arg(long)]
    dry_run: bool,

    /// Seconds to wait before painting starts (after region capture)
    #[arg(short, long, default_value_t = 3)]
    startup_delay: u32,

    /// Use hex code input for exact colors (requires marking the hex input field)
    #[arg(long)]
    hex_input: bool,

    /// Skip the safety disclaimer confirmation
    #[arg(long)]
    accept_risk: bool,

    /// Color matching algorithm: "ciede2000" (perceptual, default) or "rgb" (fast)
    #[arg(long, default_value = "ciede2000", value_parser = parse_color_algo)]
    color_match: ColorMatchAlgo,

    /// Dithering mode: "none" (default), "floyd-steinberg", or "ordered"
    #[arg(long, default_value = "none", value_parser = parse_dither_mode)]
    dither: DitherMode,

    /// Alpha threshold (0-255). Pixels with alpha below this are skipped.
    #[arg(long, default_value_t = 128)]
    alpha_threshold: u8,

    /// Skip a background color (hex, e.g. "FFFFFF" to skip white)
    #[arg(long, value_parser = parse_hex_color)]
    skip_color: Option<(u8, u8, u8)>,

    /// Save a preview of the quantized image to this path before painting
    #[arg(long)]
    preview: Option<PathBuf>,

    /// Canvas preset name (e.g. "wooden sign", "portrait frame")
    #[arg(long)]
    preset: Option<String>,

    /// Painting strategy: "hybrid" (default), "color-grouped", "line-draw", or "scanline"
    #[arg(long, default_value = "hybrid", value_parser = parse_strategy)]
    strategy: StrategyChoice,

    /// Resume a previously interrupted session from a JSON file
    #[arg(long)]
    resume: Option<PathBuf>,

    /// Save session progress for crash recovery (auto-save path)
    #[arg(long)]
    save_session: bool,
}

#[derive(Debug, Clone, Copy)]
enum StrategyChoice {
    Scanline,
    ColorGrouped,
    LineDraw,
    Hybrid,
}

fn parse_strategy(s: &str) -> Result<StrategyChoice, String> {
    match s.to_lowercase().replace('-', "").as_str() {
        "scanline" => Ok(StrategyChoice::Scanline),
        "colorgrouped" | "grouped" => Ok(StrategyChoice::ColorGrouped),
        "linedraw" | "line" => Ok(StrategyChoice::LineDraw),
        "hybrid" => Ok(StrategyChoice::Hybrid),
        _ => Err(format!("Unknown strategy '{}'. Use 'hybrid', 'color-grouped', 'line-draw', or 'scanline'.", s)),
    }
}

fn parse_color_algo(s: &str) -> Result<ColorMatchAlgo, String> {
    match s.to_lowercase().as_str() {
        "ciede2000" | "perceptual" => Ok(ColorMatchAlgo::Ciede2000),
        "rgb" | "euclidean" => Ok(ColorMatchAlgo::Rgb),
        _ => Err(format!("Unknown algorithm '{}'. Use 'ciede2000' or 'rgb'.", s)),
    }
}

fn parse_dither_mode(s: &str) -> Result<DitherMode, String> {
    match s.to_lowercase().replace('-', "").as_str() {
        "none" => Ok(DitherMode::None),
        "floydsteinberg" | "fs" => Ok(DitherMode::FloydSteinberg),
        "ordered" | "bayer" => Ok(DitherMode::Ordered),
        _ => Err(format!("Unknown dithering mode '{}'. Use 'none', 'floyd-steinberg', or 'ordered'.", s)),
    }
}

fn parse_hex_color(s: &str) -> Result<(u8, u8, u8), String> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err("Hex color must be 6 characters (e.g. FFFFFF or #FF0000)".to_string());
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| e.to_string())?;
    Ok((r, g, b))
}

fn main() {
    env_logger::init();
    let mut cli = Cli::parse();

    // Handle session resume
    if let Some(ref resume_path) = cli.resume {
        resume_session(resume_path, &cli);
        return;
    }

    // Apply canvas preset if specified
    if let Some(ref preset_name) = cli.preset {
        if let Some(preset) = rustbrush_core::canvas::find_preset(preset_name) {
            cli.canvas_width = preset.width;
            cli.canvas_height = preset.height;
            println!("Using preset: {}", preset);
        } else {
            eprintln!("Unknown preset '{}'. Available presets:", preset_name);
            for p in rustbrush_core::canvas::all_presets() {
                eprintln!("  {}", p);
            }
            std::process::exit(1);
        }
    }

    if !cli.accept_risk && !cli.dry_run {
        print_disclaimer();
        if !confirm_proceed() {
            println!("Aborted. Use --dry-run to simulate without sending input.");
            return;
        }
    }

    // Load and process the image
    let img = match rustbrush_core::image::load_image(&cli.image) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let img = rustbrush_core::image::resize(
        &img,
        cli.canvas_width,
        cli.canvas_height,
        rustbrush_core::image::AspectRatio::Stretch,
    );

    let opts = QuantizeOptions {
        algorithm: cli.color_match,
        dither: cli.dither,
        alpha_threshold: cli.alpha_threshold,
        skip_color: cli.skip_color,
        ..Default::default()
    };

    let palette = rustbrush_core::color::rust_palette();
    let pixel_plan = rustbrush_core::color::map_image_to_palette(&img, &palette, &opts);

    if let Some(ref preview_path) = cli.preview {
        let preview_img = rustbrush_core::color::build_preview(&img, &pixel_plan);
        match preview_img.save(preview_path) {
            Ok(()) => println!("Preview saved to: {}", preview_path.display()),
            Err(e) => eprintln!("Failed to save preview: {}", e),
        }
    }

    let paint_groups = painting::group_by_color(&pixel_plan);
    let total_pixels: usize = paint_groups.iter().map(|g| g.pixels.len()).sum();
    let total_colors = paint_groups.len();

    println!(
        "Image: {}x{} -> {}x{} canvas",
        img.width(), img.height(), cli.canvas_width, cli.canvas_height
    );
    println!("Colors used: {}, Total pixels: {}", total_colors, total_pixels);
    println!(
        "Color matching: {:?}, Dithering: {:?}, Strategy: {:?}",
        opts.algorithm, opts.dither, strategy_name(cli.strategy)
    );

    if cli.dry_run {
        // In dry-run, generate the plan and show stats
        let canvas = ScreenRect { x: 0, y: 0, width: cli.canvas_width, height: cli.canvas_height };
        let plan = build_strategy(cli.strategy).plan(
            &paint_groups, &canvas, cli.canvas_width, cli.canvas_height,
            cli.hex_input, 30,
        );
        println!(
            "\n[DRY RUN] Would paint {} pixels across {} colors.",
            total_pixels, total_colors
        );
        println!(
            "[DRY RUN] Strategy '{}' generates {} commands.",
            plan.metadata.strategy_name, plan.metadata.total_commands
        );
        println!(
            "[DRY RUN] Estimated time: {:.0}s",
            painting::estimate_time(&plan, cli.delay_ms)
        );
        for group in &paint_groups {
            println!("  Color #{}: {} pixels", group.hex, group.pixels.len());
        }
        return;
    }

    // --- Interactive region capture ---
    println!("\n=== Region Capture ===");
    println!("Switch to the Rust game window with the sign editor open.");

    let canvas_region = match region::capture_region_interactive("Canvas", Keycode::F9) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Canvas capture failed: {}", e);
            return;
        }
    };

    let hex_input_pos = if cli.hex_input {
        match region::capture_point_interactive("Hex color input field", Keycode::F7) {
            Ok(pos) => Some(pos),
            Err(e) => {
                eprintln!("Hex input capture failed: {}", e);
                return;
            }
        }
    } else {
        None
    };

    let palette_region = match region::capture_region_interactive("Palette", Keycode::F8) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Palette capture failed: {}", e);
            return;
        }
    };

    let palette_colors = match capture::sample_palette_colors(
        palette_region.x, palette_region.y,
        palette_region.width, palette_region.height,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Palette sampling failed: {}", e);
            return;
        }
    };
    println!("Sampled {} unique colors from palette", palette_colors.len());

    // Generate paint plan
    let screen_rect = ScreenRect {
        x: canvas_region.x,
        y: canvas_region.y,
        width: canvas_region.width,
        height: canvas_region.height,
    };
    let use_hex = hex_input_pos.is_some();
    let strategy = build_strategy(cli.strategy);
    let plan = strategy.plan(
        &paint_groups, &screen_rect,
        cli.canvas_width, cli.canvas_height,
        use_hex, 30,
    );

    println!(
        "\nPlan: {} commands ({} strategy), est. {:.0}s",
        plan.metadata.total_commands,
        plan.metadata.strategy_name,
        painting::estimate_time(&plan, cli.delay_ms)
    );

    // Countdown
    println!("Painting starts in {} seconds...", cli.startup_delay);
    println!("Make sure the brush tool is selected!");
    for i in (1..=cli.startup_delay).rev() {
        println!("  {}...", i);
        std::thread::sleep(Duration::from_secs(1));
    }

    // Set up session for save/resume
    let session_path = if cli.save_session {
        Some(Session::generate_path(&cli.image.to_string_lossy()))
    } else {
        None
    };

    let mut session = Session::new(
        plan.clone(),
        screen_rect,
        cli.image.to_string_lossy().to_string(),
        cli.canvas_width,
        cli.canvas_height,
    );

    if let Some(ref path) = session_path {
        println!("Session will be saved to: {}", path.display());
    }

    // Execute
    let control = PaintControl::new();
    control.start_listener();

    let delay = Duration::from_millis(cli.delay_ms);
    let mut input = match SafeInput::new(delay) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to initialize input: {}", e);
            return;
        }
    };

    println!("Controls: F10 = pause/resume, ESC = cancel");

    let config = ExecutorConfig {
        save_interval: if cli.save_session { 500 } else { 0 },
        session_path: session_path.clone(),
        progress_interval: 500,
        progress_tx: None,
    };

    let result = executor::execute_plan(
        &plan, &mut input, &control, &config, Some(&mut session), 0,
    );

    match result {
        ExecutionResult::Completed { commands_executed } => {
            println!("\nPainting complete! {} commands executed.", commands_executed);
            // Clean up session file on successful completion
            if let Some(ref path) = session_path {
                let _ = std::fs::remove_file(path);
            }
        }
        ExecutionResult::Cancelled { commands_executed, total_commands } => {
            let pct = commands_executed as f64 / total_commands as f64 * 100.0;
            println!(
                "\nPainting cancelled. {}/{} commands ({:.1}%).",
                commands_executed, total_commands, pct
            );
            if let Some(ref path) = session_path {
                println!("Session saved. Resume with: rustbrush {} --resume {}", cli.image.display(), path.display());
            }
        }
        ExecutionResult::Error { commands_executed, error } => {
            eprintln!("\nPainting failed after {} commands: {}", commands_executed, error);
            if let Some(ref path) = session_path {
                println!("Session saved. Resume with: rustbrush {} --resume {}", cli.image.display(), path.display());
            }
        }
    }
}

fn resume_session(resume_path: &PathBuf, cli: &Cli) {
    let mut session = match Session::load(resume_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load session: {}", e);
            std::process::exit(1);
        }
    };

    if session.is_complete() {
        println!("Session is already complete.");
        return;
    }

    println!(
        "Resuming session: {} ({:.1}% complete, {} commands remaining)",
        session.image_path,
        session.progress_percent(),
        session.remaining_commands()
    );

    if !cli.accept_risk {
        print_disclaimer();
        if !confirm_proceed() {
            return;
        }
    }

    println!("\nPainting resumes in {} seconds...", cli.startup_delay);
    for i in (1..=cli.startup_delay).rev() {
        println!("  {}...", i);
        std::thread::sleep(Duration::from_secs(1));
    }

    let control = PaintControl::new();
    control.start_listener();

    let delay = Duration::from_millis(cli.delay_ms);
    let mut input = match SafeInput::new(delay) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to initialize input: {}", e);
            return;
        }
    };

    let start_index = session.progress;
    let config = ExecutorConfig {
        save_interval: 500,
        session_path: Some(resume_path.clone()),
        progress_interval: 500,
        progress_tx: None,
    };

    let result = executor::execute_plan(
        &session.plan.clone(), &mut input, &control, &config,
        Some(&mut session), start_index,
    );

    match result {
        ExecutionResult::Completed { commands_executed } => {
            println!("\nPainting complete! {} commands executed.", commands_executed);
            let _ = std::fs::remove_file(resume_path);
        }
        ExecutionResult::Cancelled { commands_executed, total_commands } => {
            let pct = (start_index + commands_executed) as f64 / total_commands as f64 * 100.0;
            println!(
                "\nPainting cancelled. Overall {:.1}% complete.",
                pct
            );
            println!("Resume with: rustbrush {} --resume {}", session.image_path, resume_path.display());
        }
        ExecutionResult::Error { error, .. } => {
            eprintln!("\nPainting failed: {}", error);
            println!("Resume with: rustbrush {} --resume {}", session.image_path, resume_path.display());
        }
    }
}

fn build_strategy(choice: StrategyChoice) -> Box<dyn PaintStrategy> {
    match choice {
        StrategyChoice::Scanline => Box::new(painting::ScanlineStrategy),
        StrategyChoice::ColorGrouped => Box::new(painting::ColorGroupedStrategy),
        StrategyChoice::LineDraw => Box::new(painting::LineDrawStrategy::default()),
        StrategyChoice::Hybrid => Box::new(painting::HybridStrategy::default()),
    }
}

fn strategy_name(choice: StrategyChoice) -> &'static str {
    match choice {
        StrategyChoice::Scanline => "scanline",
        StrategyChoice::ColorGrouped => "color-grouped",
        StrategyChoice::LineDraw => "line-draw",
        StrategyChoice::Hybrid => "hybrid",
    }
}

fn print_disclaimer() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    SAFETY DISCLAIMER                        ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ RustBrush uses ONLY OS-level input simulation.              ║");
    println!("║ It does NOT inject into the game or read game memory.       ║");
    println!("║                                                             ║");
    println!("║ However, this tool has NOT been officially whitelisted       ║");
    println!("║ by Facepunch Studios or Easy Anti-Cheat (EAC).              ║");
    println!("║                                                             ║");
    println!("║ We recommend testing on a private server first              ║");
    println!("║ (launch with +server.secure 0 to disable EAC).             ║");
    println!("║                                                             ║");
    println!("║ USE AT YOUR OWN RISK.                                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}

fn confirm_proceed() -> bool {
    use std::io::{self, Write};
    print!("\nDo you accept the risk and wish to proceed? [y/N] ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}
