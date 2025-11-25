# Capture Module: Technical Overview

> **Module**: `src/capture.rs`
> **Purpose**: Take screenshots of the user's screen for chess position analysis
> **Performance Target**: <30ms per capture

---

## Executive Summary

The Capture module is the "camera" of ZugzwangRS. It takes a full-screen screenshot of the user's primary monitor and saves it as a PNG file. This screenshot is then passed to the OCR module for board detection and piece recognition.

**Design philosophy**: Keep capture simple and fast. All the intelligence (finding the board, cropping, recognizing pieces) lives in the OCR module. This separation means:

- Capture works regardless of which chess site or app is used
- No manual calibration needed for different window positions
- The same capture code works across platforms (macOS, Windows, Linux)

---

## How It Works: The Pipeline

The capture module has a straightforward single-stage pipeline:

```
┌─────────────────────────────────────────────────────────────────────┐
│  USER'S SCREEN                                                      │
│  (Browser with chess.com, Lichess, macOS Chess.app, etc.)          │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 1: Monitor Enumeration                                       │
│  Find all connected displays, select the primary monitor            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 2: Screen Capture                                            │
│  Capture full resolution screenshot via xcap library                │
│  (Uses native OS APIs: CoreGraphics on macOS, etc.)                │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  STAGE 3: Save to Disk                                              │
│  Write PNG to screenshots/current_board.png                         │
│  (Overwrites previous capture each cycle)                          │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  OUTPUT: PNG FILE                                                   │
│  screenshots/current_board.png                                      │
│  Ready for OCR processing                                          │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Concepts Explained

### Why Full-Screen Capture?

Rather than trying to capture just the chess window, we capture the entire screen. This approach has several advantages:

| Approach | Pros | Cons |
|----------|------|------|
| **Full-screen** (our choice) | Works with any app, no calibration needed | Larger file, OCR must find board |
| Window-specific | Smaller file, faster OCR | Requires window handle, breaks across apps |
| Manual crop region | Smallest file | Requires user calibration, breaks if window moves |

The "extra work" of finding the board in a full screenshot is handled by the OCR module's edge detection—a solved problem that's fast enough (<20ms) to not matter.

### What is xcap?

`xcap` is a Rust library for cross-platform screen capture. It abstracts away the differences between operating systems:

| Platform | Underlying Technology |
|----------|----------------------|
| macOS | CoreGraphics (CGDisplayCreateImage) |
| Windows | Windows Graphics Capture API |
| Linux | X11/XCB or Wayland protocols |

This means our capture code works identically across all platforms without any OS-specific code.

### Why PNG Format?

PNG (Portable Network Graphics) is used because:

1. **Lossless** — No compression artifacts that could confuse piece recognition
2. **Fast encoding** — Modern PNG encoders are highly optimized
3. **Universal** — Every image library can read PNG
4. **Transparency support** — Not needed now, but available if overlay features are added

---

## Deep Dive: Function Reference

### Public Functions

The module exposes one public function:

| Function | Purpose | Input | Output |
|----------|---------|-------|--------|
| `capture_screenshot()` | Capture and save screen | None | `Result<()>` (success/error) |

---

### `capture_screenshot()`

**Location**: `src/capture.rs:17–44`

**Purpose**: Capture the primary monitor's display and save it as a PNG file.

**Why it matters**: This is the entry point for all visual data in the system. Without screen capture, there's nothing to analyze.

```rust
pub fn capture_screenshot() -> Result<()>
```

#### Execution Flow

```
1. Start timer                    →  Track latency
2. Monitor::all()                 →  Enumerate connected displays
3. .next()                        →  Select first (primary) monitor
4. .capture_image()               →  Take screenshot via OS API
5. create_dir_all("screenshots")  →  Ensure output directory exists
6. screenshot.save(...)           →  Write PNG to disk
7. Print latency                  →  Performance feedback
8. (Optional) Save debug copy     →  If DEBUG_CAPTURE=1
```

#### Step-by-Step Breakdown

**Step 1: Performance Timing** (line 18)
```rust
let start = Instant::now();
```
Starts a high-precision timer to measure capture latency.

**Step 2–4: Monitor Capture** (lines 20–26)
```rust
let screenshot = Monitor::all()
    .context("Failed to enumerate monitors")?
    .into_iter()
    .next()
    .context("No monitors found")?
    .capture_image()
    .context("Failed to capture image — check Screen Recording permission")?;
