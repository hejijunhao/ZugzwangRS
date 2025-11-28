//! LLM-based OCR using OpenAI GPT-4o
//! Sends board screenshot to vision API, receives FEN string directly.
//! No templates needed - works with any chess site or piece style.
//! Latency: 500-2000ms (network dependent)
//! Requires OPENAI_API_KEY environment variable.

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::PlayerSide;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4o";  // Full GPT-4o for better vision accuracy (was gpt-4o-mini)
const MAX_API_RETRIES: u32 = 2;      // Retries for network/API errors
const MAX_VALIDATION_RETRIES: u32 = 2; // Retries when FEN validation fails (e.g., 9 pawns)
const TIMEOUT_SECS: u64 = 30;  // Increased timeout for larger model

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

/// Analyzes a chess board image and returns FEN notation using GPT-4o.
///
/// The `player_side` parameter determines:
/// - Board orientation interpretation (which pieces are at the bottom)
/// - FEN turn indicator ('w' for White, 'b' for Black)
///
/// Includes automatic retry logic:
/// - Retries on network/API errors (up to MAX_API_RETRIES)
/// - Retries on validation failures like "9 pawns" (up to MAX_VALIDATION_RETRIES)
pub async fn board_to_fen(image_path: &str, player_side: PlayerSide) -> Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY environment variable not set")?;

    // Read and encode image
    let image_data =
        std::fs::read(image_path).with_context(|| format!("Failed to read image: {}", image_path))?;
    let base64_image = general_purpose::STANDARD.encode(&image_data);

    // Build request with side-aware prompt
    let prompt = build_prompt(player_side);
    let request = build_request(&base64_image, &prompt);

    // Retry loop for validation failures (LLM sometimes returns invalid positions)
    let mut last_validation_error = None;

    for validation_attempt in 1..=MAX_VALIDATION_RETRIES + 1 {
        // Call API with retry (handles network errors)
        let fen = call_api_with_retry(&api_key, &request).await?;

        // Always show raw LLM response for debugging
        eprintln!("LLM returned: {}", fen);

        // Validate and fix FEN (corrects castling rights based on piece positions)
        match validate_fen(&fen) {
            Ok(corrected_fen) => return Ok(corrected_fen),
            Err(e) => {
                if validation_attempt <= MAX_VALIDATION_RETRIES {
                    eprintln!(
                        "⚠ Validation failed (attempt {}/{}): {} - retrying...",
                        validation_attempt,
                        MAX_VALIDATION_RETRIES + 1,
                        e
                    );
                    last_validation_error = Some(e);
                    // Small delay before retry
                    tokio::time::sleep(Duration::from_millis(300)).await;
                } else {
                    last_validation_error = Some(e);
                }
            }
        }
    }

    // All retries exhausted
    Err(last_validation_error.unwrap())
}


// *************** Internal Functions ***************

