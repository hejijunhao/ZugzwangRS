# OCR Module: Technical Overview

> **Module**: `src/ocr.rs`
> **Purpose**: Convert screenshots of chess games into machine-readable board positions
> **Performance Target**: 40–80ms per frame

---

## Executive Summary

The OCR (Optical Character Recognition) module is the "eyes" of ZugzwangRS. It takes a raw screenshot—which could contain browser tabs, menus, and other UI elements—and extracts just the chess position from it.

**What it produces**: A FEN string (Forsyth–Edwards Notation), the universal format for describing chess positions. For example:

```
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
```

This string tells the chess engine exactly where every piece is on the board.

---

## How It Works: The Pipeline

The module processes images through a 5-stage pipeline:

```
┌─────────────────────────────────────────────────────────────────────┐
│  SCREENSHOT                                                         │
│  (Full screen with browser, menus, etc.)                           │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 1: Board Detection                                           │
│  "Where is the chessboard in this image?"                          │
│  Uses edge detection to find the region with most grid-like        │
│  patterns (chess pieces and squares create many edges)             │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 2: Crop & Normalize                                          │
│  Extract the board region and resize to exactly 512×512 pixels     │
│  This ensures each square is exactly 64×64 pixels                  │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 3: Grid Splitting                                            │
│  Divide the 512×512 board into 64 individual square images         │
│  (8 rows × 8 columns = 64 squares of 64×64 pixels each)           │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 4: Piece Recognition                                         │
│  For each square: "Is this empty, or which piece is here?"         │
│  Compare against reference images (templates) of each piece type   │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 5: FEN Generation                                            │
│  Convert the 8×8 grid of identified pieces into FEN notation       │
│  Validate the result is a legal chess position                     │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OUTPUT: FEN STRING                                                 │
│  "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Concepts Explained

### What is FEN?

FEN (Forsyth–Edwards Notation) is chess's universal position format. It's a single line of text that completely describes:

- Where every piece is located
- Whose turn it is to move
- Which castling rights remain
- En passant possibilities

**Example breakdown**:
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
- Numbers = consecutive empty squares (e.g., "8" = entire empty rank)

### What is Template Matching?

Template matching is like playing a game of "which reference image looks most like this square?"

For each of the 64 squares on the board, we compare it against 12 reference images (templates):
- 6 white pieces: King, Queen, Rook, Bishop, Knight, Pawn
- 6 black pieces: king, queen, rook, bishop, knight, pawn

The template with the lowest "difference score" wins. If no template is similar enough, the square is considered empty.

### What is Edge Detection?

Edge detection finds boundaries in images—places where colors change sharply. A chessboard has many edges:
- The grid lines between squares
- The outlines of chess pieces
- The contrast between light and dark squares

By finding the region with the highest concentration of edges, we can locate the chessboard within a cluttered screenshot.

---

## Deep Dive: Function Reference

### Public Functions

The module exposes two public functions:

| Function | Purpose | Input | Output |
|----------|---------|-------|--------|
| `screenshot_to_board()` | Find and extract the chessboard | Image file path | 512×512 board image |
| `board_to_fen()` | Full pipeline: screenshot → FEN | Image file path | FEN string |

---

### `screenshot_to_board()`

**Location**: `src/ocr.rs:19–132`

**Purpose**: Automatically locate and extract the chessboard from any screenshot.

**Why it matters**: Users shouldn't need to manually crop their screenshots. This function finds the board automatically, regardless of where it appears on screen.

#### Algorithm Overview

```
Input: Full screenshot (any size)
  │
  ├─► Convert to grayscale
  │
  ├─► Run Canny edge detection
  │     (highlights boundaries between colors)
  │
  ├─► Generate candidate regions
  │     (try different positions and sizes)
  │
  ├─► Score each candidate by "edge density"
  │     (chessboards have lots of edges)
  │
  ├─► Select region with highest score
  │
  ├─► Validate: Must be square-ish, at least 64×64px
  │
  └─► Resize to exactly 512×512 pixels

