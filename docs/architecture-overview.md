# ZugzwangRS Architecture Overview

> A pure-Rust chess assistant: screen capture → analysis → terminal output

## Core Pipeline

ZugzwangRS supports two analysis modes with different pipelines:

### Engine Mode (Traditional)
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   CAPTURE   │───▶│     OCR     │───▶│   ENGINE    │───▶│   OUTPUT    │
│  xcap → JPG │    │ JPG → FEN   │    │ FEN → Move  │    │  Terminal   │
│    <30ms    │    │  40-2000ms  │    │   50-100ms  │    │    <1ms     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                         │
              ┌──────────┴──────────┐
              │                     │
        ┌─────┴─────┐         ┌─────┴─────┐
        │  Native   │         │    LLM    │
        │  40-80ms  │         │ 500-2000ms│
        └───────────┘         └───────────┘
```

### Direct Mode (LLM-only)
```
┌─────────────┐    ┌─────────────────────────────────┐    ┌─────────────┐
│   CAPTURE   │───▶│         DIRECT ANALYSIS         │───▶│   OUTPUT    │
│  xcap → JPG │    │  GPT-4o sees board → decides    │    │  Terminal   │
│    <30ms    │    │  move with explanation          │    │  + Why      │
│             │    │         500-2000ms              │    │             │
└─────────────┘    └─────────────────────────────────┘    └─────────────┘
```

**Total latency**: Engine+Native <200ms, Engine+LLM <2.5s, Direct <2.5s | **Loop interval**: Configurable (default 1000ms)

## Module Responsibilities

| Module | Purpose | Key Function |
|--------|---------|--------------|
| `main.rs` | Async pipeline orchestration | `main()` — CLI args, mode selection, tokio loop |
| `capture.rs` | Full-screen screenshot | `capture_screenshot()` → JPG file |
| `ocr.rs` | OCR facade (mode routing) | `board_to_fen(path, site, mode, side)` → FEN string |
| `ocr_native.rs` | Template-based recognition | Edge detection + template matching |
| `ocr_llm.rs` | GPT-4o vision API | `board_to_fen()` → FEN, `analyze_board()` → Move + reasoning |
| `engine.rs` | Position analysis | `analyze_position(fen)` → (move, eval) |

## Data Flow

### Engine Mode
```
Screen → screenshots/current_board.jpg → FEN string → (best_move, evaluation) → stdout
         ↑                               ↑                                      ↑
     capture.rs                       ocr.rs                               engine.rs
                                         │
                              ┌──────────┴──────────┐
                              ▼                     ▼
                        ocr_native.rs          ocr_llm.rs
                        (template match)       (GPT-4o vision)
```

### Direct Mode
```
Screen → screenshots/current_board.jpg → MoveRecommendation → stdout
         ↑                               ↑                     ↑
     capture.rs                    ocr_llm.rs            (move, eval, why)
                                 analyze_board()
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Full-screen capture** | No calibration needed; OCR auto-detects board location |
| **File-based handoff** | Simpler debugging; JPG on disk aids inspection |
| **Dual OCR modes** | Native for speed/offline, LLM for accuracy/universality |
| **Dual analysis modes** | Engine for strength (~2900 ELO), Direct for explanations |
| **Async runtime (tokio)** | Non-blocking LLM API calls; efficient I/O |
| **Site-specific templates** | Chess.com/Lichess render pieces differently; matched templates improve accuracy |
| **Tanton over Stockfish** | Pure Rust (no external binary); ~2900 ELO is sufficient |
| **Edge-based board detection** | Chessboards have high edge density; robust across UI variations |
| **Variance-based empty detection** | Fast early-exit before expensive template matching |
| **JPEG over PNG** | 40× faster encoding on high-DPI displays; sufficient quality for OCR |

## OCR Architecture (3-Layer)

### Layer 1: Facade (`ocr.rs`)
Routes requests to the appropriate implementation based on `OcrMode`:
- `OcrMode::Native` — Template matching (default)
- `OcrMode::Llm` — GPT-4o vision API

### Layer 2: Native OCR (`ocr_native.rs`)
Two-phase recognition pipeline:

1. **Board Detection**: Canny edge detection → grid search → highest edge-density region
2. **Piece Recognition**: Split 512×512 board → 64 squares → variance check → template matching (SSD)

| Stage | Latency |
|-------|---------|
| Edge detection | 10-15ms |
| Template matching | 20-40ms |
| **Total** | **40-80ms** |

### Layer 3: LLM Module (`ocr_llm.rs`)
Provides two functions:

#### FEN OCR (`board_to_fen`)
Vision API pipeline for position extraction:

