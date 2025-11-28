# Implementation Notes

Running log of implementation decisions and additions during development.

---

## Current Architecture (as of 2025-11-27)

### Pipeline Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│     CAPTURE     │───▶│       OCR       │───▶│     ENGINE      │───▶│     OUTPUT      │
│   xcap → JPEG   │    │  Image → FEN    │    │  FEN → Move     │    │    Terminal     │
│     ~200ms      │    │  ~2500-3000ms   │    │     ~85ms       │    │      <1ms       │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
        │                      │
        │               ┌──────┴──────┐
        ▼               ▼             ▼
   1920×1080        LLM Mode      Native Mode
   JPEG Q85      (full image)   (board detection
                                 + templates)
```

### Key Configuration

| Parameter | Value | Location |
|-----------|-------|----------|
| Max capture width | 1920px | `capture.rs:19` |
| Image format | JPEG Q85 | `capture.rs:70` |
| Engine depth | 6 | `engine.rs:33` |
| Min board size (Native) | 300px | `ocr_native.rs:74` |

### OCR Mode Differences

| Aspect | LLM Mode | Native Mode |
|--------|----------|-------------|
| Input | Full 1920×1080 screenshot | Cropped 512×512 board |
| Board detection | Skipped (GPT finds it) | Canny edge detection |
| Recognition | GPT-4o Mini vision API | Template matching |
| Latency | ~2500-3000ms | ~500-600ms |
| Requirements | `OPENAI_API_KEY` | `templates/{site}/` |

---

## #1: Verbose Logging Mode (2025-11-26)

### Summary
Added `--verbose` / `-v` CLI flag for real-time pipeline debugging with per-step timing.

### Usage
```bash
cargo run -- --verbose
cargo run -- -v
cargo run -- --ocr=native --site=chesscom -v
```

### Output Format
```
┌─ Cycle 1 ─────────────────────────────────────────────────
│ [1] Capture:    23.4ms
│ [2] OCR:        67.2ms
│ [3] Engine:     45.8ms
│ [4] Total:     136.4ms
├─────────────────────────────────────────────────────────────
│ FEN:  rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1
│ Best: C2 to C3 (+0.12)
└─────────────────────────────────────────────────────────────
```

### Files Changed
- `src/main.rs`: Added `--verbose` flag, cycle counter, timing instrumentation

### Related Debug Modes
| Mode | Purpose |
|------|---------|
| `--verbose` / `-v` | Real-time pipeline timing (stdout) |
| `DEBUG_CAPTURE=1` | Save extra screenshot copy |
| `DEBUG_OCR=1` | Save cropped board + square images |

Full debug: `DEBUG_CAPTURE=1 DEBUG_OCR=1 cargo run -- -v`

---

## #2: Manual Capture Trigger Mode (2025-11-26)

### Summary
Added `--trigger=auto|manual` CLI flag to control when screenshots are captured.

### Motivation
- **Cost savings**: In LLM OCR mode, auto-capture at 1s intervals = ~60 API calls/minute
- **Reduced noise**: Skip redundant analysis when the board hasn't changed
- **Better for correspondence games**: Analyze only when you're ready to move

### Usage
```bash
# Auto mode (default) - captures every interval
cargo run -- --trigger=auto --interval=1000

# Manual mode - press Enter to capture
cargo run -- --trigger=manual

# Combine with other flags
cargo run -- --trigger=manual --ocr=llm -v
```

### Output Format
**Manual mode:**
```
▶ Press Enter to capture & analyze... [user presses Enter]
Capturing screen... 206ms (6016×3384 → 1920×1080)
LLM OCR... 2576ms
Engine analysis... (depth 6) 85ms
FEN:  rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1
Best: C2 to C3 (+0.44)

▶ Press Enter to capture & analyze...
```

### Files Changed
- `src/main.rs`: Added `--trigger` CLI argument, stdin handling, conditional sleep

---

## #3: Interactive Mode Selectors (2025-11-27)

### Summary
Added interactive prompts for OCR mode and trigger mode selection at startup.

### Usage
```bash
# Interactive mode - prompts for both OCR and trigger mode
cargo run

# Skip prompts with explicit flags
cargo run -- --ocr=native --trigger=manual
```

### Interactive Flow
```
╔═══════════════════════════════════════════════════════════╗
║         Zugzwang-RS Chess Assistant v0.1.1                ║
╚═══════════════════════════════════════════════════════════╝

