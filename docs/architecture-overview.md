# ZugzwangRS Architecture Overview

> A pure-Rust chess assistant: screen capture → OCR → engine analysis → terminal output

## Core Pipeline

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   CAPTURE   │───▶│     OCR     │───▶│   ENGINE    │───▶│   OUTPUT    │
│  xcap → PNG │    │ PNG → FEN   │    │ FEN → Move  │    │  Terminal   │
│    <30ms    │    │   40-80ms   │    │   50-100ms  │    │    <1ms     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

**Total latency: <200ms per cycle** | **Loop interval: 500ms**

## Module Responsibilities

| Module | Purpose | Key Function |
|--------|---------|--------------|
| `main.rs` | Orchestrate pipeline loop | `main()` — CLI args, infinite loop |
| `capture.rs` | Full-screen screenshot | `capture_screenshot()` → PNG file |
| `ocr.rs` | Board detection + piece recognition | `board_to_fen(path, site)` → FEN string |
| `engine.rs` | Position analysis | `analyze_position(fen)` → (move, eval) |

## Data Flow

```
Screen → screenshots/current_board.png → FEN string → (best_move, evaluation) → stdout
         ↑                               ↑                                      ↑
     capture.rs                       ocr.rs                               engine.rs
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Full-screen capture** | No calibration needed; OCR auto-detects board location |
| **File-based handoff** | Simpler debugging; PNG on disk aids inspection |
| **Site-specific templates** | Chess.com/Lichess render pieces differently; matched templates improve accuracy |
| **Tanton over Stockfish** | Pure Rust (no external binary); ~2900 ELO is sufficient |
| **Edge-based board detection** | Chessboards have high edge density; robust across UI variations |
| **Variance-based empty detection** | Fast early-exit before expensive template matching |

## OCR Strategy (2-Phase)

1. **Board Detection**: Canny edge detection → grid search → highest edge-density region
2. **Piece Recognition**: Split 512×512 board → 64 squares → variance check → template matching (SSD)

## Engine Strategy

1. **Terminal check**: Detect checkmate/stalemate before searching
2. **Iterative deepening**: Search to depth 12 (configurable)
3. **PSQT evaluation**: Fast position scoring via piece-square tables

## Configuration

| Mechanism | Controls |
|-----------|----------|
| `--site=` CLI flag | Template set (chesscom, lichess, macOS) |
| `DEBUG_CAPTURE=1` | Save extra screenshot copy |
| `DEBUG_OCR=1` | Save cropped board + individual squares |

## File Layout

```
src/
├── main.rs      # Pipeline orchestration + CLI
├── capture.rs   # Screen capture (xcap)
├── ocr.rs       # Board detection + FEN generation
└── engine.rs    # Move analysis (tanton)

templates/{site}/ # Piece images: K.png, Q.png, ... p.png (12 per site)
screenshots/      # Runtime artifacts (gitignored)
```

## Future Phases

- **Phase 2**: Interactive calibration via `rdev` mouse capture
- **Phase 3**: Parallel OCR with `rayon`
- **Phase 4**: Colored output via `crossterm`, expanded CLI via `clap`
- **Phase 5**: Auto-grid detection, hotkey triggers, move validation
