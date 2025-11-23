mod capture;
mod ocr;
mod engine;
mod config;
// mod calibrate; // Enable for calibration mode

use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    // TODO: Use clap to parse args like --calibrate, --site=chesscom
    // TODO: Load or create config via config::load_or_calibrate()?

    println!("Zugzwang-RS Chess Assistant starting... (MVP Phase 1)");
    println!("Press Ctrl+C to stop.");

    loop {
        // Step 1: Capture board (hardcoded bounds for MVP)
        let board_img = capture::capture_board()
            .context("Failed to capture board")?;

        // Step 2: OCR to FEN (naive color-based for MVP)
        let fen = ocr::board_to_fen(&board_img)
            .context("Failed to recognize board")?;

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