1. **Image Encoding**: Read file → Base64 encode
2. **API Request**: POST to OpenAI with FEN extraction prompt
3. **Response Parsing**: Extract FEN from GPT-4o response
4. **Validation**: King/pawn count checks, castling rights fix, shakmaty verification

| Stage | Latency |
|-------|---------|
| Encoding | 5-10ms |
| API round-trip | 300-500ms |
| Model inference | 200-500ms |
| Validation + retry | 0-1000ms |
| **Total** | **500-2000ms** |

#### Direct Analysis (`analyze_board`)
Single-shot move recommendation:

1. **Image Encoding**: Read file → Base64 encode
2. **API Request**: POST with grandmaster analysis prompt
3. **Response Parsing**: Extract structured `MOVE:/EVAL:/WHY:` response
4. **Return**: `MoveRecommendation { best_move, evaluation, reasoning }`

| Stage | Latency |
|-------|---------|
| Encoding | 5-10ms |
| API round-trip + inference | 500-1500ms |
| **Total** | **500-2000ms** |

## Engine Strategy

1. **Terminal check**: Detect checkmate/stalemate before searching
2. **Iterative deepening**: Search to depth 12
3. **PSQT evaluation**: Fast position scoring via piece-square tables

## CLI Interface

```bash
# Interactive mode selector (default)
cargo run

# Explicit modes
cargo run -- --ocr=native --site=chesscom
cargo run -- --ocr=llm --analysis=engine
cargo run -- --ocr=llm --analysis=direct    # GPT-4o decides move directly

# Player side (for correct board orientation)
cargo run -- --side=white                    # Default
cargo run -- --side=black                    # Your pieces at bottom

# Trigger mode
cargo run -- --trigger=auto                  # Continuous (default)
cargo run -- --trigger=manual                # Press Enter to capture

# All options combined
cargo run -- --ocr=llm --analysis=direct --side=black --trigger=manual --interval=2000 -v
```

| Flag | Values | Default | Purpose |
|------|--------|---------|---------|
| `--ocr` | `native`, `llm` | Interactive prompt | OCR implementation |
| `--analysis` | `engine`, `direct` | Interactive (LLM only) | How moves are analyzed |
| `--site` | `chesscom`, `lichess`, `macOS` | `chesscom` | Template set (native only) |
| `--side` | `white`, `black` | Interactive prompt | Which side you're playing |
| `--trigger` | `auto`, `manual` | Interactive prompt | Capture timing |
| `--interval` | milliseconds | `1000` | Loop timing (auto mode) |
| `-v`, `--verbose` | flag | off | Show timing breakdown |

## Environment Variables

| Variable | Effect |
|----------|--------|
| `OPENAI_API_KEY` | Enables LLM OCR mode |
| `DEBUG_CAPTURE=1` | Save extra screenshot copy |
| `DEBUG_OCR=1` | Save cropped board + individual squares |

## File Layout

```
src/
├── main.rs        # Async pipeline orchestration + CLI (tokio)
│                  # Defines: AnalysisMode, PlayerSide enums
├── capture.rs     # Screen capture (xcap) → screenshots/current_board.jpg
├── ocr.rs         # OCR facade (mode routing)
├── ocr_native.rs  # Template-based recognition (imageproc)
├── ocr_llm.rs     # GPT-4o vision API (reqwest)
│                  # Exports: board_to_fen(), analyze_board(), MoveRecommendation
└── engine.rs      # Move analysis (tanton)

templates/{site}/  # Piece images: KW.png, KB.png, ... (12 per site)
screenshots/       # Runtime artifacts (gitignored)
```

## Performance Comparison

### OCR Modes
| Metric | Native OCR | LLM OCR |
|--------|------------|---------|
| Latency | 40-80ms | 500-2000ms |
| Accuracy | High (with good templates) | Very high (~95%) |
| Offline | Yes | No |
| Setup | Requires templates | Requires API key |
| Cost | Free | ~$0.01/request |

### Analysis Modes
| Metric | Engine (Tanton) | Direct (GPT-4o) |
|--------|-----------------|-----------------|
| Strength | ~2900 ELO | ~1800-2000 ELO |
| Latency | 50-100ms | 500-2000ms |
| Explanation | None | Yes (reasoning) |
| Offline | Yes (with Native OCR) | No |
| Best for | Competitive play | Learning |

### Total Pipeline Latency
| Configuration | Typical Latency |
|---------------|-----------------|
| Native OCR + Engine | <200ms |
| LLM OCR + Engine | 1-3 seconds |
| Direct (LLM only) | 1-2.5 seconds |

## Future Phases

- **Phase 2**: Interactive calibration via `rdev` mouse capture
- **Phase 3**: Parallel OCR with `rayon` (native mode speedup)
- **Phase 4**: Colored output via `crossterm`
- **Phase 5**: Auto-grid detection, hotkey triggers, move validation
