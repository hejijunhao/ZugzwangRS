//! Screen capture module
//! Pure screenshot service: captures primary display full-screen via `xcap` (cross-platform), saves PNG for OCR.
//! No cropping here—shifts flexibility to OCR for board detection across varying windows/apps/sites (e.g., macOS Chess.app, browsers).
//! Latency goal: <30ms (just capture + save; processing in OCR).
//! Future: Add window-specific capture, dynamic crop if perf bottleneck, or multi-monitor support.

use anyhow::{bail, Context, Result};
use image::{DynamicImage, GenericImageView};
use std::env;
use std::fs;
use std::time::Instant;
use xcap::Monitor;

/// Captures the full screenshot of the primary monitor and saves as PNG to screenshots/current_board.png.
/// OCR module will load and handle board detection/cropping for flexibility across apps/sites.
/// Debug: Set env var `DEBUG_CAPTURE=1` to also save full screen variant to screenshots/debug_full_screen.png.
/// Later phases: Optional window-specific capture or dynamic cropping here if perf needed.
/// Permissions note: On macOS, grant "Screen & System Audio Recording" permission to Terminal.app in System Settings > Privacy & Security.
pub fn capture_screenshot() -> Result<()> {
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

    fs::create_dir_all("screenshots")
        .context("Failed to create screenshots/ directory")?;

    screenshot
        .save("screenshots/current_board.png")
        .context("Failed to save screenshot to screenshots/current_board.png")?;

    let latency = start.elapsed();
    eprintln!("Full screenshot capture latency: {:?}", latency); // Use eprintln to stderr for non-blocking

    // Optional: If DEBUG_CAPTURE=1, save additional info or variants
    if env::var_os("DEBUG_CAPTURE").is_some() {
        screenshot
            .save("screenshots/debug_full_screen.png")
            .context("Failed to save debug full screenshot")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[test]
    #[ignore = "requires graphical display and screen recording permissions"]
    fn test_capture_dimensions() {
        capture_screenshot().expect("capture_screenshot failed");
        let saved_img = image::open("screenshots/current_board.png")
            .expect("Failed to load saved screenshot for validation");
        let (w, h) = saved_img.dimensions();
        assert!(w > 0 && h > 0, "saved screenshot has invalid dimensions {}x{}", w, h);
        assert!(w >= 800 && h >= 600, "Screenshot too small; expected full screen-like size"); // Rough check
    }
}