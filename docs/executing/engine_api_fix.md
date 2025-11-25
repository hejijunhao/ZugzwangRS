# Engine API Fix: tanton 1.0 best_move Method

## Issue
In `tanton` v1.0, `IterativeSearcher::best_move()` is an associated function (static method), not an instance method as assumed in the initial implementation.

## Fix
Changed from instantiating `IterativeSearcher` and calling `searcher.best_move()` to directly calling `IterativeSearcher::best_move()`.

### Before
```rust
let mut searcher = IterativeSearcher::new();
let best_move = searcher.best_move(board.shallow_clone(), SEARCH_DEPTH);
```

### After
```rust
let best_move = IterativeSearcher::best_move(board.shallow_clone(), SEARCH_DEPTH);
```

## Implementation Note
- Removed unnecessary instance creation since `best_move` doesn't require `self`.
- This aligns with `tanton` 1.0 API changes from earlier versions.
- No functional impact; search behavior remains identical.
- Fixes compilation error E0599: "no function or associated item named `new`".