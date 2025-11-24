//! OCR module (Step 2 in architecture).
//! Pure custom OCR: loads full screenshot PNG, detects/crops board region, then recognizes pieces to FEN.
//! Uses template matching or color/HSV analysis (no external libs for stealth/purity).
//! Detects board via edges/contours (imageproc), splits to 64 squares, classifies pieces/empty.
//! Outputs validated FEN string via shakmaty.
//! Uses piece templates for reference
//! Latency target: 40-80ms (includes detection; parallelize with rayon).
//! Flexible for full images: handles varying positions/sizes/apps (e.g., macOS Chess.app, browsers); multi-site templates.

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, GrayImage, imageops, ImageReader, RgbaImage};
use imageproc::edges::canny;
use std::collections::HashMap;

/// Detects the chessboard in the full screenshot and crops/resizes it to a standard board image.
/// imageproc for auto-detection via edges/contours.
/// Returns DynamicImage ready for grid splitting/OCR.
pub fn screenshot_to_board(image_path: &str) -> Result<DynamicImage> {
    let img = ImageReader::open(image_path)
        .context("Failed to open screenshot for board detection")?
        .decode()
        .context("Failed to decode screenshot")?;

    // Dynamic detection: Find board region using imageproc edges (defined locally for modularity; can extract to top-level later)

    fn find_board_region(img: &DynamicImage) -> Result<(u32, u32, u32, u32)> {
        // step 1: Edge detection (full screenshot)
        let gray: GrayImage = img.to_luma8();
        let edges: GrayImage = canny(&gray, 50.0, 150.0);

        // step 2: Generate candidate regions
        let (width, height) = img.dimensions();
        let candidates = generate_candidate_regions(width, height);

        // step 3: Score each candidate by edge density
        let mut best_candidate: Option<(u32, u32, u32, u32)> = None;
        let mut best_density = 0.0f32;

        for (x, y, size) in candidates {
            let density = calculate_edge_density(&edges, x, y, size);

            if density > best_density {
                best_density = density;
                best_candidate = Some((x, y, size, size));
            }
        }

        // step 4: Validate Best Candidate
        const MIN_EDGE_DENSITY: f32 = 0.01; // 1% of pixels should be edges

        if best_density < MIN_EDGE_DENSITY {
            anyhow::bail!(
                "No board detected: best edge density {:.3}% < {:.1}% threshold",
                best_density * 100.0,
                MIN_EDGE_DENSITY * 100.0
            );
        }

        best_candidate.ok_or_else(|| anyhow::anyhow!("No candidate regions found"))
    }

    // helper: generate search regions
    // creates a grid of candidate regions to search across the screenshot.
    // returns Vec of (x, y, size) tuples representing potential board locations.
    fn generate_candidate_regions(width: u32, height: u32) -> Vec<(u32, u32, u32)> {
        let mut candidates = Vec::new();

        // Calculate reasonable board sizes to search for
        let min_size = 200u32;
        let max_size = if width < height { width } else { height };
        let size_step = 100u32; // Try every 100 pixels

        // Grid search: Try different positions and sizes
        for size in (min_size..=max_size).step_by(size_step as usize) {
            let step = size / 4; // Overlap regions by 75% for better coverage

            let mut y = 0;
            while y + size <= height {
                let mut x = 0;
                while x + size <= width {
                    candidates.push((x, y, size));
                    x += step;
                }
                y += step;
            }
        }

        candidates
    }

    // helper: calculate edge density in region
    // counts what percentage of pixels in a region are edges (bright pixels in edge map).
    // chessboards should have high edge density due to grid lines and piece shapes.
    fn calculate_edge_density(edges: &GrayImage, x: u32, y: u32, size: u32) -> f32 {
        let mut edge_count = 0usize;
        let edge_threshold = 128u8; // pixel brightness > 128 = edge detected

        for dy in 0..size {
            for dx in 0..size {
                if let Some(pixel) = edges.get_pixel_checked(x + dx, y + dy) {
                    if pixel[0] > edge_threshold {
                        edge_count += 1;
                    }
                }
            }
        }

        edge_count as f32 / (size * size) as f32
    }

    let bounds = find_board_region(&img)
        .context("Failed to detect board region in screenshot")?;

    let (crop_x, crop_y, crop_w, crop_h) = bounds;

    let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);

    let (w, h) = cropped.dimensions();
    if w < 64 || h < 64 || (w as f32 / h as f32 - 1.0).abs() > 0.1 { // Allow ~10% aspect tolerance
        return Err(anyhow::anyhow!("Detected board invalid: {}x{} (too small/non-square)", w, h));
    }

    // Resize to standard 512x512 for uniform processing (64px squares)
    let board_img = imageops::resize(&cropped, 512, 512, imageops::FilterType::Lanczos3);

    // Debug: Save cropped for verification
    if std::env::var("DEBUG_OCR").is_ok() {
        let _ = board_img.save("screenshots/debug_cropped_board.png");
    }

    Ok(DynamicImage::ImageRgba8(board_img))
}

