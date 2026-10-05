use axum::{body::Body, http::{Request, StatusCode}, Router};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const PGN: &str = "[White \"Alice\"]\n[Black \"Bob\"]\n[Result \"1-0\"]\n\n1. e4 {best by test} e5 2. Nf3 (2. f4 exf4) 2... Nc6 $1 3. Bb5 1-0\n";
const AFTER_E4: &str = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq -";

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn post(pgn: &str) -> Request<Body> {
    Request::post("/api/games")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "pgn": pgn }).to_string()))
        .unwrap()
}

fn get(id: i64) -> Request<Body> {
    Request::get(format!("/api/games/{id}")).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn pasted_pgn_can_be_read_back_with_moves_and_positions() {
    let app = backend::app(":memory:");
    let (status, created) = call(&app, post(PGN)).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, game) = call(&app, get(created["id"].as_i64().unwrap())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(game["headers"]["White"], "Alice");
    assert_eq!(game["moves"], json!(["e4", "e5", "Nf3", "Nc6", "Bb5"]));
    let fens = game["fens"].as_array().unwrap();
    assert_eq!(fens.len(), 6); // start position + one per move
    assert!(fens[1].as_str().unwrap().starts_with(AFTER_E4));
}

#[tokio::test]
async fn invalid_pgn_is_rejected_with_a_message() {
    let app = backend::app(":memory:");
    let (status, body) = call(&app, post("1. e4 e5 2. Ke4")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Ke4"));
}

#[tokio::test]
async fn stored_game_survives_a_restart() {
    let path = std::env::temp_dir().join(format!("chess-coach-test-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path = path.to_str().unwrap();
    let (_, created) = call(&backend::app(path), post(PGN)).await;
    let (status, game) = call(&backend::app(path), get(created["id"].as_i64().unwrap())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(game["moves"].as_array().unwrap().len(), 5);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn unknown_game_is_404() {
    let (status, _) = call(&backend::app(":memory:"), get(999)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