? Select OCR mode ›
❯ Native (template matching) - fast, requires templates/
  LLM (GPT-4o Mini) - accurate, requires API key

? Select capture trigger ›
❯ Auto (continuous) - captures at regular intervals
  Manual (on-demand) - press Enter to capture
```

### Files Changed
- `src/main.rs`: Added `select_ocr_mode_interactive()` and `select_trigger_mode_interactive()`

---

## #4: High-DPI Display Optimization (2025-11-27)

### Summary
Fixed catastrophic performance issues on high-DPI displays (4K/5K/6K) by adding downsampling and switching to JPEG encoding.

### Problem
On a 6016×3384 display, the pipeline was taking 17+ seconds:

| Step | Expected | Actual | Issue |
|------|----------|--------|-------|
| Screen capture + PNG save | <30ms | ~8000ms | PNG encoding 20M pixels |
| Board detection | <50ms | ~9000ms | Edge detection on full 6K |
| Candidate search | <100ms | Never finishes | O(n³) with ~50,000 candidates |

### Solution

#### 1. Downsampling to 1920px
```rust
const MAX_CAPTURE_WIDTH: u32 = 1920;

let final_img = if orig_width > MAX_CAPTURE_WIDTH {
    let scale = MAX_CAPTURE_WIDTH as f32 / orig_width as f32;
    imageops::resize(&img, MAX_CAPTURE_WIDTH, new_height, FilterType::Triangle)
};
```

**Why 1920px?** Balances LLM OCR accuracy (~450px chessboard) with processing speed.

#### 2. JPEG instead of PNG
```rust
JpegEncoder::new_with_quality(&mut writer, 85)
    .write_image(rgb_img.as_raw(), ...)
```

**Why JPEG?** ~10-50× faster encoding than PNG for equivalent visual quality.

#### 3. Skip board detection for LLM mode
```rust
match mode {
    OcrMode::Llm => {
        // Send full screenshot directly - GPT-4o Mini finds the board itself
        crate::ocr_llm::board_to_fen(image_path).await
    }
    OcrMode::Native => {
        // Board detection required for template matching
        let board_img = screenshot_to_board(&path)?;
        // ...
    }
}
```

### Performance Results

| Step | Before (6K) | After (1920px) | Improvement |
|------|-------------|----------------|-------------|
| Capture + encode + save | ~8000ms | ~200ms | **40×** |
| Board detection (Native) | ~9000ms | ~500ms | **18×** |
| Board detection (LLM) | ~9000ms | **0ms** (skipped) | **∞** |

### Files Changed
- `src/capture.rs`: Downsampling, JPEG encoding, resolution logging
- `src/ocr.rs`: Mode-specific board detection logic
- `src/main.rs`: Updated image path to `.jpg`

---

## #5: Board Detection Accuracy Fix (2025-11-27)

### Summary
Fixed board detection selecting partial boards instead of full 8×8 boards.

### Problem
After downsampling, the edge-density algorithm was finding small high-density regions (3×3 squares) instead of the full chessboard.

**Root cause**: Pure edge density (`edges / pixels`) favors smaller regions with clustered pieces over larger regions with empty squares.

### Solution

#### 1. Size-weighted scoring
```rust
// Before: Pure density (favors small regions)
let score = density;

