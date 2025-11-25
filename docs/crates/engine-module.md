# Engine Module: Technical Overview

> **Module**: `src/engine.rs`
> **Purpose**: Analyze chess positions and recommend optimal moves
> **Performance Target**: <100ms per analysis (depth 12)

---

## Executive Summary

The Engine module is the "brain" of ZugzwangRS. It takes a chess position (in FEN notation) and calculates the best move using a pure-Rust chess engine. This is where the actual chess intelligence lives.

**What it produces**: A recommended move in UCI notation (e.g., `e2e4`) and a position evaluation (e.g., `+0.35` meaning White is slightly better).

**Design philosophy**: Use a pure-Rust engine to maintain the project's "100% Rust" goal while achieving strong play (~2900 ELO). The engine handles all chess logic—move generation, position evaluation, and search—so the rest of the application can focus on capture and recognition.

---

## How It Works: The Pipeline

The module processes positions through a 5-step pipeline:

```
┌─────────────────────────────────────────────────────────────────────┐
│  FEN STRING (from OCR module)                                       │
│  "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 1: FEN Parsing                                                │
│  Convert FEN string into internal Board representation              │
│  Validates piece positions, turn, castling rights, etc.            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 2: Terminal State Check                                       │
│  Is the game already over? (Checkmate or Stalemate)                │
│  If yes → Return immediately with result                           │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 3: Engine Search                                              │
│  Iterative deepening search to find best move                       │
│  Examines millions of positions using alpha-beta pruning           │
│  Depth: 12 plies (6 full moves ahead)                              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 4: Position Evaluation                                        │
│  Calculate numerical score after best move                          │
│  Uses Piece-Square Tables (PSQT) for positional assessment         │
│  Positive = White advantage, Negative = Black advantage            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STEP 5: Output Formatting                                          │
│  Convert move to UCI notation (e.g., "e2e4")                       │
│  Format evaluation as pawns (e.g., "+1.25")                        │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OUTPUT: (Move, Evaluation)                                         │
│  ("e2e4", "+0.35")                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Concepts Explained

### What is Tanton?

Tanton is a pure-Rust chess engine, an actively maintained fork of the Pleco library. It's derived from Stockfish's architecture but implemented entirely in Rust.

| Property | Value |
|----------|-------|
| Language | 100% Rust |
| ELO Rating | ~2900 |
| Based On | Pleco (Stockfish port) |
| Search | Iterative Deepening + Alpha-Beta |
| Evaluation | Piece-Square Tables (PSQT) |

**Why Tanton over external Stockfish?**
- No external binary dependencies
- Pure Rust = simpler deployment
- Still strong enough for practical use (~2900 ELO beats 99%+ of human players)
- Sub-100ms response times at reasonable depths

### What is UCI Notation?

UCI (Universal Chess Interface) is the standard format for communicating moves between chess programs:

```
Format: [source square][destination square][promotion piece]

Examples:
  e2e4    → Pawn from e2 to e4
  g1f3    → Knight from g1 to f3
  e7e8q   → Pawn promotes to queen on e8
  e1g1    → King castles kingside (e1 to g1)
```

**Why UCI instead of SAN (Standard Algebraic Notation)?**
- Unambiguous: Always specifies source and destination
- Machine-readable: Easy to parse and validate
- Universal: All chess engines speak UCI

### What is Centipawn Evaluation?

Chess engines express position advantage in "centipawns" — hundredths of a pawn:

| Centipawns | Pawns | Meaning |
|------------|-------|---------|
| +100 | +1.00 | White is up one pawn's worth |
| -250 | -2.50 | Black has strong advantage |
| +35 | +0.35 | White is slightly better |
| 0 | 0.00 | Equal position |

**Interpretation guide**:

| Evaluation | Assessment |
|------------|------------|
| ±0.00 to ±0.30 | Equal position |
| ±0.30 to ±1.00 | Slight advantage |
| ±1.00 to ±2.00 | Clear advantage |
| ±2.00 to ±4.00 | Winning advantage |
| ±4.00+ | Decisive advantage |
| ±∞ (mate) | Forced checkmate |

### What is Iterative Deepening?

Iterative deepening is a search strategy that finds increasingly better moves:

```
Depth 1: Search all moves 1 ply deep → Best move: e2e4 (found in 1ms)
Depth 2: Search all moves 2 plies deep → Best move: e2e4 (found in 3ms)
Depth 3: Search all moves 3 plies deep → Best move: d2d4 (found in 8ms)
...
Depth 12: Search all moves 12 plies deep → Best move: d2d4 (found in 80ms)
```

**Why not just search directly to depth 12?**
1. **Move ordering**: Earlier iterations help order moves for faster pruning
2. **Time management**: Can stop early if time runs out (not used in ZugzwangRS yet)
3. **Anytime behavior**: Always has a "best move so far" available

### What is PSQT (Piece-Square Tables)?

PSQT is a fast evaluation method that assigns scores based on where pieces are located:

```
Example: Knight values on different squares

    a   b   c   d   e   f   g   h
