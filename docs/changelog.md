# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to Rust conventions and semantic versioning.

## [0.0.3] - 2024-10-01 (OCR Module Complete)

### Added
- **ocr.rs**: Full board detection and piece recognition pipeline
  - `screenshot_to_board()`: Dynamic board detection via Canny edge detection
    - Generates candidate regions with 75% overlap grid search
    - Scores regions by edge density (chessboards have high edge count)
    - Validates minimum 1% edge density threshold
    - Crops and resizes to 512x512 (64px uniform squares)
  - `board_to_fen()`: Template-based piece recognition
    - Loads 12 piece templates from `templates/{site}/`
    - Splits board into 8x8 grayscale grid
    - Variance analysis for empty square detection (low variance = empty)
    - Template matching via `imageproc` SSD normalized (threshold 0.3)
    - FEN string builder with empty square compression
    - Validation via `shakmaty::fen::Fen::from_ascii()`
  - Debug mode: `DEBUG_OCR=1` saves cropped board + individual squares

### Notes
- Requires piece templates in `templates/chesscom/` (K.png, Q.png, etc.)
- Hardcoded to "chesscom" site for MVP; multi-site support in Phase 2
- Engine module next: pleco integration for move analysis

[0.0.3]: https://github.com/pkhelfried/ZugzwangRS/compare/v0.0.2...v0.0.3

## [0.0.2] - 2024-10-01 (Capture Module Complete)

### Changed
- **capture.rs**: Refactored from stub to pure screenshot service
  - Full-screen primary monitor capture via `xcap::Monitor`
  - Saves PNG to `screenshots/current_board.png` for OCR consumption
  - No cropping—delegates board detection to OCR for flexibility across apps/sites
  - Debug mode: `DEBUG_CAPTURE=1` saves extra copy to `debug_full_screen.png`
  - Latency logging to stderr (target: <30ms)
  - Robust error handling with `anyhow::Context`

### Added
- Dimension validation test for captured screenshots (ignored; requires display)

### Notes
- macOS requires Screen Recording permission for Terminal
- OCR module next: will load PNG and handle board detection/cropping

[0.0.2]: https://github.com/pkhelfried/ZugzwangRS/compare/v0.0.1...v0.0.2

## [0.0.1] - 2024-10-01 (MVP Skeleton Launch)

### Added
- Repository launch for ZugzwangRS: pure-Rust chess assistant capturing screens, OCR to FEN, pleco engine analysis, CLI output (<200ms latency, stealthy).
- Module skeleton:
  - `main.rs`: Pipeline loop (capture -> OCR -> engine -> print; 500ms).
  - `capture.rs`: Full-screen primary monitor capture (xcap), saves PNG to `screenshots/current_board.png` (no crop; OCR handles detection for flexibility e.g., macOS Chess.app/browsers).
  - `ocr.rs`: Loads PNG, stubs board detect/crop/grid, naive color->FEN (todo; shakmaty validate).
  - `engine.rs`: Stubs FEN->best move/eval (todo; pleco depth 12).
  - `config.rs`/`calibrate.rs`: Stubs for JSON/config, interactive bounds/templates (Phase 2).
- Tests/docs: Basic/ignored tests; CLAUDE.md/blueprint.md guidance.
- Dirs: templates/, screenshots/ (gitignore).

### Notes
- Run: `cargo run` (panics at stubs); grant macOS screen perms.
- Phase 1 next: OCR/engine impls.

[0.0.1]: https://github.com/pkhelfried/ZugzwangRS/compare/v0.0.0...v0.0.1