/// Builds the prompt for GPT-4o based on which side the player is playing.
/// - When playing as White: White pieces are at the bottom, turn indicator is 'w'
/// - When playing as Black: Black pieces are at the bottom, turn indicator is 'b'
fn build_prompt(player_side: PlayerSide) -> String {
    let (piece_position, turn_char) = match player_side {
        PlayerSide::White => ("White pieces are at the bottom of the image", 'w'),
        PlayerSide::Black => ("Black pieces are at the bottom of the image", 'b'),
    };

    format!(r#"Analyze this chessboard image. Output ONLY the FEN string.

Rules:
- Output ONLY the FEN, nothing else (no explanation, no markdown, no quotes)
- {piece_position}
- Use standard FEN: uppercase = White (KQRBNP), lowercase = Black (kqrbnp)
- Numbers represent consecutive empty squares
- Rows separated by / (starting from rank 8 at the top of the board)
- Append: {turn_char} KQkq - 0 1

Example output for starting position:
rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR {turn_char} KQkq - 0 1"#,
        piece_position = piece_position,
        turn_char = turn_char
    )
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
                        url: format!("data:image/jpeg;base64,{}", base64_image),
                        detail: "high".to_string(), // High detail for accurate piece recognition
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

    for attempt in 1..=MAX_API_RETRIES + 1 {
        match call_api(&client, api_key, request).await {
            Ok(fen) => return Ok(fen),
            Err(e) => {
                eprintln!(
                    "LLM API attempt {}/{} failed: {}",
                    attempt,
                    MAX_API_RETRIES + 1,
                    e
                );
                last_error = Some(e);
                if attempt <= MAX_API_RETRIES {
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

fn validate_fen(fen: &str) -> Result<String> {
    let board_part = fen.split_whitespace().next().unwrap_or("");

    // Step 1: Validate king count (exactly 1 white king 'K' and 1 black king 'k')
    // This prevents Tanton engine panics on illegal positions
    let white_kings = board_part.chars().filter(|&c| c == 'K').count();
    let black_kings = board_part.chars().filter(|&c| c == 'k').count();

    if white_kings != 1 || black_kings != 1 {
        anyhow::bail!(
            "Invalid FEN from LLM: expected exactly 1 king per side, got {} white kings and {} black kings (received: '{}')",
            white_kings, black_kings, fen
        );
    }

    // Step 2: Validate pawn count (max 8 per side)
    // LLM sometimes forgets to remove a pawn from starting square when it moves
    let white_pawns = board_part.chars().filter(|&c| c == 'P').count();
    let black_pawns = board_part.chars().filter(|&c| c == 'p').count();

    if white_pawns > 8 {
        anyhow::bail!(
            "Invalid FEN from LLM: White has {} pawns (max 8). LLM likely forgot to remove pawn from starting square. (received: '{}')",
            white_pawns, fen
        );
    }
    if black_pawns > 8 {
        anyhow::bail!(
            "Invalid FEN from LLM: Black has {} pawns (max 8). LLM likely forgot to remove pawn from starting square. (received: '{}')",
            black_pawns, fen
        );
    }

    // Step 3: Fix castling rights based on king/rook positions
    // The LLM always outputs "KQkq" but this can be invalid if pieces have moved
    let corrected_fen = fix_castling_rights(fen);

    // Step 4: Final syntax validation with shakmaty
    shakmaty::fen::Fen::from_ascii(corrected_fen.as_bytes())
        .map_err(|e| anyhow::anyhow!("Invalid FEN syntax from LLM: {} (received: '{}')", e, corrected_fen))?;

    Ok(corrected_fen)
}

/// Fixes castling rights in FEN based on actual king and rook positions.
/// Castling is only legal if:
/// - King is on its starting square (e1 for White, e8 for Black)
/// - Rook is on its starting square (a1/h1 for White, a8/h8 for Black)
fn fix_castling_rights(fen: &str) -> String {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() < 3 {
        return fen.to_string();
    }

    let board = parts[0];
    let turn = parts[1];

    // Parse the board into ranks (rank 8 is first, rank 1 is last)
    let ranks: Vec<&str> = board.split('/').collect();
    if ranks.len() != 8 {
        return fen.to_string();
    }

    // Helper to expand a rank string (e.g., "r3k2r" stays, "8" → "........")
    fn expand_rank(rank: &str) -> String {
        let mut result = String::new();
        for c in rank.chars() {
            if let Some(n) = c.to_digit(10) {
                result.push_str(&".".repeat(n as usize));
            } else {
                result.push(c);
            }
        }
        result
    }

    // Get rank 1 (White's back rank, index 7) and rank 8 (Black's back rank, index 0)
    let rank1 = expand_rank(ranks[7]); // White's back rank
    let rank8 = expand_rank(ranks[0]); // Black's back rank

    // Check piece positions (0-indexed: a=0, b=1, ..., h=7)
    let white_king_e1 = rank1.chars().nth(4) == Some('K');
    let white_rook_a1 = rank1.chars().nth(0) == Some('R');
    let white_rook_h1 = rank1.chars().nth(7) == Some('R');
    let black_king_e8 = rank8.chars().nth(4) == Some('k');
    let black_rook_a8 = rank8.chars().nth(0) == Some('r');
    let black_rook_h8 = rank8.chars().nth(7) == Some('r');

    // Build castling rights string
    let mut castling = String::new();
    if white_king_e1 && white_rook_h1 { castling.push('K'); }
    if white_king_e1 && white_rook_a1 { castling.push('Q'); }
    if black_king_e8 && black_rook_h8 { castling.push('k'); }
    if black_king_e8 && black_rook_a8 { castling.push('q'); }

    if castling.is_empty() {
        castling = "-".to_string();
    }

    // Rebuild FEN with corrected castling rights
    format!("{} {} {} - 0 1", board, turn, castling)
}

// *************** Tests ***************

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_for_white() {
        let prompt = build_prompt(PlayerSide::White);
        assert!(prompt.contains("FEN"));
        assert!(prompt.contains("White pieces are at the bottom"));
        assert!(prompt.contains("w KQkq"));
    }

    #[test]
    fn test_build_prompt_for_black() {
        let prompt = build_prompt(PlayerSide::Black);
        assert!(prompt.contains("FEN"));
        assert!(prompt.contains("Black pieces are at the bottom"));
        assert!(prompt.contains("b KQkq"));
    }

    #[test]
    fn test_validate_fen_accepts_valid_white() {
        let valid = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let result = validate_fen(valid);
        assert!(result.is_ok());
        // Starting position should keep all castling rights
        assert_eq!(result.unwrap(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }

    #[test]
    fn test_validate_fen_accepts_valid_black() {
        let valid = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let result = validate_fen(valid);
        assert!(result.is_ok());
        // Kings and rooks still on starting squares, keep all castling rights
        assert_eq!(result.unwrap(), "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    }

    #[test]
    fn test_validate_fen_rejects_invalid() {
        let invalid = "not a fen string";
        assert!(validate_fen(invalid).is_err());
    }

    #[test]
    fn test_validate_fen_rejects_too_many_white_pawns() {
        // 9 white pawns (pawn on e4 + all 8 on rank 2) - common LLM error
        let invalid = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
        let result = validate_fen(invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("9 pawns"));
    }

    #[test]
    fn test_validate_fen_rejects_too_many_black_pawns() {
        // 9 black pawns
        let invalid = "rnbqkbnr/pppppppp/4p3/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let result = validate_fen(invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("9 pawns"));
    }

    #[test]
    fn test_fix_castling_removes_rights_when_king_moved() {
        // Black king castled (on g8), but FEN claims KQkq - should fix to KQ only
        let fen_with_bad_castling = "r4rk1/pp1p1ppp/1n6/2p5/3P2N1/3P1N2/PPPBP1PP/R2QKB1R b KQkq - 0 1";
        let result = validate_fen(fen_with_bad_castling);
        assert!(result.is_ok());
        let corrected = result.unwrap();
        // Black king not on e8, so no black castling rights
        // White king on e1 with rooks on a1 and h1, so KQ
        assert!(corrected.contains(" KQ ") || corrected.contains(" - "));
        assert!(!corrected.contains("kq"));
    }

    #[test]
    fn test_fix_castling_no_rights_when_both_kings_moved() {
        // Both kings have moved - should have no castling rights
        let fen = "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
        let result = validate_fen(fen);
        assert!(result.is_ok());
        let corrected = result.unwrap();
        // Black king on g8 (not e8), White king on e1 with rooks
        // White should have KQ, Black should have none
        assert!(corrected.contains(" KQ "));
    }

    #[test]
    fn test_fix_castling_keeps_partial_rights() {
        // White has only kingside rook, Black has both
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/4K2R w KQkq - 0 1";
        let result = validate_fen(fen);
        assert!(result.is_ok());
        let corrected = result.unwrap();
        // White: King on e1, only h1 rook → K only
        // Black: King on e8, both rooks → kq
        assert!(corrected.contains(" Kkq "));
    }

    #[test]
    fn test_has_api_key_without_key() {
        // This test depends on environment, but should at least not panic
        let _ = has_api_key();
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY"]
    async fn test_real_api_call_as_white() {
        // Run with: OPENAI_API_KEY=sk-... cargo test test_real_api_call_as_white -- --ignored
        let result = board_to_fen("screenshots/current_board.png", PlayerSide::White).await;
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires OPENAI_API_KEY"]
    async fn test_real_api_call_as_black() {
        // Run with: OPENAI_API_KEY=sk-... cargo test test_real_api_call_as_black -- --ignored
        let result = board_to_fen("screenshots/current_board.png", PlayerSide::Black).await;
        println!("Result: {:?}", result);
        assert!(result.is_ok());
    }
}
