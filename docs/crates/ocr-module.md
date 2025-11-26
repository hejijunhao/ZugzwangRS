# OCR Module: Technical Overview

> **Module**: `src/ocr.rs` (facade), `src/ocr_native.rs`, `src/ocr_llm.rs`
> **Purpose**: Convert screenshots of chess games into machine-readable board positions
> **Performance Target**: Native <80ms, LLM 300-600ms

---

## Executive Summary

The OCR (Optical Character Recognition) module is the "eyes" of ZugzwangRS. It takes a raw screenshot—which could contain browser tabs, menus, and other UI elements—and extracts just the chess position from it.

**What it produces**: A FEN string (Forsyth–Edwards Notation), the universal format for describing chess positions. For example:

```
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
```

**Architecture**: The OCR system uses a three-layer design:

1. **Facade Layer** (`ocr.rs`) — Unified interface that routes to the appropriate implementation
2. **Native Layer** (`ocr_native.rs`) — Template-based piece recognition (fast, offline)
3. **LLM Layer** (`ocr_llm.rs`) — GPT-4o Mini vision API (accurate, universal)

---

## Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         main.rs                                      │
│                    (calls ocr::board_to_fen)                        │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    FACADE: ocr.rs                                    │
│                                                                      │
│  pub enum OcrMode { Llm, Native }                                   │
│  pub async fn board_to_fen(path, site, mode) -> Result<String>      │
│                                                                      │
│  Routes based on OcrMode:                                           │
│  - Native → spawn_blocking → ocr_native::board_to_fen()            │
│  - Llm → ocr_llm::board_to_fen().await                             │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
                ▼                               ▼
┌───────────────────────────┐   ┌───────────────────────────┐
│   NATIVE: ocr_native.rs   │   │     LLM: ocr_llm.rs       │
│                           │   │                           │
│  • Edge detection         │   │  • Base64 encode image    │
│  • Template matching      │   │  • Call GPT-4o Mini API   │
│  • Shakmaty validation    │   │  • Parse FEN response     │
│                           │   │  • Shakmaty validation    │
│  Latency: 40-80ms         │   │  Latency: 300-600ms       │
│  Requires: templates/     │   │  Requires: OPENAI_API_KEY │
└───────────────────────────┘   └───────────────────────────┘
```

---

## Layer 1: Facade (`ocr.rs`)

**Location**: `src/ocr.rs` (62 lines)

The facade provides a unified async interface for board-to-FEN conversion, abstracting away the implementation choice.

### OcrMode Enum

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OcrMode {
    /// GPT-4o Mini vision API
    Llm,
    /// Template-based matching (default for backward compatibility)
    #[default]
    Native,
}
```

| Mode | Display Name | Default | Requirements |
|------|--------------|---------|--------------|
| `Native` | "Native (template matching)" | ✅ Yes | `templates/{site}/` directory |
| `Llm` | "LLM (GPT-4o Mini)" | No | `OPENAI_API_KEY` env var |

### Public Functions

| Function | Purpose | Signature |
|----------|---------|-----------|
| `board_to_fen()` | Main entry point | `async fn board_to_fen(image_path: &str, site: &str, mode: OcrMode) -> Result<String>` |
| `llm_available()` | Check if LLM mode can be used | `fn llm_available() -> bool` |

### Async Compatibility

The facade handles the async/sync mismatch between modes:

```rust
pub async fn board_to_fen(image_path: &str, site: &str, mode: OcrMode) -> Result<String> {
    match mode {
        OcrMode::Llm => crate::ocr_llm::board_to_fen(image_path).await,
        OcrMode::Native => {
            // Native is synchronous, wrap for async compatibility
            tokio::task::spawn_blocking(move || crate::ocr_native::board_to_fen(&path, &site))
                .await?
        }
    }
}
```

**Why `spawn_blocking`?** The native OCR performs CPU-intensive image processing. Running it directly in an async context would block the Tokio runtime. `spawn_blocking` moves the work to a dedicated thread pool.

---

## Layer 2: Native OCR (`ocr_native.rs`)