```

This chain:
1. Gets list of all monitors
2. Takes the first one (primary display)
3. Captures its current contents as an image buffer

**Error handling**: If capture fails on macOS, the most likely cause is missing Screen Recording permission. The error message reminds the user to check this.

**Step 5: Directory Creation** (line 30)
```rust
fs::create_dir_all("screenshots").context("Failed to create screenshots dir")?;
```

Creates the `screenshots/` directory if it doesn't exist. Using `create_dir_all` means:
- No error if directory already exists
- Creates parent directories if needed

**Step 6: Save Screenshot** (lines 32–33)
```rust
screenshot.save("screenshots/current_board.png")
    .context("Failed to save screenshot")?;
```

Writes the captured image to disk as PNG. The filename `current_board.png` is overwritten each capture cycle—we only need the latest frame.

**Step 7: Latency Reporting** (lines 35–36)
```rust
let latency = start.elapsed();
eprintln!("Capture + save latency: {:?}", latency);
```

Prints timing to stderr (not stdout) so it doesn't interfere with the main output. Typical values: 15–30ms.

**Step 8: Debug Output** (lines 38–41)
```rust
if std::env::var("DEBUG_CAPTURE").as_ref().map_or(false, |v| v.as_str() == "1") {
    let _ = screenshot.save("screenshots/debug_full_screen.png");
}
```

When `DEBUG_CAPTURE=1` is set, saves an additional copy with a different name. This is useful for:
- Inspecting what was captured
- Comparing multiple captures
- Debugging OCR issues ("was the board even visible?")

---

## Output Files

| File | When Created | Purpose |
|------|--------------|---------|
| `screenshots/current_board.png` | Every capture | Primary output for OCR |
| `screenshots/debug_full_screen.png` | When `DEBUG_CAPTURE=1` | Debugging/inspection |

**File characteristics**:
- Full screen resolution (e.g., 2560×1600 on Retina MacBook)
- 32-bit RGBA PNG
- Typical size: 2–8 MB (depends on screen content complexity)

---

## Configuration & Tuning

### Environment Variables

| Variable | Value | Effect |
|----------|-------|--------|
| `DEBUG_CAPTURE` | `1` | Saves additional debug screenshot |

**Usage example**:
```bash
DEBUG_CAPTURE=1 cargo run
```

### Platform Requirements

#### macOS

Screen capture requires explicit user permission:

1. Open **System Settings** → **Privacy & Security** → **Screen & System Audio Recording**
2. Enable permission for **Terminal.app** (or your terminal emulator)
3. Restart Terminal after granting permission

**Symptom if missing**: Error message "Failed to capture image — check Screen Recording permission"

#### Windows

Usually works without special permissions. May require:
- Running as Administrator for some protected windows
- Allowing through Windows Security if flagged

#### Linux

Depends on display server:
- **X11**: Generally works without special permissions
- **Wayland**: May require portal permissions or running with specific flags

---

## Tests

### `test_capture_dimensions`

**Location**: `src/capture.rs:51–60`

**Purpose**: Verify that captured screenshots have valid, screen-like dimensions.

```rust
#[test]
#[ignore = "requires graphical display and screen recording permissions"]
fn test_capture_dimensions()
```

**Why ignored by default**: This test requires:
- A graphical display (won't work in headless CI)
- Screen recording permissions granted
- Actual screen content to capture

**To run**:
```bash
cargo test -- --ignored
```

**Assertions**:
1. Width and height are both > 0 (image exists)
2. Dimensions are at least 800×600 (reasonable minimum for a display)

---

## Performance Characteristics

| Operation | Typical Time | Notes |
|-----------|--------------|-------|
| Monitor enumeration | <1ms | Cached by OS |
| Screen capture | 10–20ms | Varies with resolution |
| PNG encoding + save | 5–15ms | Depends on content complexity |
| **Total** | **15–30ms** | Well under 30ms target |

### Factors Affecting Performance

| Factor | Impact | Mitigation |
|--------|--------|------------|
| Screen resolution | Higher = slower capture | None needed; still fast enough |
| Screen content | Complex content = larger PNG | Could use JPEG for speed (lossy) |
| Disk speed | Slow disk = slower save | SSD recommended; RAM disk possible |
| Multiple monitors | Slightly slower enumeration | We only capture primary |

---

## Architecture Decisions

### Why No Cropping Here?

Early versions considered cropping to a fixed region in the capture module. This was rejected because:

1. **Flexibility**: Different chess sites position the board differently
2. **Maintenance**: Would need to recalibrate when windows move
3. **Simplicity**: OCR's edge detection handles this automatically
4. **Speed**: Full-screen capture is already fast enough (<30ms)

### Why Overwrite Same File?

We always save to `current_board.png` rather than timestamped files because:

1. **Disk space**: 500ms cycle × 5MB = 10MB/second = 36GB/hour of gameplay
2. **Simplicity**: OCR always reads from the same path
3. **Purpose**: We only need the current frame, not history

If replay/history features are added later, a separate recording module would handle that.

### Why Primary Monitor Only?

Multi-monitor support was deferred because:

1. **Complexity**: Would need to detect which monitor has the chess game
2. **Use case**: Most users play chess on their primary display
3. **Workaround**: User can move chess window to primary monitor

Future enhancement: Add `--monitor=N` flag or auto-detect based on window title.

---

## Integration with Other Modules

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   main.rs    │────▶│  capture.rs  │────▶│   ocr.rs     │
│  (pipeline)  │     │ (this module)│     │ (next step)  │
└──────────────┘     └──────────────┘     └──────────────┘
                            │
                            ▼
                   screenshots/current_board.png
```

