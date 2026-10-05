use serde::Serialize;
use shakmaty::{fen::Fen, san::San, Chess, EnPassantMode, Position};
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct Game {
    pub headers: BTreeMap<String, String>,
    pub moves: Vec<String>,
    /// Start position, then the position after each move.
    pub fens: Vec<String>,
}

/// Drop `{...}` / `;...` comments, `(...)` variations and `$n` annotations.
fn clean_movetext(text: &str) -> String {
    let (mut out, mut depth, mut in_brace, mut in_line) = (String::new(), 0, false, false);
    for c in text.chars() {
        match c {
            '\n' if in_line => in_line = false,
            _ if in_line => {}
            '}' if in_brace => in_brace = false,
            _ if in_brace => {}
            '{' => in_brace = true,
            ';' => in_line = true,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            _ => out.push(c),
        }
    }
    out
}

pub fn parse(pgn: &str) -> Result<Game, String> {
    let mut headers = BTreeMap::new();
    let mut movetext = String::new();
    for line in pgn.lines() {
        let line = line.trim();
        match line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            Some(tag) => {
                if let Some((k, v)) = tag.split_once(' ') {
                    headers.insert(k.to_string(), v.trim().trim_matches('"').to_string());
                }
            }
            None => {
                movetext.push_str(line);
                movetext.push('\n');
            }
        }
    }

    let mut pos = Chess::default();
    let mut moves = vec![];
    let mut fens = vec![fen(&pos)];
    for token in clean_movetext(&movetext).split_whitespace() {
        if token.starts_with('$') || matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*") {
            continue;
        }
        // "12." or "12..." or "12.e4" -> keep only the move
        let san = token.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches('.');
        if san.is_empty() {
            continue;
        }
        let mv = san
            .parse::<San>()
            .ok()
            .and_then(|s| s.to_move(&pos).ok())
            .ok_or_else(|| format!("Illegal or unreadable move: {san} (after {} moves)", moves.len()))?;
        moves.push(San::from_move(&pos, mv).to_string());
        pos.play_unchecked(mv);
        fens.push(fen(&pos));
    }
    if moves.is_empty() {
        return Err("No moves found in PGN".into());
    }
    Ok(Game { headers, moves, fens })
}

fn fen(pos: &Chess) -> String {
    Fen::from_position(pos, EnPassantMode::Legal).to_string()
}

