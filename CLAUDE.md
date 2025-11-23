# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ZugzwangRS is a pure-Rust chess assistant that captures chess positions from browser windows, uses custom OCR for board recognition, and provides move analysis via a chess engine. The project prioritizes stealth (no browser extensions), performance (<200ms total latency), and learning Rust.

**Important Context**: This is currently in MVP/Phase 1 development with stub implementations marked by `todo!()` macros. The calibration module is commented out in main.rs until Phase 2.

## Build & Test Commands

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run
# For debug capture output (saves to screenshots/debug_board.png):
DEBUG_CAPTURE=1 cargo run
```

### Test
```bash
# Run all tests
cargo test

# Run specific module tests
cargo test --test capture
cargo test --test ocr
cargo test --test engine

# Run integration tests (note: some tests require display/permissions)
cargo test -- --ignored
```

### Future Commands (once implemented)
```bash
# Calibration mode (Phase 2)
cargo run -- --calibrate --site=chesscom
cargo run -- --calibrate --site=lichess
```

## Architecture

The application follows a 4-step pipeline in a continuous loop (500ms cycle):

1. **Screen Capture** (capture.rs) - Uses `xcap` to capture primary monitor, crops to board bounds
2. **OCR Processing** (ocr.rs) - Splits board into 8x8 grid, performs template matching to generate FEN
3. **Engine Analysis** (engine.rs) - Uses `pleco` chess engine to analyze position and suggest best move
4. **CLI Output** (main.rs) - Displays FEN, best move, and evaluation to terminal

### Key Module Responsibilities

- **capture.rs**: Screen capture with hardcoded bounds (200, 300, 480, 480) for MVP. Uses `xcap::Monitor` for cross-platform screenshots. Validates bounds and saves debug images when `DEBUG_CAPTURE` env var is set.

- **ocr.rs**: Board-to-FEN conversion. Currently stubbed with `todo!()`. Will use `imageproc` for template matching (Phase 3) or naive color analysis (Phase 1). Must validate output FEN with `shakmaty` crate.

- **engine.rs**: Move analysis using `pleco` (pure-Rust Stockfish port, ~3000 ELO). Currently stubbed. Will search to configurable depth (12-18) and return best move + evaluation in algebraic notation.

- **config.rs**: Config I/O for board_config.json using `serde`/`serde_json`. Stores bounds, site, thresholds, and template paths. Currently minimal stub.

- **calibrate.rs**: Interactive calibration using `rdev` for click events. Commented out in main.rs for Phase 1. Guides user through corner selection and piece template capture. **macOS Note**: Requires Accessibility permissions for input grabbing.

### Data Flow
```
Browser Window → xcap screenshot → crop to bounds →
8x8 grid split → template match each square → FEN string →
pleco::Board analysis → (best_move, eval) → terminal output
```

## Development Notes

### macOS Permissions
On macOS, Terminal.app requires "Screen & System Audio Recording" permission in System Settings > Privacy & Security for screen capture. Calibration mode also requires "Accessibility" permission for `rdev` input grabbing.

### Hardcoded Bounds (Phase 1)
Current board bounds are hardcoded in capture.rs:39 as `(200u32, 300u32, 480u32, 480u32)` representing (x, y, width, height). **You must manually adjust these** based on your chess browser window position. Use macOS screenshot tool to measure pixel coordinates.

### Template Storage
Templates are organized by site in `templates/{site}/` directories (chesscom, lichess, macOS). Each piece type will be stored as a PNG (K.png, Q.png, etc.) during calibration.

### Debug Workflow
1. Set `DEBUG_CAPTURE=1` environment variable to save cropped board images
2. Check `screenshots/debug_board.png` to verify capture bounds are correct
3. Validate dimensions (should be square, min 64x64 pixels)

### Implementing todo!() Stubs

**Order of Implementation** (per docs/alpha-blueprint.md):
1. **Phase 1 MVP**: Implement naive OCR (color averaging), basic pleco integration
2. **Phase 2**: Implement calibrate.rs fully, enable in main.rs
3. **Phase 3**: Enhance OCR with imageproc template matching
4. **Phase 4**: Add CLI args with clap, improve output with crossterm

**OCR Implementation Steps** (ocr.rs:17):
- Resize board to 512x512 for uniform 64px squares
- Divide into 8x8 grid (use imageproc or manual cropping)
- For Phase 1: Analyze each square via RGB/HSV averaging (dark/light, occupied/empty)
- For Phase 3: Use imageproc::template_matching with SSD/NCC against templates/
- Build FEN string (handle empty square runs, row separators)
- Validate with `shakmaty::fen::Fen::from_ascii()`

**Engine Implementation Steps** (engine.rs:14):
- Parse FEN with `pleco::Board::from_fen()`
- Initialize search to depth 12 for MVP (configurable later)
- Extract best move and centipawn evaluation
- Convert UCI move to SAN (Standard Algebraic Notation) via `shakmaty` if needed
- Handle edge cases: invalid FEN, checkmate, stalemate

### Performance Targets
- Screen capture + crop: <50ms
- OCR processing: <40ms (use `rayon` for parallel square processing)
- Engine analysis (depth 12): <100ms
- **Total pipeline**: <200ms

### Dependencies & Their Roles
- `xcap`: Cross-platform screen capture (supports Wayland)
- `image`/`imageproc`: Image manipulation and template matching
- `pleco`: Pure-Rust chess engine (alternative to external Stockfish)
- `shakmaty`: FEN validation and move legality checks
- `rdev`: Global mouse/keyboard events for calibration
- `rayon`: Optional parallelism for OCR grid processing
- `crossterm`: Terminal output (future phases)
- `clap`: CLI argument parsing (future phases)

### Testing Notes
- Tests marked `#[ignore]` require graphical display and macOS permissions
- Engine/OCR tests currently expect `todo!()` panics (will fail until implemented)
- Use small dummy images (64x64) for fast OCR unit tests
- Integration tests should use known FEN positions with expected moves

### Stealth Considerations
The project avoids browser extensions for anti-cheat stealth. Future enhancements may include:
- Random delays between moves
- Randomizing top-3 engine suggestions instead of always best move
- Hotkey trigger instead of continuous polling
- Human-like move timing variations

## Project Files

- `Cargo.toml`: Dependencies and project metadata (edition = "2024")
- `docs/alpha-blueprint.md`: Comprehensive design document and roadmap
- `templates/`: Site-specific piece templates (populated during calibration)
- `screenshots/`: Debug output directory (gitignored)
- `board_config.json`: Auto-generated calibration config (not yet created)
