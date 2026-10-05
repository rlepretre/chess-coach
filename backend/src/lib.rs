mod pgn;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

type Db = Arc<Mutex<Connection>>;
type ApiError = (StatusCode, Json<Value>);

fn err(status: StatusCode, msg: impl ToString) -> ApiError {
    (status, Json(json!({ "error": msg.to_string() })))
}

pub fn app(db_path: &str) -> Router {
    let conn = Connection::open(db_path).expect("open database");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS games (id INTEGER PRIMARY KEY, pgn TEXT NOT NULL)",
        [],
    )
    .expect("create table");
    Router::new()
        .route("/api/games", post(create_game))
        .route("/api/games/{id}", get(read_game))
        .with_state(Arc::new(Mutex::new(conn)))
}

async fn create_game(State(db): State<Db>, Json(body): Json<Value>) -> Result<(StatusCode, Json<Value>), ApiError> {
    let pgn = body["pgn"].as_str().ok_or_else(|| err(StatusCode::BAD_REQUEST, "Missing pgn"))?;
    pgn::parse(pgn).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    let conn = db.lock().unwrap();
    conn.execute("INSERT INTO games (pgn) VALUES (?1)", [pgn])
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(json!({ "id": conn.last_insert_rowid() }))))
}

async fn read_game(State(db): State<Db>, Path(id): Path<i64>) -> Result<Json<Value>, ApiError> {
    let pgn: String = db
        .lock()
        .unwrap()
        .query_row("SELECT pgn FROM games WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| err(StatusCode::NOT_FOUND, "Game not found"))?;
    let game = pgn::parse(&pgn).map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut value = serde_json::to_value(game).unwrap();
    value["id"] = json!(id);
    Ok(Json(value))
}
