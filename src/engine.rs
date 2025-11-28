//! Engine module
//! Uses `tanton` pure-Rust chess engine for move calculation (actively maintained fork of Pleco (~2900 ELO))
//! Pipeline: FEN string → Board → Search → (best_move, evaluation)

use anyhow::{anyhow, Result};
use tanton::Board;
use tanton::bots::IterativeSearcher;
use tanton::tools::Searcher;

/// Analyzes a chess position from FEN notation
pub fn analyze_position(fen: &str) -> Result<(String, String)> {
    use std::io::Write;

    eprint!("Engine analysis... ");
    let _ = std::io::stderr().flush();
    let start = std::time::Instant::now();

    // Step 1: Parse FEN string into a Board
    let board = Board::from_fen(fen)
        .map_err(|_| anyhow!("Invalid FEN: {}", fen))?;

    // Step 2: Check for terminal states (checkmate/stalemate) before expensive search
    if board.checkmate() {
        let winner = if board.turn() == tanton::Player::White { "Black" } else { "White" };
        return Ok(("--".to_string(), format!("{} wins by checkmate", winner)));
    }
    if board.stalemate() {
        return Ok(("--".to_string(), "Stalemate".to_string()));
    }
    
    // Step 3: Run engine search (iterative deepening to fixed depth)
    // Using depth 6 for faster response (depth 12 was causing hangs)
    const SEARCH_DEPTH: u16 = 6;
    eprintln!("(depth {})", SEARCH_DEPTH);
    let _ = std::io::stderr().flush();
    let best_move = IterativeSearcher::best_move(board.shallow_clone(), SEARCH_DEPTH);
    
    // Step 4: Extract evaluation score (PSQT after best move; white-positive)
    let mut eval_board = board.shallow_clone();
    eval_board.apply_move(best_move);
    let raw_eval = eval_board.psq().mg();

    // Negate for side-to-move perspective (if Black to move, flip sign)
    let eval_score = if board.turn() == tanton::Player::Black { -raw_eval } else { raw_eval };
    
    // Step 5: Format move + eval string and return
    let uci_str = best_move.stringify(); // e.g., "c2c3"
    let move_str = format_move_readable(&uci_str);
    let eval_str = format_eval(eval_score);

    eprintln!("{:.0}ms", start.elapsed().as_secs_f64() * 1000.0);

    Ok((move_str, eval_str))
}

/// Converts UCI notation to readable format: "c2c3" → "C2 to C3"
fn format_move_readable(uci: &str) -> String {
    if uci.len() >= 4 {
        let from = &uci[0..2].to_uppercase();
        let to = &uci[2..4].to_uppercase();

        // Handle promotion (e.g., "e7e8q" → "E7 to E8 (=Q)")
        if uci.len() == 5 {
            let promo = uci.chars().nth(4).unwrap().to_uppercase().next().unwrap();
            format!("{} to {} (={})", from, to, promo)
        } else {
            format!("{} to {}", from, to)
        }
    } else {
        uci.to_string() // Fallback for unexpected format
    }
}

fn format_eval(centipawns: i32) -> String {
    let pawns = centipawns as f64 / 100.0;
    if pawns >= 0.0 {
        format!("+{:.2}", pawns)
    } else {
        format!("{:.2}", pawns)
    }
}
