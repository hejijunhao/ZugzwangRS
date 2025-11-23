//! Config module.
//! Manages I/O for board_config.json (bounds, site, thresholds).
//! Uses serde for JSON serialization.
//! Auto-generates defaults or triggers calibration if missing.
//! Supports multi-site configs (e.g., templates/chesscom vs lichess).
//! Future: Cache templates, user-editable fields.

