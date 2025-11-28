mod capture;
mod ocr_native;
mod ocr_llm;
mod ocr;
mod engine;
// mod config;
// mod calibrate; // Enable for calibration mode

use anyhow::{Context, Result};
use clap::{Arg, Command};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use ocr::OcrMode;
use std::io::{self, BufRead};
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
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging for debugging")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("trigger")
                .long("trigger")
                .value_name("MODE")
                .help("Capture trigger: auto (interval-based) or manual (press Enter)")
                .value_parser(["auto", "manual"]),
        )
        .get_matches();

    // Determine OCR mode
    let ocr_mode = if let Some(mode_str) = matches.get_one::<String>("ocr") {
        // Explicit mode from CLI
        match mode_str.as_str() {
            "llm" => {
                if !ocr::llm_available() {
                    // Prompt for API key
                    prompt_for_api_key()?;
                }
                OcrMode::Llm
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
    let verbose = matches.get_flag("verbose");

    // Determine trigger mode
    let manual_mode = if let Some(trigger) = matches.get_one::<String>("trigger") {
        // Explicit mode from CLI
        trigger == "manual"
    } else {
        // No CLI flag - show interactive selector
        select_trigger_mode_interactive()?
    };

    // Startup banner
    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║         Zugzwang-RS Chess Assistant v0.1.1                ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("  OCR Mode:  {}", ocr_mode);
    let trigger_display = if manual_mode {
        "manual (press Enter)".to_string()
    } else {
        format!("auto ({}ms)", interval)
    };
    println!("  Trigger:   {}", trigger_display);
    if ocr_mode == OcrMode::Native {
        println!("  Site:      {}", site);
    }
    if verbose {
        println!("  Verbose:   enabled");
    }
    println!();
    if manual_mode {
        println!("  Press Enter to capture & analyze, Ctrl+C to stop.");
    } else {
        println!("  Press Ctrl+C to stop.");
    }
    println!();
    println!("─────────────────────────────────────────────────────────────");
    println!();

    // Main pipeline loop
    let mut cycle_count = 0u64;
    let stdin = io::stdin();

    loop {
        // In manual mode, wait for user to press Enter before capturing
        if manual_mode {
            print!("▶ Press Enter to capture & analyze... ");
            io::Write::flush(&mut io::stdout())?;
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
        }

        cycle_count += 1;
        let cycle_start = std::time::Instant::now();

        if verbose {
            println!("┌─ Cycle {} ─────────────────────────────────────────────────", cycle_count);
        }

        // Step 1: Capture full screenshot
        let step_start = std::time::Instant::now();
        capture::capture_screenshot().context("Failed to capture screenshot")?;
        if verbose {
            println!("│ [1] Capture:  {:>6.1}ms", step_start.elapsed().as_secs_f64() * 1000.0);
        }

        // Step 2: OCR to FEN (async)
        let step_start = std::time::Instant::now();
        let fen = ocr::board_to_fen("screenshots/current_board.jpg", site, ocr_mode)
            .await
            .context("Failed to recognize board from screenshot")?;
        if verbose {
            println!("│ [2] OCR:      {:>6.1}ms", step_start.elapsed().as_secs_f64() * 1000.0);
        }

        // Step 3: Engine analysis
        let step_start = std::time::Instant::now();
        let (best_move, eval) =
            engine::analyze_position(&fen).context("Failed to analyze position")?;
        if verbose {
            println!("│ [3] Engine:   {:>6.1}ms", step_start.elapsed().as_secs_f64() * 1000.0);
            println!("│ [4] Total:    {:>6.1}ms", cycle_start.elapsed().as_secs_f64() * 1000.0);
            println!("├─────────────────────────────────────────────────────────────");
        }

        // Step 4: Output
        if verbose {
            println!("│ FEN:  {}", fen);
            println!("│ Best: {} ({})", best_move, eval);
            println!("└─────────────────────────────────────────────────────────────");
        } else {
            println!("FEN:  {}", fen);
            println!("Best: {} ({})", best_move, eval);
        }
        println!();

        // Wait before next cycle (only in auto mode)
        if !manual_mode {
            tokio::time::sleep(Duration::from_millis(interval)).await;
        }
    }
}

/// Prompts the user to enter their OpenAI API key
fn prompt_for_api_key() -> Result<()> {
    println!();
    println!("  OPENAI_API_KEY not set. Enter your API key to continue:");
    println!("  (Get one at https://platform.openai.com/api-keys)");
    println!();

    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("API Key")
        .validate_with(|input: &String| {
            if input.trim().is_empty() {
                Err("API key cannot be empty")
            } else if !input.starts_with("sk-") {
                Err("API key should start with 'sk-'")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to read API key")?;

    // Set the environment variable for this session
    // SAFETY: We're single-threaded at this point (before the main loop starts)
    // and no other threads are reading environment variables concurrently.
    unsafe {
        std::env::set_var("OPENAI_API_KEY", api_key.trim());
    }
    println!();
    println!("  ✓ API key set for this session");
    println!();

    Ok(())
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
            if !llm_available {
                prompt_for_api_key()?;
            }
            OcrMode::Llm
        }
        _ => OcrMode::Native,
    };

    Ok(mode)
}

/// Interactive CLI selector for trigger mode
/// Returns true for manual mode, false for auto mode
fn select_trigger_mode_interactive() -> Result<bool> {
    let options = vec![
        "Auto (continuous) - captures at regular intervals",
        "Manual (on-demand) - press Enter to capture",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select capture trigger")
        .items(&options)
        .default(0) // Auto is default
        .interact()
        .context("Failed to get user selection")?;

    Ok(selection == 1) // 1 = Manual
}
