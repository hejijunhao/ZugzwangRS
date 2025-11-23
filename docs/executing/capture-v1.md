# Implementation Plan for src/capture.rs (v1: MVP Phase 1)

## Overview
This module handles Step 1 of the architecture: Cross-platform screen capture using the `xcap` crate to grab a screenshot of the primary display, then crop it to the chess board area as a `DynamicImage` for downstream OCR.

- **Goals**:
  - Reliable capture on macOS (primary target; xcap supports Windows/Linux/Wayland).
  - Hardcoded bounds for MVP (tune manually via screenshots).
  - Low latency (~30-50ms total).
  - Error handling with `anyhow` for user-friendly msgs (e.g., permissions).
  - Debug output to `screenshots/` (gitignore'd).

- **Dependencies**: `xcap` (with `image` feature for `DynamicImage`), `image` for crop/save.
- **Challenges**: macOS screen recording permissions (prompt user if needed); multi-monitor select; headless test skips.
- **Future (v2+)**: Load bounds from `config::Config`; integrate calibration clicks; auto-monitor detect.

Run `cargo doc --open xcap` or check source (`~/.cargo/registry/.../xcap-0.7.1/src/`) for API: Key types `Monitor` (pub use from lib.rs), methods like `Monitor::all()` → Vec<Monitor>, `monitor.screenshot()` → `XCapResult<DynamicImage>`.

## Step-by-Step Implementation
Follow one step at a time: Code → `cargo check` → `cargo test` → `cargo run` (in main loop) → iterate. Use `rust-analyzer` VS Code extension for autocomplete/hints.

### Step 1: Update Imports and Function Signature
- Add necessary uses for xcap/image/anyhow.
- Keep `pub fn capture_board() -> anyhow::Result<image::DynamicImage>`.
- Concept: `anyhow::Result<T>` = ergonomic error type (chains `.context()` for msgs).

Example imports:
```rust
use anyhow::{Context, Result};
use image::DynamicImage;
use xcap::Monitor;
```

- Test: `cargo check` (no errors? Good).

### Step 2: Initialize xcap and Capture Screenshot
- Get monitors: `let monitors = Monitor::all()?;`
- Select primary: `let primary = monitors.into_iter().next().context("No monitors found")?.clone();` (or monitors[0] if single).
- Capture: `let screenshot = primary.screenshot().context("Screenshot failed (check permissions)")?;`
- macOS Tip: System Settings > Privacy & Security > Screen & System Audio Recording > allow app/terminal.
- Test: `dbg!(screenshot.dimensions());` or save `screenshot.save("screenshots/debug_full.png")?;` → run, verify file/size.

### Step 3: Hardcode Bounds and Crop
- `let bounds = (200u32, 300u32, 480u32, 480u32);` // Tune with Cmd+Shift+4 measure.
- `let cropped = screenshot.crop_imm(bounds.0, bounds.1, bounds.2, bounds.3)
  .context("Crop failed—check bounds in screen rect")?;`
- Concept: `crop_imm` returns new image (immutable).
- Test: `cropped.save("screenshots/debug_board.png")?;` → inspect PNG (board visible?).

### Step 4: Add Optional Debug & Latency Logging
- Env debug: `if std::env::var_os("DEBUG_CAPTURE").is_some() { cropped.save(...)?; }`
- Time: `use std::time::Instant; let start = Instant::now(); /* code */ println!("Latency: {:?}", start.elapsed());`
- Why? Blueprint goal; profile with `cargo flamegraph` later.

### Step 5: Comprehensive Error Handling & Edges
- Empty screenshot: `if screenshot.dimensions() == (0, 0) { bail!("No image captured") }`
- Multi-monitor: Log `monitors.len()`; select by name/area if needed.
- Headless: Test with `#[cfg(feature = "test")] or env skip.
- Permissions: Context msgs guide user.

### Step 6: Integrate with main.rs and ocr.rs
- main.rs: `let board_img = capture::capture_board()?;` → ocr::board_to_fen(&board_img)
- Update sigs if adding params later (e.g., bounds: Option<...>).
- Full Test: `cargo run` → captures in loop (expect crop of your screen).

### Step 7: Unit/Integration Tests
- `#[test] fn test_capture_dimensions() { let img = capture_board()?; assert!(img.width() > 0); }`
- Integration: Load static PNG? Or `#[ignore] = "needs display"`.
- `cargo test`; fix failures.

### Step 8: Optimization & Validation
- Validate bounds: `if bounds.2 < 64 || bounds.3 < 64 { bail!("Too small for board") }` (8x8 min).
- Resize for OCR: `cropped.resize_exact(512, 512, image::imageops::FilterType::Lanczos3)?;` (square grid).
- Perf: Avoid full resize if possible; parallel? Not needed.

### Step 9: v1 Complete & v2 Teaser
- Mark Done: Working end-to-end capture → cropped image → debug saved.
- v2: Accept `&config::Config` for dynamic bounds; error if None (prompt calibrate).
- Measure: Add to main loop timing.

### Resources/Tips
- xcap Source: lib.rs shows platform mods (macos.rs for details).
- Image Docs: `cargo doc --open image` for crop/filters.
- Errors: `RUST_BACKTRACE=1 cargo run` for traces.
- Permissions macOS: Test in new terminal post-grant.
- Track Changes: Git commit per step.

Update this MD as you implement—e.g., note tuned bounds.
