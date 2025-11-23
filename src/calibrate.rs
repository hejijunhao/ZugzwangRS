//! Calibration module (Phase 2 enhancement).
//! One-time manual setup: user clicks board corners and pieces to define bounds/templates.
//! Uses `rdev` for global mouse/keyboard events (cross-platform).
//! Captures sub-images for templates (save PNGs to templates/{site}/).
//! Computes color thresholds for naive OCR.
//! Updates config.json.
//! Run via CLI flag --calibrate --site=chesscom.
//! Challenge: Cross-platform input grabbing, permissions (macOS accessibility).

use anyhow::Result;
use rdev::{listen, Event}; // For input events
use crate::config::Config;
use crate::capture::capture_board; // Reuse capture for sub-captures?

/// Runs interactive calibration for a site.
/// Prompts user, listens for clicks, saves config and templates.
pub fn run_calibration(site: &str) -> Result<Config> {
    println!("Calibration for site: {}. Open your chess browser window.", site);
    println!("Instructions: 1. Click top-left board corner. 2. Click bottom-right. 3. Click examples of each piece type (or auto-capture). Press ESC to finish.");

    // TODO: Set up rdev listener for clicks
    // let mut bounds = (0u32, 0u32, 0u32, 0u32);
    // Capture full screen, crop based on clicks for templates

    todo!("1. Initialize rdev event listener (handle Mouse button events). 2. Capture full screen periodically or on click. 3. On first clicks: calc board rect from corners. 4. Prompt/guide for piece selection (e.g., 'Click a white pawn'). 5. Crop small templates, save as PNG to templates/{site}/{piece}.png. 6. Compute avg colors for thresholds. 7. Create/update Config, call config::save_config. 8. Verify by running quick OCR test. 9. Handle ESC/keyboard quit. 10. macOS: Ensure Accessibility permissions for input grabbing.");

    // Return updated config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_flow() {
        // Hard to test interactive; smoke test or mock rdev
        let _config = run_calibration("test").unwrap_err();
    }
}