/// Loads cropped board image and performs OCR to generate FEN string.
/// Assumes input is a clean board image (from screenshot_to_board or manual crop).
/// MVP: Naive color/HSV per square; later templates.
pub fn board_to_fen(image_path: &str) -> Result<String> {
    // Detect and crop board from screenshot
    let board_img = screenshot_to_board(image_path)
        .context("Failed to detect/crop board from screenshot")?;

    // Convert to RGBA for processing
    let img = board_img.to_rgba8();

    // Define piece template struct (for image referencing)
    struct PieceTemplates {
        pieces: HashMap<char, GrayImage>,  // 'K' -> white king template, etc.
    }

    fn load_templates(site: &str) -> Result<PieceTemplates> {
        let mut pieces = HashMap::new();

        // loads 12 PNG files from templates/{site}/
        let piece_chars = ['K', 'Q', 'R', 'B', 'N', 'P', 'k', 'q', 'r', 'b', 'n', 'p'];

        for piece_char in piece_chars {
            let path = format!("templates/{site}/{piece_char}.png");
            let template = ImageReader::open(&path)
                .context("Failed to fetch chesspiece template")?
                .decode()
                .context("Failed to decode template")?;

            pieces.insert(piece_char, template.to_luma8());
        }

        Ok(PieceTemplates { pieces })
    }

    // Load templates for the site (hardcoded to "chesscom" for MVP)
    let _templates = load_templates("chesscom")
        .context("Failed to load piece templates")?;

    // Function to split board into 8x8 grid and match each square against templates
    fn split_into_squares(board: &RgbaImage) -> Vec<Vec<DynamicImage>> {
        let mut squares = Vec::new();
        
        for rank in 0..8 {
            let mut row = Vec::new();
            for file in 0..8 {
                let x = (file * 64) as u32;
                let y = (rank * 64) as u32;
                let square = imageops::crop_imm(board, x, y, 64, 64).to_image();
                row.push(square);
            }
            squares.push(row);
        }

        squares
    }


    // TODO: Build FEN string from matched pieces

    fn match_square(square: &DynamicImage, templates: &PieceTemplates) -> char {
        // Returns: 'K', 'Q', 'R', etc. for pieces, or '1' for empty square

        // Step 1: Convert square to grayscale for matching
        let square_gray = square.to_luma8();

        // Step 2: Check if square is empty (uniform color = no piece)
        // Calculate variance: if pixels are all similar, variance is low = empty square
        let pixels: Vec<f32> = square_gray.pixels().map(|p| p[0] as f32).collect();
        let mean = pixels.iter().sum::<f32>() / pixels.len() as f32;
        let variance = pixels.iter()
            .map(|&p| (p - mean).powi(2))
            .sum::<f32>() / pixels.len() as f32;

        // If variance is very low, square is empty (just solid background color)
        const EMPTY_VARIANCE_THRESHOLD: f32 = 100.0;
        if variance < EMPTY_VARIANCE_THRESHOLD {
            return '1';
        }

        // Step 3: TODO - Compare against templates and return best match
        // Your code here!

        '1' // Temporary: return empty for now
    }

    // For now, return placeholder FEN (starting position)
    // Append " w KQkq - 0 1" (assume white to move, full castling).
    // Validate: shakmaty::fen::Fen::from_ascii(fen.as_bytes()).map_err(|_| anyhow::anyhow!("Invalid FEN"))?
    // Suggestion (?) Perf: Use rayon for parallel square processing.
    // Debug: Save grid squares to screenshots/ocr_debug/ for tuning.

    Ok("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string())
}