Output: Normalized board image
```

#### Sub-function: `find_board_region()`

**Location**: `src/ocr.rs:26–60`

Orchestrates the board detection process using edge analysis.

```rust
fn find_board_region(img: &DynamicImage) -> Result<(u32, u32, u32, u32)>
// Returns: (x, y, width, height) of detected board
```

**Key parameters**:
- `MIN_EDGE_DENSITY = 0.01` (1%) — Minimum edge coverage to be considered a valid board

#### Sub-function: `generate_candidate_regions()`

**Location**: `src/ocr.rs:65–89`

Creates a search grid of potential board locations to evaluate.

```rust
fn generate_candidate_regions(width: u32, height: u32) -> Vec<(u32, u32, u32)>
// Returns: List of (x, y, size) tuples to check
```

**Search parameters**:
- Minimum board size: 200 pixels
- Size increment: 100 pixels
- Position overlap: 75% (ensures we don't miss boards between grid points)

#### Sub-function: `calculate_edge_density()`

**Location**: `src/ocr.rs:94–109`

Measures what percentage of a region contains edges (potential board features).

```rust
fn calculate_edge_density(edges: &GrayImage, x: u32, y: u32, size: u32) -> f32
// Returns: 0.0 to 1.0 (percentage of edge pixels)
```

**How it works**: Counts bright pixels (value > 128) in the edge-detected image. A higher count suggests more visual complexity—likely a chessboard.

---

### `board_to_fen()`

**Location**: `src/ocr.rs:137–318`

**Purpose**: The main entry point. Takes a screenshot path and returns a validated FEN string.

**Why it matters**: This single function call handles the entire OCR pipeline, making integration simple.

#### Execution Flow

```rust
pub fn board_to_fen(image_path: &str) -> Result<String>
```

```
1. screenshot_to_board()     →  Get normalized 512×512 board
2. load_templates()          →  Load reference piece images
3. split_into_squares()      →  Divide into 64 square images
4. match_square() × 64       →  Identify each piece
5. build_fen_string()        →  Encode as FEN
6. shakmaty validation       →  Verify legal position
```

---

### Internal Functions (within `board_to_fen`)

#### `load_templates()`

**Location**: `src/ocr.rs:150–167`

Loads the 12 reference piece images used for matching.

```rust
fn load_templates(site: &str) -> Result<PieceTemplates>
```

**Template organization**:
```
templates/
└── chesscom/          ← Site-specific folder
    ├── K.png          ← White King
    ├── Q.png          ← White Queen
    ├── R.png          ← White Rook
    ├── B.png          ← White Bishop
    ├── N.png          ← White Knight
    ├── P.png          ← White Pawn
    ├── k.png          ← Black king
    ├── q.png          ← Black queen
    ├── r.png          ← Black rook
    ├── b.png          ← Black bishop
    ├── n.png          ← Black knight
    └── p.png          ← Black pawn
