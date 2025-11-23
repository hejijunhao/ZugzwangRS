//! Screen capture module
//! Uses `xcap` for cross-platform screenshots of the primary display.
//! Crops to calibrated board bounds (hardcoded for MVP Phase 1).
//! Latency goal: 30-50ms.
//! Future: Integrate calibration bounds from config.

use anyhow::{bail, Context, Result};
use image::{DynamicImage, GenericImageView};
use std::env;
use std::fs;
use std::time::Instant;
use xcap::Monitor;

/// Captures the full screenshot of the primary monitor and crops to hardcoded board bounds.
/// For MVP Phase 1: Tune the `bounds` tuple in code based on your browser window position/size (use OS screenshot tool to measure).
/// Debug: Set env var `DEBUG_CAPTURE=1 cargo run` to save cropped image to `screenshots/debug_board.png`.
/// Later phases: Load dynamic bounds from config after calibration.
/// Permissions note: On macOS, grant "Screen & System Audio Recording" permission to Terminal.app in System Settings > Privacy & Security.
pub fn capture_board() -> Result<DynamicImage> {
    let start = Instant::now();

    let monitors = Monitor::all()
        .context("Failed to enumerate monitors")?;

    let primary_monitor = monitors
        .first()
        .cloned()
        .context("No monitors found")?;

    let screenshot_raw = primary_monitor
        .capture_image()
        .context("Failed to capture image. On macOS, ensure Terminal has Screen Recording permission in System Settings > Privacy & Security > Screen & System Audio Recording")?;

    let screenshot = DynamicImage::ImageRgba8(screenshot_raw);
    if screenshot.dimensions() == (0, 0) {
        bail!("Captured empty screenshot - possible permission issue or no display");
    }

    let bounds = (200u32, 300u32, 480u32, 480u32); // TODO: Manually tune based on your chess browser window (x, y, width, height of board rect)

    if bounds.2 < 64 || bounds.3 < 64 {
        bail!("Hardcoded bounds too small for a chessboard (min ~64x64 pixels)");
    }

    let (screen_w, screen_h) = screenshot.dimensions();

    if bounds.0 >= screen_w || bounds.1 >= screen_h || 
       bounds.0.saturating_add(bounds.2) > screen_w || bounds.1.saturating_add(bounds.3) > screen_h {
        bail!("Crop bounds ({},{},{},{}) exceed screenshot dimensions {}x{}", 
              bounds.0, bounds.1, bounds.2, bounds.3, screen_w, screen_h);
    }

    let cropped = screenshot.crop_imm(bounds.0, bounds.1, bounds.2, bounds.3);
    if env::var_os("DEBUG_CAPTURE").is_some() {
        fs::create_dir_all("screenshots")
            .context("Failed to create screenshots/ debug directory")?;
        cropped
            .save("screenshots/debug_board.png")
            .context("Failed to save debug board image to screenshots/")?;
    }

    let latency = start.elapsed();
    eprintln!("Capture + crop latency: {:?}", latency); // Use eprintln to stderr for non-blocking

    Ok(cropped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[test]
    #[ignore = "requires graphical display and screen recording permissions"]
    fn test_capture_dimensions() {
        let img = capture_board().expect("capture_board failed");
        let (w, h) = img.dimensions();
        assert!(w > 0 && h > 0, "captured image has invalid dimensions {}x{}", w, h);
    }
}