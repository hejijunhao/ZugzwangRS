# Engine Module Implementation Plan

**Module**: `src/engine.rs`
**Phase**: 1 (MVP)
**Target Latency**: 50-100ms at depth 12
**Status**: Stub → Implementation

---

## Overview

The engine module is Step 3 in the ZugzwangRS pipeline. It receives a FEN string from OCR, analyzes the position using the `tanton` pure-Rust chess engine, and returns the best move with evaluation.

```
OCR (FEN string) → Engine Analysis → (best_move, evaluation)
```

### Why Tanton?

**Tanton** is an actively maintained fork of Pleco (which became unmaintained in mid-2024). It provides:
- Identical API to Pleco (drop-in replacement)
- Pure Rust implementation (~2900 ELO)
- No external binary dependencies
- Active maintenance and bug fixes

```toml
# Cargo.toml change required:
tanton = "0.5"  # replaces pleco = "0.5.0"
```

---

## Implementation Steps

### Step 1: Parse FEN and Create Board

**Goal**: Convert FEN string to a `tanton::Board` for analysis.

```rust
use tanton::Board;

let board = Board::from_fen(fen)
    .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", fen))?;
```

**Edge cases**:
- Invalid FEN syntax → Return descriptive error
- Illegal position (e.g., multiple kings) → tanton may reject or behave unexpectedly

**Validation**: Consider pre-validating with `shakmaty::fen::Fen::from_ascii()` since OCR already does this.

---

### Step 2: Run Engine Search

**Goal**: Find the best move at configurable depth.

Tanton provides `tanton::bots::IterativeSearcher` for iterative deepening search:

```rust
use tanton::bots::IterativeSearcher;
use tanton::tools::Searcher;

const SEARCH_DEPTH: u16 = 12; // MVP default; configurable later

let mut searcher = IterativeSearcher::new();
let best_move = searcher.best_move(board.shallow_clone(), SEARCH_DEPTH);
```

**Depth considerations**:
| Depth | Typical Latency | Strength |
|-------|-----------------|----------|
| 8     | ~10-20ms        | Casual   |
| 12    | ~50-100ms       | Strong   |
| 16    | ~200-500ms      | Very strong |
| 18    | ~1-2s           | Near optimal |

Start with depth 12 for MVP; make configurable in Phase 4.

---

### Step 3: Extract Evaluation Score

**Goal**: Get centipawn evaluation or mate distance.

Tanton's searcher returns a `BitMove`. For evaluation, we need to query the board's score or use a separate evaluation:

```rust
use tanton::board::eval::Eval;

// After getting best move, evaluate the resulting position
let mut new_board = board.shallow_clone();
new_board.apply_move(best_move);
let eval_score = new_board.psq().mg() as i32; // Piece-square midgame score

// Format as string: "+1.50" or "-0.75"
let eval_str = format_eval(eval_score);
```

**Alternative**: Use `tanton::bots::MiniMaxSearcher` which exposes scores directly, or track the score during iterative search.

**Format evaluation string**:
```rust
fn format_eval(centipawns: i32) -> String {
    let pawns = centipawns as f64 / 100.0;
    if pawns >= 0.0 {
        format!("+{:.2}", pawns)
    } else {
        format!("{:.2}", pawns)
    }
}
```

---

### Step 4: Convert Move to Readable Format

**Goal**: Output move in UCI or SAN notation.

Tanton's `BitMove` can be converted to UCI string directly:

```rust
let move_str = best_move.stringify(); // e.g., "e2e4", "e1g1" (castling)
```

**Optional SAN conversion** (human-readable like "Nf3", "O-O"):
- Tanton doesn't provide SAN directly
- Use `shakmaty` for SAN if needed:

```rust
use shakmaty::{Chess, Position, san::San};
use shakmaty::fen::Fen;
use shakmaty::uci::Uci;

fn uci_to_san(fen: &str, uci_move: &str) -> Result<String> {
    let setup: Fen = fen.parse()?;
    let pos: Chess = setup.into_position(shakmaty::CastlingMode::Standard)?;
    let uci: Uci = uci_move.parse()?;
    let m = uci.to_move(&pos)?;
    let san = San::from_move(&pos, &m);
    Ok(san.to_string())
}
```

**MVP**: Use UCI format; add SAN in Phase 4 polish.

---

### Step 5: Handle Edge Cases

| Case | Detection | Response |
|------|-----------|----------|
| **Checkmate** | `board.checkmate()` | Return `("--", "Checkmate")` |
| **Stalemate** | `board.stalemate()` | Return `("--", "Stalemate")` |
| **No legal moves** | Empty move list | Return appropriate status |
| **Invalid FEN** | `Board::from_fen` fails | Propagate error |

```rust
if board.checkmate() {
    return Ok(("--".to_string(), "Checkmate".to_string()));
}
if board.stalemate() {
    return Ok(("--".to_string(), "Stalemate".to_string()));
}
```

---

### Step 6: Future Enhancements (Post-MVP)

**Stealth features** (Phase 5):
- Randomize among top-3 moves occasionally
- Add slight delays to mimic human thinking
- Vary depth based on position complexity

```rust
// Example: Randomize top moves for human-like play
fn pick_human_like_move(board: &Board, depth: u16) -> BitMove {
    // Get top 3 moves with scores
    // Randomly select with weighted probability
    // Favor best move 70%, second 20%, third 10%
}
```

**Configurable depth** (Phase 4):
- Add `--depth` CLI flag
- Store in config.json
