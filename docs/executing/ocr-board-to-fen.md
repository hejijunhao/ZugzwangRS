  Implementation Strategy

  Architecture Overview

  pub fn board_to_fen(image_path: &str) -> Result<String> {
      // 1. Get the 512×512 board image
      let board_img = screenshot_to_board(image_path)?;

      // 2. Load piece templates (12 PNGs)
      let templates = load_templates()?;

      // 3. Split into 8×8 grid (64 squares)
      let squares = split_into_grid(&board_img);

      // 4. Recognize each square (template matching)
      let board_state = recognize_squares(&squares, &templates)?;

      // 5. Build FEN string
      let fen = build_fen_string(&board_state);

      // 6. Validate with shakmaty
      validate_fen(&fen)?;

      // TODO Phase 2: LLM fallback mode
      // if template_matching_confidence < threshold {
      //     return llm_vision_fen(image_path)?;
      // }

      Ok(fen)
  }

  ---
  Step-by-Step Implementation Plan

  Step 1: Template Storage Structure

  First, we need templates. For MVP, let's create a simple structure:

  templates/
    └── chesscom-2d-default/
        ├── wK.png  (white King)
        ├── wQ.png  (white Queen)
        ├── wR.png  (white Rook)
        ├── wB.png  (white Bishop)
        ├── wN.png  (white Knight)
        ├── wP.png  (white Pawn)
        ├── bK.png  (black King)
        ├── bQ.png  (black Queen)
        ├── bR.png  (black Rook)
        ├── bB.png  (black Bishop)
        ├── bN.png  (black Knight)
        └── bP.png  (black Pawn)

  For now: We'll need to manually capture these (screenshot a position, crop each piece to 64×64)

  Phase 2: calibrate.rs will automate this

  ---
  Step 2: Load Templates Function

  struct PieceTemplates {
      pieces: HashMap<char, GrayImage>,  // 'K' -> white king template, etc.
  }

  fn load_templates(site: &str) -> Result<PieceTemplates> {
      // Load all 12 PNG files from templates/{site}/
      // Convert to grayscale for matching
      // Store in HashMap: 'K' -> king image, 'Q' -> queen, etc.
      // Uppercase = white, lowercase = black
  }

  ---
  Step 3: Grid Splitting Function

  fn split_into_grid(board: &DynamicImage) -> Vec<Vec<DynamicImage>> {
      // board is 512×512, so each square is 64×64
      let mut grid = Vec::new();

      for rank in 0..8 {  // rows (8, 7, 6... 1)
          let mut row = Vec::new();
          for file in 0..8 {  // columns (a, b, c... h)
              let x = file * 64;
              let y = rank * 64;
              let square = board.crop_imm(x, y, 64, 64);
              row.push(square);
          }
          grid.push(row);
      }

      grid  // Returns Vec<Vec<DynamicImage>> - 8×8 grid of square images
  }

  ---
  Step 4: Template Matching Function

  use imageproc::template_matching::{match_template, MatchTemplateMethod};

  fn recognize_square(
      square: &DynamicImage,
      templates: &PieceTemplates
  ) -> Option<char> {
      // Convert square to grayscale
      let square_gray = square.to_luma8();

      // Try matching against all 12 piece templates
      let mut best_match = None;
      let mut best_score = 0.0;

      for (piece_char, template) in &templates.pieces {
          let result = match_template(
              &square_gray,
              template,
              MatchTemplateMethod::SumOfSquaredErrors  // or CrossCorrelation
          );

          // Get the best match score
          let score = get_best_score(&result);

          if score > best_score {
              best_score = score;
              best_match = Some(*piece_char);
          }
      }

      // If best score is too low, consider it an empty square
      if best_score < EMPTY_THRESHOLD {
          None  // Empty square
      } else {
          best_match  // 'K', 'q', 'P', etc.
      }
  }

  fn recognize_squares(
      grid: &Vec<Vec<DynamicImage>>,
      templates: &PieceTemplates
  ) -> Result<Vec<Vec<Option<char>>>> {
      // Apply recognize_square to each of the 64 squares
      // Returns 8×8 grid of piece characters (or None for empty)
  }

  ---
  Step 5: Build FEN String

  fn build_fen_string(board_state: &Vec<Vec<Option<char>>>) -> String {
      let mut fen = String::new();

      // Process ranks 8 to 1 (top to bottom from white's perspective)
      for rank in 0..8 {
          let mut empty_count = 0;

          // Process files a to h (left to right)
          for file in 0..8 {
              match board_state[rank][file] {
                  None => {
                      // Empty square - count consecutive empties
                      empty_count += 1;
                  }
                  Some(piece) => {
                      // Piece found - flush empty count if any
                      if empty_count > 0 {
                          fen.push_str(&empty_count.to_string());
                          empty_count = 0;
                      }
                      fen.push(piece);  // 'K', 'q', 'P', etc.
                  }
              }
          }

          // Flush remaining empties
          if empty_count > 0 {
              fen.push_str(&empty_count.to_string());
          }

          // Add rank separator (except after rank 1)
          if rank < 7 {
              fen.push('/');
          }
      }

      // Append metadata: side to move, castling, en passant, etc.
      fen.push_str(" w KQkq - 0 1");

      fen
  }

  ---
  Step 6: Validation

  use shakmaty::fen::Fen;

  fn validate_fen(fen_str: &str) -> Result<()> {
      Fen::from_ascii(fen_str.as_bytes())
          .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))?;

      Ok(())
  }

  ---
  Before We Implement: Template Creation

  We have a chicken-and-egg problem: We need templates to test, but don't have them yet.

  Options:

  A. Manual Template Creation (Quick Start)
  1. Open Chess.com in 2D mode
  2. Set up a position with all piece types
  3. Take screenshot
  4. Manually crop each piece to 64×64 PNG
  5. Save to templates/chesscom-2d-default/

  B. Stub with Placeholders (Test Infrastructure First)
  fn load_templates() -> Result<PieceTemplates> {
      // Return empty templates for now
      // Just build the infrastructure, test with dummy data
      todo!("Need to create piece template PNGs first")
  }

  C. Use Existing Dataset
  Find online chess piece sprites and adapt them (may need licensing check)

  ---
  My Recommendation

  Let's implement in this order:

  1. Grid splitting (easy, no dependencies)
  2. FEN building (can test with hardcoded board states)
  3. Template loading infrastructure (stub for now)
  4. Template matching (implement, but won't work until templates exist)
  5. Create templates manually (one-time setup)
  6. Test end-to-end

  This lets us build the machinery before worrying about the templates.