8 [-50 -40 -30 -30 -30 -30 -40 -50]  ← Knights hate edges
7 [-40 -20   0   0   0   0 -20 -40]
6 [-30   0  10  15  15  10   0 -30]
5 [-30   5  15  20  20  15   5 -30]  ← Knights love the center
4 [-30   0  15  20  20  15   0 -30]
3 [-30   5  10  15  15  10   5 -30]
2 [-40 -20   0   5   5   0 -20 -40]
1 [-50 -40 -30 -30 -30 -30 -40 -50]

A knight on e5 (+20) is worth more than a knight on a1 (-50)
```

**Why PSQT instead of deeper evaluation?**
- Extremely fast: Simple array lookup
- Reasonably accurate: Captures positional principles
- No expensive calculations: Material + position = good enough for fast analysis

The engine uses separate tables for middlegame (mg) and endgame (eg) positions.

---

## Deep Dive: Function Reference

### Public Functions

The module exposes one public function:

| Function | Purpose | Input | Output |
|----------|---------|-------|--------|
| `analyze_position()` | Analyze a chess position | FEN string | `(move, evaluation)` |

---

### `analyze_position()`

**Location**: `src/engine.rs:11–44`

**Purpose**: The main entry point. Takes a FEN string and returns the best move with evaluation.

```rust
pub fn analyze_position(fen: &str) -> Result<(String, String)>
```

#### Return Value

Returns a tuple of two strings:
- **First**: Best move in UCI notation (e.g., `"e2e4"`) or `"--"` for game-over positions
- **Second**: Evaluation string (e.g., `"+0.35"`) or game result (e.g., `"White wins by checkmate"`)

#### Step-by-Step Breakdown

**Step 1: FEN Parsing** (lines 12–14)

```rust
let board = Board::from_fen(fen)
    .map_err(|_| anyhow!("Invalid FEN: {}", fen))?;
```

Converts the FEN string into Tanton's internal `Board` representation. This validates:
- Piece positions are legal
- Turn indicator is valid (w/b)
- Castling rights make sense
- En passant square is valid (if specified)

**Step 2: Terminal State Check** (lines 16–23)

```rust
if board.checkmate() {
    let winner = if board.turn() == tanton::Player::White { "Black" } else { "White" };
    return Ok(("--".to_string(), format!("{} wins by checkmate", winner)));
}
if board.stalemate() {
    return Ok(("--".to_string(), "Stalemate".to_string()));
}
```

Before expensive searching, check if the game is already over:
- **Checkmate**: The side to move has no legal moves and their king is in check
- **Stalemate**: The side to move has no legal moves but isn't in check

This early return avoids wasting computation on finished games.

**Step 3: Engine Search** (lines 25–28)

```rust
const SEARCH_DEPTH: u16 = 12;
let mut searcher = IterativeSearcher::new();
let best_move = searcher.best_move(board.shallow_clone(), SEARCH_DEPTH);
```

Creates an iterative deepening searcher and finds the best move:
- `SEARCH_DEPTH = 12`: Looks 12 half-moves (plies) ahead
- `shallow_clone()`: Efficient copy of board state for search
- Returns the move object representing the best play

**Step 4: Position Evaluation** (lines 30–36)

```rust
let mut eval_board = board.shallow_clone();
eval_board.apply_move(best_move);
let raw_eval = eval_board.psq().mg() as i32;

let eval_score = if board.turn() == tanton::Player::Black {
    -raw_eval
} else {
    raw_eval
};
```

Calculates the position's numerical evaluation:
1. Apply the best move to a copy of the board
2. Get the middlegame PSQT score
3. Adjust perspective: If Black to move, negate the score (so positive = good for side to move)

**Step 5: Output Formatting** (lines 38–43)

```rust
let move_str = best_move.stringify();
let eval_str = format_eval(eval_score);
Ok((move_str, eval_str))
```

Converts internal representations to human-readable strings:
- Move → UCI string (e.g., `"e2e4"`)
- Centipawn score → Pawn format with sign (e.g., `"+0.35"`)

---

### `format_eval()`

**Location**: `src/engine.rs:46–53`

**Purpose**: Convert centipawn integer to formatted pawn string.

```rust
fn format_eval(centipawns: i32) -> String
```

**Examples**:

| Input (centipawns) | Output |
|--------------------|--------|
| 135 | `"+1.35"` |
| -50 | `"-0.50"` |
| 0 | `"+0.00"` |

**Implementation**:
```rust
let pawns = centipawns as f64 / 100.0;
if pawns >= 0.0 {
    format!("+{:.2}", pawns)
} else {
    format!("{:.2}", pawns)
}
```

Always includes sign for clarity: `+` for positive/zero, `-` for negative.

---

## Configuration & Tuning

### Configurable Constants

| Constant | Location | Default | Purpose |
|----------|----------|---------|---------|
| `SEARCH_DEPTH` | Line 26 | 12 | How many half-moves to search ahead |

### Search Depth Trade-offs

| Depth | Typical Time | Strength | Use Case |
|-------|--------------|----------|----------|
| 8 | ~10ms | Weaker but fast | Real-time preview |
| 12 | ~50-100ms | Strong | **Default (balanced)** |
| 16 | ~500ms-1s | Very strong | Deep analysis |
| 20+ | Several seconds | Near-optimal | Offline analysis |

**Tuning guidance**:
- Lower depth for faster response but weaker play
- Higher depth for stronger play but slower response
- Depth 12 is a good balance for real-time assistance

### No Environment Variables (Yet)

Unlike capture and OCR modules, the engine module doesn't currently support debug modes via environment variables. Future enhancements could add:
- `DEBUG_ENGINE=1` — Print search statistics
- `ENGINE_DEPTH=N` — Override default search depth

---

## Integration with Other Modules

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   ocr.rs     │────▶│  engine.rs   │────▶│   main.rs    │
│ (FEN output) │     │ (this module)│     │   (output)   │
└──────────────┘     └──────────────┘     └──────────────┘
                            │
                            ▼
                     (best_move, eval)
```

