Rust Chess Assistant - Refined Project Blueprint

### Executive Summary
A pure-Rust CLI tool that captures your chess browser window, uses template-based OCR for board state (FEN), analyzes moves via a high-strength engine (Pleco/Stockfish), and outputs suggestions in the terminal. Stealth-focused with manual calibration; expandable to commentary via LLMs later.

### Core Strategic Decisions
1. **100% Rust Stack**  
   - **Rationale**: Unchanged—excellent for learning/deployment.  
   - **Refinement/Challenge**: Achievable, but if OCR proves tricky, a hybrid with minimal FFI (e.g., Tesseract bindings) could save time; stick pure for now. Trade-off holds.

2. **CLI-Only Output**  
   - **Rationale**: Unchanged—minimal and flexible.  
   - **Refinement**: Use 'crossterm' instead of 'colored' for more robust terminal handling (e.g., cross-platform clear screen). It's lightweight and active.

3. **Pure OCR (No Browser Extension)**  
   - **Rationale**: Unchanged—max stealth.  
   - **Refinement/Challenge**: Template matching is robust for stylized pieces, but accuracy dips with anti-aliasing/themes. Mitigate: Store multiple template sets per site. If <95% hit rate, fallback to user-input FEN for MVP.

4. **Engine for Move Calculation**  
   - **Rationale**: Consistency over LLMs—good call.  
   - **Refinement/Challenge**: Stockfish is top-tier, but requires external install (breaks "100% Rust" purity). Alternative: Pleco (pure-Rust Stockfish port, ~3000 ELO, sub-100ms). If you want 3500+ ELO, keep Stockfish but embed binary paths. Depth 18 is fine; start at 12 for faster prototyping.

5. **One-Time Manual Calibration**  
   - **Rationale**: Unchanged—reliable.  
   - **Refinement**: Add auto-detection fallback (e.g., Hough lines in imageproc for grid) as Phase 5 enhancement. Cache configs per site (e.g., chess.com vs. lichess).

### Architecture Overview
(Unchanged visually, but with refinements noted inline.)

┌─────────────────────────────────────────────────────────────┐  
│  User Action: Play chess in browser (chess.com/lichess)    │  
└───────────────────────┬─────────────────────────────────────┘  
                        │  
                        ▼  
┌─────────────────────────────────────────────────────────────┐  
│  Step 1: Screen Capture (xcap)                             │  
│  - Full screenshot → Crop to board bounds (calibrated)    │  
│  - Latency: 30-50ms                                         │  
└───────────────────────┬─────────────────────────────────────┘  
                        │  
                        ▼  
┌─────────────────────────────────────────────────────────────┐  
│  Step 2: OCR (imageproc + Custom)                          │  
│  - Split board into 64 squares (grid detection)           │  
│  - Template matching (SSD/NCC via imageproc)              │  
│  - Color thresholding for empty squares                   │  
│  - Output: FEN string                                     │  
│  - Latency: 20-40ms (optimized with rayon for parallelism)│  
└───────────────────────┬─────────────────────────────────────┘  
                        │  
                        ▼  
┌─────────────────────────────────────────────────────────────┐  
│  Step 3: Engine Analysis (pleco or stockfish crate)        │  
│  - Input: FEN string                                      │  
│  - Engine depth: 12-18 (configurable)                     │  
│  - Output: Best move (algebraic) + eval                   │  
│  - Latency: 50-100ms                                       │  
└───────────────────────┬─────────────────────────────────────┘  
                        │  
                        ▼  
┌─────────────────────────────────────────────────────────────┐  
│  Step 4: CLI Output (crossterm)                            │  
│  - Clear screen cross-platform                            │  
│  - Display: FEN, best move, evaluation (colored)         │  
│  - Update frequency: Every 500ms (configurable)           │  
└─────────────────────────────────────────────────────────────┘  

Total Latency: ~100-200ms. Refinement: Add error path (e.g., if FEN invalid, retry capture).

### Technology Stack
Updated with latest versions (Nov 2025); added/changed for better maintenance.

