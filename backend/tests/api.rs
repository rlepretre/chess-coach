use axum::{body::Body, http::{Request, StatusCode}, Router};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use backend::EngineConfig;

fn no_engine() -> EngineConfig {
    EngineConfig { command: "/nonexistent/engine".into(), depth: 1 }
}


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
    let app = backend::app(":memory:", no_engine());
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
    let app = backend::app(":memory:", no_engine());
    let (status, body) = call(&app, post("1. e4 e5 2. Ke4")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Ke4"));
}

#[tokio::test]
async fn stored_game_survives_a_restart() {
    let path = std::env::temp_dir().join(format!("chess-coach-test-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path = path.to_str().unwrap();
    let (_, created) = call(&backend::app(path, no_engine()), post(PGN)).await;
    let (status, game) = call(&backend::app(path, no_engine()), get(created["id"].as_i64().unwrap())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(game["moves"].as_array().unwrap().len(), 5);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn unknown_game_is_404() {
    let (status, _) = call(&backend::app(":memory:", no_engine()), get(999)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// A tiny UCI engine written in bash: it answers every `go` with three lines
// and a score that depends on whose turn it is, so tests are fast and exact.
const FAKE_ENGINE: &str = r#"#!/bin/bash
while read -r line; do
  case "$line" in
    uci) echo "uciok" ;;
    isready) echo "readyok" ;;
    position*) fen="$line" ;;
    go*)
      if [[ "$fen" == *" b "* ]]; then cp=30; pv="e7e5 g1f3"; else cp=50; pv="e2e4 e7e5"; fi
      echo "info depth 4 multipv 1 score cp 1 pv a2a3"
      echo "info depth 5 multipv 1 score cp $cp pv $pv"
      echo "info depth 5 multipv 2 score cp 10 pv d2d4"
      echo "info depth 5 multipv 3 score mate 3 pv g1f3"
      echo "bestmove e2e4" ;;
    quit) exit 0 ;;
  esac
done
"#;

fn fake_engine() -> EngineConfig {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("fake-uci-{}-{:?}.sh", std::process::id(), std::thread::current().id()));
    std::fs::write(&path, FAKE_ENGINE).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    EngineConfig { command: path.to_str().unwrap().into(), depth: 5 }
}

async fn wait_for_analysis(app: &Router, id: i64) -> Value {
    for _ in 0..100 {
        let (_, game) = call(app, get(id)).await;
        if game["analysis_done"] == true {
            return game;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("analysis did not finish");
}

#[tokio::test]
async fn every_position_gets_top_lines_scored_from_whites_point_of_view() {
    let app = backend::app(":memory:", fake_engine());
    let (_, created) = call(&app, post(PGN)).await;
    let game = wait_for_analysis(&app, created["id"].as_i64().unwrap()).await;

    let analysis = game["analysis"].as_array().unwrap();
    assert_eq!(analysis.len(), 6);
    assert!(analysis.iter().all(|a| a["lines"].as_array().unwrap().len() == 3));

    let start = &analysis[0]["lines"];
    assert_eq!(start[0]["score"], json!({"cp": 50}));
    assert_eq!(start[0]["pv"], json!(["e4", "e5"])); // shown in SAN, deepest result only
    assert_eq!(start[2]["score"], json!({"mate": 3}));

    // Black to move: the engine says +30 for Black, so White sees -30.
    assert_eq!(analysis[1]["lines"][0]["score"], json!({"cp": -30}));
    assert_eq!(analysis[1]["lines"][0]["pv"], json!(["e5", "Nf3"]));
}

#[tokio::test]
async fn finished_analysis_is_kept_after_a_restart() {
    let path = std::env::temp_dir().join(format!("chess-coach-analysis-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path = path.to_str().unwrap();
    let app = backend::app(path, fake_engine());
    let (_, created) = call(&app, post(PGN)).await;
    let id = created["id"].as_i64().unwrap();
    wait_for_analysis(&app, id).await;

    // New app with an engine that cannot start: only stored results can answer.
    let (_, game) = call(&backend::app(path, no_engine()), get(id)).await;
    assert_eq!(game["analysis_done"], true);
    assert_eq!(game["analysis"].as_array().unwrap().len(), 6);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn game_stays_readable_when_the_engine_cannot_start() {
    let app = backend::app(":memory:", no_engine());
    let (_, created) = call(&app, post(PGN)).await;
    let id = created["id"].as_i64().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let (status, game) = call(&app, get(id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(game["analysis_done"], false);
    assert!(game["analysis_error"].is_string());
}

#[tokio::test]
async fn checkmate_positions_are_scored_without_asking_the_engine() {
    let app = backend::app(":memory:", fake_engine());
    let (_, created) = call(&app, post("1. f3 e5 2. g4 Qh4#")).await;
    let game = wait_for_analysis(&app, created["id"].as_i64().unwrap()).await;
    let last = game["analysis"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["lines"], json!([]));
    assert_eq!(last["score"], json!({"cp": -10000})); // Black won
}

#[tokio::test]
async fn real_stockfish_finds_a_sane_evaluation() {
    if std::process::Command::new("stockfish").arg("quit").output().is_err() {
        eprintln!("stockfish not installed, skipping");
        return;
    }
    let app = backend::app(":memory:", EngineConfig { command: "stockfish".into(), depth: 8 });
    let (_, created) = call(&app, post("1. e4")).await;
    let game = wait_for_analysis(&app, created["id"].as_i64().unwrap()).await;
    let start = &game["analysis"][0]["lines"];
    assert_eq!(start.as_array().unwrap().len(), 3);
    let cp = start[0]["score"]["cp"].as_i64().unwrap();
    assert!((-100..=100).contains(&cp), "start position should be roughly equal, got {cp}");
    assert!(!start[0]["pv"].as_array().unwrap().is_empty());
}
