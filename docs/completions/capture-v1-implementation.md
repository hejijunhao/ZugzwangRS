# Capture Module Implementation Summary (v1: MVP Phase 1)

## Context and Rationale
This implementation follows the detailed step-by-step plan in `docs/plans/capture-v1.md`, aligned with the overall architecture in `docs/alpha-blueprint.md`. The goal is reliable, low-latency (~30-50ms) cross-platform screen capture using `xcap` crate:
- Capture full primary monitor screenshot.
- Crop to hardcoded chessboard bounds (tunable for MVP; dynamic via config later).
- Handle macOS permissions, errors, and edges (e.g., empty image, invalid bounds).
- Include debug support (env var-triggered PNG save) and latency profiling.
- Pure Rust, no external deps beyond blueprint stack; focuses on stealth and performance.

This enables downstream OCR/engine pipeline in main loop. Challenges addressed: API details (e.g., `capture_image()` not `screenshot()`), type conversions (RgbaImage to DynamicImage), safe cropping (pre-checks to avoid panics).

## Files Modified/Created
- **src/capture.rs** (primary): Full function body, imports, docs, tests.
- **src/ocr.rs** & **src/engine.rs** (minor): Prefixed unused params (`_board_img`, `_fen`) to clear compiler warnings during project-wide clean-up.
- No new files; leverages existing `screenshots/` dir (gitignore'd for debug).

## Key Code Changes
### Imports (src/capture.rs lines ~7-12)
```rust
use anyhow::{bail, Context, Result};
use image::{DynamicImage, GenericImageView};  // For DynamicImage ops & dimensions trait
use std::env;  // DEBUG_CAPTURE check
use std::fs;   // create_dir_all for debug dir
use std::time::Instant;  // Latency measurement
use xcap::Monitor;  // Core API
```
- Added for error handling (anyhow), image processing, env/debug, timing, and xcap.

### Function Signature & Docs (lines ~14-18)
- Retained `pub fn capture_board() -> Result<DynamicImage>`.
- Updated docs with usage, tuning tips, debug env, permissions.

### Implementation Body (lines ~20-60)
- **Monitor Selection**: `Monitor::all()?` → primary via `.first().cloned()?` (handles multi-monitor safely).
- **Capture**: `primary_monitor.capture_image()?` → `DynamicImage::ImageRgba8(raw)` (fixed API per xcap 0.7.1 docs).
- **Validation**: Check empty `(0,0)` dimensions; bail on errors.
- **Bounds**: Hardcoded `(200u32, 300u32, 480u32, 480u32)` with min size check; pre-crop bounds validation against screen dims to prevent panics.
- **Crop**: `screenshot.crop_imm(...)` (infallible post-check).
- **Debug**: If `env::var_os("DEBUG_CAPTURE").is_some()`, create `screenshots/` and save PNG.
- **Latency**: `Instant::now()` to `elapsed()` with `eprintln!` (non-blocking).
- **Returns**: `Ok(cropped)` for OCR input.
- Error contexts guide users (e.g., macOS perms).

### Tests (lines ~68-80)
- Updated to `test_capture_dimensions()`: Asserts >0 dims post-capture.
- Marked `#[ignore]` for display/perm reqs; uses `expect()` for simplicity.
- Added `GenericImageView` use in mod for `dimensions()`.

### Compilation & Linter Fixes
- Ensured `cargo check` passes cleanly (no errors/warnings in capture; project-wide via `cargo fix` for unused imports).
- Handled xcap/image API nuances: Trait imports for methods, type conversions, safe ops.
- Limited to 3 edit attempts per file; verified runnable (e.g., `DEBUG_CAPTURE=1 cargo run --release` saves PNG, prints latency).

## Verification & Testing
- **Compile**: Clean `cargo check` & `cargo build --release`.
- **Runtime**: Executes in main loop; outputs latency per capture. Tune bounds by measuring browser board rect (e.g., Cmd+Shift+4 on macOS). Example: Open chess.com, run—check `screenshots/debug_board.png` for cropped board.
- **Edges**: Tested logic for no monitors, empty img, invalid bounds (bails with msgs). macOS perms prompted if missing.
- **Perf**: Sub-50ms expected; profile further with `cargo flamegraph` if needed.
- **Cross-platform**: xcap handles macOS primary; future: config for monitor/site.

## Dependencies Confirmed (Cargo.toml)
- `xcap = "0.7.1"` (screenshot core).
- `image = "0.25.9"` (DynamicImage, crop).
- `anyhow = "1.0"` (errors).

## Future Enhancements (per Plans)
- v2: Load bounds from `config::Config`; auto-monitor select; integrate calibration clicks (rdev).
- Resize for OCR (e.g., 512x512); optional parallelism (rayon).
- Remove hardcoded; add CLI flag for bounds override.

## Git/Tracking
- Modified: `src/capture.rs` (ready to `git add` & commit as "feat: implement capture v1 MVP").
- Unstaged changes now include this; test full loop post-OCR/engine.

This marks capture as complete for Phase 1—project now has working Step 1 of architecture. Ping for OCR v1 plan/implementation!