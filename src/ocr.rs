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
use imageproc::template_matching::{match_template, MatchTemplateMethod};
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

    // Function to split board into 8x8 grid of grayscale squares for template matching
    // Returns 8 rows (ranks 8→1 top to bottom) × 8 columns (files a→h left to right)
    fn split_into_squares(board: &RgbaImage) -> Vec<Vec<GrayImage>> {
        let mut squares = Vec::with_capacity(8);

        // Convert to grayscale once for efficiency
        let gray_board = DynamicImage::ImageRgba8(board.clone()).to_luma8();

        for rank in 0..8 {
            let mut row = Vec::with_capacity(8);
            for file in 0..8 {
                let x = (file * 64) as u32;
                let y = (rank * 64) as u32;
                let square = imageops::crop_imm(&gray_board, x, y, 64, 64).to_image();
                row.push(square);
            }
            squares.push(row);
        }
        squares
    }


    // matchmaking function
    fn match_square(square: &GrayImage, templates: &PieceTemplates) -> char {
        // Returns: 'K', 'Q', 'R', etc. for pieces, or '1' for empty square

        // Step 1: Check if square is empty via variance analysis
        // Low variance = uniform color = no piece present
        let pixels: Vec<f32> = square.pixels().map(|p| p[0] as f32).collect();
        let mean = pixels.iter().sum::<f32>() / pixels.len() as f32;
        let variance = pixels.iter()
            .map(|&p| (p - mean).powi(2))
            .sum::<f32>() / pixels.len() as f32;

        const EMPTY_VARIANCE_THRESHOLD: f32 = 100.0;
        if variance < EMPTY_VARIANCE_THRESHOLD {
            return '1';
        }

        // Step 2: Template matching using Sum of Squared Differences (Normalized)
        // Lower score = better match (0.0 = perfect match)
        let mut best_match: char = '1';
        let mut best_score: f32 = f32::MAX;

        // Threshold: if no template scores below this, consider square empty
        const MATCH_THRESHOLD: f32 = 0.3;

        for (&piece_char, template) in &templates.pieces {
            // Resize template to match square size (64x64) if needed
            let template_resized = if template.dimensions() != (64, 64) {
                imageops::resize(template, 64, 64, imageops::FilterType::Lanczos3)
            } else {
                template.clone()
            };

            // match_template returns a score image; for same-size images, it's 1x1
            let result = match_template(
                square,
                &template_resized,
                MatchTemplateMethod::SumOfSquaredErrorsNormalized,
            );

            // Get the match score (single pixel for same-size comparison)
            let score = result.get_pixel(0, 0)[0];

            if score < best_score {
                best_score = score;
                best_match = piece_char;
            }
        }

        // Only return piece if match is confident enough
        if best_score < MATCH_THRESHOLD {
            best_match
        } else {
            '1' // No confident match = empty square
        }
    }

    // Builds FEN string from 8x8 grid of piece characters
    // Takes matched pieces where '1' = empty, 'K'/'k' = king, etc.
    // Returns validated FEN string with game state appended
    fn build_fen_string(board: [[char; 8]; 8]) -> Result<String> {
        let mut fen_parts: Vec<String> = Vec::with_capacity(8);

        for row in &board {
            let mut rank_str = String::new();
            let mut empty_count = 0;

            for &piece in row {
                if piece == '1' {
                    empty_count += 1;
                } else {
                    // Flush empty count before adding piece
                    if empty_count > 0 {
                        rank_str.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    rank_str.push(piece);
                }
            }

            // Flush any remaining empty squares at end of rank
            if empty_count > 0 {
                rank_str.push_str(&empty_count.to_string());
            }

            fen_parts.push(rank_str);
        }

        // Join ranks with '/' and append game state
        // Note: We assume white to move, full castling rights for MVP
        // (proper turn detection would require move history or time analysis)
        let piece_placement = fen_parts.join("/");
        let full_fen = format!("{} w KQkq - 0 1", piece_placement);

        // Validate FEN with shakmaty
        shakmaty::fen::Fen::from_ascii(full_fen.as_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid FEN generated: {} (FEN: {})", e, full_fen))?;

        Ok(full_fen)
    }

    // Wire it all together: split → match → build FEN
    let templates = load_templates("chesscom")
        .context("Failed to load piece templates")?;

    let squares = split_into_squares(&img);

    // Debug: Save grid squares if DEBUG_OCR is set
    if std::env::var("DEBUG_OCR").is_ok() {
        let _ = std::fs::create_dir_all("screenshots/ocr_debug");
        for (rank, row) in squares.iter().enumerate() {
            for (file, square) in row.iter().enumerate() {
                let path = format!("screenshots/ocr_debug/square_{}_{}.png", rank, file);
                let _ = square.save(&path);
            }
        }
    }

    // Match each square against templates to identify pieces
    let mut board: [[char; 8]; 8] = [['1'; 8]; 8];
    for (rank, row) in squares.iter().enumerate() {
        for (file, square) in row.iter().enumerate() {
            board[rank][file] = match_square(square, &templates);
        }
    }

    build_fen_string(board)
}


