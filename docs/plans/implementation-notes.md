# Implementation Notes

Running log of implementation decisions and additions during development.

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
When enabled, each cycle displays:
- Cycle number
- Per-step timing (Capture, OCR, Engine)
- Total cycle time
- Results in box-drawing frame

```
┌─ Cycle 1 ─────────────────────────────────────────────────
│ [1] Capture:    23.4ms
│ [2] OCR:        67.2ms
│ [3] Engine:     45.8ms
│ [4] Total:     136.4ms
├─────────────────────────────────────────────────────────────
│ FEN:  rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1
│ Best: e7e5 (+0.12)
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
