# ZugzwangRS Architecture Overview

> A pure-Rust chess assistant: screen capture → OCR → engine analysis → terminal output

## Core Pipeline

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   CAPTURE   │───▶│     OCR     │───▶│   ENGINE    │───▶│   OUTPUT    │
│  xcap → PNG │    │ PNG → FEN   │    │ FEN → Move  │    │  Terminal   │
│    <30ms    │    │  40-600ms   │    │   50-100ms  │    │    <1ms     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                         │
              ┌──────────┴──────────┐
              │                     │
        ┌─────┴─────┐         ┌─────┴─────┐
        │  Native   │         │    LLM    │
        │  40-80ms  │         │ 300-600ms │
        └───────────┘         └───────────┘
```

**Total latency**: Native <200ms, LLM <800ms | **Loop interval**: Configurable (default 1000ms)

## Module Responsibilities

| Module | Purpose | Key Function |
|--------|---------|--------------|
| `main.rs` | Async pipeline orchestration | `main()` — CLI args, mode selection, tokio loop |
| `capture.rs` | Full-screen screenshot | `capture_screenshot()` → PNG file |
| `ocr.rs` | OCR facade (mode routing) | `board_to_fen(path, site, mode)` → FEN string |
| `ocr_native.rs` | Template-based recognition | Edge detection + template matching |
| `ocr_llm.rs` | GPT-4o Mini vision API | Base64 encode → API call → FEN extraction |
| `engine.rs` | Position analysis | `analyze_position(fen)` → (move, eval) |

## Data Flow

```
Screen → screenshots/current_board.png → FEN string → (best_move, evaluation) → stdout
         ↑                               ↑                                      ↑
     capture.rs                       ocr.rs                               engine.rs
                                         │
                              ┌──────────┴──────────┐
                              ▼                     ▼
                        ocr_native.rs          ocr_llm.rs
                        (template match)       (GPT-4o Mini)
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Full-screen capture** | No calibration needed; OCR auto-detects board location |
| **File-based handoff** | Simpler debugging; PNG on disk aids inspection |
| **Dual OCR modes** | Native for speed/offline, LLM for accuracy/universality |
| **Async runtime (tokio)** | Non-blocking LLM API calls; efficient I/O |
| **Site-specific templates** | Chess.com/Lichess render pieces differently; matched templates improve accuracy |
| **Tanton over Stockfish** | Pure Rust (no external binary); ~2900 ELO is sufficient |
| **Edge-based board detection** | Chessboards have high edge density; robust across UI variations |
| **Variance-based empty detection** | Fast early-exit before expensive template matching |

## OCR Architecture (3-Layer)

### Layer 1: Facade (`ocr.rs`)
Routes requests to the appropriate implementation based on `OcrMode`:
- `OcrMode::Native` — Template matching (default)
- `OcrMode::Llm` — GPT-4o Mini vision API

### Layer 2: Native OCR (`ocr_native.rs`)
Two-phase recognition pipeline:

1. **Board Detection**: Canny edge detection → grid search → highest edge-density region
2. **Piece Recognition**: Split 512×512 board → 64 squares → variance check → template matching (SSD)

| Stage | Latency |
|-------|---------|
| Edge detection | 10-15ms |
| Template matching | 20-40ms |
| **Total** | **40-80ms** |

### Layer 3: LLM OCR (`ocr_llm.rs`)
Vision API pipeline:

1. **Image Encoding**: Read file → Base64 encode
2. **API Request**: POST to OpenAI with vision prompt
3. **Response Parsing**: Extract FEN from GPT-4o Mini response
4. **Validation**: Verify FEN with shakmaty

| Stage | Latency |
|-------|---------|
| Encoding | 5-10ms |
| API round-trip | 200-400ms |
| Model inference | 100-200ms |
| **Total** | **300-600ms** |

## Engine Strategy

1. **Terminal check**: Detect checkmate/stalemate before searching
2. **Iterative deepening**: Search to depth 12
3. **PSQT evaluation**: Fast position scoring via piece-square tables

## CLI Interface

```bash
# Interactive mode selector (default)
cargo run

# Explicit OCR mode
cargo run -- --ocr=native --site=chesscom
cargo run -- --ocr=llm

# Custom loop interval
cargo run -- --interval=500

# All options
cargo run -- --ocr=native --site=lichess --interval=2000
```

| Flag | Values | Default | Purpose |
|------|--------|---------|---------|
| `--ocr` | `native`, `llm` | Interactive prompt | OCR implementation |
| `--site` | `chesscom`, `lichess`, `macOS` | `chesscom` | Template set (native only) |
| `--interval` | milliseconds | `1000` | Loop timing |

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
├── capture.rs     # Screen capture (xcap)
├── ocr.rs         # OCR facade (mode routing)
├── ocr_native.rs  # Template-based recognition (imageproc)
├── ocr_llm.rs     # GPT-4o Mini vision API (reqwest)
└── engine.rs      # Move analysis (tanton)

templates/{site}/  # Piece images: KW.png, KB.png, ... (12 per site)
screenshots/       # Runtime artifacts (gitignored)
```

## Performance Comparison

| Metric | Native Mode | LLM Mode |
|--------|-------------|----------|
| Latency | 40-80ms | 300-600ms |
| Accuracy | High (with good templates) | Very high |
| Offline | Yes | No |
| Setup | Requires templates | Requires API key |
| Cost | Free | ~$0.001/request |

## Future Phases

- **Phase 2**: Interactive calibration via `rdev` mouse capture
- **Phase 3**: Parallel OCR with `rayon` (native mode speedup)
- **Phase 4**: Colored output via `crossterm`
- **Phase 5**: Auto-grid detection, hotkey triggers, move validation