**Data contract**:
- **Input**: None (reads from screen)
- **Output**: `screenshots/current_board.png` (full-screen PNG)
- **Caller**: `main.rs` pipeline loop
- **Consumer**: `ocr::board_to_fen()` reads the saved file

---

## Limitations & Future Improvements

### Current Limitations

| Limitation | Impact | Potential Fix |
|------------|--------|---------------|
| Primary monitor only | Can't capture from secondary displays | Add `--monitor=N` flag |
| Full screen only | Larger files than needed | Add optional crop bounds |
| Fixed output path | Can't customize save location | Add config option |
| No window-specific capture | Must have chess visible on screen | Add window title matching |

### Possible Enhancements

| Enhancement | Benefit | Complexity |
|-------------|---------|------------|
| Window-specific capture | Smaller files, works when minimized | Medium |
| In-memory passing | Skip disk I/O entirely | Low |
| JPEG output option | Smaller files, faster saves | Low |
| Multi-monitor detection | Auto-find chess window | High |

### In-Memory Optimization

Currently, the pipeline is:
```
Capture → Save to disk → OCR reads from disk
```

A future optimization could pass the image buffer directly:
```
Capture → Pass buffer to OCR
```

This would eliminate ~10ms of disk I/O. Not implemented because:
1. Current performance is acceptable
2. File-based approach aids debugging
3. Adds complexity to module interfaces

---

## Summary

The Capture module provides a simple, reliable way to grab screenshots:

1. **Single responsibility**: Just capture and save—nothing more
2. **Cross-platform**: Works on macOS, Windows, and Linux via `xcap`
3. **Fast**: <30ms total latency
4. **Debuggable**: `DEBUG_CAPTURE=1` saves inspection copy

The module intentionally avoids intelligence (cropping, window detection) to stay simple and let the OCR module handle all image analysis.
