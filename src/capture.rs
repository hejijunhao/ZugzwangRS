//! Screen capture module
//! Pure screenshot service: captures primary display full-screen via `xcap` (cross-platform), saves PNG for OCR.
//! No cropping here—shifts flexibility to OCR for board detection across varying windows/apps/sites (e.g., macOS Chess.app, browsers).
//! Latency goal: <200ms (capture + downsample + save).
//! Note: High-DPI displays (4K/5K/6K) are aggressively downsampled for performance.
//! Future: Add window-specific capture, dynamic crop if perf bottleneck, or multi-monitor support.

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageEncoder, imageops};
use image::codecs::jpeg::JpegEncoder;
use std::fs::{self, File};
use std::io::BufWriter;
use std::time::Instant;
use xcap::Monitor;

/// Maximum width for captured screenshots. Images larger than this are downsampled.
/// 1920px gives LLM OCR enough detail to read pieces accurately.
/// Higher than 1280 for better accuracy, still much faster than full 6K.
const MAX_CAPTURE_WIDTH: u32 = 1920;

/// Captures the full screenshot of the primary monitor and saves as PNG to screenshots/current_board.png.
/// OCR module will load and handle board detection/cropping for flexibility across apps/sites.
/// Debug: Set env var `DEBUG_CAPTURE=1` to also save full screen variant to screenshots/debug_full_screen.png.
/// Later phases: Optional window-specific capture or dynamic cropping here if perf needed.
/// On macOS, grant Screen Recording permission to Terminal in System Settings > Privacy & Security.
pub fn capture_screenshot() -> Result<()> {
    use std::io::Write;

    eprint!("Capturing screen... ");
    let _ = std::io::stderr().flush();

    let start = Instant::now();

    let screenshot = Monitor::all()
        .context("Failed to enumerate monitors")?
        .into_iter()
        .next()
        .context("No monitors found")?
        .capture_image()
        .context("Failed to capture image — check Screen Recording permission")?;

    // Convert to image crate format for processing
    let (orig_width, orig_height) = (screenshot.width(), screenshot.height());
    let img = DynamicImage::ImageRgba8(screenshot);

    // Downsample if larger than MAX_CAPTURE_WIDTH (critical for 4K/5K/6K displays)
    // This prevents O(n²) blowup in edge detection and candidate region search
    let final_img = if orig_width > MAX_CAPTURE_WIDTH {
        let scale = MAX_CAPTURE_WIDTH as f32 / orig_width as f32;
        let new_height = (orig_height as f32 * scale) as u32;
        let resized = imageops::resize(
            img.as_rgba8().unwrap(),
            MAX_CAPTURE_WIDTH,
            new_height,
            imageops::FilterType::Triangle, // Fast bilinear filtering
        );
        DynamicImage::ImageRgba8(resized)
    } else {
        img
    };

    fs::create_dir_all("screenshots").context("Failed to create screenshots dir")?;

    // Save as JPEG for speed (PNG encoding is very slow)
    // Quality 85 is visually lossless and ~10x faster than PNG
    let output_path = "screenshots/current_board.jpg";
    let file = File::create(output_path).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);
    let rgb_img = final_img.to_rgb8();
    JpegEncoder::new_with_quality(&mut writer, 85)
        .write_image(
            rgb_img.as_raw(),
            rgb_img.width(),
            rgb_img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .context("Failed to encode JPEG")?;

    let latency = start.elapsed();
    let (final_w, final_h) = final_img.dimensions();
    if orig_width > MAX_CAPTURE_WIDTH {
        eprintln!("{:.0}ms ({}×{} → {}×{})",
            latency.as_secs_f64() * 1000.0,
            orig_width, orig_height,
            final_w, final_h
        );
    } else {
        eprintln!("{:.0}ms", latency.as_secs_f64() * 1000.0);
    }

    // Debug: DEBUG_CAPTURE=1 → save extra copy (the downsampled version)
    if std::env::var("DEBUG_CAPTURE").is_ok_and(|v| v == "1") {
        let _ = final_img.save("screenshots/debug_full_screen.jpg"); // fire-and-forget
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