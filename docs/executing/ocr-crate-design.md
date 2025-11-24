The OCR module (ocr.rs) is currently stripped down to just its documentation comments, which is a great starting point for iterative development—keeps it clean while we plan the structure. Since this is Phase 1 MVP focused on naive color-based recognition (evolving to template matching later), the module's responsibility is to turn a full-screen screenshot PNG into a valid FEN string. It needs to be flexible for uncropped inputs (board could be anywhere on screen, varying sizes/sites), performant (<40-80ms), and robust with error handling (e.g., no board detected → retry or error).

Here's a brainstorm of what should belong in ocr.rs, broken down by logical components. I'll describe the high-level design, key functions (with signatures and rationale), data flow, Rust-specific considerations for learning, and implementation tips based on the project docs (CLAUDE.md, alpha-blueprint.md). This avoids full code but outlines the skeleton so you can build step-by-step (e.g., start with image loading and a stub FEN, then add detection, etc.). We can iterate on specifics as you code.

### Overall Design Principles
- **Input/Output**: pub fn board_to_fen(path: &str) -> Result<String><String> remains the main public API (called from main.rs). It orchestrates everything: load → detect → process → FEN → validate. Returns anyhow::Result for ergonomic errors.
- **Phased Approach**:
  - Phase 1 (MVP): Naive RGB/HSV color averaging per square. Assume simple board detection (e.g., find high-contrast square region). Classify squares as empty (matches board tile color) or occupied (contrast/variance), but piece types will be rough/ stubbed (e.g., color hue for white/black pawn/queen guess; accuracy ~50-70% initially, tune thresholds).
  - Later Phases: Add imageproc for edge/contour detection (board find), template matching (imageproc::template_matching with SSD metric against templates/{site}/*.png), rayon for parallel square processing.
- **Dependencies**: Leverage existing Cargo.toml crates:
  - `image`: Load/save DynamicImage, crop/resize, pixel access (e.g., img.get_pixel).
  - `imageproc`: Edges (canny), geometric transforms, template ops (Phase 3).
  - `shakmaty`: FEN validation (shakmaty::fen::Fen::from_ascii).
  - Optional `rayon` for par_iter on 64 squares (fast win for learning parallelism).
  - `anyhow` for contexts like "Failed to detect board".
- **Debug/Perf**: Always time steps (std::time::Instant). Save intermediates (cropped board, grid squares) to screenshots/ocr/ if DEBUG_OCR=1 env var. Handle edge cases: tiny boards (<64px), no board (full black?), invalid FEN (retry capture?).
- **Rust Learning Opportunities**: 
  - Traits: image::GenericImageView for pixel ops.
  - Enums/Results: Custom errors? Or stick to anyhow.
  - Loops/Iterators: For grid splitting, use (0..8).cartesian_product or nested loops; par_iter for rayon.
  - Ownership: DynamicImage clones/crops cheaply, but avoid unnecessary for perf.
  - Tests: Unit for classify_square (mock 64x64 PNGs), integration for full flow (use temp files).

### Key Functions and What They Do
1. **Core Pipeline Function** (start here for first code step):
   - `pub fn board_to_fen(path: &str) -> Result<String>`
     - Load: image::open(path)? → DynamicImage.
     - Detect: find_board_region(&img)? → (x, y, w, h) or Rect struct (define simple [u32;4]).
     - Crop/Preprocess: crop_rect(img, region)? → resize to 512x512 (image::imageops::resize for uniform 64px squares).
     - Grid: split_into_squares(&resized_img)? → Vec<DynamicImage><DynamicImage> (64 images, row-major).
     - Classify: squares.par_iter().map(|sq| classify_square(sq, row_idx, col_idx)).collect::<Vec<char><char>>() (use indices for light/dark square expectation).
     - Build: construct_fen(pieces_vec)? → String like "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" (stub turn/castling for MVP; assume white to move).
     - Validate: shakmaty::fen::Fen::from_ascii(fen.as_bytes()).map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))? → if ok, return fen.
     - Rationale: Encapsulates full flow; easy to test end-to-end. Time each sub-step for profiling.

2. **Board Detection** (challenging step; implement simple version first):
   - `fn find_board_region(img: &DynamicImage) -> Result<(u32, u32, u32, u32)>`
     - Phase 1 Naive: Scan image for largest ~square region with alternating high/low contrast (e.g., divide into quadrants, compute variance/edges, find chessboard pattern via simple checkered filter).
     - Use imageproc::edges::canny(&gray_img, low_thresh, high_thresh) → edge map, then count edges or use hough for lines (imageproc has geometric primitives).
     - Fallback: Assume central 50% of image if detection fails, or load from config (future).
     - Output: Bounds for crop; validate aspect ~1:1, size >200x200.
     - Tip: Convert to grayscale first (img.to_luma8()) for edge detection. Learning: Iter over pixels with img.enumerate_pixels() or to_rgb8().pixels().

3. **Preprocessing Helpers**:
   - `fn crop_and_resize(img: DynamicImage, bounds: (u32,u32,u32,u32)) -> Result<DynamicImage>`
     - image::imageops::crop(&img, x, y, w, h) → subimage, then resize(512,512, image::imageops::FilterType::Lanczos3) for sharp squares.
     - Why resize? Uniform processing; 64px squares easy for analysis.
   - Optional: `fn preprocess_square(sq: &DynamicImage) -> DynamicImage` (grayscale, normalize contrast for classify).

4. **Grid Splitting**:
   - `fn split_into_squares(img: &DynamicImage) -> Result<Vec<DynamicImage>>` (or [[DynamicImage;8];8] for fixed size).
     - Calc square_size = 512 / 8 = 64.
     - Nested loop: for row in 0..8 { for col in 0..8 { crop from (col*64, row*64, 64,64) } }
     - Parallel? rayon::iter::ParallelIterator on outer rows.
     - Rationale: 64 fixed outputs; easy to index back to chess coords (a1 bottom-left).

5. **Classification (Phase 1 Core)**:
   - `fn classify_square(sq: &DynamicImage, row: usize, col: usize) -> char`
     - Expected tile: Chess boards alternate light/dark; compute expected_rgb based on (row+col) %2 (light/dark from config or sample).
     - Avg color: Collect pixels → mean R/G/B (iterate or image::imageops::colorops?).
     - HSV convert: Implement helper fn rgb_to_hsv(r,g,b) -> (h,s,v) (math formulas: standard HSV from RGB).
     - Logic:
       - If avg close to expected tile (e.g., euclidean dist < threshold, low variance) → empty '/1' etc. handled in build.
       - Else occupied: High variance → piece. Hue for color (low S/V → empty/white pieces light; black dark). Stub type: e.g., if tall-ish (aspect after moments) → queen, else pawn (rough; improve with contours).
       - Output char: 'p','P','r', etc. or '?' for unknown (but aim for full FEN).
     - Thresholds: Hardcode or from config (e.g., const LIGHT_TILE: [u8;3] = [240,217,181]; // standard chess light).
     - Learning: Vec ops for pixels, f64 math for HSV/dist. Test with known empty/pawn PNGs.

6. **FEN Building**:
   - `fn construct_fen(pieces: Vec<char>) -> Result<String>`
     - Reshape to 8x8 rows.
     - Per row: String pieces + compress empties (count consecutive '/' -> num).
     - Join rows with '/', add " w KQkq - 0 1" (stub; detect turn via whose pieces more? Later).
     - Rationale: Logic separate for testability (feed known grid → expected FEN).

7. **Validation**:
   - Inline in board_to_fen, or separate `fn validate_fen(fen: &str) -> Result<()>`.
     - use shakmaty::fen::Fen; Fen::from_ascii(fen.as_bytes())?; (parse strict).
     - If invalid, log + return Err (e.g., retry capture in main?).

8. **Helpers & Utils**:
   - `fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64)`: Math impl (if/else for max channel, etc.; search Rust HSV for formula).
   - `fn avg_color(img: &DynamicImage) -> [f64;3]`: Sum pixels / count.
   - `fn save_debug(img: &DynamicImage, name: &str)`: If env var, save to screenshots/ocr/{name}.png.
   - Config integration: Later, fn load_templates(site: &str) -> HashMap<char, DynamicImage> for Phase 3 matching.

### Data Flow & Potential Challenges
```
load PNG (image::open) → detect_board (edges/contrast) → crop+resize (imageops) → split 8x8 → par_classify (color/HSV) → build FEN → shakmaty validate → String
```
- Challenges: Board detection accuracy (lighting, tilted boards? → affine transform later). Piece distinction naive (focus empty vs occupied first). Multi-site (chess.com vs lichess pieces differ → config thresholds).
- Perf: Profile with Instant; rayon adds ~10 lines (add rayon = { features = [""] }? No, already dep).
- Tests: #[cfg(test)] mod tests { #[test] fn test_classify_empty() { /* create ImageBuffer<Rgb<u8><u8>> mock */ } }. Use tempdir for files. Ignore graphical tests.
- Integration: After impl, run DEBUG_CAPTURE=1 cargo run → check screenshots/ocr/ for visuals. cargo test for units.

This structure keeps ocr.rs modular (~200-300 lines total for Phase 1). Start small: Add imports + board_to_fen stub returning hardcoded FEN, compile/test, then layer detection. What part do you want to tackle first (e.g., loading + resize)? Or any tweaks to this plan?