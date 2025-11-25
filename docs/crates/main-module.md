# Main Module: Technical Overview

> **Module**: `src/main.rs`
> **Purpose**: Orchestrate the capture → OCR → engine → output pipeline
> **Cycle Time**: 500ms per iteration

---

## Executive Summary

The Main module is the "conductor" of ZugzwangRS. It doesn't do any chess-specific work itself—instead, it coordinates the other modules in a continuous loop, passing data through the pipeline and presenting results to the user.

**What it does**:
1. Parse command-line arguments (site selection)
2. Run an infinite loop that captures, recognizes, analyzes, and outputs
3. Handle errors gracefully with context

**Design philosophy**: Keep orchestration simple and transparent. Each step in the pipeline is a single function call with clear error handling. The main module is intentionally thin—all domain logic lives in specialized modules.

---

## How It Works: The Pipeline Loop

The application runs a continuous 500ms cycle:

```
┌─────────────────────────────────────────────────────────────────────┐
│  STARTUP                                                            │
│  Parse CLI args (--site), print welcome message                    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
        ┌───────────────────────────────────────────┐
        │              MAIN LOOP (500ms)            │
        │                                           │
        │  ┌─────────────────────────────────────┐  │
        │  │  Step 1: capture::capture_screenshot│  │
        │  │  Take full-screen screenshot        │  │
        │  │  Output: screenshots/current_board.png │
        │  └──────────────────┬──────────────────┘  │
        │                     │                     │
        │                     ▼                     │
        │  ┌─────────────────────────────────────┐  │
        │  │  Step 2: ocr::board_to_fen          │  │
        │  │  Detect board, recognize pieces     │  │
        │  │  Output: FEN string                 │  │
        │  └──────────────────┬──────────────────┘  │
        │                     │                     │
        │                     ▼                     │
        │  ┌─────────────────────────────────────┐  │
        │  │  Step 3: engine::analyze_position   │  │
        │  │  Find best move and evaluation      │  │
        │  │  Output: (move, eval) tuple         │  │
        │  └──────────────────┬──────────────────┘  │
        │                     │                     │
        │                     ▼                     │
        │  ┌─────────────────────────────────────┐  │
        │  │  Step 4: println! output            │  │
        │  │  Display FEN, best move, eval       │  │
        │  └──────────────────┬──────────────────┘  │
        │                     │                     │
        │                     ▼                     │
        │  ┌─────────────────────────────────────┐  │
        │  │  Sleep 500ms                        │  │
        │  │  Wait before next cycle             │  │
        │  └──────────────────┬──────────────────┘  │
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
                              └──────► (repeat forever)
```

---

## Key Concepts Explained

### Why a Continuous Loop?

The assistant monitors the chess game in real-time, updating recommendations as the position changes:

| Approach | Pros | Cons |
|----------|------|------|
| **Continuous loop** (our choice) | Always up-to-date, no user action needed | Uses CPU continuously |
| Hotkey-triggered | Only runs when needed, saves resources | Requires user to remember hotkey |
| File watcher | Efficient, event-driven | Complex setup, platform-specific |

The continuous loop is simplest for MVP. Future phases may add hotkey support via `rdev`.

### Why 500ms Sleep?

The sleep duration balances responsiveness with resource usage:

| Sleep Duration | Updates/sec | CPU Usage | Use Case |
|----------------|-------------|-----------|----------|
| 100ms | 10 | High | Bullet chess |
| **500ms** | 2 | Moderate | **Standard play (default)** |
| 1000ms | 1 | Low | Slow games, battery saving |

At 500ms, the assistant updates roughly every half-second—fast enough for blitz games, efficient enough for extended use.

### Module Dependencies

```
main.rs
   │
   ├──► capture.rs    (screen capture)
   │
   ├──► ocr.rs        (board recognition)
   │
   ├──► engine.rs     (move analysis)
   │
   ├──► config.rs     (commented out - Phase 2)
   │
   └──► calibrate.rs  (commented out - Phase 2)
```

Currently, only three modules are active. Configuration and calibration are planned for Phase 2.

### CLI Argument: `--site`

The `--site` flag tells the OCR module which piece templates to use:

```bash
# Default (chess.com)
cargo run

# Explicit site selection
cargo run -- --site=chesscom
cargo run -- --site=lichess
cargo run -- --site=macOS
```

| Site | Template Path | Notes |
|------|---------------|-------|
| `chesscom` | `templates/chesscom/` | Default, most common |
| `lichess` | `templates/lichess/` | Different piece style |
| `macOS` | `templates/macOS/` | macOS Chess.app |

**Why site-specific templates?** Each chess platform renders pieces differently. Using matched templates dramatically improves OCR accuracy.

---

## Deep Dive: Code Reference

### Module Declarations (Lines 1–5)

