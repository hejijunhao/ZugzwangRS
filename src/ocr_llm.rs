//! LLM-based OCR using OpenAI GPT-4o Mini
//! Sends board screenshot to vision API, receives FEN string directly.
//! No templates needed - works with any chess site or piece style.
//! Latency: 300-600ms (network dependent)
//! Requires OPENAI_API_KEY environment variable.

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4o-mini";
const MAX_RETRIES: u32 = 2;
const TIMEOUT_SECS: u64 = 15;

// *************** Request/Response Types ***************

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlDetail },
}

#[derive(Serialize)]
struct ImageUrlDetail {
    url: String,
    detail: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}


// *************** Public API ***************

/// Checks if the OpenAI API key is available
pub fn has_api_key() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
}

/// Analyzes a chess board image and returns FEN notation using GPT-4o Mini.
pub async fn board_to_fen(image_path: &str) -> Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY environment variable not set")?;

    // Read and encode image
    let image_data =
        std::fs::read(image_path).with_context(|| format!("Failed to read image: {}", image_path))?;
    let base64_image = general_purpose::STANDARD.encode(&image_data);

    // Build request
    let prompt = build_prompt();
    let request = build_request(&base64_image, &prompt);

    // Call API with retry
    let fen = call_api_with_retry(&api_key, &request).await?;

    // Always show raw LLM response for debugging
    eprintln!("LLM returned: {}", fen);

    // Validate FEN
    validate_fen(&fen)?;

    Ok(fen)
}


// *************** Internal Functions ***************

fn build_prompt() -> String {
    r#"Analyze this chessboard image. Output ONLY the FEN string.

Rules:
- Output ONLY the FEN, nothing else (no explanation, no markdown, no quotes)
- White pieces are at the bottom of the image
- Use standard FEN: uppercase = White (KQRBNP), lowercase = Black (kqrbnp)
- Numbers represent consecutive empty squares
- Rows separated by /
- Append: w KQkq - 0 1

Example output:
rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"#
        .to_string()
}

fn build_request(base64_image: &str, prompt: &str) -> ChatRequest {
    ChatRequest {
        model: MODEL.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text {
                    text: prompt.to_string(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrlDetail {
                        url: format!("data:image/png;base64,{}", base64_image),
                        detail: "low".to_string(), // Faster processing, sufficient for chess
                    },
                },
            ],
        }],
        max_tokens: 100,
    }
}

async fn call_api_with_retry(api_key: &str, request: &ChatRequest) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
        .context("Failed to create HTTP client")?;

    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES + 1 {
        match call_api(&client, api_key, request).await {
            Ok(fen) => return Ok(fen),
            Err(e) => {
                eprintln!(
                    "LLM OCR attempt {}/{} failed: {}",
                    attempt,
                    MAX_RETRIES + 1,
                    e
                );
                last_error = Some(e);
                if attempt <= MAX_RETRIES {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

async fn call_api(client: &Client, api_key: &str, request: &ChatRequest) -> Result<String> {
    let response = client
        .post(API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(request)
        .send()
        .await
        .context("Failed to send request to OpenAI")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI API error {}: {}", status, body);
    }

    let api_response: ChatResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI response")?;

    let fen = api_response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))?;

    Ok(fen)
}

fn validate_fen(fen: &str) -> Result<()> {
    // Step 1: Basic syntax validation
    shakmaty::fen::Fen::from_ascii(fen.as_bytes())
        .map_err(|e| anyhow::anyhow!("Invalid FEN syntax from LLM: {} (received: '{}')", e, fen))?;

    // Step 2: Validate king count (exactly 1 white king 'K' and 1 black king 'k')
    // This prevents Tanton engine panics on illegal positions
    let board_part = fen.split_whitespace().next().unwrap_or("");
    let white_kings = board_part.chars().filter(|&c| c == 'K').count();
    let black_kings = board_part.chars().filter(|&c| c == 'k').count();

    if white_kings != 1 || black_kings != 1 {
        anyhow::bail!(
            "Invalid FEN from LLM: expected exactly 1 king per side, got {} white kings and {} black kings (received: '{}')",
            white_kings, black_kings, fen
        );
    }

    Ok(())
}

// *************** Tests ***************

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_contains_rules() {
        let prompt = build_prompt();
        assert!(prompt.contains("FEN"));
        assert!(prompt.contains("White"));
        assert!(prompt.contains("w KQkq"));
    }

    #[test]
    fn test_validate_fen_accepts_valid() {
        let valid = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert!(validate_fen(valid).is_ok());
    }

    #[test]
    fn test_validate_fen_rejects_invalid() {
        let invalid = "not a fen string";
        assert!(validate_fen(invalid).is_err());
    }

    #[test]
    fn test_has_api_key_without_key() {
        // This test depends on environment, but should at least not panic
        let _ = has_api_key();
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY"]
    async fn test_real_api_call() {
        // Run with: OPENAI_API_KEY=sk-... cargo test test_real_api_call -- --ignored
        let result = board_to_fen("screenshots/current_board.png").await;
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
}