**Location**: `src/ocr_native.rs` (339 lines)

Pure-Rust template-based OCR using `imageproc` for edge detection and template matching.

### Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│  SCREENSHOT (full screen with browser, menus, etc.)                 │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 1: Board Detection (screenshot_to_board)                     │
│  • Convert to grayscale                                             │
│  • Canny edge detection (50.0, 150.0 thresholds)                   │
│  • Generate candidate regions (grid search)                         │
│  • Score by edge density (chessboards have high edge count)        │
│  • Select best candidate, validate minimum 1% edge density         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 2: Crop & Normalize                                          │
│  • Extract detected region                                          │
│  • Validate: minimum 64x64px, ~square aspect ratio (±10%)          │
│  • Resize to exactly 512×512 pixels (Lanczos3 filter)              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 3: Template Loading (load_templates)                         │
│  • Load 12 piece templates from templates/{site}/                   │
│  • Naming: {Piece}{Color}.png (KW.png, KB.png, etc.)               │
│  • Convert to grayscale for matching                                │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 4: Grid Splitting (split_into_squares)                       │
│  • Divide 512×512 board into 8×8 grid                              │
│  • Each square: 64×64 pixels, grayscale                            │
│  • Row 0 = Rank 8 (Black's back rank)                              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 5: Piece Recognition (match_square × 64)                     │
│  Phase 1: Variance check (fast empty detection)                     │
│    • Calculate pixel variance                                       │
│    • If variance < 100.0 → empty square (return '1')               │
│  Phase 2: Template matching (if variance high enough)               │
│    • Compare against all 12 templates using SSD Normalized         │
│    • Best match below 0.3 threshold → return piece char            │
│    • No confident match → empty square                              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 6: FEN Generation (build_fen_string)                         │
│  • Convert 8×8 char grid to FEN notation                           │
│  • Run-length encode empty squares (e.g., "111P1111" → "3P4")      │
│  • Append game state: " w KQkq - 0 1"                              │
│  • Validate with shakmaty::fen::Fen::from_ascii()                  │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OUTPUT: FEN STRING                                                  │
│  "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"     │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Functions

#### `screenshot_to_board()`

**Location**: `src/ocr_native.rs:18-136`

Detects and extracts the chessboard from a full screenshot.

```rust
pub fn screenshot_to_board(image_path: &str) -> Result<DynamicImage>
```

**Internal helpers**:

| Function | Purpose | Location |
|----------|---------|----------|
| `find_board_region()` | Orchestrates edge-based detection | Lines 25-59 |
| `generate_candidate_regions()` | Creates search grid | Lines 64-88 |
| `calculate_edge_density()` | Scores region by edge pixels | Lines 93-108 |

**Search parameters**:
- Minimum board size: 200 pixels
- Size increment: 100 pixels
- Position overlap: 75% (ensures thorough coverage)
- Minimum edge density: 1%

#### `load_templates()`

**Location**: `src/ocr_native.rs:146-177`

Loads the 12 piece templates for the specified chess site.

**Template naming convention** (case-insensitive filesystem safe):

| FEN Char | Filename | Piece |
|----------|----------|-------|
| `K` | `KW.png` | White King |
| `Q` | `QW.png` | White Queen |
| `R` | `RW.png` | White Rook |
| `B` | `BW.png` | White Bishop |
| `N` | `NW.png` | White Knight |
| `P` | `PW.png` | White Pawn |
| `k` | `KB.png` | Black King |
| `q` | `QB.png` | Black Queen |
| `r` | `RB.png` | Black Rook |
| `b` | `BB.png` | Black Bishop |
| `n` | `NB.png` | Black Knight |
| `p` | `PB.png` | Black Pawn |

**Why `{Piece}{Color}.png` instead of `K.png`/`k.png`?** macOS uses a case-insensitive filesystem by default, so `K.png` and `k.png` would collide.

#### `match_square()`

**Location**: `src/ocr_native.rs:202-256`

Two-phase piece detection for a single 64×64 pixel square.

**Phase 1: Empty detection (fast path)**
```rust
const EMPTY_VARIANCE_THRESHOLD: f32 = 100.0;
if variance < EMPTY_VARIANCE_THRESHOLD {
    return '1';  // Empty square
}
```

Low pixel variance indicates uniform color (empty square). This skips expensive template matching for ~half the squares.

**Phase 2: Template matching**
```rust
const MATCH_THRESHOLD: f32 = 0.3;
// Uses SumOfSquaredErrorsNormalized
// Score 0.0 = perfect match, 1.0 = no similarity
```

Compares against all 12 templates, returns best match if score < 0.3.

### Template Directory Structure

```
templates/
├── chesscom/          ← Chess.com piece style
│   ├── KW.png         ← White King
│   ├── QW.png         ← White Queen
│   ├── RW.png         ← White Rook
│   ├── BW.png         ← White Bishop
│   ├── NW.png         ← White Knight
│   ├── PW.png         ← White Pawn
│   ├── KB.png         ← Black King
│   ├── QB.png         ← Black Queen
│   ├── RB.png         ← Black Rook
│   ├── BB.png         ← Black Bishop
│   ├── NB.png         ← Black Knight
│   └── PB.png         ← Black Pawn
├── lichess/           ← Lichess piece style
│   └── (same 12 files)
└── macOS/             ← macOS Chess.app style
    └── (same 12 files)
```

---

## Layer 3: LLM OCR (`ocr_llm.rs`)

**Location**: `src/ocr_llm.rs` (245 lines)

Uses OpenAI's GPT-4o Mini vision model to analyze chess board images directly.

### Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│  SCREENSHOT FILE                                                     │
│  screenshots/current_board.png                                       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 1: Image Encoding                                              │
│  • Read file bytes                                                   │
│  • Base64 encode                                                     │
│  • Embed as data:image/png;base64,{data}                           │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 2: API Request                                                 │
│  • Build ChatRequest with vision prompt                             │
│  • POST to https://api.openai.com/v1/chat/completions              │
│  • Model: gpt-4o-mini                                               │
│  • Detail: "low" (faster processing)                                │
│  • Max tokens: 100                                                   │
│  • Timeout: 15 seconds                                               │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 3: Retry Logic                                                 │
│  • Up to 3 attempts total (1 initial + 2 retries)                  │
│  • 500ms delay between retries                                       │
│  • Logs failures to stderr                                          │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 4: Response Parsing                                            │
│  • Extract content from choices[0].message.content                  │
│  • Trim whitespace                                                   │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 5: FEN Validation                                              │
│  • Parse with shakmaty::fen::Fen::from_ascii()                     │
│  • Reject invalid FEN responses                                      │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OUTPUT: FEN STRING                                                  │
│  "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"     │
└─────────────────────────────────────────────────────────────────────┘
```

### Configuration Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `API_URL` | `https://api.openai.com/v1/chat/completions` | OpenAI endpoint |
| `MODEL` | `gpt-4o-mini` | Vision-capable model |
| `MAX_RETRIES` | `2` | Additional attempts after first failure |
| `TIMEOUT_SECS` | `15` | Request timeout |

### The Vision Prompt

**Location**: `src/ocr_llm.rs:102-116`

```
Analyze this chessboard image. Output ONLY the FEN string.

Rules:
- Output ONLY the FEN, nothing else (no explanation, no markdown, no quotes)
- White pieces are at the bottom of the image
- Use standard FEN: uppercase = White (KQRBNP), lowercase = Black (kqrbnp)
- Numbers represent consecutive empty squares
- Rows separated by /
- Append: w KQkq - 0 1

Example output:
rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1
```

**Design choices**:
- **"Output ONLY"**: Prevents markdown formatting or explanations
- **"White at bottom"**: Establishes consistent orientation
- **Example output**: Demonstrates exact expected format
- **"low" detail mode**: Faster API response, sufficient for chess recognition

### Key Functions

| Function | Purpose | Location |
|----------|---------|----------|
| `has_api_key()` | Check if `OPENAI_API_KEY` is set | Line 67-69 |
| `board_to_fen()` | Main async entry point | Lines 72-97 |
| `build_prompt()` | Construct vision prompt | Lines 102-116 |
| `build_request()` | Create ChatRequest structure | Lines 118-137 |
| `call_api_with_retry()` | HTTP call with retry logic | Lines 139-166 |
| `call_api()` | Single API request | Lines 168-196 |
| `validate_fen()` | Shakmaty FEN validation | Lines 198-202 |

### Request/Response Types

```rust
// Request structure
struct ChatRequest {
    model: String,           // "gpt-4o-mini"
    messages: Vec<ChatMessage>,
    max_tokens: u32,         // 100
}

struct ChatMessage {
    role: String,            // "user"
    content: Vec<ContentPart>,
}

enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrlDetail },
}

struct ImageUrlDetail {
    url: String,             // "data:image/png;base64,..."
    detail: String,          // "low"
}
```

---

## Mode Comparison

| Aspect | Native | LLM |
|--------|--------|-----|
| **Latency** | 40-80ms | 300-600ms |
| **Accuracy** | High (with good templates) | Very High |
| **Offline** | ✅ Yes | ❌ No |
| **Setup** | Requires templates | Requires API key |
| **Cost** | Free | ~$0.001/request |
| **Piece styles** | Must match templates | Works with any style |
| **Network** | Not required | Required |

**When to use Native**:
- Real-time analysis during games (low latency critical)
- Offline usage
- Cost-sensitive applications
- Known chess site with existing templates

**When to use LLM**:
- Unknown piece styles
- Maximum accuracy needed
- No templates available
- Latency is acceptable

---

## Key Concepts Explained

### What is FEN?

FEN (Forsyth–Edwards Notation) is chess's universal position format:

```
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
└──────────────────────────────────────────┘ │ └──┘ │ │ │
         Piece positions (8 ranks)           │   │   │ │ └─ Move number
                                             │   │   │ └─── Half-move clock
                                    Who moves┘   │   └───── En passant square
                                     (w=white)   └───────── Castling rights
```

**Piece notation**:
- Uppercase = White pieces (K, Q, R, B, N, P)
- Lowercase = Black pieces (k, q, r, b, n, p)
- Numbers = consecutive empty squares

### What is Canny Edge Detection?

Canny edge detection finds boundaries in images where colors change sharply. Chessboards have many edges:
- Grid lines between squares
- Piece outlines
- Contrast between light/dark squares

The algorithm uses two thresholds (50.0, 150.0 in our implementation):
- Low threshold: Weak edges (included if connected to strong edges)
- High threshold: Strong edges (always included)

### What is Template Matching?

Template matching compares a small image (template) against regions of a larger image to find the best match.

**SSD Normalized (Sum of Squared Differences)**:
- Computes pixel-by-pixel difference
- Normalizes to 0.0–1.0 range
- 0.0 = perfect match
- 1.0 = completely different

---

## Configuration & Tuning

### Environment Variables

| Variable | Effect | Example |
|----------|--------|---------|
| `DEBUG_OCR` | Saves intermediate images | `DEBUG_OCR=1 cargo run` |
| `OPENAI_API_KEY` | Enables LLM mode | `OPENAI_API_KEY=sk-... cargo run` |

**Debug outputs** (when `DEBUG_OCR=1`):
- `screenshots/debug_cropped_board.png` — Detected board region
- `screenshots/ocr_debug/square_R_F.png` — Individual squares (R=rank, F=file)

### Tunable Thresholds (Native)

| Constant | Location | Default | Purpose |
|----------|----------|---------|---------|
| `MIN_EDGE_DENSITY` | ocr_native.rs:48 | 0.01 (1%) | Minimum edge coverage to accept a board |
| `EMPTY_VARIANCE_THRESHOLD` | ocr_native.rs:213 | 100.0 | Below this = empty square |
| `MATCH_THRESHOLD` | ocr_native.rs:224 | 0.3 | Below this = confident piece match |

**Tuning guidance**:
- Empty squares detected as pieces → Lower `EMPTY_VARIANCE_THRESHOLD`
- Pieces not detected → Raise `MATCH_THRESHOLD`
- Wrong board region detected → Adjust `MIN_EDGE_DENSITY`

### Tunable Constants (LLM)

| Constant | Location | Default | Purpose |
|----------|----------|---------|---------|
| `MAX_RETRIES` | ocr_llm.rs:15 | 2 | Retry attempts after failure |
| `TIMEOUT_SECS` | ocr_llm.rs:16 | 15 | Request timeout |

---

## Performance Characteristics

### Native Mode

| Stage | Typical Time | Notes |
|-------|--------------|-------|
| Image load | 5-10ms | Depends on file size |
| Edge detection | 10-15ms | O(pixels) |
| Candidate search | 5-10ms | Depends on screen size |
| Grid splitting | 2-5ms | Fixed 64 operations |
| Template matching | 20-40ms | 64 squares × 12 templates |
| FEN building | <1ms | String operations |
| **Total** | **40-80ms** | Within target |

### LLM Mode

| Stage | Typical Time | Notes |
|-------|--------------|-------|
| Image encoding | 5-10ms | Base64 encoding |
| Network latency | 200-400ms | API round-trip |
| Model processing | 100-200ms | GPT-4o Mini inference |
| FEN validation | <1ms | Shakmaty parsing |
| **Total** | **300-600ms** | Network dependent |

---

## Tests

### Facade Tests (`ocr.rs`)

```rust
#[test]
fn test_ocr_mode_display()     // Verify Display impl
fn test_ocr_mode_default()     // Verify Native is default
```

### LLM Tests (`ocr_llm.rs`)

```rust
#[test]
fn test_build_prompt_contains_rules()  // Prompt has required instructions
fn test_validate_fen_accepts_valid()   // Valid FEN passes
fn test_validate_fen_rejects_invalid() // Invalid FEN rejected
fn test_has_api_key_without_key()      // Doesn't panic without key

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
fn test_real_api_call()                // Integration test with real API
```

**Running ignored tests**:
```bash
OPENAI_API_KEY=sk-... cargo test test_real_api_call -- --ignored
```

---

## Limitations & Future Improvements

### Current Limitations

| Limitation | Impact | Potential Fix |
|------------|--------|---------------|
| Assumes white to move | Cannot detect whose turn | Move history analysis |
| Assumes all castling rights | May be incorrect mid-game | Track position changes |
| No board orientation detection | Assumes white at bottom | Detect via king positions |
| Native requires exact templates | Fails with new piece styles | Add more template sets |
| LLM requires network | Offline unusable | Cache common positions? |
| Single-threaded native matching | Slower than possible | Add `rayon` parallelization |

### Possible Enhancements

| Enhancement | Benefit | Complexity |
|-------------|---------|------------|
| Rayon parallel matching | 2-4× faster native OCR | Low |
| Template auto-generation | Eliminate manual template creation | High |
| Board orientation detection | Support flipped boards | Medium |
| Move detection | Identify whose turn it is | High |
| Hybrid mode | Use LLM to verify native results | Medium |
| Position caching | Skip OCR for unchanged boards | Medium |

---

## Summary

The OCR module provides flexible board-to-FEN conversion through a three-layer architecture:

1. **Facade (`ocr.rs`)**: Unified async interface with mode selection
2. **Native (`ocr_native.rs`)**: Fast, offline template matching (~50ms)
3. **LLM (`ocr_llm.rs`)**: Accurate, universal GPT-4o Mini vision (~400ms)

Both implementations validate output via `shakmaty` to ensure legal FEN strings.

### Quick Reference

```bash
# Run with native OCR
cargo run -- --ocr=native --site=chesscom

# Run with LLM OCR
OPENAI_API_KEY=sk-... cargo run -- --ocr=llm

# Debug native OCR
DEBUG_OCR=1 cargo run -- --ocr=native
```

### Key Files

| File | Purpose |
|------|---------|
| `src/ocr.rs` | Facade and mode selection |
| `src/ocr_native.rs` | Template-based recognition |
| `src/ocr_llm.rs` | GPT-4o Mini vision API |
| `templates/{site}/` | Piece template images |
