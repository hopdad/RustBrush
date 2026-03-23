//! RustBrush - Automatic sign painter for Rust (the game)
//!
//! SAFETY: This tool uses ONLY OS-level input simulation (SendInput/xdotool).
//! It does NOT read or write game memory, inject DLLs, or hook into any process.
//! However, it has NOT been whitelisted by Facepunch/EAC. Use at your own risk.

mod color;
mod input;
mod painter;
mod screen;

use clap::Parser;
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
")]
struct Cli {
    /// Path to the image file to paint
    image: PathBuf,

    /// Canvas width in pixels (default: sign size)
    #[arg(short = 'W', long, default_value_t = 256)]
    canvas_width: u32,

    /// Canvas height in pixels (default: sign size)
    #[arg(short = 'H', long, default_value_t = 256)]
    canvas_height: u32,

    /// Delay between mouse actions in milliseconds
    #[arg(short, long, default_value_t = 15)]
    delay_ms: u64,

    /// Dry-run mode: simulate painting without sending any input
    #[arg(long)]
    dry_run: bool,

    /// Seconds to wait before painting starts (to switch to game window)
    #[arg(short, long, default_value_t = 5)]
    startup_delay: u32,

    /// Skip the safety disclaimer confirmation
    #[arg(long)]
    accept_risk: bool,
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

    // Map every pixel to the nearest Rust in-game palette color
    let palette = color::rust_palette();
    let pixel_plan = color::map_image_to_palette(&img, &palette);

    // Group pixels by color for efficient painting (one color-select per color)
    let paint_groups = painter::group_by_color(&pixel_plan, cli.canvas_width, cli.canvas_height);

    let total_pixels: usize = paint_groups.iter().map(|g| g.pixels.len()).sum();
    let total_colors = paint_groups.len();

    println!("Image: {}x{} -> {}x{} canvas", img.width(), img.height(), cli.canvas_width, cli.canvas_height);
    println!("Colors used: {}, Total pixels: {}", total_colors, total_pixels);

    if cli.dry_run {
        println!("\n[DRY RUN] Would paint {} pixels across {} colors.", total_pixels, total_colors);
        println!("[DRY RUN] No input will be sent. Estimated time: {:.0}s",
            total_pixels as f64 * cli.delay_ms as f64 / 1000.0);
        for group in &paint_groups {
            println!("  Color #{:02X}{:02X}{:02X}: {} pixels",
                group.color.0, group.color.1, group.color.2, group.pixels.len());
        }
        return;
    }

    println!("\nSwitching to game window in {} seconds...", cli.startup_delay);
    println!("Make sure the sign editor is open and the brush tool is selected!");
    std::thread::sleep(Duration::from_secs(cli.startup_delay as u64));

    let delay = Duration::from_millis(cli.delay_ms);
    match painter::paint(&paint_groups, delay) {
        Ok(()) => println!("\nPainting complete!"),
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
