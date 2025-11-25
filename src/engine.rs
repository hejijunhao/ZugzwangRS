//! Engine module
//! Uses `tanton` pure-Rust chess engine for move calculation (actively maintained fork of Pleco (~2900 ELO))
//! Pipeline: FEN string → Board → Search → (best_move, evaluation)

use anyhow::{anyhow, Result};
use tanton::Board;
use tanton::bots::IterativeSearcher;
use tanton::tools::Searcher;

/// Analyzes a chess position from FEN notation
pub fn analyze_position(fen: &str) -> Result<(String, String)> {
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
    const SEARCH_DEPTH: u16 = 12;
    let mut searcher = IterativeSearcher::new();
    let best_move = searcher.best_move(board.shallow_clone(), SEARCH_DEPTH);
    
    // Step 4: Extract evaluation score (PSQT after best move; white-positive)
    let mut eval_board = board.shallow_clone();
    eval_board.apply_move(best_move);
    let raw_eval = eval_board.psq().mg() as i32;

    // Negate for side-to-move perspective (if Black to move, flip sign)
    let eval_score = if board.turn() == tanton::Player::Black { -raw_eval } else { raw_eval };
    
    // Step 5: Format UCI move + eval string and return
    let move_str = best_move.stringify();

    let eval_str = format_eval(eval_score);

    Ok((move_str, eval_str))
}

fn format_eval(centipawns: i32) -> String {
    let pawns = centipawns as f64 / 100.0;
    if pawns >= 0.0 {
        format!("+{:.2}", pawns)
    } else {
        format!("{:.2}", pawns)
    }
}
}