```

**Why site-specific?** Chess.com, Lichess, and macOS Chess.app all render pieces differently. Using site-specific templates improves accuracy.

---

#### `split_into_squares()`

**Location**: `src/ocr.rs:171–188`

Divides the 512×512 board into 64 individual 64×64 pixel squares.

```rust
fn split_into_squares(board: &RgbaImage) -> Vec<Vec<GrayImage>>
// Returns: 8×8 grid of grayscale square images
```

**Grid layout** (matches FEN order):
```
Row 0: a8 b8 c8 d8 e8 f8 g8 h8  ← Rank 8 (Black's back rank)
Row 1: a7 b7 c7 d7 e7 f7 g7 h7  ← Rank 7
  ...
Row 7: a1 b1 c1 d1 e1 f1 g1 h1  ← Rank 1 (White's back rank)
```

**Optimization**: Converts to grayscale once upfront, rather than per-square.

---

#### `match_square()`

**Location**: `src/ocr.rs:192–246`

The core recognition function. Determines what piece (if any) occupies a square.

```rust
fn match_square(square: &GrayImage, templates: &PieceTemplates) -> char
// Returns: 'K', 'Q', 'R', 'B', 'N', 'P' (white)
//          'k', 'q', 'r', 'b', 'n', 'p' (black)
//          '1' (empty)
```

**Two-phase detection**:

**Phase 1: Quick empty-square check** (lines 195–206)
```rust
// Calculate pixel variance
// Low variance = uniform color = empty square
const EMPTY_VARIANCE_THRESHOLD: f32 = 100.0;
if variance < EMPTY_VARIANCE_THRESHOLD {
    return '1';  // Empty
}
```

This fast check skips expensive template matching for obviously empty squares.

**Phase 2: Template matching** (lines 208–245)
```rust
// Compare against all 12 piece templates
// Using Sum of Squared Differences (Normalized)
// Lower score = better match
const MATCH_THRESHOLD: f32 = 0.3;
```

**Matching algorithm**: `SumOfSquaredErrorsNormalized`
- Computes pixel-by-pixel difference between square and template
- Normalizes to 0.0–1.0 range (0.0 = identical, 1.0 = completely different)
- Best match under threshold wins; otherwise square is empty

---

#### `build_fen_string()`

**Location**: `src/ocr.rs:251–290`

Converts the 8×8 character grid into proper FEN notation.

```rust
fn build_fen_string(board: [[char; 8]; 8]) -> Result<String>
```

**FEN encoding rules applied**:

1. **Run-length encoding for empty squares**:
   ```
   ['1', '1', '1', 'P', '1', '1', '1', '1']  →  "3P4"
   ```

2. **Rank separator**: `/` between each row

3. **Game state suffix**: ` w KQkq - 0 1`
   - `w` = White to move (assumed for MVP)
   - `KQkq` = All castling rights available (assumed)
   - `-` = No en passant square
   - `0 1` = Move counters

4. **Validation**: Uses `shakmaty` crate to verify the FEN represents a legal chess position

---

## Configuration & Tuning

### Environment Variables

| Variable | Effect | Example |
|----------|--------|---------|
| `DEBUG_OCR` | Saves intermediate images for inspection | `DEBUG_OCR=1 cargo run` |

**Debug outputs** (when `DEBUG_OCR=1`):
- `screenshots/debug_cropped_board.png` — The detected board region
- `screenshots/ocr_debug/square_R_F.png` — Individual squares (R=rank, F=file)

### Tunable Thresholds

| Constant | Location | Default | Purpose |
|----------|----------|---------|---------|
| `MIN_EDGE_DENSITY` | Line 49 | 0.01 (1%) | Minimum edge coverage to accept a board |
| `EMPTY_VARIANCE_THRESHOLD` | Line 203 | 100.0 | Below this = empty square |
| `MATCH_THRESHOLD` | Line 214 | 0.3 | Below this = confident piece match |

**Tuning guidance**:
- If empty squares are detected as pieces → Lower `EMPTY_VARIANCE_THRESHOLD`
- If pieces aren't detected → Raise `MATCH_THRESHOLD`
- If wrong board region detected → Adjust `MIN_EDGE_DENSITY`

---

## Limitations & Future Improvements

### Current Limitations

| Limitation | Impact | Planned Fix |
|------------|--------|-------------|
| Assumes white to move | Cannot detect whose turn it is | Would require move history analysis |
| Hardcoded site ("chesscom") | Must manually change for other sites | Add CLI argument `--site=` |
| No board orientation detection | Assumes white at bottom | Detect via king positions |
| Single-threaded matching | Slower than possible | Add `rayon` parallelization |

### Accuracy Factors

Recognition accuracy depends on:

1. **Template quality** — Templates should match the exact piece style being played
2. **Screen resolution** — Higher resolution = better matching
3. **Board size on screen** — Minimum 200×200 pixels recommended
4. **Anti-aliasing** — Heavy smoothing can reduce edge clarity

### Performance Characteristics

| Stage | Typical Time | Notes |
|-------|--------------|-------|
| Edge detection | 10–15ms | O(pixels) |
| Candidate search | 5–10ms | Depends on screen size |
| Grid splitting | 2–5ms | Fixed 64 operations |
| Template matching | 20–40ms | 64 squares × 12 templates |
| FEN building | <1ms | String operations |
| **Total** | **40–70ms** | Within 80ms target |

---

## Summary

The OCR module transforms raw screenshots into chess positions through a multi-stage pipeline:

1. **Find** the board using edge detection
2. **Normalize** to a standard 512×512 size
3. **Split** into 64 individual squares
4. **Match** each square against piece templates
5. **Encode** as a validated FEN string

This FEN output feeds directly into the engine module for move analysis.
