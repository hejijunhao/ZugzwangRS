# CLI Site Option Integration

## Summary
Added user-selectable `--site` CLI flag to replace hardcoded "chesscom" in OCR template loading, enabling multi-site support (chesscom, lichess, macOS).

## Changes

### main.rs
- Imported `clap::{Arg, Command}`.
- Added CLI argument parsing with `--site` flag:
  - Default: "chesscom"
  - Allowed values: ["chesscom", "lichess", "macOS"]
  - Help text and validation.
- Pass parsed `site` to `ocr::board_to_fen()`.
- Added startup print: "Targeting site: {site}".

### ocr.rs
- Modified `board_to_fen(image_path: &str, site: &str)` to accept `site` parameter.
- Updated `load_templates(site)` call to use dynamic `site` instead of hardcoded "chesscom".

## Usage
- `cargo run` → Defaults to chesscom.
- `cargo run -- --site lichess` → Targets Lichess templates.
- `cargo run -- --help` → Shows options.

## Notes
- Requires `clap` crate (already in Cargo.toml).
- Templates must exist in `templates/{site}/` (populated via calibration in Phase 2).
- Aligns with blueprint Phase 4 (CLI args).