| Component       | Crate          | Version | Purpose |
|-----------------|----------------|---------|---------|
| Screen Capture | `xcap`        | 0.7.1  | Cross-platform screenshots. Active, supports Wayland. |
| Image Processing | `image`     | 0.25.9 | Crop/resize/color analysis. Recent release with metadata support. |
| Input Events   | `rdev`        | 0.5.3  | Mouse click capture for calibration. Stable, though older—works well. |
| Template Matching/OCR | `imageproc` | 0.25.3 | SSD/NCC matching + contours/Hough for grid. Pairs with `image`; GPU-optional via deps. |
| Chess Engine   | `pleco`       | 0.5.0  | Pure-Rust engine/library (Stockfish-derived). No external binary; fallback to `stockfish` 0.2.1 if higher ELO needed. |
| Board Logic    | `shakmaty`    | 0.28.0 | FEN validation/move parsing. More active than `chess` 3.2.0 (last update 2021). |
| Serialization  | `serde` + `serde_json` | 1.0.228 / 1.0.130 | Config I/O. Battle-tested. |
| Error Handling | `anyhow`      | 1.0.93 | Ergonomic errors. |
| Terminal UI    | `crossterm`   | 0.28.1 | Color/output/clear (replaces `colored` 2.1.0 for better cross-platform). |
| Parallelism (Optional) | `rayon` | 1.10.0 | Speed up OCR loops if needed. |

No async (tokio) needed—sync with `std::thread::sleep`. If adding hotkeys later, consider minimal tokio.

### Project Structure
Mostly unchanged; added `ocr.rs` deps and debug dir.

```
rust-chess-assistant/
├── Cargo.toml
├── board_config.json          # Auto-generated
├── templates/                 # Per-site piece images
│   ├── chesscom/             # Site-specific subdirs for multi-config
│   │   ├── K.png
│   │   └── ... 
│   └── lichess/
├── screenshots/               # Debug (gitignore)
└── src/
    ├── main.rs                # Entry, loop
    ├── capture.rs             # Screen logic
    ├── calibrate.rs           # Setup
    ├── ocr.rs                 # Recognition (uses imageproc)
    ├── engine.rs              # Pleco/Stockfish
    └── config.rs              # I/O
```

### Implementation Roadmap
Times adjusted for realism (e.g., OCR tuning). Start with Phase 1.

**Phase 1: MVP (3-4 hours)**  
Goal: Capture → Basic OCR → Engine move → Print.  
- Use hardcoded bounds.  
- Naive OCR: Color avg per square (refine later).  
- Integrate Pleco: Parse FEN with shakmaty, eval with pleco.  
Test: Static image; verify FEN/move output.

**Phase 2: Calibration (1-2 hours)**  
Goal: Click-based setup.  
- rdev for clicks; save bounds/colors/templates.  
Test: Calibrate, reload; check config.json.

**Phase 3: Accurate OCR (3-4 hours)**  
Goal: Template matching.  
- Use imageproc::template_matching (SSD).  
- Capture templates in calibration.  
- Add confidence (e.g., match score >0.9).  
Test: 10 boards; log accuracy (aim 90%+ initially, tune thresholds).

**Phase 4: Polish & Production (1-2 hours)**  
Goal: User-friendly.  
- Crossterm for output.  
- Flags: clap for --calibrate, --site=chesscom.  
- Handle SIGINT.  
Test: Full game simulation; no crashes.

**Phase 5: Enhancements (2+ hours, optional)**  
- Auto-grid detection (imageproc Hough).  
- Move validation (shakmaty legality check).  
- Random delays/hotkey trigger (rdev).

### Detection & Safety Strategy
Unchanged—excellent. Refinement: For "human-like errors," add config flag to suggest sub-optimal moves (e.g., top-3 random).

### Quick Start Guide
Updated crates/engine.

1. Create: `cargo new rust-chess-assistant`  
2. Add: `cargo add xcap image rdev imageproc pleco shakmaty serde serde_json anyhow crossterm rayon clap@4`  
3. Dirs: `mkdir templates screenshots`  
4. Code: Implement per phases.  
5. Build: `cargo build --release`  
6. Calibrate: `./target/release/rust-chess-assistant --calibrate --site=chesscom`  
7. Run: `./target/release/rust-chess-assistant`

### Key Configuration Files
Unchanged, but add "site": "chesscom" to json for multi-support.

### Decision Log (Updated)
| Decision | Alternative | Reason Chosen |
|----------|-------------|--------------|
| Pure OCR | Browser Ext | Stealth priority. |
| Manual Calib | Auto-Detect | Accuracy; add partial auto later. |
| Pleco Engine | Stockfish/LLM | Pure Rust, strong enough; 50ms, no install. (Challenge: If ELO critical, swap to Stockfish.) |
| CLI Output | Overlay | Simplicity. |
| Imageproc Matching | Custom/Tesseract | Efficient, pure-Rust; robust for pieces. |
| Sync Loop | Async | No bottlenecks. |

### Future Enhancements
Unchanged; add "LLM integration via API (optional crate)" for commentary.

### Success Criteria
Unchanged—solid metrics.

This refined blueprint is actionable—start with MVP in main.rs. If Pleco integration stumps you, fallback to Stockfish (add `cargo add stockfish`). Ping for code skeletons if needed!