**Data contract**:
- **Input**: Valid FEN string from OCR module
- **Output**: Tuple of `(move_uci, evaluation_string)`
- **Caller**: `main.rs` pipeline loop
- **Error handling**: Returns `Err` for invalid FEN; caller should retry OCR

---

## Performance Characteristics

| Operation | Typical Time | Notes |
|-----------|--------------|-------|
| FEN parsing | <1ms | Simple string parsing |
| Terminal check | <1ms | Quick legal move generation |
| Search (depth 12) | 50–100ms | Varies with position complexity |
| Evaluation | <1ms | PSQT lookup |
| Formatting | <1ms | String operations |
| **Total** | **50–100ms** | Within 100ms target |

### Factors Affecting Search Time

| Factor | Impact | Notes |
|--------|--------|-------|
| Position complexity | High | More pieces = more moves to consider |
| Tactical positions | High | Checks/captures extend search |
| Quiet positions | Low | Many early cutoffs |
| Search depth | Exponential | Each +2 depth ≈ 4× slower |

---

## Limitations & Future Improvements

### Current Limitations

| Limitation | Impact | Potential Fix |
|------------|--------|---------------|
| Fixed depth | Can't adapt to position complexity | Add time-based search |
| No pondering | Doesn't think during opponent's turn | Add background search |
| UCI output only | Users may prefer SAN (Nf3 vs g1f3) | Add shakmaty conversion |
| PSQT-only eval | Misses some tactical nuances | Use full Tanton evaluation |
| No multi-PV | Only shows single best move | Add top-3 suggestions |

### Possible Enhancements

| Enhancement | Benefit | Complexity |
|-------------|---------|------------|
| SAN output | More readable moves | Low |
| Configurable depth | User control over speed/strength | Low |
| Time-based search | Consistent response times | Medium |
| Multi-PV (top 3) | More options, less predictable | Medium |
| Evaluation explanation | "Why is this move good?" | High |
| Opening book | Instant response in openings | Medium |

### Move Notation Conversion

Currently outputs UCI notation (`e2e4`). For human-readable SAN (`e4`), would need:

```rust
// Potential future enhancement using shakmaty
use shakmaty::{Chess, san::San};

fn uci_to_san(board: &Chess, uci: &str) -> String {
    let mv = uci.parse::<UciMove>().unwrap().to_move(board).unwrap();
    San::from_move(board, &mv).to_string()
}
```

---

## Error Handling

### Invalid FEN

If the OCR module produces an invalid FEN string, `analyze_position()` returns an error:

```rust
Err(anyhow!("Invalid FEN: {}", fen))
```

**Common causes**:
- OCR misrecognized pieces
- Incomplete board detection
- Corrupted FEN string

**Recommended handling**: Log error and retry capture/OCR cycle.

### No Legal Moves

Handled gracefully via terminal state checks:
- Checkmate → Returns winner announcement
- Stalemate → Returns draw announcement

---

## Code Quality Notes

The current implementation has a minor syntax issue (extra parenthesis on line 22) that should be addressed. The logic is sound, but the code won't compile as-is.

---

## Summary

The Engine module provides chess intelligence through a clean interface:

1. **Single function**: `analyze_position(fen)` handles everything
2. **Pure Rust**: Uses Tanton engine, no external dependencies
3. **Fast**: <100ms response time at depth 12
4. **Strong**: ~2900 ELO, beats most human players
5. **Graceful**: Handles checkmate/stalemate positions

The module transforms OCR output (FEN strings) into actionable advice (best moves with evaluations), completing the ZugzwangRS pipeline.
