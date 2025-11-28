//! OCR facade module - dispatches to LLM or native implementation based on mode
//!
//! This module provides a unified interface for board-to-FEN conversion:
//! - **LLM mode**: Sends full screenshot to GPT-4o Mini (it finds the board itself)
//! - **Native mode**: Detects/crops board first, then uses template matching
//!
//! The modes differ in board detection:
//! - LLM skips CPU-intensive edge detection (GPT handles it)
//! - Native requires board detection for accurate template matching

use anyhow::{Context, Result};

/// Path where the cropped board image is saved for OCR processing
const CROPPED_BOARD_PATH: &str = "screenshots/cropped_board.png";

/// OCR implementation mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OcrMode {
    /// GPT-4o Mini vision API
    Llm,
    /// Template-based matching (default for backward compatibility)
    #[default]
    Native,
}

impl std::fmt::Display for OcrMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrMode::Llm => write!(f, "LLM (GPT-4o Mini)"),
            OcrMode::Native => write!(f, "Native (template matching)"),
        }
    }
}

/// Checks if the LLM OCR mode is available (API key is set)
pub fn llm_available() -> bool {
    crate::ocr_llm::has_api_key()
}

/// Main entry point for board-to-FEN conversion.
///
/// For Native mode: Detects and crops the chessboard, then uses template matching.
/// For LLM mode: Sends the full screenshot directly to GPT-4o Mini (it can find the board itself).
pub async fn board_to_fen(image_path: &str, site: &str, mode: OcrMode) -> Result<String> {
    use std::io::Write;
    use std::time::Instant;

    match mode {
        OcrMode::Llm => {
            // LLM mode: Skip board detection - GPT-4o Mini can find the board in the full image
            // This saves 5-10 seconds of CPU-intensive edge detection
            eprint!("LLM OCR... ");
            let _ = std::io::stderr().flush();
            let ocr_start = Instant::now();
            let result = crate::ocr_llm::board_to_fen(image_path).await;
            eprintln!("{:.0}ms", ocr_start.elapsed().as_secs_f64() * 1000.0);
            result
        }
        OcrMode::Native => {
            // Native mode: Need board detection for template matching
            eprint!("Board detection... ");
            let _ = std::io::stderr().flush();
            let detect_start = Instant::now();

            let path = image_path.to_string();
            let cropped_path = tokio::task::spawn_blocking(move || -> Result<String> {
                let board_img = crate::ocr_native::screenshot_to_board(&path)
                    .context("Failed to detect/crop board from screenshot")?;

                // Save cropped board for OCR processing
                board_img
                    .save(CROPPED_BOARD_PATH)
                    .context("Failed to save cropped board image")?;

                Ok(CROPPED_BOARD_PATH.to_string())
            })
            .await
            .map_err(|e| anyhow::anyhow!("Board detection task failed: {}", e))??;

            eprintln!("{:.0}ms", detect_start.elapsed().as_secs_f64() * 1000.0);

            // Template matching on cropped board
            eprint!("Template matching... ");
            let _ = std::io::stderr().flush();
            let ocr_start = Instant::now();
            let site = site.to_string();
            let result = tokio::task::spawn_blocking(move || {
                crate::ocr_native::cropped_board_to_fen(&cropped_path, &site)
            })
            .await
            .map_err(|e| anyhow::anyhow!("Native OCR task failed: {}", e))?;

            eprintln!("{:.0}ms", ocr_start.elapsed().as_secs_f64() * 1000.0);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_mode_display() {
        assert_eq!(format!("{}", OcrMode::Llm), "LLM (GPT-4o Mini)");
        assert_eq!(format!("{}", OcrMode::Native), "Native (template matching)");
    }

    #[test]
    fn test_ocr_mode_default() {
        assert_eq!(OcrMode::default(), OcrMode::Native);
    }
}