// After: Density × size (favors larger boards)
let score = density * size as f32;
```

#### 2. Increased minimum board size
```rust
let min_size = 300u32;  // Was 200
```

#### 3. Finer search granularity
```rust
let size_step = 50u32;  // Was 100
```

### Files Changed
- `src/ocr_native.rs`: Scoring formula, min_size, size_step, debug logging

---

## #6: FEN Validation & Engine Crash Prevention (2025-11-27)

### Summary
Added king count validation to prevent Tanton engine panics on invalid FEN positions.

### Problem
GPT-4o Mini occasionally returns syntactically valid but semantically invalid FEN strings (missing kings, 9 pawns). The `shakmaty` parser accepts these, but Tanton panics:

```
thread 'main' panicked at tanton/src/core/bitboard.rs:97:9:
assertion `left == right` failed: left: 0, right: 1
```

### Solution
```rust
fn validate_fen(fen: &str) -> Result<()> {
    // Step 1: Basic syntax validation
    shakmaty::fen::Fen::from_ascii(fen.as_bytes())?;

    // Step 2: Validate king count (exactly 1 per side)
    let board_part = fen.split_whitespace().next().unwrap_or("");
    let white_kings = board_part.chars().filter(|&c| c == 'K').count();
    let black_kings = board_part.chars().filter(|&c| c == 'k').count();

    if white_kings != 1 || black_kings != 1 {
        anyhow::bail!("Invalid FEN: expected 1 king per side");
    }
    Ok(())
}
```

### Files Changed
- `src/ocr_llm.rs`: Enhanced `validate_fen()` with king count check

---

## #7: Engine Search Depth Tuning (2025-11-27)

### Summary
Reduced Tanton engine search depth from 12 to 6 for real-time responsiveness.

### Problem
At depth 12, the engine hung indefinitely on opening positions (20+ legal moves = exponential blowup).

### Solution
```rust
const SEARCH_DEPTH: u16 = 6;  // Was 12
```

### Performance
| Depth | Time (opening) | Strength |
|-------|----------------|----------|
| 6 | ~85ms | Good for real-time |
| 12 | Hangs | Too slow |

### Future Enhancement
Implement time-based search (e.g., "think for 100ms") instead of fixed depth.

### Files Changed
- `src/engine.rs`: Reduced `SEARCH_DEPTH`, added progress indicator

---

## #8: Human-Readable Move Format (2025-11-27)

### Summary
Changed move output from UCI notation to readable format.

### Before/After
| UCI | Readable |
|-----|----------|
| `c2c3` | `C2 to C3` |
| `g1f3` | `G1 to F3` |
| `e7e8q` | `E7 to E8 (=Q)` |

### Implementation
```rust
fn format_move_readable(uci: &str) -> String {
    let from = &uci[0..2].to_uppercase();
    let to = &uci[2..4].to_uppercase();

    if uci.len() == 5 {
        let promo = uci.chars().nth(4).unwrap().to_uppercase();
        format!("{} to {} (={})", from, to, promo)
    } else {
        format!("{} to {}", from, to)
    }
}
```

### Files Changed
- `src/engine.rs`: Added `format_move_readable()` function

---

## #9: Real-time Progress Logging (2025-11-27)

### Summary
Added real-time progress indicators with explicit `flush()` calls.

### Output Format
```
Capturing screen... 206ms (6016×3384 → 1920×1080)
LLM OCR... 2576ms
Engine analysis... (depth 6) 85ms
FEN:  rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1
Best: C2 to C3 (+0.44)
```

### Implementation
```rust
eprint!("Capturing screen... ");
let _ = std::io::stderr().flush();  // Force immediate display
// ... do work ...
eprintln!("{:.0}ms", elapsed);
```

### Files Changed
- `src/capture.rs`, `src/ocr.rs`, `src/engine.rs`: Progress indicators with flush

---

## Current Pipeline Performance (2025-11-27)

### LLM Mode (Recommended)
```
▶ Press Enter to capture & analyze...
Capturing screen... 206ms (6016×3384 → 1920×1080)
LLM OCR... 2576ms
Engine analysis... (depth 6) 85ms
FEN:  rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1
Best: C2 to C3 (+0.44)
```

| Step | Time |
|------|------|
| Capture + downsample + JPEG | ~200ms |
| LLM OCR (GPT-4o Mini) | ~2500-3000ms |
| Engine (depth 6) | ~85ms |
| **Total** | **~3.3s** |

### Native Mode (When templates available)
```
Capturing screen... 180ms (6016×3384 → 1920×1080)
Board detection... 450ms
Template matching... 95ms
Engine analysis... (depth 6) 85ms
FEN:  ...
Best: ...
```

| Step | Time |
|------|------|
| Capture + downsample + JPEG | ~180ms |
| Board detection | ~450ms |
| Template matching | ~95ms |
| Engine (depth 6) | ~85ms |
| **Total** | **~810ms** |

---

## Screenshot File Management

| File | Purpose | Updated |
|------|---------|---------|
| `screenshots/current_board.jpg` | Latest capture (sent to OCR) | Every cycle |
| `screenshots/cropped_board.png` | Detected board (Native mode only) | Native mode only |
| `screenshots/debug_full_screen.jpg` | Debug copy | When `DEBUG_CAPTURE=1` |

**Note**: Files are overwritten each cycle — no history is kept.