```rust
mod capture;
mod ocr;
mod engine;
// mod config;
// mod calibrate; // Enable for calibration mode
```

Declares which modules are part of the application:
- **Active**: `capture`, `ocr`, `engine`
- **Disabled**: `config`, `calibrate` (Phase 2 features)

The commented modules are intentionally excluded to keep MVP builds simple.

### Imports (Lines 7–10)

```rust
use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::thread;
use std::time::Duration;
```

| Import | Purpose |
|--------|---------|
| `anyhow::Context` | Add context to errors (e.g., "Failed to capture screenshot") |
| `anyhow::Result` | Ergonomic error handling |
| `clap::{Arg, Command}` | Command-line argument parsing |
| `std::thread` | Sleep functionality |
| `std::time::Duration` | Time duration representation |

### CLI Setup (Lines 14–28)

```rust
let matches = Command::new("Zugzwang-RS")
    .version("0.0.3")
    .author("Your Name")
    .about("Pure-Rust chess assistant for browser windows")
    .arg(
        Arg::new("site")
            .long("site")
            .value_name("SITE")
            .help("Chess site to target (e.g., chesscom, lichess)")
            .default_value("chesscom")
            .value_parser(["chesscom", "lichess", "macOS"])
    )
    .get_matches();

let site = matches.get_one::<String>("site").unwrap();
```

Uses `clap` to define the CLI interface:

| Property | Value | Purpose |
|----------|-------|---------|
| `version` | `"0.0.3"` | Shown with `--version` |
| `default_value` | `"chesscom"` | Used when `--site` not specified |
| `value_parser` | `["chesscom", "lichess", "macOS"]` | Restricts to valid options |

**Error handling**: If user passes invalid site (e.g., `--site=chess24`), clap automatically shows an error message with valid options.

### Startup Messages (Lines 30–32)

```rust
println!("Zugzwang-RS Chess Assistant starting... (MVP Phase 1)");
println!("Targeting site: {}", site);
println!("Press Ctrl+C to stop.");
```

Simple user feedback showing:
- Application name and phase
- Which site templates will be used
- How to exit (Ctrl+C)

### Main Loop (Lines 34–53)

```rust
loop {
    // Step 1: Capture
    capture::capture_screenshot()
        .context("Failed to capture screenshot")?;

    // Step 2: OCR
    let fen = ocr::board_to_fen("screenshots/current_board.png", site)
        .context("Failed to recognize board from screenshot")?;

    // Step 3: Engine
    let (best_move, eval) = engine::analyze_position(&fen)
        .context("Failed to analyze position")?;

    // Step 4: Output
    println!("Detected FEN: {}", fen);
    println!("Best move: {}", best_move);
    println!("Evaluation: {}", eval);

    thread::sleep(Duration::from_millis(500));
}
```

#### Step-by-Step Breakdown

**Step 1: Screen Capture** (lines 35–37)
```rust
capture::capture_screenshot()
    .context("Failed to capture screenshot")?;
```
- Captures full screen to `screenshots/current_board.png`
- Adds context to any error for debugging
- `?` propagates errors up to `main()`

**Step 2: OCR Processing** (lines 39–41)
```rust
let fen = ocr::board_to_fen("screenshots/current_board.png", site)
    .context("Failed to recognize board from screenshot")?;
```
- Reads the screenshot file
- Detects board region automatically
- Recognizes pieces using site-specific templates
- Returns FEN string describing the position

**Step 3: Engine Analysis** (lines 43–45)
```rust
let (best_move, eval) = engine::analyze_position(&fen)
    .context("Failed to analyze position")?;
```
- Parses FEN into internal board representation
- Searches for best move (depth 12)
- Returns move in UCI notation + evaluation

**Step 4: Output** (lines 47–50)
```rust
println!("Detected FEN: {}", fen);
println!("Best move: {}", best_move);
println!("Evaluation: {}", eval);
```
- Simple terminal output for MVP
- Future: Will use `crossterm` for colored, formatted output

**Step 5: Sleep** (line 52)
```rust
thread::sleep(Duration::from_millis(500));
```
- Pauses 500ms before next iteration
- Prevents CPU spinning
- Gives user time to read output

---

## Error Handling Strategy

### Context Wrapping

Each pipeline step wraps errors with `.context()`:

```rust
capture::capture_screenshot()
    .context("Failed to capture screenshot")?;
```

This produces helpful error chains:

```
Error: Failed to capture screenshot

Caused by:
    Failed to capture image — check Screen Recording permission
```

### Error Propagation

The `?` operator propagates errors to `main()`, which returns `Result<()>`. When an error occurs:

1. Error bubbles up from module → main
2. `anyhow` prints the error chain
3. Program exits with non-zero status

### No Recovery (MVP)

Currently, any error terminates the program. Future enhancements could add:
- Retry logic for transient failures
- Graceful degradation (skip cycle, continue loop)
- User notification without termination

