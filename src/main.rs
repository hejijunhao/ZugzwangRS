mod capture;
mod ocr_native;
mod ocr_llm;
mod ocr;
mod engine;
// mod config;
// mod calibrate; // Enable for calibration mode

use anyhow::{Context, Result};
use clap::{Arg, Command};
use dialoguer::{theme::ColorfulTheme, Select};
use ocr::OcrMode;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let matches = Command::new("Zugzwang-RS")
        .version("0.1.1")
        .author("Crimson Sun")
        .about("Pure-Rust chess assistant for browser windows")
        .arg(
            Arg::new("ocr")
                .long("ocr")
                .value_name("MODE")
                .help("OCR mode: native (default) or llm")
                .value_parser(["native", "llm"]),
        )
        .arg(
            Arg::new("interval")
                .long("interval")
                .value_name("MS")
                .help("Loop interval in milliseconds")
                .default_value("1000")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new("site")
                .long("site")
                .value_name("SITE")
                .help("Chess site for native OCR templates")
                .default_value("chesscom")
                .value_parser(["chesscom", "lichess", "macOS"]),
        )
        .get_matches();

    // Determine OCR mode
    let ocr_mode = if let Some(mode_str) = matches.get_one::<String>("ocr") {
        // Explicit mode from CLI
        match mode_str.as_str() {
            "llm" => {
                if !ocr::llm_available() {
                    eprintln!("Warning: --ocr=llm specified but OPENAI_API_KEY not set");
                    eprintln!("Falling back to native mode");
                    OcrMode::Native
                } else {
                    OcrMode::Llm
                }
            }
            "native" => OcrMode::Native,
            _ => unreachable!(),
        }
    } else {
        // No CLI flag - show interactive selector
        select_ocr_mode_interactive()?
    };

    let interval = *matches.get_one::<u64>("interval").unwrap();
    let site = matches.get_one::<String>("site").unwrap();

    // Startup banner
    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║         Zugzwang-RS Chess Assistant v0.1.1                ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("  OCR Mode:  {}", ocr_mode);
    println!("  Interval:  {}ms", interval);
    if ocr_mode == OcrMode::Native {
        println!("  Site:      {}", site);
    }
    println!();
    println!("  Press Ctrl+C to stop.");
    println!();
    println!("─────────────────────────────────────────────────────────────");
    println!();

    // Main pipeline loop
    loop {
        // Step 1: Capture full screenshot
        capture::capture_screenshot().context("Failed to capture screenshot")?;

        // Step 2: OCR to FEN (async)
        let fen = ocr::board_to_fen("screenshots/current_board.png", site, ocr_mode)
            .await
            .context("Failed to recognize board from screenshot")?;

        // Step 3: Engine analysis
        let (best_move, eval) =
            engine::analyze_position(&fen).context("Failed to analyze position")?;

        // Step 4: Output
        println!("FEN:  {}", fen);
        println!("Best: {} ({})", best_move, eval);
        println!();

        // Wait before next cycle
        tokio::time::sleep(Duration::from_millis(interval)).await;
    }
}

/// Interactive CLI selector for OCR mode
fn select_ocr_mode_interactive() -> Result<OcrMode> {
    let llm_available = ocr::llm_available();

    let options = if llm_available {
        vec![
            "Native (template matching) - fast, requires templates/",
            "LLM (GPT-4o Mini) - accurate, requires API key",
        ]
    } else {
        vec![
            "Native (template matching) - fast, requires templates/",
            "LLM (GPT-4o Mini) - OPENAI_API_KEY not set",
        ]
    };

    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║         Zugzwang-RS Chess Assistant v0.1.1                ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select OCR mode")
        .items(&options)
        .default(0) // Native is default
        .interact()
        .context("Failed to get user selection")?;

    let mode = match selection {
        0 => OcrMode::Native,
        1 => {
            if llm_available {
                OcrMode::Llm
            } else {
                eprintln!();
                eprintln!("  ⚠️  OPENAI_API_KEY not set. Using Native mode instead.");
                eprintln!("     Set it with: export OPENAI_API_KEY=\"sk-...\"");
                eprintln!();
                OcrMode::Native
            }
        }
        _ => OcrMode::Native,
    };

    Ok(mode)
}
