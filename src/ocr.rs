//! OCR facade module - dispatches to LLM or native implementation based on mode
//! This module provides a unified interface for board-to-FEN conversion, allowing users to choose between:
//! - **LLM mode**: Uses GPT-4o Mini vision API (accurate, universal, requires API key)
//! - **Native mode**: Uses template matching (fast, offline, requires templates)

use anyhow::Result;

/// OCR implementation mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OcrMode {
    /// GPT-4o Mini vision API
    Llm,
    /// Template-based matching (default for backward compatibility)
    #[default]
    Native,
}

impl std::fmt::Display for OcrMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrMode::Llm => write!(f, "LLM (GPT-4o Mini)"),
            OcrMode::Native => write!(f, "Native (template matching)"),
        }
    }
}

/// Checks if the LLM OCR mode is available (API key is set)
pub fn llm_available() -> bool {
    crate::ocr_llm::has_api_key()
}

/// Main entry point for board-to-FEN conversion.
pub async fn board_to_fen(image_path: &str, site: &str, mode: OcrMode) -> Result<String> {
    match mode {
        OcrMode::Llm => crate::ocr_llm::board_to_fen(image_path).await,
        OcrMode::Native => {
            // Native is synchronous, wrap for async compatibility
            let path = image_path.to_string();
            let site = site.to_string();
            tokio::task::spawn_blocking(move || crate::ocr_native::board_to_fen(&path, &site))
                .await
                .map_err(|e| anyhow::anyhow!("Native OCR task failed: {}", e))?
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_mode_display() {
        assert_eq!(format!("{}", OcrMode::Llm), "LLM (GPT-4o Mini)");
        assert_eq!(format!("{}", OcrMode::Native), "Native (template matching)");
    }

    #[test]
    fn test_ocr_mode_default() {
        assert_eq!(OcrMode::default(), OcrMode::Native);
    }
}
