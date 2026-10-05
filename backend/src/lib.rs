mod engine;
mod pgn;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use shakmaty::{fen::Fen, san::San, uci::UciMove, CastlingMode, Chess, Color, Position};
use std::sync::{Arc, Mutex};

type Db = Arc<Mutex<Connection>>;
type ApiError = (StatusCode, Json<Value>);

#[derive(Clone)]
pub struct EngineConfig {
    pub command: String,
    pub depth: u32,
    pub threads: u32,
    pub hash_mb: u32,
}

#[derive(Clone)]
struct AppState {
    db: Db,
    engine: EngineConfig,
}

fn err(status: StatusCode, msg: impl ToString) -> ApiError {
    (status, Json(json!({ "error": msg.to_string() })))
}

pub fn app(db_path: &str, engine: EngineConfig) -> Router {
    let conn = Connection::open(db_path).expect("open database");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS games (id INTEGER PRIMARY KEY, pgn TEXT NOT NULL, analysis_error TEXT);
         CREATE TABLE IF NOT EXISTS positions (
             game_id INTEGER NOT NULL, ply INTEGER NOT NULL, analysis TEXT NOT NULL,
             PRIMARY KEY (game_id, ply));",
    )
    .expect("create tables");
    // Databases made before analysis existed lack this column (CREATE TABLE IF NOT EXISTS does not add it).
    let has_column: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_table_info('games') WHERE name = 'analysis_error'", [], |r| r.get(0))
        .expect("inspect games table");
    if has_column == 0 {
        conn.execute("ALTER TABLE games ADD COLUMN analysis_error TEXT", []).expect("add analysis_error");
    }
    Router::new()
        .route("/api/games", post(create_game))
        .route("/api/games/{id}", get(read_game))
        .with_state(AppState { db: Arc::new(Mutex::new(conn)), engine })
}

async fn create_game(State(st): State<AppState>, Json(body): Json<Value>) -> Result<(StatusCode, Json<Value>), ApiError> {
    let pgn = body["pgn"].as_str().ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing pgn"))?;
    let game = pgn::parse(pgn).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    let id = {
        let conn = st.db.lock().unwrap();
        conn.execute("INSERT INTO games (pgn) VALUES (?1)", [pgn])
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        conn.last_insert_rowid()
    };
    // ponytail: one thread and one engine process per game; a job queue if many games are imported at once.
    // Analysis interrupted by a restart is not resumed.
    std::thread::spawn(move || analyse_game(st, id, game.fens));
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn read_game(State(st): State<AppState>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let conn = st.db.lock().unwrap();
    let (pgn, analysis_error): (String, Option<String>) = conn
        .query_row("SELECT pgn, analysis_error FROM games WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => err(StatusCode::NOT_FOUND, "Game not found"),
            e => err(StatusCode::INTERNAL_SERVER_ERROR, e),
        })?;
    let game = pgn::parse(&pgn).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut analysis = vec![Value::Null; game.fens.len()];
    let mut stmt = conn
        .prepare("SELECT ply, analysis FROM positions WHERE game_id = ?1")
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let rows = stmt
        .query_map([id], |r| Ok((r.get::<_, i64>(0)? as usize, r.get::<_, String>(1)?)))
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    for (ply, json) in rows.flatten() {
        if ply < analysis.len() {
            analysis[ply] = serde_json::from_str(&json).unwrap_or(Value::Null);
        }
    }

    let mut value = serde_json::to_value(&game).unwrap();
    value["id"] = json!(id);
    value["analysis_done"] = json!(analysis.iter().all(|a| !a.is_null()));
    value["analysis_error"] = json!(analysis_error);
    value["analysis"] = json!(analysis);
    Ok(Json(value))
}

fn analyse_game(st: AppState, id: i64, fens: Vec<String>) {
    let fail = |msg: String| {
        let _ = st.db.lock().unwrap().execute("UPDATE games SET analysis_error = ?1 WHERE id = ?2", (msg, id));
    };
    let mut uci = match engine::Uci::start(&st.engine) {
        Ok(u) => u,
        Err(e) => return fail(format!("Cannot start engine '{}': {e}", st.engine.command)),
    };
    for (ply, fen) in fens.iter().enumerate() {
        match analyse_position(&mut uci, fen, st.engine.depth) {
            Ok(a) => {
                let _ = st.db.lock().unwrap().execute(
                    "INSERT OR REPLACE INTO positions (game_id, ply, analysis) VALUES (?1, ?2, ?3)",
                    (id, ply as i64, a.to_string()),
                );
            }
            Err(e) => return fail(format!("Engine failed: {e}")),
        }
    }
}

/// `{"score": {"cp": n} | {"mate": n}, "lines": [{"rank", "score", "pv": [SAN...]}]}`, all from White's point of view.
fn analyse_position(uci: &mut engine::Uci, fen: &str, depth: u32) -> std::io::Result<Value> {
    let pos: Chess = fen
        .parse::<Fen>()
        .ok()
        .and_then(|f| f.into_position(CastlingMode::Standard).ok())
        .ok_or_else(|| std::io::Error::other("bad FEN"))?;

    // The engine has nothing to say about finished games; score them ourselves.
    if pos.legal_moves().is_empty() {
        let cp = match (pos.is_check(), pos.turn()) {
            (true, Color::White) => -10000,
            (true, Color::Black) => 10000,
            _ => 0,
        };
        return Ok(json!({ "score": { "cp": cp }, "lines": [] }));
    }

    let sign = if pos.turn() == Color::White { 1 } else { -1 };
    let lines: Vec<Value> = uci
        .analyse(fen, depth)?
        .into_iter()
        .map(|l| {
            let score = match (l.cp, l.mate) {
                (Some(cp), _) => json!({ "cp": cp * sign }),
                (_, Some(m)) => json!({ "mate": m * sign }),
                _ => Value::Null,
            };
            json!({ "rank": l.rank, "score": score, "pv": pv_to_san(&pos, &l.pv) })
        })
        .collect();
    Ok(json!({ "score": lines.first().map(|l| l["score"].clone()), "lines": lines }))
}

/// Converts engine moves to SAN, stopping at the first one that is not legal.
fn pv_to_san(start: &Chess, pv: &[String]) -> Vec<String> {
    let mut pos = start.clone();
    let mut out = vec![];
    for m in pv {
        let Some(mv) = m.parse::<UciMove>().ok().and_then(|u| u.to_move(&pos).ok()) else { break };
        out.push(San::from_move(&pos, mv).to_string());
        pos.play_unchecked(mv);
    }
    out
}
