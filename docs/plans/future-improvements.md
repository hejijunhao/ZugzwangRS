### Remaining Assessment Updates
No major regressions from fixes. Core strengths (modularity, validation, debug aids) hold. However, the deeper issues I noted previously persist and could benefit from attention:

- **Tests**: Still no units in `ocr_native.rs`—critical for a compute-heavy module. Suggestion: Add quick mocks (e.g., `image::GrayImage::new` for squares/templates) to test variance/match_square/FEN build. Facade could test blocking wrap (e.g., assert native runs sync under async).
- **Accuracy/Robustness**: Hardcoded FEN suffix ("w KQkq - 0 1") in both native/LLM remains a limitation—engine gets wrong state (e.g., wrong turn → suboptimal moves). Quick win: Detect turn via secondary LLM prompt or native UI scan (e.g., match clock templates).
- **Native Perf/Edges**: Candidate search still potentially explosive on ultra-high-res; cap iterations or sample. Template load: Add `?` chain with per-file error logging for partial failures.
- **LLM**: Prompt could append detected errors for re-tries (e.g., if shakmaty fails). Cost/privacy: Consider optional anonymization (blur non-board?).
- **Integration**: In facade, `site` to LLM ignored—add `#[allow(unused_variables)]` or doc/#[cfg_attr(test, ...)] if keeping for future.