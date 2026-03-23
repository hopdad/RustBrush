//! RustBrush - Automatic sign painter for Rust (the game)
//!
//! SAFETY: This tool uses ONLY OS-level input simulation (SendInput/xdotool).
//! It does NOT read or write game memory, inject DLLs, or hook into any process.
//! However, it has NOT been whitelisted by Facepunch/EAC. Use at your own risk.

use clap::Parser;
use rustbrush_core::color::{ColorMatchAlgo, DitherMode, QuantizeOptions};
use rustbrush_core::painting;
use rustbrush_platform::capture;
use rustbrush_platform::hotkey::{PaintControl, region};
use rustbrush_platform::input::{InputDriver, SafeInput};
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

    /// Canvas preset name (e.g. "wooden sign", "portrait frame")
    #[arg(long)]
    preset: Option<String>,
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

    // Resize to canvas dimensions
    let img = rustbrush_core::image::resize(
        &img,
        cli.canvas_width,
        cli.canvas_height,
        rustbrush_core::image::AspectRatio::Stretch,
    );

    // Build quantization options
    let opts = QuantizeOptions {
        algorithm: cli.color_match,
        dither: cli.dither,
        alpha_threshold: cli.alpha_threshold,
        skip_color: cli.skip_color,
        ..Default::default()
    };

    // Map every pixel to the nearest Rust in-game palette color
    let palette = rustbrush_core::color::rust_palette();
    let pixel_plan = rustbrush_core::color::map_image_to_palette(&img, &palette, &opts);

    // Save preview if requested
    if let Some(ref preview_path) = cli.preview {
        let preview_img = rustbrush_core::color::build_preview(&img, &pixel_plan);
        match preview_img.save(preview_path) {
            Ok(()) => println!("Preview saved to: {}", preview_path.display()),
            Err(e) => eprintln!("Failed to save preview: {}", e),
        }
    }

    // Group pixels by color
    let paint_groups = painting::group_by_color(&pixel_plan);

    let total_pixels: usize = paint_groups.iter().map(|g| g.pixels.len()).sum();
    let total_colors = paint_groups.len();

    println!(
        "Image: {}x{} -> {}x{} canvas",
        img.width(), img.height(), cli.canvas_width, cli.canvas_height
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
            "[DRY RUN] Estimated time: {:.0}s",
            total_pixels as f64 * cli.delay_ms as f64 / 1000.0
        );
        for group in &paint_groups {
            println!(
                "  Color #{}: {} pixels",
                group.hex, group.pixels.len()
            );
        }
        return;
    }

    // --- Interactive region capture ---
    println!("\n=== Region Capture ===");
    println!("Switch to the Rust game window with the sign editor open.");
    println!("You'll mark regions by pressing a hotkey, then clicking and dragging.\n");

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
        palette_region.x,
        palette_region.y,
        palette_region.width,
        palette_region.height,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Palette sampling failed: {}", e);
            return;
        }
    };

    println!("Sampled {} unique colors from palette", palette_colors.len());

    // Countdown before painting
    println!("\nPainting starts in {} seconds...", cli.startup_delay);
    println!("Make sure the brush tool is selected!");
    for i in (1..=cli.startup_delay).rev() {
        println!("  {}...", i);
        std::thread::sleep(Duration::from_secs(1));
    }

    // Set up hotkey controls and start painting
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

    let use_hex = hex_input_pos.is_some();
    let total_pixels_count = total_pixels;
    let mut painted = 0usize;

    println!(
        "Starting painting: {} colors, {} pixels{}",
        total_colors, total_pixels_count,
        if use_hex { " (hex input mode)" } else { "" }
    );
    println!("Controls: F10 = pause/resume, ESC = cancel");

    for (i, group) in paint_groups.iter().enumerate() {
        if !control.check() {
            println!(
                "\nPainting cancelled. {}/{} pixels painted ({:.1}%).",
                painted, total_pixels_count,
                painted as f64 / total_pixels_count as f64 * 100.0
            );
            return;
        }

        println!(
            "[{}/{}] Color #{} ({} pixels)",
            i + 1, total_colors, group.hex, group.pixels.len()
        );

        // Select the color
        if let Some(hex_pos) = hex_input_pos {
            if let Err(e) = select_color_by_hex(&mut input, &group.hex, hex_pos) {
                eprintln!("Color selection failed: {}", e);
                return;
            }
        } else {
            let entry = find_nearest_palette_entry(
                group.color.0, group.color.1, group.color.2,
                &palette_colors,
            );
            if let Err(e) = input.move_to(entry.screen_x, entry.screen_y) {
                eprintln!("Move failed: {}", e);
                return;
            }
            if let Err(e) = input.click() {
                eprintln!("Click failed: {}", e);
                return;
            }
        }

        std::thread::sleep(Duration::from_millis(30));

        // Paint each pixel in this group
        for &(px, py) in &group.pixels {
            if !control.check() {
                println!(
                    "\nPainting cancelled. {}/{} pixels painted ({:.1}%).",
                    painted, total_pixels_count,
                    painted as f64 / total_pixels_count as f64 * 100.0
                );
                return;
            }

            let (screen_x, screen_y) = painting::pixel_to_screen(
                px, py,
                cli.canvas_width, cli.canvas_height,
                &painting::ScreenRect {
                    x: canvas_region.x,
                    y: canvas_region.y,
                    width: canvas_region.width,
                    height: canvas_region.height,
                },
            );

            if let Err(e) = input.move_to(screen_x, screen_y) {
                eprintln!("Move failed: {}", e);
                return;
            }
            if let Err(e) = input.click() {
                eprintln!("Click failed: {}", e);
                return;
            }

            painted += 1;
            if painted % 500 == 0 {
                let pct = (painted as f64 / total_pixels_count as f64) * 100.0;
                println!("  Progress: {}/{} ({:.1}%)", painted, total_pixels_count, pct);
            }
        }
    }

    println!("\nPainting complete! {} pixels painted.", painted);
}

fn select_color_by_hex(
    input: &mut impl InputDriver,
    hex: &str,
    hex_input_pos: (i32, i32),
) -> Result<(), String> {
    input.move_to(hex_input_pos.0, hex_input_pos.1)?;
    input.click()?;
    std::thread::sleep(Duration::from_millis(30));
    input.select_all()?;
    std::thread::sleep(Duration::from_millis(20));
    input.type_text(hex)?;
    std::thread::sleep(Duration::from_millis(20));
    input.press_key_return()?;
    std::thread::sleep(Duration::from_millis(30));
    Ok(())
}

fn find_nearest_palette_entry(
    r: u8, g: u8, b: u8,
    palette: &[capture::PaletteEntry],
) -> &capture::PaletteEntry {
    palette
        .iter()
        .min_by_key(|e| {
            let dr = r as i32 - e.r as i32;
            let dg = g as i32 - e.g as i32;
            let db = b as i32 - e.b as i32;
            (dr * dr + dg * dg + db * db) as u32
        })
        .unwrap()
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
