mod capture;
mod ocr;
mod engine;
mod config;
// mod calibrate; // Enable for calibration mode

use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    // Parse CLI arguments
    let matches = Command::new("Zugzwang-RS")
        .version("0.0.3")
        .author("Your Name")
        .about("Pure-Rust chess assistant for browser windows")
        .arg(
            Arg::new("site")
                .long("site")
                .value_name("SITE")
                .help("Chess site to target (e.g., chesscom, lichess)")
                .default_value("chesscom")
                .value_parser(["chesscom", "lichess", "macOS"]) // Allowed values
        )
        // TODO: Add --calibrate flag for Phase 2
        .get_matches();

    let site = matches.get_one::<String>("site").unwrap(); // Safe due to default

    println!("Zugzwang-RS Chess Assistant starting... (MVP Phase 1)");
    println!("Targeting site: {}", site);
    println!("Press Ctrl+C to stop.");

    loop {
        // Step 1: Capture full screenshot to PNG (board detection/crop in OCR)
        capture::capture_screenshot()
            .context("Failed to capture screenshot")?;

        // Step 2: OCR loads PNG, detects/crops board to FEN
        let fen = ocr::board_to_fen("screenshots/current_board.png", site)
            .context("Failed to recognize board from screenshot")?;

        // Step 3: Engine analysis
        let (best_move, eval) = engine::analyze_position(&fen)
            .context("Failed to analyze position")?;

        // Step 4: Output (basic println for MVP; crossterm later)
        println!("Detected FEN: {}", fen);
        println!("Best move: {}", best_move);
        println!("Evaluation: {}", eval);

        thread::sleep(Duration::from_millis(500));
    }
}
