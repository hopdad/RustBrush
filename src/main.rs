//! RustBrush - Automatic sign painter for Rust (the game)
//!
//! SAFETY: This tool uses ONLY OS-level input simulation (SendInput/xdotool).
//! It does NOT read or write game memory, inject DLLs, or hook into any process.
//! However, it has NOT been whitelisted by Facepunch/EAC. Use at your own risk.

mod color;
mod hotkeys;
mod input;
mod painter;
mod region;
mod screen;

use clap::Parser;
use color::{ColorMatchAlgo, DitherMode, QuantizeOptions};
use device_query::Keycode;
use std::path::PathBuf;
use std::time::Duration;

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

HEX INPUT MODE (--hex-input):
  For exact color reproduction, use --hex-input to type hex codes
  directly into the game's color input field instead of clicking
  the palette. After marking canvas (F9), you'll also press F7
  and click on the hex input field.
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
}

fn parse_color_algo(s: &str) -> Result<ColorMatchAlgo, String> {
    match s.to_lowercase().as_str() {
        "ciede2000" | "perceptual" => Ok(ColorMatchAlgo::Ciede2000),
        "rgb" | "euclidean" => Ok(ColorMatchAlgo::Rgb),
        _ => Err(format!("Unknown color matching algorithm '{}'. Use 'ciede2000' or 'rgb'.", s)),
    }
}

fn parse_dither_mode(s: &str) -> Result<DitherMode, String> {
    match s.to_lowercase().replace('-', "").as_str() {
        "none" => Ok(DitherMode::None),
        "floydsteinberg" | "fs" => Ok(DitherMode::FloydSteinberg),
        "ordered" | "bayer" => Ok(DitherMode::Ordered),
        _ => Err(format!(
            "Unknown dithering mode '{}'. Use 'none', 'floyd-steinberg', or 'ordered'.",
            s
        )),
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
    let cli = Cli::parse();

    if !cli.accept_risk && !cli.dry_run {
        print_disclaimer();
        if !confirm_proceed() {
            println!("Aborted. Use --dry-run to simulate without sending input.");
            return;
        }
    }

    // Load and process the image
    let img = match image::open(&cli.image) {
        Ok(img) => img.to_rgba8(),
        Err(e) => {
            eprintln!("Failed to load image '{}': {}", cli.image.display(), e);
            std::process::exit(1);
        }
    };

    // Resize to canvas dimensions
    let img = image::imageops::resize(
        &img,
        cli.canvas_width,
        cli.canvas_height,
        image::imageops::FilterType::Lanczos3,
    );

    // Build quantization options from CLI args
    let opts = QuantizeOptions {
        algorithm: cli.color_match,
        dither: cli.dither,
        alpha_threshold: cli.alpha_threshold,
        skip_color: cli.skip_color,
        ..Default::default()
    };

    // Map every pixel to the nearest Rust in-game palette color
    let palette = color::rust_palette();
    let pixel_plan = color::map_image_to_palette(&img, &palette, &opts);

    // Save preview if requested
    if let Some(ref preview_path) = cli.preview {
        let preview_img = color::build_preview(&img, &pixel_plan);
        match preview_img.save(preview_path) {
            Ok(()) => println!("Preview saved to: {}", preview_path.display()),
            Err(e) => eprintln!("Failed to save preview: {}", e),
        }
    }

    // Group pixels by color for efficient painting
    let paint_groups = painter::group_by_color(&pixel_plan, cli.canvas_width, cli.canvas_height);

    let total_pixels: usize = paint_groups.iter().map(|g| g.pixels.len()).sum();
    let total_colors = paint_groups.len();

    println!(
        "Image: {}x{} -> {}x{} canvas",
        img.width(),
        img.height(),
        cli.canvas_width,
        cli.canvas_height
    );
    println!("Colors used: {}, Total pixels: {}", total_colors, total_pixels);
    println!(
        "Color matching: {:?}, Dithering: {:?}, Alpha threshold: {}",
        opts.algorithm, opts.dither, opts.alpha_threshold
    );
    if let Some((r, g, b)) = opts.skip_color {
        println!("Skipping background color: #{:02X}{:02X}{:02X}", r, g, b);
    }

    if cli.dry_run {
        println!(
            "\n[DRY RUN] Would paint {} pixels across {} colors.",
            total_pixels, total_colors
        );
        println!(
            "[DRY RUN] No input will be sent. Estimated time: {:.0}s",
            total_pixels as f64 * cli.delay_ms as f64 / 1000.0
        );
        for group in &paint_groups {
            println!(
                "  Color #{:02X}{:02X}{:02X}: {} pixels",
                group.color.0, group.color.1, group.color.2, group.pixels.len()
            );
        }
        return;
    }

    // --- Interactive region capture ---
    println!("\n=== Region Capture ===");
    println!("Switch to the Rust game window with the sign editor open.");
    println!("You'll mark regions by pressing a hotkey, then clicking and dragging.\n");

    // Capture canvas region (F9)
    let canvas = match region::capture_region_interactive("Canvas", Keycode::F9) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Canvas capture failed: {}", e);
            return;
        }
    };

    // Capture hex input position if hex mode is enabled
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

    // Capture palette region (F8) - still needed even in hex mode for fallback
    let palette_region = match region::capture_region_interactive("Palette", Keycode::F8) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Palette capture failed: {}", e);
            return;
        }
    };

    // Sample colors from the palette
    let palette_colors = match region::sample_palette_colors(&palette_region) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Palette sampling failed: {}", e);
            return;
        }
    };

    let layout = region::CapturedLayout {
        canvas,
        palette: palette_region,
        palette_colors,
        hex_input_pos,
    };

    // Countdown before painting
    println!("\nPainting starts in {} seconds...", cli.startup_delay);
    println!("Make sure the brush tool is selected!");
    for i in (1..=cli.startup_delay).rev() {
        println!("  {}...", i);
        std::thread::sleep(Duration::from_secs(1));
    }

    // Set up hotkey controls and start painting
    let control = hotkeys::PaintControl::new();
    control.start_listener();

    let delay = Duration::from_millis(cli.delay_ms);
    match painter::paint(
        &paint_groups,
        &layout,
        cli.canvas_width,
        cli.canvas_height,
        delay,
        &control,
    ) {
        Ok(painter::PaintResult::Completed { painted }) => {
            println!("\nPainting complete! {} pixels painted.", painted);
        }
        Ok(painter::PaintResult::Cancelled {
            painted,
            total_pixels,
        }) => {
            println!(
                "\nPainting cancelled. {}/{} pixels painted ({:.1}%).",
                painted,
                total_pixels,
                painted as f64 / total_pixels as f64 * 100.0
            );
        }
        Err(e) => eprintln!("\nPainting failed: {}", e),
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
