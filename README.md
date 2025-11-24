# ZugzwangRS 🐎♟️

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/Rust-ed2021-orange.svg)](https://www.rust-lang.org/tools/install)

ZugzwangRS is an open-source, pure-Rust chess assistant that captures screenshots of chess positions from browser windows (e.g., Chess.com, Lichess.org), performs custom OCR to recognize the board as FEN notation, and analyzes moves using the Pleco chess engine. It emphasizes performance (<200ms latency), cross-platform compatibility (starting with macOS), and Rust learning—while avoiding browser extensions for privacy and stealth in personal use.

**Disclaimer**: This tool is for educational, offline analysis, or personal practice. Always respect online platform rules and fair play principles; it's not designed or endorsed for cheating in competitive games.

## Project Overview

Currently in MVP/Phase 1: Core modules have stub implementations (`todo!()` in `ocr.rs` and `engine.rs`), and calibration is disabled. The pipeline loops every 500ms to capture, recognize, analyze, and output chess positions.

1. **Screen Capture** (`capture.rs`): Grabs full primary monitor screenshot with `xcap` and saves as PNG.
2. **OCR Processing** (`ocr.rs`): (Stubbed) Loads image, detects board, generates FEN via custom recognition.
3. **Engine Analysis** (`engine.rs`): (Stubbed) Uses `pleco` for move suggestions and eval.
4. **CLI Output** (`main.rs`): Displays results in terminal.

For full specs and roadmap, see [docs/alpha-blueprint.md](docs/alpha-blueprint.md).

## Installation

1. Install Rust (if not already): Visit [rustup.rs](https://rustup.rs/) and follow the instructions.
2. Clone this repo:
   ```bash
   git clone https://github.com/hejijunhao/ZugzwangRS.git
   cd ZugzwangRS
   ```
3. Build:
   ```bash
   cargo build --release
   ```

Note: On macOS, grant Terminal "Screen Recording" permission in System Settings > Privacy & Security for capture to work.

## Usage

Once built, run the assistant:

```bash
cargo run
```

This starts the 500ms analysis loop, printing FEN, best move, and evaluation (stubs will panic until implemented).

### Debug Mode
Enable extra screenshot saves:
```bash
DEBUG_CAPTURE=1 cargo run
```
- Saves `screenshots/current_board.png` (main full screen capture).
- Also saves `screenshots/debug_full_screen.png` for comparison.

Position your chess site (e.g., Lichess game) on the primary monitor. Check saved images to ensure the board is captured.

### Testing
```bash
cargo test  # Unit tests
cargo test --test capture  # Module-specific
# Ignored integration tests (need display/permissions):
cargo test -- --ignored
```

### Future Commands
- Calibration: `cargo run -- --calibrate --site=chesscom` (Phase 2)
- CLI flags via `clap`: Depth, site, etc. (Phase 4)

## Development Notes

### Platform-Specific Setup
- **macOS**: Grant Terminal.app "Screen & System Audio Recording" permission (System Settings > Privacy & Security > Screen & System Audio Recording). For future calibration: "Accessibility" permission.
- **Linux/Windows**: Ensure screen capture permissions (e.g., Wayland/X11 support via `xcap`).
- **General**: `screenshots/` dir is gitignored; debug images saved there.

### Getting Involved
Position browser window on primary monitor for captures. Use debug mode to inspect screenshots before OCR/engine impl.

For detailed dev workflow, perf targets, and `todo!()` guidance, see [CLAUDE.md](CLAUDE.md) and docs/.

## Key Modules

- **capture.rs**: Full screen capture and save to PNG (no cropping yet; flexible for OCR detection).
- **ocr.rs**: Loads PNG, detects board, recognizes pieces to FEN (stubbed; naive color for MVP, `imageproc` templates in Phase 3).
- **engine.rs**: Analyzes FEN for best move/eval using `pleco` (stubbed).
- **config.rs**: Minimal stub for future `board_config.json` (bounds, site, thresholds).
- **calibrate.rs**: Interactive calibration (commented out; future: clicks for bounds/templates via `rdev`).

## Technology Stack

| Component | Crate | Purpose |
|-----------|-------|---------|
| Screen Capture | `xcap` 0.7.1 | Cross-platform screenshots |
| Image Processing | `image` 0.25.9, `imageproc` 0.25.0 | Manipulation, template matching |
| Chess Engine | `pleco` 0.5.0 | Pure-Rust analysis (~3000 ELO) |
| FEN Validation | `shakmaty` 0.29.4 | Board logic, move checks |
| Input Events | `rdev` 0.5.3 | Calibration clicks |
| Config | `serde`, `serde_json` | JSON I/O |
| Errors | `anyhow` | Handling |
| Terminal | `crossterm` 0.29.0 | Output (future) |
| CLI | `clap` 4 | Args (future) |

## Performance Targets
- Total pipeline: <200ms
- Parallelism via `rayon` for OCR.

## Roadmap (Phases)
1. **Phase 1 MVP**: Naive OCR, basic pleco.
2. **Phase 2**: Full calibration, enable in main.rs.
3. **Phase 3**: imageproc template matching.
4. **Phase 4**: Clap args, crossterm output.

## Stealth Considerations
- Avoids extensions for anti-cheat evasion.
- Future: Random delays, top-3 move randomization, hotkey triggers.

## Contributing
- Implement `todo!()` stubs per blueprint.
- Add tests for edge cases.
- Tune OCR for accuracy (>95%).

## Contributing

Contributions are welcome! ZugzwangRS is an open-source project perfect for Rust enthusiasts, chess fans, and those interested in computer vision/AI.

### How to Contribute
1. Fork the repo and create a feature branch (`git checkout -b feature/amazing-ocr-tweak`).
2. Commit changes (`git commit -m "Add OCR template matching"`).
3. Push to branch (`git push origin feature/amazing-ocr-tweak`).
4. Open a Pull Request.

Please add tests and update docs/alpha-blueprint.md if changing roadmap. Focus on:
- Implementing `todo!()` in `ocr.rs`/`engine.rs`.
- Improving accuracy (e.g., multi-site templates).
- Adding cross-platform tests.
- Performance optimizations.

Report bugs or suggest features via GitHub Issues. Code of conduct: Be respectful and follow Rust community guidelines.

For implementation details, see [docs/alpha-blueprint.md](docs/alpha-blueprint.md).

## License
Apache License 2.0. See [LICENSE](LICENSE) file.

For more details, see [docs/alpha-blueprint.md](docs/alpha-blueprint.md) and other docs/ files.