---

## Configuration & Tuning

### Runtime Configuration

| Parameter | Current | Configurable? | Notes |
|-----------|---------|---------------|-------|
| Site | `--site` flag | Yes | CLI argument |
| Cycle time | 500ms | No (hardcoded) | Change line 52 |
| Search depth | 12 | No (hardcoded) | Change in engine.rs |

### Environment Variables

The main module itself doesn't use environment variables, but the modules it calls do:

| Variable | Module | Effect |
|----------|--------|--------|
| `DEBUG_CAPTURE` | capture.rs | Save extra debug screenshot |
| `DEBUG_OCR` | ocr.rs | Save cropped board + squares |

**Combined debug mode**:
```bash
DEBUG_CAPTURE=1 DEBUG_OCR=1 cargo run
```

---

## Output Format (MVP)

Current output is plain text:

```
Zugzwang-RS Chess Assistant starting... (MVP Phase 1)
Targeting site: chesscom
Press Ctrl+C to stop.
Capture + save latency: 23ms
Detected FEN: rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1
Best move: e7e5
Evaluation: +0.12
```

### Future Output (Phase 4)

Using `crossterm`, output will be:
- Colored (green for good eval, red for bad)
- Formatted (clear screen between updates)
- Aligned (tabular display)

---

## Performance Budget

| Step | Target | Typical | Notes |
|------|--------|---------|-------|
| Capture | <30ms | 15-25ms | xcap + PNG save |
| OCR | <80ms | 40-70ms | Edge detection + template matching |
| Engine | <100ms | 50-100ms | Depth 12 search |
| Output | <1ms | <1ms | println! |
| **Total** | **<200ms** | **110-200ms** | Per cycle (excluding sleep) |

The 500ms sleep ensures the loop doesn't spin faster than needed, even when processing completes quickly.

---

## Limitations & Future Improvements

### Current Limitations

| Limitation | Impact | Planned Fix |
|------------|--------|-------------|
| No graceful shutdown | Ctrl+C may leave state inconsistent | Add SIGINT handler |
| Plain text output | Hard to read at a glance | Use crossterm for formatting |
| Fixed cycle time | Can't adapt to game speed | Add `--interval` flag |
| No pause/resume | Always running or stopped | Add hotkey toggle |
| Errors terminate | Single failure stops everything | Add retry logic |

### Planned Enhancements (Per Roadmap)

**Phase 2: Calibration**
```rust
// Uncomment to enable
mod config;
mod calibrate;

// Add CLI flag
.arg(Arg::new("calibrate").long("calibrate").action(ArgAction::SetTrue))
```

**Phase 4: Polish**
```rust
// Replace println! with crossterm
use crossterm::{execute, style::Print, terminal::Clear};

execute!(stdout(), Clear(ClearType::All))?;
execute!(stdout(), Print(format!("Best: {} ({})", best_move, eval)))?;
```

**Phase 5: Hotkeys**
```rust
// Add hotkey support via rdev
use rdev::{listen, Event, EventType, Key};

// Toggle analysis on/off with F9
if event.event_type == EventType::KeyPress(Key::F9) {
    running.toggle();
}
```

---

## Integration Points

### Module Interfaces

| Module | Called Function | Input | Output |
|--------|-----------------|-------|--------|
| capture | `capture_screenshot()` | None | `Result<()>` + file on disk |
| ocr | `board_to_fen(path, site)` | File path, site name | `Result<String>` (FEN) |
| engine | `analyze_position(fen)` | FEN string | `Result<(String, String)>` |

### Data Flow

```
                    screenshots/current_board.png
                              │
capture::capture_screenshot() ─┘

                              │
                              ▼
ocr::board_to_fen() ──────────┼──► FEN string
                              │
                              ▼
engine::analyze_position() ───┼──► (move, eval)
                              │
                              ▼
println! ─────────────────────┴──► Terminal output
```

---

## Summary

The Main module is the minimal orchestrator for ZugzwangRS:

1. **CLI parsing**: Single `--site` flag via clap
2. **Pipeline loop**: Capture → OCR → Engine → Output → Sleep
3. **Error handling**: Context-wrapped errors with `?` propagation
4. **Simple output**: Plain `println!` for MVP

The module intentionally stays thin—all chess intelligence lives in specialized modules. This separation makes the codebase easier to test, extend, and maintain.

### Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Pipeline orchestration |
| `screenshots/current_board.png` | Inter-module communication |
| `templates/{site}/` | Site-specific piece templates |

### Quick Reference

```bash
# Run with defaults
cargo run

# Run with specific site
cargo run -- --site=lichess

# Run with debug output
DEBUG_CAPTURE=1 DEBUG_OCR=1 cargo run

# Show help
cargo run -- --help
```
