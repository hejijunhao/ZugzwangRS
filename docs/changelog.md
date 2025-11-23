# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to Rust conventions and semantic versioning.

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
