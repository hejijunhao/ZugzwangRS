//! Screen capture module
//! Pure screenshot service: captures primary display full-screen via `xcap` (cross-platform), saves PNG for OCR.
//! No cropping here—shifts flexibility to OCR for board detection across varying windows/apps/sites (e.g., macOS Chess.app, browsers).
//! Latency goal: <30ms (just capture + save; processing in OCR).
//! Future: Add window-specific capture, dynamic crop if perf bottleneck, or multi-monitor support.

use anyhow::{Context, Result};
use std::fs;
use std::time::Instant;
use xcap::Monitor;

/// Captures the full screenshot of the primary monitor and saves as PNG to screenshots/current_board.png.
/// OCR module will load and handle board detection/cropping for flexibility across apps/sites.
/// Debug: Set env var `DEBUG_CAPTURE=1` to also save full screen variant to screenshots/debug_full_screen.png.
/// Later phases: Optional window-specific capture or dynamic cropping here if perf needed.
/// On macOS, grant Screen Recording permission to Terminal in System Settings > Privacy & Security.
pub fn capture_screenshot() -> Result<()> {
    let start = Instant::now();

    let screenshot = Monitor::all()
        .context("Failed to enumerate monitors")?
        .into_iter()
        .next()
        .context("No monitors found")?
        .capture_image()
        .context("Failed to capture image — check Screen Recording permission")?;



    fs::create_dir_all("screenshots").context("Failed to create screenshots dir")?;

    screenshot.save("screenshots/current_board.png")
        .context("Failed to save screenshot")?;

    let latency = start.elapsed();
    eprintln!("Capture + save latency: {:?}", latency);

    // Debug: DEBUG_CAPTURE=1 → save extra copy
    if std::env::var("DEBUG_CAPTURE").as_ref().map_or(false, |v| v.as_str() == "1") {
        let _ = screenshot.save("screenshots/debug_full_screen.png"); // fire-and-forget
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