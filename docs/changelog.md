# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to Rust conventions and semantic versioning.

## [0.1.1] - 2025-11-26 (Cleanup & Edition Upgrade)

### Changed
- **Edition**: Upgraded to `edition = "2024"` (now stable since Rust 1.85.0, Feb 2025)
- **Dependencies**: Removed unused deps (`crossterm`, `rayon`, `rdev`, `serde_json`)
  - Kept as comments in Cargo.toml for future phase reference
  - `serde` retained (used by `ocr_llm.rs` for API serialization)

### Fixed
- Assessment document (`docs/executing/assessment-25nov25.md`) updated with accurate project status

### Notes
- All 12 chesscom templates now complete: KW, QW, RW, BW, NW, PW, KB, QB, RB, BB, NB, PB
- Build time improved with fewer dependencies
- Project now compiles on stable Rust with Edition 2024 features available

---

## [0.1.0] - 2025-11-26 (LLM OCR & Async Runtime)

### Added
- **LLM OCR Mode** (`src/ocr_llm.rs`): Alternative OCR using GPT-4o Mini vision API
  - Sends cropped board image to OpenAI for FEN extraction
  - More accurate than template matching, especially for non-standard piece sets
  - Requires `OPENAI_API_KEY` environment variable
- **Native OCR Module** (`src/ocr_native.rs`): Refactored template matching into dedicated module
- **OCR Facade** (`src/ocr.rs`): Unified interface supporting both Native and LLM modes
- **Interactive Mode Selector**: When no `--ocr` flag provided, displays menu to choose OCR mode
- **CLI Enhancements**:
  - `--ocr=native|llm` flag for explicit mode selection
  - `--interval=MS` flag for configurable loop timing (default: 1000ms)
- **Async Runtime**: Migrated to `tokio` for non-blocking LLM API calls
- **Dependencies**: Added `tokio`, `reqwest`, `base64`, `dialoguer` for LLM/async support

### Changed
- **main.rs**: Complete rewrite with async/await, interactive UI, configurable intervals
- **Edition**: Fixed `edition = "2024"` → `"2021"` (2024 is nightly-only)
- **Version**: Standardized to 0.1.0 across Cargo.toml, main.rs banners, and CLI

### Fixed
- Version inconsistency between Cargo.toml, main.rs, and changelog

### Notes
- Native mode requires `templates/{site}/` PNG files
- LLM mode requires `OPENAI_API_KEY` set in environment
- Both modes validated via `shakmaty` FEN parsing
- Performance: Native <200ms, LLM ~500-1000ms (API latency)

---

## [0.0.5] - 2024-10-02 (Phase 1 MVP Live - Full Pipeline)

### Added
- **main.rs**: Complete end-to-end MVP pipeline integration
  - Continuous 500ms loop: capture full screen → OCR FEN (site-aware) → engine analysis → CLI output.
  - Clap CLI: `--site=chesscom` (default) for templates.
  - Error chaining with anyhow (robust panics).
- **docs/executing/cli_site_option.md**: Documentation for `--site` CLI flag implementation.
- **docs/executing/engine_api_fix.md**: Documentation for tanton API fix.

### Fixed
- **engine.rs**: Resolved compilation error with `IterativeSearcher::best_move()` by changing from instance method to associated function (tanton v1.0 API update).
- **ocr.rs**: Updated `board_to_fen()` to accept dynamic `site` parameter instead of hardcoded "chesscom" for multi-site template support.

### Changed
- main.rs: Commented unused Phase 2 mods (`config`, `calibrate`).
- Version bump to 0.0.5; now fully operational (`cargo run` analyzes live chess tabs).

### Notes
- Requires Screen Recording perm (macOS); `templates/chesscom/` PNGs for OCR.
- Debug: `DEBUG_CAPTURE=1` / `DEBUG_OCR=1`.
- Performance: Full cycle <200ms target.
- Next: Phase 2 calibration/templates, tests, crossterm UI.

## [0.0.4] - 2024-10-02 (Engine Module Complete)

### Added
- **engine.rs**: Full tanton pure-Rust chess engine integration (~2900 ELO at depth 12)
  - `analyze_position(fen)`: FEN parse, terminal checks (checkmate/stalemate), iterative deepening search, PSQT evaluation post-best-move.
  - Returns UCI move + formatted eval (e.g., "e2e4 +0.27", "-- Checkmate").
  - Handles invalid FEN errors gracefully.
  - Latency target: <100ms.

### Changed
- Cargo.toml: pleco → tanton="0.5" (active maintained fork, identical API).

### Notes
- Full Phase 1 MVP pipeline now operational: capture → OCR → engine → CLI output (<200ms loop).
- Requires `templates/chesscom/` PNGs for OCR accuracy.
- Tests: Add unit tests for starting pos/mates; integration via main.rs loop.
- Next: Phase 2 calibration, CLI polish.

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
